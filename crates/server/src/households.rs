//! Household database folders under `<data>/households/<slug>/`.
//!
//! Each folder is an ordinary FinTrack database (the same layout the desktop
//! app used), so a folder can be copied in or out by hand. Open connections are
//! cached, one per household, behind a mutex: SQLite allows one writer at a
//! time anyway, and households are small.

use crate::auth::store::{AuthDb, Household};
use anyhow::{Context, bail};
use fintrack_core::db::{DB_FILE, Database, is_database_dir};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub type SharedDb = Arc<Mutex<Database>>;

/// Folder holding deleted households, so a mistaken delete can be undone by hand.
pub const TRASH_DIR: &str = ".trash";
/// Where a demo household keeps the bank exports it was built from.
pub const SAMPLE_CSV_DIR: &str = "sample-csvs";

pub struct Households {
    dir: PathBuf,
    open: Mutex<HashMap<String, SharedDb>>,
}

/// A URL- and filesystem-safe slug: `"The Smiths' Home"` → `"the-smiths-home"`.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-')
            && !out.is_empty()
            && (c.is_whitespace() || "-_./".contains(c))
        {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-');
    let out: String = out.chars().take(40).collect();
    let out = out.trim_end_matches('-');
    if out.is_empty() {
        "household".into()
    } else {
        out.to_string()
    }
}

pub fn validate_name(name: &str) -> anyhow::Result<()> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 60 {
        bail!("Household names need 1–60 characters.");
    }
    Ok(())
}

