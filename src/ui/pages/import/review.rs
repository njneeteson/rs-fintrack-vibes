//! Review step: account assignment per group, duplicate preview, commit.

use super::{DoneInfo, Loaded};
use crate::help::tips;
use crate::import::dedup::DedupStatus;
use crate::import::parse::{ParseOutcome, parse_all};
use crate::import::{ImportGroup, assign_accounts, group_by_account};
use crate::model::{Account, CsvFormat};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{account_combo, card, enum_combo, help, money_text};
use egui::{RichText, TextEdit, Ui};

pub enum ReviewOutcome {
    Continue,
    Cancel,
    EditMapping(Box<(Loaded, CsvFormat)>),
    Imported(DoneInfo),
}

struct GroupState {
    group: ImportGroup,
    /// Duplicate status per row for the selected account.
    statuses: Vec<DedupStatus>,
    new_account: Option<Account>,
}

impl GroupState {
    fn refresh(&mut self, cx: &mut PageCtx<'_>) {
        self.statuses = match self.group.account_id {
            Some(a) => cx.db.preview_duplicates(a, &self.group).unwrap_or_else(|e| {
                cx.out.error(format!("{e:#}"));
                Vec::new()
            }),
            None => Vec::new(),
        };
    }

    fn new_count(&self) -> usize {
        if self.group.account_id.is_none() {
            return 0;
        }
        self.statuses.iter().filter(|s| !s.is_duplicate()).count()
    }
}

pub struct ReviewState {
    loaded: Option<Loaded>,
    format: CsvFormat,
    parse: ParseOutcome,
    groups: Vec<GroupState>,
}

impl ReviewState {
    /// Parse the file with `format`. On failure the inputs are handed back so the caller can open the mapper.
    pub fn new(loaded: Loaded, format: CsvFormat, cx: &mut PageCtx<'_>) -> Result<Self, Box<(Loaded, CsvFormat, String)>> {
        let mut parse = match parse_all(&loaded.raw, &format.spec) {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("{}: {e}", format.name);
                return Err(Box::new((loaded, format, msg)));
            }
        };
        if parse.rows.is_empty() {
            let msg = format!("No rows could be read with format '{}' — please check the mapping.", format.name);
            return Err(Box::new((loaded, format, msg)));
        }
        let mut groups = group_by_account(std::mem::take(&mut parse.rows));
        assign_accounts(&mut groups, &cx.data.accounts, &loaded.raw.file_name, Some(format.id));
        let mut groups: Vec<GroupState> =
            groups.into_iter().map(|group| GroupState { group, statuses: Vec::new(), new_account: None }).collect();
        for g in &mut groups {
            g.refresh(cx);
        }
        Ok(Self { loaded: Some(loaded), format, parse, groups })
    }

    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) -> ReviewOutcome {
        let mut outcome = ReviewOutcome::Continue;
        let file_name = self.loaded.as_ref().map(|l| l.raw.file_name.clone()).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(RichText::new(&file_name).strong());
            ui.label("— detected format:");
            ui.label(RichText::new(&self.format.name).strong());
            help(ui, tips::FORMAT_DETECT);
            if ui.button("Edit mapping").on_hover_text("Change how this file's columns are read").clicked() {
                outcome = ReviewOutcome::EditMapping(Box::new((self.loaded.take().expect("loaded"), self.format.clone())));
            }
        });
        if !self.parse.errors.is_empty() || self.parse.skipped > 0 {
            egui::CollapsingHeader::new(format!(
                "{} rows could not be read, {} skipped by filter",
                self.parse.errors.len(),
                self.parse.skipped
            ))
            .show(ui, |ui| {
                for e in self.parse.errors.iter().take(50) {
                    ui.label(format!("Line {}: {}", e.line, e.message));
                }
            });
        }
        ui.add_space(4.0);
        egui::ScrollArea::vertical().id_salt("review-scroll").max_height((ui.available_height() - 40.0).max(120.0)).show(
            ui,
            |ui| {
                for (i, g) in self.groups.iter_mut().enumerate() {
                    group_card(ui, cx, i, g, &file_name);
                    ui.add_space(6.0);
                }
            },
        );
        if matches!(outcome, ReviewOutcome::EditMapping(..)) {
            return outcome;
        }
        let new_rows: usize = self.groups.iter().map(GroupState::new_count).sum();
        let unassigned: usize = self.groups.iter().filter(|g| g.group.account_id.is_none()).map(|g| g.group.rows.len()).sum();
        ui.horizontal(|ui| {
            let label =
                if new_rows == 0 { "Nothing new — finish".to_string() } else { format!("Import {new_rows} new transactions") };
            let any_assigned = self.groups.iter().any(|g| g.group.account_id.is_some());
            if ui.add_enabled(any_assigned, egui::Button::new(RichText::new(label).strong())).clicked()
                && let Some(info) = self.commit(cx, unassigned)
            {
                outcome = ReviewOutcome::Imported(info);
            }
            if unassigned > 0 {
                ui.colored_label(cx.palette.warning, format!("⚠ {unassigned} rows have no account and will be skipped"));
            }
            if ui.button("Cancel").clicked() {
                outcome = ReviewOutcome::Cancel;
            }
        });
        outcome
    }

    fn commit(&mut self, cx: &mut PageCtx<'_>, unassigned_rows: usize) -> Option<DoneInfo> {
        let loaded = self.loaded.as_ref()?;
        let groups: Vec<ImportGroup> = self.groups.iter().map(|g| g.group.clone()).collect();
        let result = (|| -> anyhow::Result<DoneInfo> {
            let archived =
                if cx.settings.archive_imports { cx.db.archive_csv(&loaded.path)?.display().to_string() } else { String::new() };
            let summary = cx.db.commit_import(&loaded.raw.file_name, &archived, &self.format.name, &groups)?;
            for g in groups.iter().filter(|g| g.account_id.is_some()) {
                cx.db.learn_account_identity(
                    g.account_id.unwrap_or_default(),
                    g.account_number.as_deref(),
                    Some(self.format.id),
                )?;
            }
            let categorized = if cx.settings.auto_categorize_on_import { cx.db.apply_rules()? } else { 0 };
            Ok(DoneInfo { file_name: loaded.raw.file_name.clone(), summary, categorized, unassigned_rows })
        })();
        cx.out.report(result, |d| format!("Imported {} transactions", d.summary.inserted))
    }
}

