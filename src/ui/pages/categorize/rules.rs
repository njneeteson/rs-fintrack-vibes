//! List of rules with enable toggles, match counts and a rule tester.

use crate::categorize::{MatchInput, RuleEngine};
use crate::help::tips;
use crate::model::{Money, Rule};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{account_combo, confirm_button, empty_state, help, money_drag};
use egui::{Id, RichText, TextEdit, Ui};
use std::collections::HashMap;

#[derive(Default)]
pub struct RulesView {
    search: String,
    test_text: String,
    test_amount: Money,
    test_account: Option<i64>,
}

impl RulesView {
    /// Returns a rule to edit when the user clicks Edit.
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) -> Option<Rule> {
        let mut edit = None;
        let data = cx.data;
        let engine = RuleEngine::new(&data.rules);
        for (id, err) in &engine.errors {
            ui.colored_label(cx.palette.critical, format!("⚠ Rule #{id} is disabled because it is invalid: {err}"));
        }
        egui::CollapsingHeader::new("🔍 Test a description").id_salt("rule-tester").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut self.test_text)
                        .hint_text("Description as it appears in the CSV")
                        .desired_width(260.0),
                );
                money_drag(ui, &mut self.test_amount);
                account_combo(ui, "tester-acct", &mut self.test_account, &data.accounts, "Any account");
            });
            let input = MatchInput {
                account_id: self.test_account.unwrap_or(0),
                amount: self.test_amount,
                description: &self.test_text,
                memo: "",
            };
            match engine.first_match(input) {
                Some(r) => ui.label(format!("→ {}  (rule: {})", data.categories.full_name(r.category_id), r.describe())),
                None => ui.label(RichText::new("No rule matches.").weak()),
            };
        });
        ui.horizontal(|ui| {
            ui.add(TextEdit::singleline(&mut self.search).hint_text("Filter rules…").desired_width(200.0));
            help(ui, tips::RULE_PRIORITY);
        });
        if data.rules.is_empty() {
            empty_state(ui, "No rules yet. Create one from the Unmapped tab or with ‘New rule’.");
            return None;
        }
        let mut hits: HashMap<i64, usize> = HashMap::new();
        for t in &data.transactions {
            if let Some(r) = t.rule_id {
                *hits.entry(r).or_default() += 1;
            }
        }
        let needle = self.search.to_lowercase();
        egui::ScrollArea::vertical().id_salt("rules-scroll").show(ui, |ui| {
            egui::Grid::new("rules-grid").striped(true).num_columns(7).spacing([12.0, 6.0]).show(ui, |ui| {
                for h in ["On", "Priority", "Rule", "Account", "Category", "Applied to", ""] {
                    ui.strong(h);
                }
                ui.end_row();
                for r in data.rules.iter().filter(|r| {
                    needle.is_empty()
                        || r.pattern.to_lowercase().contains(&needle)
                        || data.categories.full_name(r.category_id).to_lowercase().contains(&needle)
                }) {
                    let mut enabled = r.enabled;
                    if ui.checkbox(&mut enabled, "").on_hover_text("Enable or disable this rule").changed() {
                        let updated = Rule { enabled, ..r.clone() };
                        let saved = cx.db.save_rule(&updated).and_then(|_| cx.db.apply_rules());
                        cx.out.report(saved, |n| format!("Rule updated — {n} transactions changed"));
                    }
                    ui.label(r.priority.to_string());
                    ui.label(r.describe());
                    ui.label(r.account_id.map(|a| data.account_name(a)).unwrap_or_else(|| "any".into()));
                    ui.label(RichText::new(data.categories.full_name(r.category_id)).strong());
                    ui.label(format!("{} txns", hits.get(&r.id).copied().unwrap_or(0)))
                        .on_hover_text("Transactions whose automatic category comes from this rule");
                    ui.horizontal(|ui| {
                        if ui.small_button("✏ Edit").clicked() {
                            edit = Some(r.clone());
                        }
                        if confirm_button(ui, Id::new(("del-rule", r.id)), "🗑", "Delete?") {
                            let deleted = cx.db.delete_rule(r.id).and_then(|_| cx.db.apply_rules());
                            cx.out.report(deleted, |_| "Rule deleted".into());
                        }
                    });
                    ui.end_row();
                }
            });
        });
        edit
    }
}
