//! Which users can open which household, and with what permission.

use super::*;

impl AuthDb {
    /// Households `user` can open. Admins get every household with edit rights.
    pub fn access_for(&self, user: &User) -> anyhow::Result<Vec<Access>> {
        if user.is_admin() {
            return Ok(self
                .households()?
                .into_iter()
                .map(|household| Access {
                    household,
                    perm: Perm::Edit,
                })
                .collect());
        }
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT h.id, h.slug, h.name, h.created_at, m.perm FROM memberships m
             JOIN households h ON h.id = m.household_id
             WHERE m.user_id=?1 ORDER BY h.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([user.id], |r| {
            let perm: String = r.get(4)?;
            Ok(Access {
                household: household_from_row(r)?,
                perm: Perm::from_key(&perm).unwrap_or(Perm::View),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every explicit membership, as `(user_id, household_id, perm)`.
    pub fn memberships(&self) -> anyhow::Result<Vec<(i64, i64, Perm)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT user_id, household_id, perm FROM memberships")?;
        let rows = stmt.query_map([], |r| {
            let perm: String = r.get(2)?;
            Ok((
                r.get(0)?,
                r.get(1)?,
                Perm::from_key(&perm).unwrap_or(Perm::View),
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Grant, change or (with `None`) revoke a user's access to a household.
    pub fn set_membership(
        &self,
        user_id: i64,
        household_id: i64,
        perm: Option<Perm>,
    ) -> anyhow::Result<()> {
        let conn = self.conn();
        match perm {
            Some(p) => conn.execute(
                "INSERT INTO memberships (user_id, household_id, perm) VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id, household_id) DO UPDATE SET perm=excluded.perm",
                params![user_id, household_id, p.key()],
            )?,
            None => conn.execute(
                "DELETE FROM memberships WHERE user_id=?1 AND household_id=?2",
                params![user_id, household_id],
            )?,
        };
        Ok(())
    }
}
