//! The access matrix: for every user and household, no access / view only / can edit.

use super::{ACCESS, admin_tabs, parse_perm, perm_select};
use crate::auth::store::{Household, Perm, User};
use crate::error::AppResult;
use crate::extract::AdminCtx;
use crate::flash;
use crate::state::Shared;
use crate::views::components::{card, empty, heading};
use crate::views::{Page, page};
use axum::response::Response;
use axum::routing::get;
use axum::{Form, Router};
use maud::{Markup, html};
use std::collections::HashMap;

const INTRO: &str =
    "Choose which households each person can open, and whether they can change things.";
const PERM_TIP: &str = "View only: dashboards, transactions and reports. \
    Can edit: also import, categorize, budgets and the mortgage.";

pub fn routes() -> Router<Shared> {
    Router::new().route("/admin/access", get(show).post(save))
}

fn field_name(user: i64, household: i64) -> String {
    format!("p_{user}_{household}")
}

/// One user's access to one household: a picker, or a note for admins.
fn cell(u: &User, h: &Household, perms: &HashMap<(i64, i64), Perm>) -> Markup {
    if u.is_admin() {
        return html! { span class="muted" { "Admin: can edit" } };
    }
    let current = perms.get(&(u.id, h.id)).copied();
    perm_select(
        &field_name(u.id, h.id),
        current,
        &format!("{} in {}", u.username, h.name),
    )
}

fn matrix(
    cx: &AdminCtx,
    users: &[User],
    households: &[Household],
    perms: &HashMap<(i64, i64), Perm>,
) -> Markup {
    html! {
        form method="post" action=(cx.url("/admin/access")) {
            div class="table-wrap" {
                table {
                    thead { tr { th { "User" } @for h in households { th { (h.name) } } } }
                    tbody {
                        @for u in users {
                            tr {
                                td {
                                    @if u.guest {
                                        a href=(cx.url("/admin/guest")) { "Guest" }
                                        @if u.disabled { span class="muted" { " (turned off)" } }
                                    } @else {
                                        (u.username)
                                        @if u.disabled { span class="muted" { " (disabled)" } }
                                    }
                                }
                                @for h in households { td { (cell(u, h, perms)) } }
                            }
                        }
                    }
                }
            }
            p { button type="submit" class="primary" { "Save access" } }
        }
    }
}

async fn show(cx: AdminCtx) -> AppResult<Page> {
    let users = cx.st.auth.users()?;
    let households = cx.st.auth.households()?;
    let perms: HashMap<(i64, i64), Perm> = cx
        .st
        .auth
        .memberships()?
        .into_iter()
        .map(|(u, h, p)| ((u, h), p))
        .collect();
    let body = html! {
        (heading("Admin", INTRO, html! {}))
        (admin_tabs(&cx, ACCESS))
        (card("Household access", PERM_TIP, html! {
                @if households.is_empty() {
                    (empty("Create a household first."))
                } @else {
                    (matrix(&cx, &users, &households, &perms))
                }
            }))
    };
    Ok(page(&cx, "Access", body))
}

async fn save(cx: AdminCtx, Form(fields): Form<HashMap<String, String>>) -> AppResult<Response> {
    let users = cx.st.auth.users()?;
    let households = cx.st.auth.households()?;
    for u in users.iter().filter(|u| !u.is_admin()) {
        for h in &households {
            if let Some(value) = fields.get(&field_name(u.id, h.id)) {
                cx.st.auth.set_membership(u.id, h.id, parse_perm(value))?;
            }
        }
    }
    Ok(flash::info(
        &cx.st,
        &cx.url("/admin/access"),
        "Access saved.",
    ))
}
