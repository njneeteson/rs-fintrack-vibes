//! Rule editor window with a live match preview.

use crate::categorize::{CompiledRule, MatchInput};
use crate::help::tips;
use crate::model::{CategoryKind, Money, Rule};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{
    account_combo, category_combo, confirm_button, enum_combo, help, label_with_help, money_drag, money_text,
};
use egui::{Id, RichText, TextEdit};

pub struct RuleEditor {
    rule: Rule,
    min: Money,
    max: Money,
    new_cat_parent: Option<i64>,
    new_cat_name: String,
}

impl RuleEditor {
    pub fn new(rule: Rule) -> Self {
        Self {
            min: rule.min_amount.unwrap_or_default(),
            max: rule.max_amount.unwrap_or_default(),
            rule,
            new_cat_parent: None,
            new_cat_name: String::new(),
        }
    }

    /// Show the window. Returns `false` when it should close.
    pub fn show(&mut self, ctx: &egui::Context, cx: &mut PageCtx<'_>) -> bool {
        let mut open = true;
        let mut close = false;
        let title = if self.rule.id == 0 { "New mapping rule" } else { "Edit mapping rule" };
        egui::Window::new(title).id(Id::new("rule-editor")).open(&mut open).collapsible(false).default_width(560.0).show(
            ctx,
            |ui| {
                self.form(ui, cx);
                ui.separator();
                let compiled = self.preview(ui, cx);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add_enabled(compiled && self.rule.category_id != 0, egui::Button::new("Save rule")).clicked() {
                        let saved = cx.db.save_rule(&self.rule).and_then(|_| cx.db.apply_rules());
                        if cx.out.report(saved, |n| format!("Rule saved — {n} transactions recategorized")).is_some() {
                            close = true;
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if self.rule.id != 0
                        && confirm_button(ui, Id::new("delete-rule-editor"), "🗑 Delete rule", "Delete this rule?")
                    {
                        let deleted = cx.db.delete_rule(self.rule.id).and_then(|_| cx.db.apply_rules());
                        cx.out.report(deleted, |_| "Rule deleted".into());
                        close = true;
                    }
                });
            },
        );
        open && !close
    }

    fn form(&mut self, ui: &mut egui::Ui, cx: &mut PageCtx<'_>) {
        let data = cx.data;
        egui::Grid::new("rule-form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            label_with_help(ui, "When", tips::RULE_MATCH);
            ui.horizontal(|ui| {
                enum_combo(ui, "rule-field", &mut self.rule.field);
                enum_combo(ui, "rule-kind", &mut self.rule.match_kind);
            });
            ui.end_row();

            ui.label("Text");
            ui.add(TextEdit::singleline(&mut self.rule.pattern).hint_text("e.g. SAVE ON FOODS").desired_width(300.0));
            ui.end_row();

            ui.label("Direction");
            enum_combo(ui, "rule-dir", &mut self.rule.direction);
            ui.end_row();

            label_with_help(ui, "Amount between", tips::RULE_AMOUNT);
            ui.horizontal(|ui| {
                let mut use_min = self.rule.min_amount.is_some();
                ui.checkbox(&mut use_min, "min");
                if use_min {
                    money_drag(ui, &mut self.min);
                }
                let mut use_max = self.rule.max_amount.is_some();
                ui.checkbox(&mut use_max, "max");
                if use_max {
                    money_drag(ui, &mut self.max);
                }
                self.rule.min_amount = use_min.then_some(self.min.abs());
                self.rule.max_amount = use_max.then_some(self.max.abs());
            });
            ui.end_row();

            ui.label("Only in account");
            account_combo(ui, "rule-account", &mut self.rule.account_id, &data.accounts, "Any account");
            ui.end_row();

            ui.label(RichText::new("Assign category").strong());
            let mut cat = (self.rule.category_id != 0).then_some(self.rule.category_id);
            if category_combo(ui, Id::new("rule-cat"), &mut cat, &data.categories, "", 260.0) {
                self.rule.category_id = cat.unwrap_or(0);
            }
            ui.end_row();

            ui.label("…or new category");
            ui.horizontal(|ui| {
                let parent_text = self.new_cat_parent.map_or("(new top-level)".to_string(), |p| data.categories.full_name(p));
                egui::ComboBox::from_id_salt("rule-newcat-parent").selected_text(parent_text).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.new_cat_parent, None, "(new top-level)");
                    for c in data.categories.top_level() {
                        ui.selectable_value(&mut self.new_cat_parent, Some(c.id), &c.name);
                    }
                });
                ui.add(TextEdit::singleline(&mut self.new_cat_name).hint_text("Name").desired_width(140.0));
                if ui.add_enabled(!self.new_cat_name.trim().is_empty(), egui::Button::new("Add")).clicked() {
                    let added = cx.db.add_category(&self.new_cat_name, self.new_cat_parent, CategoryKind::Expense);
                    if let Some(id) = cx.out.report(added, |_| "Category added".into()) {
                        self.rule.category_id = id;
                        self.new_cat_name.clear();
                    }
                }
            });
            ui.end_row();

            label_with_help(ui, "Priority", tips::RULE_PRIORITY);
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.rule.priority).range(0..=10_000));
                ui.checkbox(&mut self.rule.enabled, "Enabled");
            });
            ui.end_row();
        });
    }

    /// Live preview of matching transactions. Returns whether the rule compiles.
    fn preview(&self, ui: &mut egui::Ui, cx: &PageCtx<'_>) -> bool {
        let compiled = match CompiledRule::compile(&self.rule) {
            Ok(c) => c,
            Err(e) => {
                if !self.rule.pattern.is_empty() {
                    ui.colored_label(cx.palette.critical, format!("⚠ {e}"));
                }
                return false;
            }
        };
        let matches: Vec<_> = cx.data.transactions.iter().filter(|t| compiled.matches(MatchInput::from(*t))).collect();
        let unmapped = matches.iter().filter(|t| t.is_uncategorized()).count();
        let overridden = matches.iter().filter(|t| t.is_manual()).count();
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Matches {} transactions", matches.len())).strong());
            ui.label(format!("· {unmapped} currently unmapped · {overridden} with manual overrides (kept)"));
            help(ui, "Rules with a lower priority number that also match will win; see the Rules tab.");
        });
        egui::ScrollArea::vertical().id_salt("rule-preview").max_height(180.0).show(ui, |ui| {
            egui::Grid::new("rule-preview-grid").striped(true).num_columns(4).show(ui, |ui| {
                for t in matches.iter().take(50) {
                    ui.label(t.date.to_string());
                    ui.label(&t.description);
                    ui.label(money_text(t.amount, &cx.palette));
                    ui.label(RichText::new(t.category_id().map(|c| cx.data.categories.full_name(c)).unwrap_or_default()).weak());
                    ui.end_row();
                }
            });
        });
        true
    }
}
