//! Markup for the review step and the "import complete" page.

use super::super::session::{Done, PendingImport};
use crate::extract::EditCtx;
use crate::views::components::{Num, Text, card, heading, link_button, notice, thead, tip};
use crate::views::format::{date, money, plural};
use crate::views::forms::{account_select, enum_select, field, text_input};
use fintrack_core::help::tips;
use fintrack_core::import::dedup::DedupStatus;
use fintrack_core::import::{ImportGroup, detect_account};
use fintrack_core::model::{Account, AccountKind};
use maud::{Markup, html};

const PREVIEW_ROWS: usize = 300;
const MAX_ERRORS: usize = 50;

/// What the review page shows, copied out of the pending import.
pub struct Snapshot {
    pub file_name: String,
    pub format_name: String,
    pub format_id: i64,
    pub groups: Vec<ImportGroup>,
    pub errors: Vec<String>,
    pub skipped: usize,
    pub queued: usize,
}

impl Snapshot {
    /// `None` while the file still needs its columns mapped.
    pub fn of(p: &PendingImport) -> Option<Self> {
        let (format, parsed) = (p.format.as_ref()?, p.parsed.as_ref()?);
        Some(Self {
            file_name: p.raw.file_name.clone(),
            format_name: format.name.clone(),
            format_id: format.id,
            groups: parsed.groups.clone(),
            errors: parsed
                .errors
                .iter()
                .take(MAX_ERRORS)
                .map(|e| format!("Line {}: {}", e.line, e.message))
                .collect(),
            skipped: parsed.skipped,
            queued: p.queue.len(),
        })
    }
}

/// One group of rows (one account number) on the review page.
struct Group<'a> {
    index: usize,
    rows: &'a ImportGroup,
    statuses: &'a [DedupStatus],
}

fn title(g: &ImportGroup) -> String {
    match &g.account_number {
        Some(n) => format!("Rows for account number {n}"),
        None => "Transactions in this file".into(),
    }
}

fn new_account_form(action: &str, g: &ImportGroup) -> Markup {
    html! {
        details {
            summary { "Create a new account for these rows" }
            form method="post" action=(action) class="form-grid" {
                (field("Name", "", html! {
                    input type="text" name="name" placeholder="e.g. Scotia Chequing" required;
                }))
                (field("Bank", "", text_input("institution", "")))
                (field("Type", "", enum_select("kind", AccountKind::Chequing, false)))
                input type="hidden" name="account_number"
                    value=(g.account_number.clone().unwrap_or_default());
                div { button type="submit" { "Create account" } }
            }
        }
    }
}

fn status(s: Option<&DedupStatus>) -> Markup {
    html! {
        @match s {
            Some(DedupStatus::Duplicate) => span class="muted" { "duplicate" },
            Some(DedupStatus::New { .. }) => "new",
            None => span class="muted" { "—" },
        }
    }
}

