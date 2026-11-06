//! FinTrack — an offline household finance tracker.
//!
//! The crate is split into a UI-free core (everything except [`ui`]) and the
//! egui desktop front-end. The core is fully unit-tested and has no knowledge of
//! the UI; the UI only talks to the core through [`db::Database`] and the pure
//! functions in [`import`], [`categorize`], [`analytics`] and [`mortgage`].
//!
//! The application never opens a network connection: there is no networking
//! crate in the dependency tree and all data lives in a local directory.

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
pub mod settings;
pub mod ui;
