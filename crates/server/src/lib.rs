//! FinTrack server: the household finance tracker as a web app for a home network.
//!
//! * [`auth`]: local accounts, sessions, sign-in throttling, cross-site protection.
//! * [`households`]: one FinTrack database folder per household.
//! * [`extract`]: who may see and change what, checked on every request.
//! * [`routes`]: the pages; [`views`]: shared HTML building blocks.
//!
//! All finance logic (import, categorization, analytics, mortgage) comes from
//! `fintrack-core`.

pub mod assets;
pub mod auth;
pub mod cli;
pub mod config;
pub mod error;
pub mod extract;
pub mod flash;
pub mod form;
pub mod households;
pub mod htmx;
pub mod routes;
pub mod state;
pub mod views;

pub use config::Config;
pub use routes::router;
pub use state::{AppState, Shared};