fn preview_rows(g: &Group<'_>) -> Markup {
    html! {
        details {
            summary { "Preview rows" }
            div class="table-wrap" {
                table {
                    (thead(&[
                        ("Date", Text), ("Description", Text), ("Amount", Num), ("Status", Text),
                    ]))
                    tbody {
                        @for (k, r) in g.rows.rows.iter().enumerate().take(PREVIEW_ROWS) {
                            tr {
                                td class="date" { (date(r.date)) }
                                td { (r.description) }
                                td class="num" { (money(r.amount)) }
                                td { (status(g.statuses.get(k))) }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn group_card(h: &EditCtx, id: &str, g: &Group<'_>, accounts: &[Account], s: &Snapshot) -> Markup {
    let rows = g.rows;
    let base = format!("/import/{id}/groups/{}", g.index);
    let number = rows.account_number.as_deref();
    let detected = detect_account(accounts, number, &s.file_name, Some(s.format_id));
    let dups = g.statuses.iter().filter(|s| s.is_duplicate()).count();
    let choose = Some("— choose account —");
    let body = html! {
        @if let Some((a, b)) = rows.date_range() {
            p { (plural(rows.rows.len(), "row", "rows")) " from " (date(a)) " to " (date(b)) }
        }
        form method="post" action=(h.at(&format!("{base}/account"))) class="toolbar" {
            (field("Import into account", "",
                account_select("account", accounts, rows.account_id, choose, true)))
            noscript { button type="submit" { "Set" } }
            @if rows.account_id.is_some() && rows.account_id == detected {
                span class="muted" { "(detected automatically)" }
            }
        }
        @if rows.account_id.is_none() {
            (new_account_form(&h.at(&format!("{base}/new-account")), rows))
        }
        @if rows.account_id.is_some() {
            p {
                strong { (g.statuses.len() - dups) " new" }
                " · " (dups) " already imported (will be skipped) "
                (tip(tips::DUPLICATES))
            }
        }
        (preview_rows(g))
    };
    card(&title(rows), tips::ACCOUNT_DETECT, body)
}

fn commit_label(new_rows: usize) -> String {
    if new_rows == 0 {
        "Nothing new — finish".into()
    } else {
        format!("Import {new_rows} new transactions")
    }
}

pub fn review_page(
    h: &EditCtx,
    id: &str,
    s: &Snapshot,
    accounts: &[Account],
    statuses: &[Vec<DedupStatus>],
) -> Markup {
    let assigned = |g: &&ImportGroup| g.account_id.is_some();
    let new_rows: usize = s
        .groups
        .iter()
        .zip(statuses)
        .filter(|(g, _)| assigned(g))
        .map(|(_, st)| st.iter().filter(|x| !x.is_duplicate()).count())
        .sum();
    let unassigned: usize = s
        .groups
        .iter()
        .filter(|g| !assigned(g))
        .map(|g| g.rows.len())
        .sum();
    let nothing_assigned = !s.groups.iter().any(|g| assigned(&g));
    let at = |step: &str| h.at(&format!("/import/{id}{step}"));
    html! {
        (heading(&format!("Import {}", s.file_name), &format!("Detected format: {}", s.format_name),
            link_button(&at("/mapping"), "Edit mapping", false)))
        @if !s.errors.is_empty() || s.skipped > 0 {
            details class="notice warn" {
                summary {
                    (s.errors.len()) " rows couldn't be read, " (s.skipped) " skipped by the filter"
                }
                ul { @for e in &s.errors { li { (e) } } }
            }
        }
        @for (index, rows) in s.groups.iter().enumerate() {
            (group_card(h, id, &Group { index, rows, statuses: &statuses[index] }, accounts, s))
        }
        @if unassigned > 0 {
            (notice("warn", html! { (unassigned) " rows have no account and will be skipped." }))
        }
        div class="toolbar" {
            form method="post" action=(at("/commit")) class="inline" {
                button type="submit" class="primary" disabled[nothing_assigned] {
                    (commit_label(new_rows))
                }
            }
            form method="post" action=(at("/cancel")) class="inline" {
                button type="submit" { "Cancel" }
            }
            @if s.queued > 0 {
                span class="muted" { (plural(s.queued, "more file", "more files")) " waiting" }
            }
        }
    }
}

pub struct Finished<'a> {
    pub file: &'a str,
    pub done: &'a Done,
    pub queued: usize,
    pub unmapped: usize,
}

pub fn done_page(h: &EditCtx, id: &str, f: &Finished<'_>) -> Markup {
    let d = f.done;
    let categorize = format!("Categorize {} unmapped", f.unmapped);
    html! {
        (heading("Import complete", "", html! {}))
        (card("", "", html! {
            p {
                strong {
                    "✔ Imported " (d.summary.inserted) " new transactions from " (f.file) "."
                }
            }
            p {
                (d.summary.duplicates) " duplicates skipped (already in the database). "
                (d.categorized) " transactions categorized by your rules."
            }
            @if d.unassigned_rows > 0 {
                p { (d.unassigned_rows) " rows were skipped because no account was chosen." }
            }
            div class="actions" {
                @if f.queued > 0 {
                    form method="post" action=(h.at(&format!("/import/{id}/next"))) class="inline" {
                        button type="submit" class="primary" {
                            "Next file (" (f.queued) " waiting)"
                        }
                    }
                }
                @if f.unmapped > 0 {
                    (link_button(&h.at("/categorize"), &categorize, f.queued == 0))
                }
                (link_button(&h.at("/transactions"), "View transactions", false))
                (link_button(&h.at("/import"), "Import another file", false))
            }
        }))
    }
}
