//! Column mapper for CSV layouts FinTrack doesn't know yet (or editing an existing format).

use super::Loaded;
use crate::help::tips;
use crate::import::detect::{find_header_row, guess_spec, header_signature};
use crate::import::parse::{DATE_FORMATS, parse_all, parse_date};
use crate::import::reader::RawCsv;
use crate::model::{AmountColumns, CsvFormat, DirectionColumn, FormatSpec, SkipFilter};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, help, label_with_help, money_text};
use egui::{ComboBox, RichText, TextEdit, Ui};

const PREVIEW_ROWS: usize = 200;

pub enum MappingOutcome {
    Continue,
    Cancel,
    Saved(Box<(Loaded, CsvFormat)>),
}

pub struct MappingState {
    loaded: Option<Loaded>,
    format_id: i64,
    builtin: bool,
    name: String,
    spec: FormatSpec,
    header_row: Option<usize>,
    preview: RawCsv,
}

fn file_stem(loaded: &Loaded) -> String {
    loaded.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "New".into())
}

impl MappingState {
    pub fn from_guess(loaded: Loaded) -> Self {
        let guess = guess_spec(&loaded.raw);
        let name = format!("{} format", file_stem(&loaded));
        Self::new(loaded, 0, false, name, guess.spec, guess.header_row)
    }

    pub fn from_format(loaded: Loaded, f: &CsvFormat) -> Self {
        let header_row = if f.spec.has_header { find_header_row(&loaded.raw, &f.spec.header_signature) } else { None };
        // A header that doesn't match this file means the layout changed: start from a fresh guess.
        if f.spec.has_header && header_row.is_none() {
            let mut s = Self::from_guess(loaded);
            s.format_id = f.id;
            s.builtin = f.builtin;
            s.name = f.name.clone();
            return s;
        }
        Self::new(loaded, f.id, f.builtin, f.name.clone(), f.spec.clone(), header_row)
    }

    fn new(loaded: Loaded, format_id: i64, builtin: bool, name: String, spec: FormatSpec, header_row: Option<usize>) -> Self {
        let preview = RawCsv {
            file_name: loaded.raw.file_name.clone(),
            rows: loaded.raw.rows.iter().take(PREVIEW_ROWS).cloned().collect(),
        };
        let mut s = Self { loaded: Some(loaded), format_id, builtin, name, spec, header_row, preview };
        s.sync_header();
        s
    }

    fn raw(&self) -> &RawCsv {
        &self.loaded.as_ref().expect("loaded until saved").raw
    }

    fn sync_header(&mut self) {
        let raw = &self.loaded.as_ref().expect("loaded").raw;
        self.spec.has_header = self.header_row.is_some();
        self.spec.header_signature = self.header_row.map(|h| header_signature(&raw.rows[h])).unwrap_or_default();
        let first_data = self.header_row.map_or(0, |h| h + 1);
        self.spec.column_count = raw.rows.iter().skip(first_data).take(20).map(Vec::len).max().unwrap_or(0);
    }

