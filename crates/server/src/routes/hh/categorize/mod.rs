//! Categorize: uncategorized transactions grouped by payee, mapping rules,
//! the rule editor, and re-applying rules.

mod editor;
mod rule_form;
mod rules;
mod unmapped;

use super::data::Data;
use crate::error::AppResult;
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::state::Shared;
use crate::views::components::{heading, tabs};
use axum::Router;
use axum::response::Response;
use axum::routing::{get, post};
use fintrack_core::help::tips;
use maud::{Markup, html};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/categorize", get(unmapped::show))
        .route("/categorize/assign", post(unmapped::assign))
        .route("/categorize/apply", post(apply))
        .route("/categorize/rules", get(rules::show).post(editor::create))
        .route("/categorize/rules/new", get(editor::new))
        .route(
            "/categorize/rules/{id}",
            get(editor::edit).post(editor::update),
        )
        .route("/categorize/rules/{id}/toggle", post(rules::toggle))
        .route("/categorize/rules/{id}/delete", post(rules::delete))
}

const UNMAPPED: &str = "Unmapped";
const RULES: &str = "Rules";

/// Heading, tabs and the shared actions of both Categorize tabs.
fn header(h: &HhCtx, data: &Data, active: &str) -> Markup {
    let unmapped = format!("{UNMAPPED} ({})", data.uncategorized());
    let rules = format!("{RULES} ({})", data.rules.len());
    let current = if active == UNMAPPED {
        unmapped.as_str()
    } else {
        rules.as_str()
    };
    html! {
        (heading("Categorize", tips::RULES, html! {
            @if h.hh.can_edit() {
                a class="btn primary" href=(h.at("/categorize/rules/new")) { "New rule" }
                form method="post" action=(h.at("/categorize/apply")) class="inline" {
                    button type="submit" title=(tips::REAPPLY) { "Re-apply all rules" }
                }
            }
        }))
        (tabs(&[(&unmapped, h.at("/categorize")), (&rules, h.at("/categorize/rules"))], current))
    }
}

async fn apply(h: EditCtx) -> AppResult<Response> {
    let changed = h.hh.run(|db| db.apply_rules()).await?;
    Ok(flash::info(
        &h.st,
        &h.at("/categorize/rules"),
        format!("Rules re-applied: {changed} transactions changed."),
    ))
}
