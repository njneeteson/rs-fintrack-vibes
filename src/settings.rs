//! Application settings, stored as JSON in the user's local config directory
//! (on Windows: `%APPDATA%\FinTrack\config\settings.json`).

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

str_enum! {
    pub enum Theme {
        System => ("system", "Follow system"),
        Light => ("light", "Light"),
        Dark => ("dark", "Dark"),
    }
}

const MAX_RECENT: usize = 8;

/// Environment variable that overrides the settings file location.
pub const SETTINGS_ENV: &str = "FINTRACK_SETTINGS";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Database folder opened automatically on startup.
    pub default_database: Option<PathBuf>,
    pub open_default_on_startup: bool,
    /// Recently opened database folders, most recent first.
    pub recent_databases: Vec<PathBuf>,
    pub theme: Theme,
    /// Copy every imported CSV into the database's `imports/` folder.
    pub archive_imports: bool,
    /// Apply categorization rules to newly imported transactions.
    pub auto_categorize_on_import: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_database: None,
            open_default_on_startup: true,
            recent_databases: Vec::new(),
            theme: Theme::System,
            archive_imports: true,
            auto_categorize_on_import: true,
        }
    }
}

impl AppSettings {
    /// Location of the settings file. The `FINTRACK_SETTINGS` environment
    /// variable overrides it (useful for portable installs and testing).
    pub fn default_path() -> PathBuf {
        if let Some(p) = std::env::var_os(SETTINGS_ENV).filter(|p| !p.is_empty()) {
            return PathBuf::from(p);
        }
        directories::ProjectDirs::from("", "", "FinTrack")
            .map(|d| d.config_dir().join("settings.json"))
            .unwrap_or_else(|| PathBuf::from("fintrack-settings.json"))
    }

    /// Load settings, falling back to defaults if the file is missing or unreadable.
    pub fn load_from(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            // Tolerate a UTF-8 byte-order mark (added by some Windows editors).
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    pub fn save_to(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?).with_context(|| format!("writing {}", path.display()))
    }

    /// Move `path` to the front of the recent list.
    pub fn remember(&mut self, path: &Path) {
        self.recent_databases.retain(|p| p != path);
        self.recent_databases.insert(0, path.to_path_buf());
        self.recent_databases.truncate(MAX_RECENT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_recent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        assert_eq!(AppSettings::load_from(&path), AppSettings::default());
        let mut s = AppSettings::default();
        s.remember(Path::new("a"));
        s.remember(Path::new("b"));
        s.remember(Path::new("a"));
        assert_eq!(s.recent_databases, vec![PathBuf::from("a"), PathBuf::from("b")]);
        s.default_database = Some("a".into());
        s.save_to(&path).unwrap();
        assert_eq!(AppSettings::load_from(&path), s);
    }
}
