//! Shared UI plumbing: navigation targets, status messages, and the context
//! passed to every page.

use super::data::DataCache;
use super::pages::transactions::TxnFilter;
use super::theme::Palette;
use crate::db::Database;
use crate::model::Rule;
use crate::settings::AppSettings;
use std::path::PathBuf;
use std::time::Instant;

str_enum! {
    pub enum View {
        Dashboard => ("dashboard", "Dashboard"),
        Transactions => ("transactions", "Transactions"),
        Import => ("import", "Import CSV"),
        Categorize => ("categorize", "Mapping rules"),
        Categories => ("categories", "Categories"),
        Accounts => ("accounts", "Accounts"),
        Analytics => ("analytics", "Analytics"),
        Budgets => ("budgets", "Budgets"),
        Mortgage => ("mortgage", "Mortgage"),
        Settings => ("settings", "Settings"),
        Help => ("help", "Help"),
    }
}

impl View {
    pub fn icon(self) -> &'static str {
        match self {
            View::Dashboard => "🏠",
            View::Transactions => "📃",
            View::Import => "📥",
            View::Categorize => "🔀",
            View::Categories => "🗂",
            View::Accounts => "🏦",
            View::Analytics => "📊",
            View::Budgets => "🎯",
            View::Mortgage => "🏡",
            View::Settings => "⚙",
            View::Help => "❓",
        }
    }

    /// Settings and Help work without an open database.
    pub fn needs_database(self) -> bool {
        !matches!(self, View::Settings | View::Help)
    }
}

/// Requests that only the application shell can fulfil.
#[derive(Clone, Debug, PartialEq)]
pub enum AppAction {
    Open(PathBuf),
    Create(PathBuf),
    /// Open the built-in demo database.
    Demo,
    Close,
    Backup,
}

#[derive(Clone, Debug)]
pub struct Status {
    pub text: String,
    pub error: bool,
    pub at: Instant,
}

/// Side effects requested by a page during a frame, applied by the shell afterwards.
#[derive(Default)]
pub struct Outbox {
    /// Reload the data cache from the database.
    pub reload: bool,
    pub navigate: Option<View>,
    pub status: Option<Status>,
    pub app: Option<AppAction>,
    /// Open the Transactions page with this filter.
    pub txn_filter: Option<TxnFilter>,
    /// Open the rule editor with this draft.
    pub rule_draft: Option<Rule>,
    /// Paths to queue on the Import page.
    pub import_files: Vec<PathBuf>,
    pub settings_changed: bool,
}

impl Outbox {
    pub fn info(&mut self, text: impl Into<String>) {
        self.status = Some(Status { text: text.into(), error: false, at: Instant::now() });
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.status = Some(Status { text: text.into(), error: true, at: Instant::now() });
    }

    /// Report the outcome of a database mutation; on success the data cache is reloaded.
    pub fn report<T>(&mut self, result: anyhow::Result<T>, success: impl FnOnce(&T) -> String) -> Option<T> {
        match result {
            Ok(v) => {
                let msg = success(&v);
                if !msg.is_empty() {
                    self.info(msg);
                }
                self.reload = true;
                Some(v)
            }
            Err(e) => {
                self.error(format!("{e:#}"));
                None
            }
        }
    }
}

/// Everything a page needs for one frame.
pub struct PageCtx<'a> {
    pub db: &'a Database,
    pub data: &'a DataCache,
    pub settings: &'a AppSettings,
    pub out: &'a mut Outbox,
    pub palette: Palette,
}
