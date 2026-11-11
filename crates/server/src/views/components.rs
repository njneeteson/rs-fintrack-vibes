//! Small reusable pieces of page markup.

use maud::{Markup, PreEscaped, html};

/// A `?` badge that shows `text` on hover or focus.
pub fn tip(text: &str) -> Markup {
    html! {
        @if !text.is_empty() {
            span class="tip" tabindex="0" role="note" aria-label=(text) data-tip=(text) { "?" }
        }
    }
}

/// Page heading with an optional one-line explanation and actions on the right.
pub fn heading(title: &str, intro: &str, actions: Markup) -> Markup {
    html! {
        div class="page-head" {
            div {
                h1 { (title) }
                @if !intro.is_empty() { p class="intro" { (intro) } }
            }
            div class="actions" { (actions) }
        }
    }
}

/// A titled panel. `hint` becomes a tooltip beside the title.
pub fn card(title: &str, hint: &str, body: Markup) -> Markup {
    html! {
        section class="card" {
            @if !title.is_empty() {
                h2 { (title) " " (tip(hint)) }
            }
            (body)
        }
    }
}

/// A headline number with a label and a secondary line.
pub fn stat_tile(label: &str, value: Markup, sub: &str, hint: &str) -> Markup {
    html! {
        div class="tile" {
            div class="tile-label" { (label) " " (tip(hint)) }
            div class="tile-value" { (value) }
            @if !sub.is_empty() { div class="tile-sub" { (sub) } }
        }
    }
}

pub fn empty(message: &str) -> Markup {
    html! { p class="empty" { (message) } }
}

/// A status note that never relies on colour alone: icon + text.
pub fn notice(kind: &str, message: Markup) -> Markup {
    let icon = match kind {
        "error" => "⛔",
        "warn" => "⚠",
        _ => "ℹ",
    };
    html! {
        div class={ "notice " (kind) } role="status" {
            span aria-hidden="true" { (icon) }
            " " (message)
        }
    }
}

/// A link styled as a button.
pub fn link_button(href: &str, label: &str, primary: bool) -> Markup {
    html! { a class={ "btn" @if primary { " primary" } } href=(href) { (label) } }
}

/// A progress bar with its label beside it (status shown as text too).
pub fn meter(fraction: f64, status_class: &str, label: &str) -> Markup {
    let width = (fraction.clamp(0.0, 1.0) * 100.0).round();
    html! {
        div class="meter-row" {
            div class={ "meter " (status_class) } role="img" aria-label=(label) {
                span style={ "width:" (width) "%" } {}
            }
            span class="meter-label" { (label) }
        }
    }
}

/// Column alignment for [`thead`]: text (left) or numbers (right).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Text,
    Num,
}

pub use Align::{Num, Text};

/// A table header row.
pub fn thead(cols: &[(&str, Align)]) -> Markup {
    html! {
        thead {
            tr {
                @for (label, align) in cols {
                    th class=[(*align == Num).then_some("num")] { (label) }
                }
            }
        }
    }
}

/// A row of tabs, as `(label, href)`; the one equal to `active` is highlighted.
pub fn tabs(items: &[(&str, String)], active: &str) -> Markup {
    html! {
        nav class="tabs" {
            @for (label, href) in items {
                a href=(href) aria-current=[(*label == active).then_some("page")] { (label) }
            }
        }
    }
}

/// Previous / next links for a paged list.
pub fn pager(page: usize, pages: usize, href_for: impl Fn(usize) -> String) -> Markup {
    html! {
        @if pages > 1 {
            nav class="pager" aria-label="Pages" {
                @if page > 1 { a class="btn" href=(href_for(page - 1)) { "← Previous" } }
                span { "Page " (page) " of " (pages) }
                @if page < pages { a class="btn" href=(href_for(page + 1)) { "Next →" } }
            }
        }
    }
}

/// Paragraph text from a help body, which separates paragraphs with blank lines.
pub fn paragraphs(text: &str) -> Markup {
    html! {
        @for p in text.split("\n\n") {
            p { (PreEscaped(escape_with_breaks(p))) }
        }
    }
}

fn escape_with_breaks(text: &str) -> String {
    maud::html! { (text) }.into_string().replace('\n', "<br>")
}