    /// Display names for each column: `A · Header` or `A · sample value`.
    fn column_names(&self) -> Vec<String> {
        let raw = self.raw();
        let ncols = raw.max_columns();
        let sample_row = raw.rows.get(self.header_row.map_or(0, |h| h + 1));
        (0..ncols)
            .map(|c| {
                let letter = column_letter(c);
                let header = self.header_row.and_then(|h| raw.rows[h].get(c)).filter(|s| !s.is_empty());
                let text = header.cloned().or_else(|| sample_row.and_then(|r| r.get(c)).cloned()).unwrap_or_default();
                let short: String = text.chars().take(28).collect();
                format!("{letter} · {short}")
            })
            .collect()
    }

    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) -> MappingOutcome {
        let names = self.column_names();
        let mut outcome = MappingOutcome::Continue;
        ui.label(RichText::new(format!("Mapping columns for {}", self.raw().file_name)).strong());
        ui.label(RichText::new("Tell FinTrack which column holds what. The preview below updates as you go.").weak());
        ui.add_space(4.0);
        egui::ScrollArea::vertical().id_salt("mapping-scroll").show(ui, |ui| {
            card(ui, "Column mapping", tips::FORMAT_DETECT, |ui| self.form(ui, &names));
            ui.add_space(6.0);
            card(ui, "File contents (first rows)", tips::HEADER_ROW, |ui| self.raw_preview(ui));
            ui.add_space(6.0);
            let ok_rows = card(ui, "How rows will be imported", "", |ui| self.parsed_preview(ui, cx));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Format name:");
                ui.add(TextEdit::singleline(&mut self.name).desired_width(280.0));
                let can_save = ok_rows > 0 && !self.name.trim().is_empty();
                let label = if self.format_id == 0 { "Save format & continue" } else { "Save changes & continue" };
                if ui.add_enabled(can_save, egui::Button::new(label)).clicked() {
                    outcome = self.save(cx, self.format_id);
                }
                if self.format_id != 0 && ui.add_enabled(can_save, egui::Button::new("Save as new format")).clicked() {
                    outcome = self.save(cx, 0);
                }
                if ui.button("Cancel").clicked() {
                    outcome = MappingOutcome::Cancel;
                }
            });
        });
        outcome
    }

    fn save(&mut self, cx: &mut PageCtx<'_>, id: i64) -> MappingOutcome {
        let builtin = id != 0 && self.builtin;
        let mut format = CsvFormat { id, name: self.name.trim().to_string(), spec: self.spec.clone(), builtin };
        match cx.out.report(cx.db.save_format(&format), |_| "Format saved".into()) {
            Some(new_id) => {
                format.id = new_id;
                MappingOutcome::Saved(Box::new((self.loaded.take().expect("loaded"), format)))
            }
            None => MappingOutcome::Continue,
        }
    }

    fn form(&mut self, ui: &mut Ui, names: &[String]) {
        egui::Grid::new("mapping-form").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            label_with_help(ui, "Header row", tips::HEADER_ROW);
            let before = self.header_row;
            let header_text = |h: Option<usize>| h.map_or("No header row".to_string(), |h| format!("Row {}", h + 1));
            ComboBox::from_id_salt("map-header").selected_text(header_text(self.header_row)).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.header_row, None, "No header row");
                for i in 0..self.preview.rows.len().min(25) {
                    ui.selectable_value(
                        &mut self.header_row,
                        Some(i),
                        format!("Row {}: {}", i + 1, self.preview.rows[i].join(", ").chars().take(60).collect::<String>()),
                    );
                }
            });
            if self.header_row != before {
                self.sync_header();
            }
            ui.end_row();

            label_with_help(ui, "Date column", tips::DATE_FORMAT);
            ui.horizontal(|ui| {
                column_combo(ui, "map-date", &mut self.spec.date_column, names);
                let fmt_label =
                    |f: &str| if f.is_empty() { "Automatic".to_string() } else { format!("{f}  (e.g. {})", example_date(f)) };
                ComboBox::from_id_salt("map-date-fmt").selected_text(fmt_label(&self.spec.date_format)).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.spec.date_format, String::new(), "Automatic");
                    for f in DATE_FORMATS {
                        ui.selectable_value(&mut self.spec.date_format, f.to_string(), fmt_label(f));
                    }
                });
            });
            ui.end_row();

            ui.label("Description column");
            column_combo(ui, "map-desc", &mut self.spec.description_column, names);
            ui.end_row();

            label_with_help(ui, "Memo / details columns", tips::MEMO_COLS);
            ui.horizontal_wrapped(|ui| {
                for (i, n) in names.iter().enumerate() {
                    let mut on = self.spec.memo_columns.contains(&i);
                    if ui.checkbox(&mut on, n).changed() {
                        self.spec.memo_columns.retain(|&c| c != i);
                        if on {
                            self.spec.memo_columns.push(i);
                            self.spec.memo_columns.sort_unstable();
                        }
                    }
                }
            });
            ui.end_row();

            label_with_help(ui, "Amount", tips::AMOUNT_MODE);
            self.amount_ui(ui, names);
            ui.end_row();

            label_with_help(ui, "Direction column", tips::DIRECTION_COL);
            ui.horizontal(|ui| {
                let mut on = self.spec.direction.is_some();
                if ui.checkbox(&mut on, "Use").changed() {
                    self.spec.direction = on.then(|| DirectionColumn { column: 0, outflow_marker: "debit".into() });
                }
                if let Some(d) = &mut self.spec.direction {
                    column_combo(ui, "map-dir", &mut d.column, names);
                    ui.label("money out when it contains");
                    ui.add(TextEdit::singleline(&mut d.outflow_marker).desired_width(90.0));
                }
            });
            ui.end_row();

            label_with_help(ui, "Account number column", tips::ACCOUNT_COL);
            opt_column_combo(ui, "map-acct", &mut self.spec.account_number_column, names);
            ui.end_row();

            ui.label("Balance column");
            opt_column_combo(ui, "map-bal", &mut self.spec.balance_column, names);
            ui.end_row();

            label_with_help(ui, "Skip rows", tips::SKIP_FILTER);
            ui.horizontal(|ui| {
                let mut on = self.spec.skip_filter.is_some();
                if ui.checkbox(&mut on, "Use").changed() {
                    self.spec.skip_filter = on.then(|| SkipFilter { column: 0, contains: "pending".into() });
                }
                if let Some(s) = &mut self.spec.skip_filter {
                    ui.label("where");
                    column_combo(ui, "map-skip", &mut s.column, names);
                    ui.label("contains");
                    ui.add(TextEdit::singleline(&mut s.contains).desired_width(90.0));
                }
            });
            ui.end_row();
        });
    }

    fn amount_ui(&mut self, ui: &mut Ui, names: &[String]) {
        ui.vertical(|ui| {
            let single = matches!(self.spec.amount, AmountColumns::Single { .. });
            ui.horizontal(|ui| {
                if ui.radio(single, "Single signed column").clicked() && !single {
                    self.spec.amount = AmountColumns::Single { column: 0, negate: false };
                }
                if ui.radio(!single, "Separate debit / credit columns").clicked() && single {
                    self.spec.amount = AmountColumns::DebitCredit { debit: 0, credit: 0 };
                }
            });
            ui.horizontal(|ui| match &mut self.spec.amount {
                AmountColumns::Single { column, negate } => {
                    column_combo(ui, "map-amt", column, names);
                    ui.checkbox(negate, "Flip sign");
                    help(ui, tips::NEGATE);
                }
                AmountColumns::DebitCredit { debit, credit } => {
                    ui.label("Debit (out):");
                    column_combo(ui, "map-debit", debit, names);
                    ui.label("Credit (in):");
                    column_combo(ui, "map-credit", credit, names);
                }
            });
        });
    }

    fn raw_preview(&self, ui: &mut Ui) {
        let raw = &self.preview;
        let ncols = raw.max_columns();
        egui::ScrollArea::both().id_salt("raw-preview").max_height(220.0).show(ui, |ui| {
            egui::Grid::new("raw-grid").striped(true).show(ui, |ui| {
                ui.label("");
                for c in 0..ncols {
                    ui.strong(column_letter(c));
                }
                ui.end_row();
                for (i, row) in raw.rows.iter().take(15).enumerate() {
                    let is_header = Some(i) == self.header_row;
                    ui.label(RichText::new(format!("{}", i + 1)).weak());
                    for c in 0..ncols {
                        let cell: String = row.get(c).map(|s| s.chars().take(30).collect()).unwrap_or_default();
                        ui.label(if is_header { RichText::new(cell).strong().underline() } else { RichText::new(cell) });
                    }
                    ui.end_row();
                }
            });
        });
    }

    /// Returns the number of rows parsed successfully.
    fn parsed_preview(&self, ui: &mut Ui, cx: &PageCtx<'_>) -> usize {
        match parse_all(&self.preview, &self.spec) {
            Err(e) => {
                ui.colored_label(cx.palette.critical, format!("⚠ {e}"));
                0
            }
            Ok(out) => {
                let total = self.preview.rows.len().saturating_sub(self.header_row.map_or(0, |h| h + 1));
                ui.label(format!(
                    "{} of {} rows read successfully{}{}",
                    out.rows.len(),
                    total,
                    if out.errors.is_empty() { String::new() } else { format!(", {} with errors", out.errors.len()) },
                    if out.skipped == 0 { String::new() } else { format!(", {} skipped", out.skipped) },
                ));
                if let Some(e) = out.errors.first() {
                    ui.colored_label(cx.palette.warning, format!("⚠ Line {}: {}", e.line, e.message));
                }
                egui::Grid::new("parsed-grid").striped(true).num_columns(4).show(ui, |ui| {
                    for h in ["Date", "Description", "Memo", "Amount"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for r in out.rows.iter().take(10) {
                        ui.label(r.date.to_string());
                        ui.label(&r.description);
                        ui.label(RichText::new(&r.memo).weak());
                        ui.label(money_text(r.amount, &cx.palette));
                        ui.end_row();
                    }
                });
                out.rows.len()
            }
        }
    }
}

fn column_letter(c: usize) -> String {
    let mut n = c + 1;
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

fn example_date(fmt: &str) -> String {
    let d = chrono::NaiveDate::from_ymd_opt(2026, 1, 31).expect("valid");
    let s = d.format(fmt).to_string();
    debug_assert!(parse_date(&s, fmt).is_some());
    s
}

fn column_combo(ui: &mut Ui, id: &str, value: &mut usize, names: &[String]) {
    ComboBox::from_id_salt(id)
        .selected_text(names.get(*value).cloned().unwrap_or_else(|| format!("Column {}", *value + 1)))
        .show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                ui.selectable_value(value, i, n);
            }
        });
}

fn opt_column_combo(ui: &mut Ui, id: &str, value: &mut Option<usize>, names: &[String]) {
    let text = value.and_then(|v| names.get(v).cloned()).unwrap_or_else(|| "None".into());
    ComboBox::from_id_salt(id).selected_text(text).show_ui(ui, |ui| {
        ui.selectable_value(value, None, "None");
        for (i, n) in names.iter().enumerate() {
            ui.selectable_value(value, Some(i), n);
        }
    });
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
