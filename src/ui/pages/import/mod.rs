//! CSV import wizard: pick files → (map columns for unknown formats) → review accounts & duplicates → done.

mod mapping;
mod review;

use crate::db::ImportSummary;
use crate::help::tips;
use crate::import::detect::detect_format;
use crate::import::reader::RawCsv;
use crate::ui::state::{PageCtx, View};
use crate::ui::widgets::{card, confirm_button, empty_state, heading, help};
use egui::{Id, RichText, Ui};
use mapping::{MappingOutcome, MappingState};
use review::{ReviewOutcome, ReviewState};
use std::collections::VecDeque;
use std::path::PathBuf;

/// A CSV file read from disk.
pub struct Loaded {
    pub path: PathBuf,
    pub raw: RawCsv,
}

pub struct DoneInfo {
    file_name: String,
    summary: ImportSummary,
    categorized: usize,
    unassigned_rows: usize,
}

enum Stage {
    Idle,
    Mapping(Box<MappingState>),
    Review(Box<ReviewState>),
    Done(DoneInfo),
}

pub struct ImportPage {
    queue: VecDeque<PathBuf>,
    stage: Stage,
}

impl Default for ImportPage {
    fn default() -> Self {
        Self { queue: VecDeque::new(), stage: Stage::Idle }
    }
}

impl ImportPage {
    pub fn enqueue(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.queue.extend(paths);
    }

    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Import CSV", tips::IMPORT);
        if matches!(self.stage, Stage::Idle)
            && let Some(path) = self.queue.pop_front()
        {
            self.stage = load(path, cx);
        }
        let next = match &mut self.stage {
            Stage::Idle => {
                self.idle(ui, cx);
                None
            }
            Stage::Mapping(m) => match m.show(ui, cx) {
                MappingOutcome::Continue => None,
                MappingOutcome::Cancel => Some(Stage::Idle),
                MappingOutcome::Saved(handoff) => {
                    let (loaded, format) = *handoff;
                    Some(review_or_mapping(loaded, format, cx))
                }
            },
            Stage::Review(r) => match r.show(ui, cx) {
                ReviewOutcome::Continue => None,
                ReviewOutcome::Cancel => Some(Stage::Idle),
                ReviewOutcome::EditMapping(handoff) => {
                    let (loaded, format) = *handoff;
                    Some(Stage::Mapping(Box::new(MappingState::from_format(loaded, &format))))
                }
                ReviewOutcome::Imported(info) => Some(Stage::Done(info)),
            },
            Stage::Done(info) => done(ui, cx, info, self.queue.front()).then_some(Stage::Idle),
        };
        if let Some(s) = next {
            self.stage = s;
        }
    }

    fn idle(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        card(ui, "Add files", tips::IMPORT, |ui| {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("📂 Choose CSV files…").size(16.0)).clicked()
                    && let Some(files) =
                        rfd::FileDialog::new().set_title("Import bank CSV files").add_filter("CSV", &["csv", "txt"]).pick_files()
                {
                    self.queue.extend(files);
                }
                ui.label(RichText::new("…or drag and drop CSV files anywhere onto this window.").weak());
            });
            ui.label(
                RichText::new(
                    "Files are read locally. A copy is archived in the database's imports/ folder when enabled in Settings.",
                )
                .small()
                .weak(),
            );
        });
        ui.add_space(8.0);
        formats_card(ui, cx);
        ui.add_space(8.0);
        history_card(ui, cx);
    }
}

