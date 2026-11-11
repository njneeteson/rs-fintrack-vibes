//! The page shell: document head, top bar, side menu and flash message.

use super::components::notice;
use super::nav::{Nav, household_menu};
use crate::auth::store::{Perm, Theme};
use crate::extract::{Ctx, HhCtx};
use crate::flash::Kind;
use crate::state::Shared;
use axum::response::Html;
use maud::{DOCTYPE, Markup, html};

pub type Page = Html<String>;

fn asset(st: &Shared, name: &str) -> String {
    st.url(&format!("/static/{name}?v={}", crate::assets::version()))
}

/// htmx settings: no `eval` (the CSP forbids it anyway), no inline scripts from
/// responses, no page snapshots in browser storage (finance data stays out of
/// localStorage), and error pages are shown rather than silently ignored.
const HTMX_CONFIG: &str = r#"{"allowEval":false,"allowScriptTags":false,
"includeIndicatorStyles":false,"historyCacheSize":0,"refreshOnHistoryMiss":true,
"responseHandling":[{"code":"204","swap":false},{"code":"...","swap":true}]}"#;

fn document(st: &Shared, theme: Theme, title: &str, content: Markup) -> Page {
    // On <body>, not <html>: htmx swaps the body when navigating, so a theme
    // change (or signing in as someone else) takes effect immediately.
    let data_theme = match theme {
        Theme::System => None,
        t => Some(t.key()),
    };
    let doc = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="color-scheme" content="light dark";
                meta name="htmx-config" content=(HTMX_CONFIG);
                title { (title) " · FinTrack" }
                link rel="icon" href=(asset(st, "favicon.svg")) type="image/svg+xml";
                link rel="stylesheet" href=(asset(st, "app.css"));
                @for script in crate::assets::SCRIPTS {
                    script src=(asset(st, script)) defer {}
                }
            }
            // Links and forms load in place (htmx); the bar shows while loading.
            body data-theme=[data_theme] hx-boost="true" hx-indicator="#loading" {
                div id="loading" class="loading-bar" aria-hidden="true" {}
                (content)
            }
        }
    };
    Html(doc.into_string())
}

fn brand(st: &Shared) -> Markup {
    html! {
        a class="brand" href=(st.url("/")) {
            img src=(asset(st, "favicon.svg")) alt="";
            "FinTrack"
        }
    }
}

fn household_switcher(cx: &Ctx, current: Option<&str>) -> Markup {
    let label = current
        .and_then(|slug| cx.access.iter().find(|a| a.household.slug == slug))
        .map_or("Households", |a| a.household.name.as_str());
    html! {
        details class="menu left" {
            summary { (label) }
            div class="menu-body" {
                @for a in &cx.access {
                    a href=(cx.url(&format!("/h/{}/", a.household.slug))) {
                        (a.household.name)
                        @if a.perm == Perm::View { span class="muted" { " · view only" } }
                    }
                }
                a href=(cx.url("/households")) { "All households…" }
            }
        }
    }
}

fn user_menu(cx: &Ctx) -> Markup {
    html! {
        details class="menu" {
            summary { (cx.user.name()) }
            div class="menu-body" {
                @if cx.user.guest {
                    div class="menu-note" { "Browsing as a guest" }
                } @else {
                    div class="menu-note" {
                        "Signed in as " (cx.user.username) " · " (cx.user.role.label())
                    }
                    a href=(cx.url("/account")) { "My account" }
                }
                @if cx.user.is_admin() {
                    a href=(cx.url("/admin/users")) { "Manage users" }
                    a href=(cx.url("/admin/households")) { "Manage households" }
                }
                a href=(cx.url("/help")) { "Help" }
                form method="post" action=(cx.url("/logout")) {
                    button type="submit" { "Sign out" }
                }
            }
        }
    }
}

fn top_bar(cx: &Ctx, current: Option<&str>) -> Markup {
    html! {
        header class="topbar" {
            (brand(&cx.st))
            @if !cx.access.is_empty() { (household_switcher(cx, current)) }
            div class="spacer" {}
            (user_menu(cx))
        }
    }
}

/// The pending message as a toast (fades after a few seconds unless it's an error).
fn flash(cx: &Ctx) -> Markup {
    html! {
        @if let Some(f) = cx.take_flash() {
            div class="toasts" aria-live="polite" {
                @let kind = if f.kind == Kind::Error { "error" } else { "info" };
                (notice(kind, html! { (f.message) }))
            }
        }
    }
}

/// A page inside a household, with the household side menu.
pub fn hh_page(h: &HhCtx, active: Nav, title: &str, body: Markup) -> Page {
    let content = html! {
        (top_bar(h, Some(&h.hh.info.slug)))
        div class="shell" {
            (household_menu(h, active))
            main id="main" { (flash(h)) (body) }
        }
    };
    document(
        &h.st,
        h.user.theme,
        &format!("{title} · {}", h.hh.info.name),
        content,
    )
}

/// A signed-in page outside any household (account, admin, help).
pub fn page(cx: &Ctx, title: &str, body: Markup) -> Page {
    let content = html! {
        (top_bar(cx, None))
        main id="main" { (flash(cx)) (body) }
    };
    document(&cx.st, cx.user.theme, title, content)
}

/// A page for signed-out visitors (sign-in, first-run setup).
pub fn bare_page(st: &Shared, title: &str, error: Option<&str>, body: Markup) -> Page {
    let content = html! {
        main class="narrow" {
            div class="auth-card" {
                (brand(st))
                @if let Some(e) = error { (notice("error", html! { (e) })) }
                (body)
            }
        }
    };
    document(st, Theme::System, title, content)
}
