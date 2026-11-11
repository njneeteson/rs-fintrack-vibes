//! Admin pages: user accounts, the guest account, households, and who can
//! access which household.

mod access;
mod guest;
mod households;
mod users;

use crate::auth::store::Perm;
use crate::extract::Ctx;
use crate::state::Shared;
use crate::views::components::tabs;
use axum::Router;
use maud::{Markup, html};

pub fn routes() -> Router<Shared> {
    Router::new()
        .merge(users::routes())
        .merge(guest::routes())
        .merge(households::routes())
        .merge(access::routes())
}

const USERS: &str = "Users";
const GUEST: &str = "Guest";
const HOUSEHOLDS: &str = "Households";
const ACCESS: &str = "Access";

fn admin_tabs(cx: &Ctx, active: &str) -> Markup {
    tabs(
        &[
            (USERS, cx.url("/admin/users")),
            (GUEST, cx.url("/admin/guest")),
            (HOUSEHOLDS, cx.url("/admin/households")),
            (ACCESS, cx.url("/admin/access")),
        ],
        active,
    )
}

/// No access / View only / Can edit, for one person and household.
fn perm_select(name: &str, current: Option<Perm>, label: &str) -> Markup {
    html! {
        select name=(name) aria-label=(label) {
            option value="" selected[current.is_none()] { "No access" }
            @for p in Perm::ALL {
                option value=(p.key()) selected[current == Some(*p)] { (p.label()) }
            }
        }
    }
}

/// The permission chosen in a [`perm_select`] (`None` for no access).
fn parse_perm(value: &str) -> Option<Perm> {
    Perm::from_key(value)
}