/// Summary after an import. Returns `true` when the user is finished with this screen.
fn done(ui: &mut Ui, cx: &mut PageCtx<'_>, info: &DoneInfo, next_file: Option<&PathBuf>) -> bool {
    let mut finished = false;
    {
        card(ui, "Import complete", "", |ui| {
            ui.label(
                RichText::new(format!("✔ Imported {} new transactions from {}", info.summary.inserted, info.file_name))
                    .size(16.0),
            );
            ui.label(format!("{} duplicates skipped (already in the database).", info.summary.duplicates));
            if info.unassigned_rows > 0 {
                ui.label(format!("{} rows were skipped because no account was chosen for them.", info.unassigned_rows));
            }
            ui.label(format!("{} transactions were categorized by your rules.", info.categorized));
            let unmapped = cx.data.uncategorized_count();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if unmapped > 0 && ui.button(format!("Categorize {unmapped} unmapped transactions")).clicked() {
                    cx.out.navigate = Some(View::Categorize);
                    finished = true;
                }
                if ui.button("View transactions").clicked() {
                    cx.out.navigate = Some(View::Transactions);
                    finished = true;
                }
                let label = match next_file {
                    Some(p) => format!("Next file: {}", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()),
                    None => "Import another file".to_string(),
                };
                if ui.button(label).clicked() {
                    finished = true;
                }
            });
        });
    }
    finished
}

fn load(path: PathBuf, cx: &mut PageCtx<'_>) -> Stage {
    match RawCsv::from_path(&path) {
        Err(e) => {
            cx.out.error(format!("Could not read {}: {e:#}", path.display()));
            Stage::Idle
        }
        Ok(raw) => {
            let loaded = Loaded { path, raw };
            match detect_format(&loaded.raw, &cx.data.formats) {
                Some(d) => review_or_mapping(loaded, cx.data.formats[d.format_index].clone(), cx),
                None => {
                    cx.out.info(
                        "This file's format isn't known yet — map its columns once and FinTrack will recognise it next time.",
                    );
                    Stage::Mapping(Box::new(MappingState::from_guess(loaded)))
                }
            }
        }
    }
}

fn review_or_mapping(loaded: Loaded, format: crate::model::CsvFormat, cx: &mut PageCtx<'_>) -> Stage {
    match ReviewState::new(loaded, format, cx) {
        Ok(r) => Stage::Review(Box::new(r)),
        Err(failed) => {
            let (loaded, format, msg) = *failed;
            cx.out.error(msg);
            Stage::Mapping(Box::new(MappingState::from_format(loaded, &format)))
        }
    }
}

fn formats_card(ui: &mut Ui, cx: &mut PageCtx<'_>) {
    egui::CollapsingHeader::new(format!("Saved CSV formats ({})", cx.data.formats.len())).id_salt("formats").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(
                    "Formats are recognised automatically. Edit one by importing a matching file and choosing 'Edit mapping'.",
                )
                .weak(),
            );
            help(ui, tips::FORMAT_DETECT);
        });
        egui::Grid::new("formats-grid").striped(true).num_columns(3).show(ui, |ui| {
            for f in &cx.data.formats {
                ui.label(&f.name);
                ui.label(RichText::new(if f.builtin { "built-in" } else { "custom" }).weak());
                if confirm_button(ui, Id::new(("del-format", f.id)), "🗑", "Delete this format?") {
                    cx.out.report(cx.db.delete_format(f.id), |_| "Format deleted".into());
                }
                ui.end_row();
            }
        });
    });
}

fn history_card(ui: &mut Ui, cx: &mut PageCtx<'_>) {
    card(ui, "Import history", tips::UNDO_IMPORT, |ui| {
        if cx.data.imports.is_empty() {
            empty_state(ui, "No imports yet.");
            return;
        }
        egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            egui::Grid::new("import-history").striped(true).num_columns(5).show(ui, |ui| {
                for h in ["When", "File", "Format", "Added / duplicates", ""] {
                    ui.strong(h);
                }
                ui.end_row();
                for rec in &cx.data.imports {
                    ui.label(&rec.imported_at);
                    let r = ui.label(&rec.file_name);
                    if !rec.archived_path.is_empty() {
                        r.on_hover_text(format!("Archived copy: {}", rec.archived_path));
                    }
                    ui.label(&rec.format_name);
                    ui.label(format!("{} / {}", rec.rows_inserted, rec.rows_duplicate));
                    ui.horizontal(|ui| {
                        if confirm_button(ui, Id::new(("undo-import", rec.id)), "Undo", "Delete its transactions?") {
                            cx.out.report(cx.db.undo_import(rec.id), |_| format!("Removed import of {}", rec.file_name));
                        }
                    });
                    ui.end_row();
                }
            });
        });
    });
}
