//! The in-app guide (topics from `fintrack_core::help`).

use crate::extract::{Ctx, HhCtx};
use crate::views::components::{card, heading, paragraphs};
use crate::views::{Nav, Page, hh_page, page};
use fintrack_core::help::TOPICS;
use maud::{Markup, html};

const INTRO: &str =
    "How FinTrack works. Hover or focus any ? for a short explanation of that control.";

fn guide() -> Markup {
    html! {
        (heading("Help", INTRO, html! {}))
        div class="tiles" {
            @for (i, t) in TOPICS.iter().enumerate() {
                a class="tile" href={ "#topic-" (i) } { (t.title) }
            }
        }
        @for (i, t) in TOPICS.iter().enumerate() {
            div id={ "topic-" (i) } class="help-topic" { (card(t.title, "", paragraphs(t.body))) }
        }
    }
}

pub async fn global(cx: Ctx) -> Page {
    page(&cx, "Help", guide())
}

pub async fn in_household(h: HhCtx) -> Page {
    hh_page(&h, Nav::Dashboard, "Help", guide())
}
