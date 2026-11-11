//! The transaction table: one row per transaction with an inline category picker.

use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::views::components::tip;
use crate::views::format::{date, money};
use crate::views::forms::category_select;
use fintrack_core::categorize::suggest_pattern;
use fintrack_core::help::tips;
use fintrack_core::model::Transaction;
use maud::{Markup, html};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// Id of the form the row checkboxes belong to (the bulk-action bar).
pub const BULK_FORM: &str = "bulk";
const RULE_TITLE: &str = "Create a mapping rule from this transaction";

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Where the automatic category came from, for the tooltip.
fn origin(t: &Transaction, data: &Data) -> String {
    let rule = t
        .rule_id
        .and_then(|id| data.rules.iter().find(|r| r.id == id));
    match (t.manual_category_id, rule) {
        (Some(_), _) => {
            "✋ Manual override. Choose “Automatic” to go back to the rule result.".into()
        }
        (None, Some(r)) => format!("From rule: {}", r.describe()),
        (None, None) => "No rule matches. Pick a category, or create a rule.".into(),
    }
}

fn category_cell(h: &HhCtx, t: &Transaction, data: &Data, back: &str) -> Markup {
    let auto = match t.auto_category_id {
        Some(c) => format!("Automatic ({})", data.cats.full_name(c)),
        None => "Automatic (uncategorized)".into(),
    };
    let action = h.at(&format!("/transactions/{}/category", t.id));
    html! {
        @if h.hh.can_edit() {
            // With htmx the server answers with just this row, so the page stays put.
            form method="post" action=(action) class="inline"
                hx-post=(action) hx-target="closest tr" hx-swap="outerHTML" {
                input type="hidden" name="back" value={ (back) "#t" (t.id) };
                (category_select("category", &data.cats, t.manual_category_id, Some(&auto), true))
                noscript { button type="submit" class="small" { "Set" } }
            }
        } @else {
            @match t.category_id() {
                Some(c) => (data.cats.full_name(c)),
                None => span class="muted" { "Uncategorized" },
            }
        }
        " " (tip(&origin(t, data)))
        @if t.is_manual() { span class="muted" title="Manual override" { " ✋" } }
    }
}

pub fn row(h: &HhCtx, t: &Transaction, data: &Data, back: &str) -> Markup {
    let details = h.at(&format!("/transactions/{}?back={}", t.id, enc(back)));
    let rule_link = format!(
        "/categorize/rules/new?pattern={}&category={}",
        enc(&suggest_pattern(&t.description)),
        t.category_id().map(|c| c.to_string()).unwrap_or_default()
    );
    html! {
        tr id={ "t" (t.id) } {
            @if h.hh.can_edit() {
                td {
                    input type="checkbox" name="ids" value=(t.id) form=(BULK_FORM)
                        aria-label="Select";
                }
            }
            td class="date" { (date(t.date)) }
            td class="nowrap" { (data.account_name(t.account_id)) }
            td {
                (t.description)
                @if !t.memo.is_empty() { span class="sub" { (t.memo) } }
                @if !t.note.is_empty() { span class="sub" { "📝 " (t.note) } }
            }
            td class="num" { (money(t.amount)) }
            td { (category_cell(h, t, data, back)) }
            td class="nowrap" {
                a class="btn small" href=(details) { "Details" }
                @if h.hh.can_edit() {
                    " "
                    a class="btn small" href=(h.at(&rule_link)) title=(RULE_TITLE) { "+ Rule" }
                }
            }
        }
    }
}

pub fn table(h: &HhCtx, rows: &[&Transaction], data: &Data, back: &str) -> Markup {
    html! {
        div class="table-wrap" {
            table {
                thead {
                    tr {
                        @if h.hh.can_edit() {
                            th {
                                input type="checkbox" data-select-all="ids"
                                    aria-label="Select all shown";
                            }
                        }
                        th { "Date" }
                        th { "Account" }
                        th { "Description" }
                        th class="num" { "Amount" }
                        th { "Category " (tip(tips::OVERRIDE)) }
                        th {}
                    }
                }
                tbody { @for t in rows { (row(h, t, data, back)) } }
            }
        }
    }
}
