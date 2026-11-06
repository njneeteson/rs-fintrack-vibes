//! Local SQLite storage.
//!
//! A FinTrack *database* is a folder:
//!
//! ```text
//! MyFinances/
//! ├── fintrack.sqlite   all accounts, transactions, categories, rules, budgets, mortgages
//! └── imports/          a copy of every imported CSV (optional, see settings)
//! ```
//!
//! Each table has its own submodule adding methods to [`Database`].

mod accounts;
mod budgets;
mod categories;
mod formats;
mod imports;
mod mortgages;
mod rules;
mod schema;
mod seed;
mod sql;
mod transactions;

pub use imports::{ImportRecord, ImportSummary};

use anyhow::{Context, bail};
use rusqlite::{Connection, Params, Row};
use std::path::{Path, PathBuf};

pub const DB_FILE: &str = "fintrack.sqlite";
pub const IMPORTS_DIR: &str = "imports";

pub struct Database {
    conn: Connection,
    root: PathBuf,
}

/// Accept either a database folder or the `fintrack.sqlite` file inside it.
pub fn resolve_dir(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|n| n == DB_FILE) {
        path.parent().map(Path::to_path_buf).unwrap_or_default()
    } else {
        path.to_path_buf()
    }
}

pub fn is_database_dir(path: &Path) -> bool {
    resolve_dir(path).join(DB_FILE).is_file()
}

impl Database {
    /// Create a new database folder (it may exist but must not already contain a database).
    pub fn create(dir: &Path) -> anyhow::Result<Self> {
        let dir = resolve_dir(dir);
        if dir.join(DB_FILE).exists() {
            bail!("{} already contains a FinTrack database", dir.display());
        }
        std::fs::create_dir_all(dir.join(IMPORTS_DIR)).with_context(|| format!("creating {}", dir.display()))?;
        let db = Self::connect(&dir)?;
        db.seed_defaults()?;
        Ok(db)
    }

    /// Open an existing database folder.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let dir = resolve_dir(path);
        if !dir.join(DB_FILE).is_file() {
            bail!("no FinTrack database ({DB_FILE}) found in {}", dir.display());
        }
        Self::connect(&dir)
    }

    fn connect(dir: &Path) -> anyhow::Result<Self> {
        let mut conn = Connection::open(dir.join(DB_FILE)).context("opening database")?;
        schema::migrate(&mut conn).context("upgrading database schema")?;
        Ok(Self { conn, root: dir.to_path_buf() })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Display name (the folder name).
    pub fn name(&self) -> String {
        self.root.file_name().map_or_else(|| self.root.display().to_string(), |n| n.to_string_lossy().into_owned())
    }

    pub fn imports_dir(&self) -> PathBuf {
        self.root.join(IMPORTS_DIR)
    }

    /// Copy an imported CSV into `imports/` with a timestamp prefix.
    pub fn archive_csv(&self, src: &Path) -> anyhow::Result<PathBuf> {
        let dir = self.imports_dir();
        std::fs::create_dir_all(&dir)?;
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "import.csv".into());
        let dest = dir.join(format!("{}_{name}", chrono::Local::now().format("%Y%m%d-%H%M%S")));
        std::fs::copy(src, &dest).with_context(|| format!("copying {} to {}", src.display(), dest.display()))?;
        Ok(dest)
    }

    /// Write a consistent snapshot of the database to `dest` (a `.sqlite` file).
    pub fn backup_to(&self, dest: &Path) -> anyhow::Result<()> {
        if dest.exists() {
            std::fs::remove_file(dest).with_context(|| format!("replacing {}", dest.display()))?;
        }
        self.conn.execute("VACUUM INTO ?1", [dest.to_string_lossy()]).context("writing backup")?;
        Ok(())
    }

    fn query_all<T, P: Params>(
        &self,
        sql: &str,
        params: P,
        map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> anyhow::Result<Vec<T>> {
        let mut stmt = self.conn.prepare_cached(sql)?;
        let rows = stmt.query_map(params, map)?;
        Ok(rows.collect::<rusqlite::Result<Vec<T>>>()?)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn temp_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::create(&dir.path().join("Finances")).unwrap();
        (dir, db)
    }

    #[test]
    fn create_open_and_backup() {
        let (dir, db) = temp_db();
        let root = db.root().to_path_buf();
        assert!(is_database_dir(&root));
        assert!(Database::create(&root).is_err());
        assert!(!db.categories().unwrap().is_empty());
        assert!(!db.formats().unwrap().is_empty());
        let backup = dir.path().join("backup.sqlite");
        db.backup_to(&backup).unwrap();
        assert!(backup.is_file());
        drop(db);
        let db = Database::open(&root.join(DB_FILE)).unwrap();
        assert_eq!(db.name(), "Finances");
        assert!(Database::open(dir.path()).is_err());
    }
}