fn group_card(ui: &mut Ui, cx: &mut PageCtx<'_>, i: usize, g: &mut GroupState, file_name: &str) {
    let title = match &g.group.account_number {
        Some(n) => format!("Rows for account number {n}"),
        None => "Transactions in this file".to_string(),
    };
    card(ui, &title, tips::ACCOUNT_DETECT, |ui| {
        if let Some((a, b)) = g.group.date_range() {
            ui.label(format!("{} rows from {a} to {b}", g.group.rows.len()));
        }
        ui.horizontal(|ui| {
            ui.label("Import into account:");
            let detected = crate::import::detect_account(&cx.data.accounts, g.group.account_number.as_deref(), file_name, None);
            if account_combo(ui, ("review-acct", i), &mut g.group.account_id, &cx.data.accounts, "— choose account —") {
                g.refresh(cx);
            }
            if g.group.account_id.is_some() && g.group.account_id == detected {
                ui.label(RichText::new("(detected automatically)").weak());
            }
            if g.new_account.is_none() && ui.button("➕ New account…").clicked() {
                g.new_account =
                    Some(Account { account_number: g.group.account_number.clone().unwrap_or_default(), ..Default::default() });
            }
        });
        if let Some(draft) = &mut g.new_account {
            let mut created = None;
            let mut cancel = false;
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(TextEdit::singleline(&mut draft.name).hint_text("e.g. Scotia Chequing").desired_width(160.0));
                ui.label("Bank:");
                ui.add(TextEdit::singleline(&mut draft.institution).desired_width(110.0));
                enum_combo(ui, ("new-acct-kind", i), &mut draft.kind);
                if ui.button("Create").clicked() {
                    created = cx.out.report(cx.db.save_account(draft), |_| "Account created".into());
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
            if let Some(id) = created {
                g.group.account_id = Some(id);
                g.new_account = None;
                g.refresh(cx);
            } else if cancel {
                g.new_account = None;
            }
        }
        if g.group.account_id.is_some() {
            let dups = g.statuses.iter().filter(|s| s.is_duplicate()).count();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{} new", g.new_count())).strong());
                ui.label(format!("· {dups} already imported (will be skipped)"));
                help(ui, tips::DUPLICATES);
            });
        }
        egui::CollapsingHeader::new("Preview rows").id_salt(("preview", i)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt(("preview-scroll", i)).max_height(260.0).show(ui, |ui| {
                egui::Grid::new(("preview-grid", i)).striped(true).num_columns(4).show(ui, |ui| {
                    for h in ["Date", "Description", "Amount", "Status"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (k, r) in g.group.rows.iter().enumerate().take(500) {
                        ui.label(r.date.to_string());
                        ui.label(&r.description);
                        ui.label(money_text(r.amount, &cx.palette));
                        match g.statuses.get(k) {
                            Some(DedupStatus::Duplicate) => ui.label(RichText::new("duplicate").weak()),
                            Some(DedupStatus::New { .. }) => ui.label("new"),
                            None => ui.label(RichText::new("—").weak()),
                        };
                        ui.end_row();
                    }
                });
            });
        });
    });
}
