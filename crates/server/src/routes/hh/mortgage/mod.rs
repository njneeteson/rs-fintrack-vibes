//! Mortgage tracker: details, prepayments, renewals and a payoff projection.

mod form;
mod projection;

use crate::error::{AppError, AppResult};
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::state::Shared;
use crate::views::components::{empty, heading};
use crate::views::forms::action_button;
use crate::views::{Nav, Page, hh_page};
use axum::Router;
use axum::extract::{Path, Query};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use fintrack_core::model::{Mortgage, Prepayment, RateChange};
use maud::{Markup, html};
use serde::Deserialize;

const INTRO: &str = "Track your mortgage and see how prepayments and renewals change the \
    payoff date and the interest you pay.";

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/mortgage", get(index).post(add))
        .route("/mortgage/{id}", get(show).post(form::save_details))
        .route("/mortgage/{id}/delete", post(delete))
        .route("/mortgage/{id}/prepayments", post(form::add_prepayment))
        .route(
            "/mortgage/{id}/prepayments/{item}/toggle",
            post(form::toggle_prepayment),
        )
        .route(
            "/mortgage/{id}/prepayments/{item}/delete",
            post(form::delete_prepayment),
        )
        .route("/mortgage/{id}/rates", post(form::add_rate))
        .route(
            "/mortgage/{id}/rates/{item}/delete",
            post(form::delete_rate),
        )
}

/// A mortgage with its prepayments and rate changes.
pub struct Loaded {
    pub all: Vec<Mortgage>,
    pub m: Mortgage,
    pub prepayments: Vec<Prepayment>,
    pub rates: Vec<RateChange>,
}

async fn load(h: &HhCtx, id: i64) -> AppResult<Loaded> {
    let loaded =
        h.hh.run(move |db| {
            let all = db.mortgages()?;
            let Some(m) = all.iter().find(|m| m.id == id).cloned() else {
                return Ok(None);
            };
            Ok(Some(Loaded {
                prepayments: db.prepayments(id)?,
                rates: db.rate_changes(id)?,
                all,
                m,
            }))
        })
        .await?;
    loaded.ok_or(AppError::NotFound)
}

fn picker(h: &HhCtx, all: &[Mortgage], current: i64) -> Markup {
    html! {
        @if all.len() > 1 {
            nav class="actions" aria-label="Mortgages" {
                @for m in all {
                    @let href = h.at(&format!("/mortgage/{}", m.id));
                    a class={ "btn" @if m.id == current { " primary" } } href=(href) {
                        (m.name)
                    }
                }
            }
        }
    }
}

async fn index(h: HhCtx) -> AppResult<Response> {
    let all = h.hh.run(|db| db.mortgages()).await?;
    if let Some(m) = all.first() {
        return Ok(flash::redirect(&h.at(&format!("/mortgage/{}", m.id))));
    }
    let body = html! {
        (heading("Mortgage", INTRO, html! {
            @if h.hh.can_edit() {
                (action_button(&h.at("/mortgage"), "Add mortgage", "primary", None))
            }
        }))
        (empty("Add a mortgage to track its progress and see how prepayments change the \
            payoff date and interest."))
    };
    Ok(hh_page(&h, Nav::Mortgage, "Mortgage", body).into_response())
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ProjectionQuery {
    /// What-if extra amount added to every payment (not saved).
    pub extra: String,
    /// `"all"` for every payment instead of yearly totals.
    pub view: String,
}

async fn show(
    h: HhCtx,
    Path((_, id)): Path<(String, i64)>,
    Query(q): Query<ProjectionQuery>,
) -> AppResult<Page> {
    let l = load(&h, id).await?;
    let body = html! {
        (heading("Mortgage", INTRO, html! {
            @if h.hh.can_edit() {
                (action_button(&h.at("/mortgage"), "Add another mortgage", "", None))
                (action_button(&h.at(&format!("/mortgage/{id}/delete")), "Delete", "danger",
                    Some("Delete this mortgage and its prepayments?")))
            }
        }))
        (picker(&h, &l.all, id))
        div class="grid" {
            div class="stack" { (form::inputs(&h, &l, &q)) }
            div class="stack" { (projection::view(&h, &l, &q)) }
        }
    };
    Ok(hh_page(&h, Nav::Mortgage, &l.m.name, body))
}

async fn add(h: EditCtx) -> AppResult<Response> {
    let id =
        h.hh.run(|db| db.save_mortgage(&Mortgage::default()))
            .await?;
    Ok(flash::info(
        &h.st,
        &h.at(&format!("/mortgage/{id}")),
        "Mortgage added. Fill in its details.",
    ))
}

async fn delete(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    h.hh.run(move |db| db.delete_mortgage(id)).await?;
    Ok(flash::info(&h.st, &h.at("/mortgage"), "Mortgage deleted."))
}
