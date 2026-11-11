//! `auth.sqlite`: users, households, who can see which household, and sessions.
//!
//! Household *data* lives in each household's own FinTrack database folder;
//! this database only records that the household exists and who may open it.

use anyhow::{Context, bail};
use chrono::Utc;
use fintrack_core::str_enum;
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

str_enum! {
    /// Admins manage users and households and can open every household.
    pub enum Role {
        User => ("user", "User"),
        Admin => ("admin", "Admin"),
    }
}

str_enum! {
    /// What a user may do in one household.
    pub enum Perm {
        View => ("view", "View only"),
        Edit => ("edit", "Can edit"),
    }
}

str_enum! {
    pub enum Theme {
        System => ("system", "Match my device"),
        Light => ("light", "Light"),
        Dark => ("dark", "Dark"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role: Role,
    pub disabled: bool,
    pub must_change_password: bool,
    pub default_household: Option<i64>,
    pub theme: Theme,
    pub created_at: String,
    pub last_login: Option<String>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    /// Display name, falling back to the username.
    pub fn name(&self) -> &str {
        if self.display_name.trim().is_empty() {
            &self.username
        } else {
            &self.display_name
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Household {
    pub id: i64,
    /// URL and folder name; never changes after creation.
    pub slug: String,
    pub name: String,
    pub created_at: String,
}

/// A household together with the current user's permission on it.
#[derive(Clone, Debug, PartialEq)]
pub struct Access {
    pub household: Household,
    pub perm: Perm,
}

/// Fields an admin sets when creating or editing a user.
#[derive(Clone, Debug, Default)]
pub struct UserFields {
    pub username: String,
    pub display_name: String,
    pub role: Option<Role>,
    pub disabled: bool,
}

pub struct AuthDb {
    conn: Mutex<Connection>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS households (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    display_name TEXT NOT NULL DEFAULT '',
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    disabled INTEGER NOT NULL DEFAULT 0,
    must_change_password INTEGER NOT NULL DEFAULT 0,
    default_household INTEGER REFERENCES households(id) ON DELETE SET NULL,
    theme TEXT NOT NULL DEFAULT 'system',
    created_at TEXT NOT NULL,
    last_login TEXT
);
CREATE TABLE IF NOT EXISTS memberships (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    household_id INTEGER NOT NULL REFERENCES households(id) ON DELETE CASCADE,
    perm TEXT NOT NULL,
    PRIMARY KEY (user_id, household_id)
);
CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    persistent INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    last_seen INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS sessions_user ON sessions(user_id);
";

const USER_COLS: &str = "u.id, u.username, u.display_name, u.role, u.disabled, \
    u.must_change_password, u.default_household, u.theme, u.created_at, u.last_login";

fn user_from_row(r: &Row<'_>) -> rusqlite::Result<User> {
    let role: String = r.get(3)?;
    let theme: String = r.get(7)?;
    Ok(User {
        id: r.get(0)?,
        username: r.get(1)?,
        display_name: r.get(2)?,
        role: Role::from_key(&role).unwrap_or(Role::User),
        disabled: r.get(4)?,
        must_change_password: r.get(5)?,
        default_household: r.get(6)?,
        theme: Theme::from_key(&theme).unwrap_or(Theme::System),
        created_at: r.get(8)?,
        last_login: r.get(9)?,
    })
}

fn household_from_row(r: &Row<'_>) -> rusqlite::Result<Household> {
    Ok(Household {
        id: r.get(0)?,
        slug: r.get(1)?,
        name: r.get(2)?,
        created_at: r.get(3)?,
    })
}

fn now_text() -> String {
    Utc::now().format("%Y-%m-%d %H:%M UTC").to_string()
}

/// Usernames: 1–40 of letters, digits, `.`, `-`, `_`, `@`.
pub fn validate_username(name: &str) -> anyhow::Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 40
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'));
    if !ok {
        bail!("Usernames use 1–40 letters, digits, dots, dashes, underscores or @.");
    }
    Ok(())
}

impl AuthDb {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Self::init(conn)
    }

    pub fn in_memory() -> anyhow::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> anyhow::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA).context("creating auth tables")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

mod households;
mod memberships;
mod sessions;
mod users;

#[cfg(test)]
mod tests;