impl Households {
    pub fn new(dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self {
            dir,
            open: Mutex::new(HashMap::new()),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn folder(&self, slug: &str) -> PathBuf {
        self.dir.join(slug)
    }

    /// The (cached) connection for a household.
    pub fn get(&self, slug: &str) -> anyhow::Result<SharedDb> {
        let mut open = self.open.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(db) = open.get(slug) {
            return Ok(db.clone());
        }
        let db = Arc::new(Mutex::new(Database::open(&self.folder(slug))?));
        open.insert(slug.to_string(), db.clone());
        Ok(db)
    }

    fn forget(&self, slug: &str) {
        self.open
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(slug);
    }

    fn unique_slug(&self, auth: &AuthDb, name: &str) -> anyhow::Result<String> {
        let base = slugify(name);
        for n in 1..1000 {
            let slug = if n == 1 {
                base.clone()
            } else {
                format!("{base}-{n}")
            };
            if auth.household_by_slug(&slug)?.is_none() && !self.folder(&slug).exists() {
                return Ok(slug);
            }
        }
        bail!("couldn't find a free folder name for {name:?}")
    }

    /// Run `build` to create the database folder, then register it. Cleans up on failure.
    fn create_with(
        &self,
        auth: &AuthDb,
        name: &str,
        build: impl FnOnce(&Path) -> anyhow::Result<()>,
    ) -> anyhow::Result<Household> {
        validate_name(name)?;
        let slug = self.unique_slug(auth, name)?;
        let folder = self.folder(&slug);
        if let Err(e) = build(&folder) {
            let _ = std::fs::remove_dir_all(&folder);
            return Err(e);
        }
        let id = auth.add_household(&slug, name)?;
        auth.household(id)?.context("household vanished")
    }

    /// A new household with the default categories.
    pub fn create_empty(&self, auth: &AuthDb, name: &str) -> anyhow::Result<Household> {
        self.create_with(auth, name, |folder| Database::create(folder).map(drop))
    }

    /// A household filled with the built-in demo data (and its sample CSVs).
    pub fn create_demo(&self, auth: &AuthDb, name: &str) -> anyhow::Result<Household> {
        self.create_with(auth, name, |folder| {
            fintrack_core::demo::generate_at(folder, &folder.join(SAMPLE_CSV_DIR)).map(drop)
        })
    }

    /// A household from an uploaded `fintrack.sqlite` (e.g. from the desktop app or a backup).
    pub fn create_from_upload(
        &self,
        auth: &AuthDb,
        name: &str,
        bytes: &[u8],
    ) -> anyhow::Result<Household> {
        if !bytes.starts_with(b"SQLite format 3\0") {
            bail!(
                "That file isn't a FinTrack database (expected fintrack.sqlite or a FinTrack \
                    backup)."
            );
        }
        self.create_with(auth, name, |folder| {
            std::fs::create_dir_all(folder)?;
            std::fs::write(folder.join(DB_FILE), bytes)?;
            // Opening migrates it to the current schema; reading proves it's a FinTrack database.
            let db = Database::open(folder).context("That file isn't a FinTrack database")?;
            db.categories()
                .context("That file isn't a FinTrack database")?;
            Ok(())
        })
    }

    /// Unregister a household and move its folder to `.trash/`.
    pub fn delete(&self, auth: &AuthDb, h: &Household) -> anyhow::Result<()> {
        self.forget(&h.slug);
        auth.remove_household(h.id)?;
        let from = self.folder(&h.slug);
        if from.exists() {
            let trash = self.dir.join(TRASH_DIR);
            std::fs::create_dir_all(&trash)?;
            let to = trash.join(format!(
                "{}-{}",
                h.slug,
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ));
            std::fs::rename(&from, &to)
                .with_context(|| format!("moving {} to {}", from.display(), to.display()))?;
        }
        Ok(())
    }

    /// A consistent snapshot of a household's database, as bytes.
    pub fn backup(&self, slug: &str) -> anyhow::Result<Vec<u8>> {
        let db = self.get(slug)?;
        let tmp = self.dir.join(format!(
            ".backup-{}.sqlite",
            crate::auth::session::new_token()
        ));
        let result = (|| {
            db.lock()
                .unwrap_or_else(|e| e.into_inner())
                .backup_to(&tmp)?;
            Ok(std::fs::read(&tmp)?)
        })();
        let _ = std::fs::remove_file(&tmp);
        result
    }

    /// Register database folders that were copied into `households/` by hand.
    /// Returns the newly registered households.
    pub fn adopt_folders(&self, auth: &AuthDb) -> anyhow::Result<Vec<Household>> {
        let mut adopted = Vec::new();
        for entry in std::fs::read_dir(&self.dir)?.flatten() {
            let path = entry.path();
            let Some(folder) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if folder.starts_with('.')
                || !is_database_dir(&path)
                || auth.household_by_slug(&folder)?.is_some()
            {
                continue;
            }
            if slugify(&folder) != folder {
                tracing::warn!(
                    "skipping {}: folder names under households/ must be lowercase letters, \
                        digits and dashes (try {})",
                    path.display(),
                    slugify(&folder)
                );
                continue;
            }
            let id = auth.add_household(&folder, &folder.replace('-', " "))?;
            adopted.extend(auth.household(id)?);
        }
        Ok(adopted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slugify("The Smiths' Home"), "the-smiths-home");
        assert_eq!(slugify("  Cottage  2026 "), "cottage-2026");
        assert_eq!(slugify("!!!"), "household");
        assert_eq!(slugify("Élan"), "lan");
    }

    #[test]
    fn create_delete_backup_and_adopt() {
        let tmp = tempfile::tempdir().unwrap();
        let auth = AuthDb::in_memory().unwrap();
        let hs = Households::new(tmp.path().join("households")).unwrap();
        let a = hs.create_empty(&auth, "Home").unwrap();
        let b = hs.create_empty(&auth, "Home").unwrap();
        assert_eq!((a.slug.as_str(), b.slug.as_str()), ("home", "home-2"));
        let bytes = hs.backup("home").unwrap();
        let c = hs.create_from_upload(&auth, "Restored", &bytes).unwrap();
        assert!(
            !hs.get(&c.slug)
                .unwrap()
                .lock()
                .unwrap()
                .categories()
                .unwrap()
                .is_empty()
        );
        assert!(hs.create_from_upload(&auth, "Bad", b"hello").is_err());
        assert!(!hs.folder("bad").exists());

        hs.delete(&auth, &b).unwrap();
        assert!(auth.household_by_slug("home-2").unwrap().is_none());
        assert_eq!(
            std::fs::read_dir(hs.dir().join(TRASH_DIR)).unwrap().count(),
            1
        );

        Database::create(&hs.folder("copied-in")).unwrap();
        let adopted = hs.adopt_folders(&auth).unwrap();
        assert_eq!(adopted.len(), 1);
        assert_eq!(adopted[0].slug, "copied-in");
        assert!(hs.adopt_folders(&auth).unwrap().is_empty());
    }
}
