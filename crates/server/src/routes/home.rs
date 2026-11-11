//! Landing: go to the user's household, or let them pick one.

use crate::auth::store::Perm;
use crate::extract::Ctx;
use crate::flash;
use crate::views::components::{card, empty, heading, link_button};
use crate::views::{Page, page};
use axum::response::Response;
use maud::html;

/// `/` → the default household, the only household, or the picker.
pub async fn root(cx: Ctx) -> Response {
    let preferred = cx
        .user
        .default_household
        .and_then(|id| cx.access.iter().find(|a| a.household.id == id));
    let only = (cx.access.len() == 1).then(|| &cx.access[0]);
    match preferred.or(only) {
        Some(a) => flash::redirect(&cx.url(&format!("/h/{}", a.household.slug))),
        None => flash::redirect(&cx.url("/households")),
    }
}

const INTRO: &str = "Each household has its own accounts, transactions, budgets and mortgage.";

pub async fn picker(cx: Ctx) -> Page {
    let body = html! {
        (heading("Households", INTRO, html! {
            @if cx.user.is_admin() {
                (link_button(&cx.url("/admin/households"), "Manage households", false))
            }
        }))
        @if cx.access.is_empty() {
            (card("", "", html! {
                @if cx.user.is_admin() {
                    p { "There are no households yet." }
                    (link_button(&cx.url("/admin/households"), "Create a household", true))
                } @else {
                    (empty("You don't have access to any household yet. Ask an admin to add you."))
                }
            }))
        }
        div class="grid" {
            @for a in &cx.access {
                (card(&a.household.name, "", html! {
                    p class="muted" {
                        @if a.perm == Perm::View { "View only" } @else { "You can view and edit" }
                    }
                    (link_button(&cx.url(&format!("/h/{}", a.household.slug)), "Open", true))
                }))
            }
        }
    };
    page(&cx, "Households", body)
}
