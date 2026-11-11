//! FinTrack core — everything that isn't the web server.
//!
//! CSV import and duplicate detection ([`import`]), rule-based categorization
//! ([`categorize`]), reports ([`analytics`]), mortgage projections
//! ([`mortgage`]) and SQLite storage ([`db`]). The crate knows nothing about
//! HTTP, users or households; `fintrack-server` adds those on top.
//!
//! Nothing here opens a network connection, and each household's data lives
//! in its own database folder.

#[macro_use]
pub mod enum_macro;

pub mod analytics;
pub mod categorize;
pub mod db;
pub mod demo;
pub mod help;
pub mod import;
pub mod model;
pub mod mortgage;
