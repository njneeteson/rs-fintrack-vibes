//! Accounts: identifiers for import detection, opening balances, archiving.

use crate::analytics::balances::balance_on;
use crate::help::tips;
use crate::model::Account;
use crate::ui::filters::today;
use crate::ui::state::PageCtx;
use crate::ui::widgets::{confirm_button, empty_state, enum_combo, heading, label_with_help, money_drag, money_text};
use egui::{Id, RichText, TextEdit, Ui};

#[derive(Default)]
pub struct AccountsPage {
    editing: Option<Account>,
}

impl AccountsPage {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Accounts", tips::ACCOUNT_DETECT);
        if ui.button("➕ New account").clicked() {
            self.editing = Some(Account::default());
        }
        ui.add_space(4.0);
        let data = cx.data;
        if data.accounts.is_empty() {
            empty_state(ui, "No accounts yet. Create one here, or while importing your first CSV.");
        }
        let today = today();
        egui::Grid::new("accounts-grid").striped(true).num_columns(7).spacing([14.0, 6.0]).show(ui, |ui| {
            for h in ["Name", "Type", "Bank", "Detection", "Transactions", "Balance today", ""] {
                ui.strong(h);
            }
            ui.end_row();
            for a in &data.accounts {
                let name = if a.archived {
                    RichText::new(format!("{} (archived)", a.name)).weak()
                } else {
                    RichText::new(&a.name).strong()
                };
                ui.label(name);
                ui.label(a.kind.label());
                ui.label(&a.institution);
                let mut ids = Vec::new();
                if !a.account_number.is_empty() {
                    ids.push(format!("# {}", a.account_number));
                }
                if !a.filename_hint.is_empty() {
                    ids.push(format!("file ~ “{}”", a.filename_hint));
                }
                if let Some(f) = a.format_id.and_then(|f| data.formats.iter().find(|x| x.id == f)) {
                    ids.push(format!("format: {}", f.name));
                }
                ui.label(RichText::new(if ids.is_empty() { "—".into() } else { ids.join(" · ") }).small());
                let own: Vec<_> = data.transactions.iter().filter(|t| t.account_id == a.id).collect();
                let range = match (own.last(), own.first()) {
                    (Some(first), Some(last)) => format!("{} ({} – {})", own.len(), first.date, last.date),
                    _ => "0".into(),
                };
                ui.label(range);
                ui.label(money_text(balance_on(a, &data.transactions, today), &cx.palette));
                ui.horizontal(|ui| {
                    if ui.small_button("✏ Edit").clicked() {
                        self.editing = Some(a.clone());
                    }
                    if confirm_button(
                        ui,
                        Id::new(("del-acct", a.id)),
                        "🗑",
                        &format!("Delete with its {} transactions?", own.len()),
                    ) {
                        cx.out.report(cx.db.delete_account(a.id), |_| "Account deleted".into());
                    }
                });
                ui.end_row();
            }
        });
        self.editor(ui, cx);
    }

    fn editor(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        let Some(a) = &mut self.editing else { return };
        let mut close = false;
        egui::Modal::new(Id::new("acct-edit")).show(ui.ctx(), |ui| {
            ui.heading(if a.id == 0 { "New account" } else { "Edit account" });
            egui::Grid::new("acct-form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Name");
                ui.add(TextEdit::singleline(&mut a.name).hint_text("e.g. Scotia Chequing"));
                ui.end_row();
                ui.label("Bank / institution");
                ui.text_edit_singleline(&mut a.institution);
                ui.end_row();
                ui.label("Type");
                enum_combo(ui, "acct-kind", &mut a.kind);
                ui.end_row();
                label_with_help(ui, "Account number", tips::ACCOUNT_NUMBER);
                ui.add(TextEdit::singleline(&mut a.account_number).hint_text("last 4 digits or full"));
                ui.end_row();
                label_with_help(ui, "Filename hint", tips::FILENAME_HINT);
                ui.add(TextEdit::singleline(&mut a.filename_hint).hint_text("e.g. visa"));
                ui.end_row();
                ui.label("Usual CSV format");
                let text =
                    a.format_id.and_then(|f| cx.data.formats.iter().find(|x| x.id == f)).map_or("None", |f| f.name.as_str());
                egui::ComboBox::from_id_salt("acct-format").selected_text(text).show_ui(ui, |ui| {
                    ui.selectable_value(&mut a.format_id, None, "None");
                    for f in &cx.data.formats {
                        ui.selectable_value(&mut a.format_id, Some(f.id), &f.name);
                    }
                });
                ui.end_row();
                label_with_help(ui, "Opening balance", tips::OPENING_BALANCE);
                money_drag(ui, &mut a.opening_balance);
                ui.end_row();
                ui.label("Archived");
                ui.checkbox(&mut a.archived, "Hide from pickers and auto-detection");
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    close = cx.out.report(cx.db.save_account(a), |_| "Account saved".into()).is_some();
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        if close {
            self.editing = None;
        }
    }
}
