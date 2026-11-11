//! The shared guest account.
//!
//! One account, created the first time an admin turns guest access on. It has
//! no usable password (signing in as guest is a button on the sign-in page),
//! is "disabled" while guest access is off, and sees exactly the households an
//! admin grants it, like any other user.

use super::*;

const GUEST_USERNAME: &str = "guest";
/// Not a valid password hash, so password sign-in can never succeed.
const NO_PASSWORD: &str = "!guest";

impl AuthDb {
    /// The guest account, if guest access has ever been set up.
    pub fn guest(&self) -> anyhow::Result<Option<User>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {USER_COLS} FROM users u WHERE u.guest = 1"),
                [],
                user_from_row,
            )
            .optional()?)
    }

    /// The guest account if guests may currently sign in.
    pub fn enabled_guest(&self) -> anyhow::Result<Option<User>> {
        Ok(self.guest()?.filter(|g| !g.disabled))
    }

    /// A free username for the guest account ("guest", or "guest-2"… if a
    /// real person already uses it).
    fn free_guest_username(&self) -> anyhow::Result<String> {
        let conn = self.conn();
        for n in 1..100 {
            let name = if n == 1 {
                GUEST_USERNAME.to_string()
            } else {
                format!("{GUEST_USERNAME}-{n}")
            };
            let taken: bool = conn.query_row(
                "SELECT COUNT(*) > 0 FROM users WHERE username = ?1 COLLATE NOCASE",
                [&name],
                |r| r.get(0),
            )?;
            if !taken {
                return Ok(name);
            }
        }
        bail!("no free username for the guest account")
    }

    /// Turn guest access on or off and set the name guests see in the menu.
    /// Creates the account the first time; turning it off signs guests out.
    pub fn set_guest(&self, enabled: bool, display_name: &str) -> anyhow::Result<User> {
        let id = match self.guest()? {
            Some(g) => g.id,
            None => {
                let username = self.free_guest_username()?;
                let conn = self.conn();
                conn.execute(
                    "INSERT INTO users
                         (username, display_name, password_hash, role, guest, created_at)
                     VALUES (?1, '', ?2, 'user', 1, ?3)",
                    params![username, NO_PASSWORD, now_text()],
                )?;
                conn.last_insert_rowid()
            }
        };
        let conn = self.conn();
        conn.execute(
            "UPDATE users SET display_name = ?1, disabled = ?2 WHERE id = ?3",
            params![display_name.trim(), !enabled, id],
        )?;
        if !enabled {
            conn.execute("DELETE FROM sessions WHERE user_id = ?1", [id])?;
        }
        drop(conn);
        self.user(id)?.context("guest account vanished")
    }
}
