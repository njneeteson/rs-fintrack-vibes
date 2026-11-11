//! The guest account: turn it on or off and choose what guests can see and do.

use super::{GUEST, admin_tabs, parse_perm, perm_select};
use crate::auth::store::{Household, User};
use crate::error::AppResult;
use crate::extract::AdminCtx;
use crate::flash;
use crate::state::Shared;
use crate::views::components::{Text, card, empty, heading, notice, thead};
use crate::views::forms::{checkbox, field, text_input};
use crate::views::{Page, page};
use axum::response::Response;
use axum::routing::get;
use axum::{Form, Router};
use maud::{Markup, html};
use std::collections::HashMap;

pub fn routes() -> Router<Shared> {
    Router::new().route("/admin/guest", get(show).post(save))
}

const INTRO: &str = "Let visitors look around without an account of their own. When guest \
    access is on, the sign-in page has a “Continue as guest” button.";
const ON_NOTE: &str = "Guest access is on. Anyone who can reach FinTrack can open the \
    households below with the access shown.";
const DEFAULT_NAME: &str = "Guest";

fn field_name(household: i64) -> String {
    format!("p_{household}")
}

fn access_table(
    guest: Option<&User>,
    households: &[Household],
    cx: &AdminCtx,
) -> AppResult<Markup> {
    let perms: HashMap<i64, _> = match guest {
        Some(g) => cx
            .st
            .auth
            .memberships()?
            .into_iter()
            .filter(|(u, _, _)| *u == g.id)
            .map(|(_, h, p)| (h, p))
            .collect(),
        None => HashMap::new(),
    };
    Ok(html! {
        @if households.is_empty() {
            (empty("Create a household first."))
        } @else {
            table {
                (thead(&[("Household", Text), ("Guests can", Text)]))
                tbody {
                    @for h in households {
                        tr {
                            td { (h.name) }
                            td {
                                (perm_select(&field_name(h.id), perms.get(&h.id).copied(),
                                    &format!("Guest access to {}", h.name)))
                            }
                        }
                    }
                }
            }
        }
    })
}

async fn show(cx: AdminCtx) -> AppResult<Page> {
    let guest = cx.st.auth.guest()?;
    let households = cx.st.auth.households()?;
    let enabled = guest.as_ref().is_some_and(|g| !g.disabled);
    let name = guest
        .as_ref()
        .map(|g| g.display_name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| DEFAULT_NAME.into());
    let body = html! {
        (heading("Admin", INTRO, html! {}))
        (admin_tabs(&cx, GUEST))
        @if enabled {
            (notice("info", html! { (ON_NOTE) }))
        }
        form method="post" action=(cx.url("/admin/guest")) class="stack" {
            (card("Guest access", "", html! {
                div class="form-stack" {
                    (checkbox("enabled", enabled, html! { "Allow signing in as a guest" }))
                    (field("Name shown to guests", "Shown in the menu while browsing as a guest.",
                        text_input("display_name", &name)))
                    p class="muted" {
                        "Turning guest access off signs out anyone browsing as a guest. "
                        "Guests have no password and can't open My account."
                    }
                }
            }))
            (card("What guests can see and do", "View only: dashboards, transactions and \
                reports. Can edit: also import, categorize, budgets and the mortgage.",
                access_table(guest.as_ref(), &households, &cx)?))
            p { button type="submit" class="primary" { "Save guest settings" } }
        }
    };
    Ok(page(&cx, "Guest access", body))
}

async fn save(cx: AdminCtx, Form(fields): Form<HashMap<String, String>>) -> AppResult<Response> {
    let enabled = fields.contains_key("enabled");
    let name = fields
        .get("display_name")
        .map(|n| n.trim())
        .filter(|n| !n.is_empty())
        .unwrap_or(DEFAULT_NAME);
    let guest = cx.st.auth.set_guest(enabled, name)?;
    for h in cx.st.auth.households()? {
        if let Some(value) = fields.get(&field_name(h.id)) {
            cx.st
                .auth
                .set_membership(guest.id, h.id, parse_perm(value))?;
        }
    }
    tracing::info!(
        "{} turned guest access {}",
        cx.user.username,
        if enabled { "on" } else { "off" }
    );
    let msg = if enabled {
        "Guest access is on."
    } else {
        "Guest access is off."
    };
    Ok(flash::info(&cx.st, &cx.url("/admin/guest"), msg))
}
