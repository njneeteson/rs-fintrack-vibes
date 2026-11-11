//! The mapping page: form controls, the raw file, and how rows will be read.

use super::form::{MappingForm, SINGLE, SPLIT};
use crate::views::components::{Num, Text, card, notice, thead, tip};
use crate::views::format::{date, money};
use crate::views::forms::{Live, LiveTarget, checkbox, field, live_form};
use fintrack_core::help::tips;
use fintrack_core::import::parse::{DATE_FORMATS, parse_all};
use fintrack_core::import::reader::RawCsv;
use fintrack_core::model::FormatSpec;
use maud::{Markup, html};

const RAW_PREVIEW_ROWS: usize = 15;
const PARSED_PREVIEW_ROWS: usize = 10;
const HEADER_CHOICES: usize = 25;
const CELL_CHARS: usize = 30;

/// Spreadsheet-style column letters: 0 → A, 25 → Z, 26 → AA.
pub fn column_letter(c: usize) -> String {
    let mut n = c + 1;
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

fn truncate(s: &str, chars: usize) -> String {
    s.chars().take(chars).collect()
}

/// The file's columns, named `"A · Date"` (letter plus header or first data cell).
struct Columns(Vec<String>);

impl Columns {
    fn of(raw: &RawCsv, header_row: Option<usize>) -> Self {
        let sample = raw.rows.get(header_row.map_or(0, |h| h + 1));
        let name = |c: usize| {
            let header = header_row
                .and_then(|h| raw.rows[h].get(c))
                .filter(|s| !s.is_empty());
            let text = header.or_else(|| sample.and_then(|r| r.get(c)));
            format!(
                "{} · {}",
                column_letter(c),
                truncate(text.map_or("", |t| t), 28)
            )
        };
        Self((0..raw.max_columns()).map(name).collect())
    }

    /// A required column picker.
    fn pick(&self, name: &str, selected: &str) -> Markup {
        self.select(name, selected, None)
    }

    /// An optional column picker whose empty choice reads `none`.
    fn optional(&self, name: &str, selected: &str, none: &str) -> Markup {
        self.select(name, selected, Some(none))
    }

    fn select(&self, name: &str, selected: &str, none: Option<&str>) -> Markup {
        html! {
            select name=(name) {
                @if let Some(label) = none {
                    option value="" selected[selected.is_empty()] { (label) }
                }
                @for (i, n) in self.0.iter().enumerate() {
                    option value=(i) selected[selected == i.to_string()] { (n) }
                }
            }
        }
    }
}

fn example_date(fmt: &str) -> String {
    chrono::NaiveDate::from_ymd_opt(2026, 1, 31)
        .map(|d| d.format(fmt).to_string())
        .unwrap_or_default()
}

fn header_select(raw: &RawCsv, f: &MappingForm) -> Markup {
    html! {
        // Column names come from the header row, so changing it reloads the whole form.
        select name="header_row" data-autosubmit {
            option value="" selected[f.header_row.is_empty()] { "No header row" }
            @for (i, row) in raw.rows.iter().take(HEADER_CHOICES).enumerate() {
                option value=(i) selected[f.header_row == i.to_string()] {
                    "Row " (i + 1) ": " (truncate(&row.join(", "), 60))
                }
            }
        }
    }
}

fn date_controls(cols: &Columns, f: &MappingForm) -> Markup {
    html! {
        (field("Date column", tips::DATE_FORMAT, cols.pick("date_column", &f.date_column)))
        (field("Date format", tips::DATE_FORMAT, html! {
            select name="date_format" {
                option value="" selected[f.date_format.is_empty()] { "Automatic" }
                @for d in DATE_FORMATS {
                    option value=(d) selected[f.date_format == *d] {
                        (d) " (e.g. " (example_date(d)) ")"
                    }
                }
            }
        }))
    }
}

fn amount_controls(cols: &Columns, f: &MappingForm) -> Markup {
    let split = f.amount_mode == SPLIT;
    html! {
        (field("Amount", tips::AMOUNT_MODE, html! {
            select name="amount_mode" {
                option value=(SINGLE) selected[!split] { "One signed column" }
                option value=(SPLIT) selected[split] { "Separate debit / credit columns" }
            }
        }))
        (field("Amount column (one column)", "", cols.pick("amount_column", &f.amount_column)))
        (checkbox("negate", !f.negate.is_empty(), html! { "Flip sign " (tip(tips::NEGATE)) }))
        (field("Debit / money out (two columns)", "",
            cols.pick("debit_column", &f.debit_column)))
        (field("Credit / money in (two columns)", "",
            cols.pick("credit_column", &f.credit_column)))
    }
}

fn optional_controls(cols: &Columns, f: &MappingForm) -> Markup {
    html! {
        (field("Direction column", tips::DIRECTION_COL,
            cols.optional("direction_column", &f.direction_column, "None")))
        (field("…money out when it contains", "", html! {
            input type="text" name="direction_marker" value=(f.direction_marker)
                placeholder="debit";
        }))
        (field("Account number column", tips::ACCOUNT_COL,
            cols.optional("account_column", &f.account_column, "None")))
        (field("Balance column", "", cols.optional("balance_column", &f.balance_column, "None")))
        (field("Skip rows where column", tips::SKIP_FILTER,
            cols.optional("skip_column", &f.skip_column, "Don't skip rows")))
        (field("…contains", "", html! {
            input type="text" name="skip_contains" value=(f.skip_contains) placeholder="pending";
        }))
    }
}

fn memo_controls(cols: &Columns, f: &MappingForm) -> Markup {
    html! {
        fieldset class="form-grid" {
            legend { "Memo / details columns " (tip(tips::MEMO_COLS)) }
            @for (i, n) in cols.0.iter().enumerate() {
                label class="check" {
                    input type="checkbox" name="memo" value=(i)
                        checked[f.memo.contains(&i.to_string())];
                    (n)
                }
            }
        }
    }
}

/// Id of the previews that refresh as the mapping form changes.
pub const PREVIEW: &str = "mapping-preview";

/// The whole mapping form. With JavaScript the previews update as you change
/// it; without, Preview reloads the page. Save posts it.
pub fn mapping_form(action: &str, raw: &RawCsv, f: &MappingForm) -> Markup {
    let cols = Columns::of(raw, f.header_row());
    let editing = !f.format_id.is_empty();
    let body = html! {
        input type="hidden" name="edited" value="1";
        input type="hidden" name="format_id" value=(f.format_id);
        div class="form-grid" {
            (field("Header row", tips::HEADER_ROW, header_select(raw, f)))
            (date_controls(&cols, f))
            (field("Description column", "",
                cols.pick("description_column", &f.description_column)))
            (amount_controls(&cols, f))
            (optional_controls(&cols, f))
        }
        (memo_controls(&cols, f))
        div class="toolbar" {
            (field("Format name", "", html! {
                input type="text" name="name" value=(f.name) required;
            }))
            noscript { button type="submit" { "Preview" } }
            button type="submit" class="primary" formmethod="post" name="action" value="save" {
                @if editing { "Save changes & continue" } @else { "Save format & continue" }
            }
            @if editing {
                button type="submit" formmethod="post" name="action" value="save_new" {
                    "Save as new format"
                }
            }
        }
    };
    let target = LiveTarget {
        id: PREVIEW,
        also: &[],
    };
    live_form(action, "stack", target, Live::Preview, body)
}

pub fn raw_preview(raw: &RawCsv, header_row: Option<usize>) -> Markup {
    let ncols = raw.max_columns();
    let cell = |row: &[String], c: usize| truncate(row.get(c).map_or("", |s| s), CELL_CHARS);
    let body = html! {
        div class="table-wrap" {
            table {
                thead { tr { th {} @for c in 0..ncols { th { (column_letter(c)) } } } }
                tbody {
                    @for (i, row) in raw.rows.iter().take(RAW_PREVIEW_ROWS).enumerate() {
                        tr {
                            td class="muted" { (i + 1) }
                            @for c in 0..ncols {
                                @if Some(i) == header_row {
                                    th { (cell(row, c)) }
                                } @else {
                                    td { (cell(row, c)) }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    card("File contents (first rows)", tips::HEADER_ROW, body)
}

/// How the rows will be read with `spec`.
pub fn parsed_preview(raw: &RawCsv, spec: &FormatSpec) -> Markup {
    const TITLE: &str = "How rows will be imported";
    let out = match parse_all(raw, spec) {
        Ok(out) => out,
        Err(e) => return card(TITLE, "", notice("error", html! { (e) })),
    };
    let total = raw.rows.len().saturating_sub(spec.has_header as usize);
    let body = html! {
        p {
            strong { (out.rows.len()) " of about " (total) " rows read" }
            @if !out.errors.is_empty() { ", " (out.errors.len()) " with errors" }
            @if out.skipped > 0 { ", " (out.skipped) " skipped" }
        }
        @if let Some(e) = out.errors.first() {
            (notice("warn", html! { "Line " (e.line) ": " (e.message) }))
        }
        div class="table-wrap" {
            table {
                (thead(&[("Date", Text), ("Description", Text), ("Memo", Text), ("Amount", Num)]))
                tbody {
                    @for r in out.rows.iter().take(PARSED_PREVIEW_ROWS) {
                        tr {
                            td class="date" { (date(r.date)) }
                            td { (r.description) }
                            td class="muted" { (r.memo) }
                            td class="num" { (money(r.amount)) }
                        }
                    }
                }
            }
        }
    };
    card(TITLE, "", body)
}

#[cfg(test)]
mod tests {
    #[test]
    fn letters() {
        assert_eq!(super::column_letter(0), "A");
        assert_eq!(super::column_letter(25), "Z");
        assert_eq!(super::column_letter(26), "AA");
    }
}
