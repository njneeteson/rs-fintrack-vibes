//! Pages inside a household: `/h/{hh}/…`.
//!
//! Reading needs membership ([`HhCtx`](crate::extract::HhCtx)); every
//! handler that changes data takes [`EditCtx`](crate::extract::EditCtx).

pub mod accounts;
pub mod analytics;
pub mod budgets;
pub mod categories;
pub mod categorize;
pub mod dashboard;
pub mod data;
pub mod help;
pub mod import;
pub mod mortgage;
pub mod range;
pub mod transactions;

use crate::extract::HhCtx;
use crate::state::Shared;
use axum::Router;
use axum::routing::get;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/", get(dashboard::show))
        .route("/help", get(help::in_household))
        .merge(transactions::routes())
        .merge(import::routes())
        .merge(budgets::routes())
        .merge(categorize::routes())
        .merge(categories::routes())
        .merge(accounts::routes())
        .merge(analytics::routes())
        .merge(mortgage::routes())
}

/// Where to go after a form posted with a `back` field: that URL if it's inside
/// this household, else `fallback` (a path inside the household).
pub fn back_or(h: &HhCtx, back: &str, fallback: &str) -> String {
    let home = h.at("");
    let inside = back == home
        || back.starts_with(&format!("{home}/"))
        || back.starts_with(&format!("{home}?"));
    if inside && !back.contains("://") {
        back.to_string()
    } else {
        h.at(fallback)
    }
}
