//! Admin pages: user accounts, households, and who can access which household.

mod access;
mod households;
mod users;

use crate::extract::Ctx;
use crate::state::Shared;
use crate::views::components::tabs;
use axum::Router;
use maud::Markup;

pub fn routes() -> Router<Shared> {
    Router::new()
        .merge(users::routes())
        .merge(households::routes())
        .merge(access::routes())
}

const USERS: &str = "Users";
const HOUSEHOLDS: &str = "Households";
const ACCESS: &str = "Access";

fn admin_tabs(cx: &Ctx, active: &str) -> Markup {
    tabs(
        &[
            (USERS, cx.url("/admin/users")),
            (HOUSEHOLDS, cx.url("/admin/households")),
            (ACCESS, cx.url("/admin/access")),
        ],
        active,
    )
}
