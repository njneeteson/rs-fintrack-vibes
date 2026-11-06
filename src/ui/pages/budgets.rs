//! Budgets: monthly amounts per category, compared with actuals for any period.

use crate::analytics::budget::{BudgetLine, average_monthly, budget_report};
use crate::analytics::{Granularity, Period};
use crate::help::tips;
use crate::model::{CategoryKind, Money};
use crate::ui::filters::today;
use crate::ui::state::PageCtx;
use crate::ui::theme::{Health, Palette};
use crate::ui::widgets::labeled_bar;
use crate::ui::widgets::{category_combo, confirm_button, empty_state, enum_combo, heading, help, money_drag, stat_tile};
use chrono::NaiveDate;
use egui::{Id, RichText, Ui};
use std::collections::HashMap;

pub struct BudgetsPage {
    granularity: Granularity,
    anchor: NaiveDate,
    new_category: Option<i64>,
    new_amount: Money,
    /// In-progress edits of monthly amounts, saved when editing finishes.
    edits: HashMap<i64, Money>,
}

impl Default for BudgetsPage {
    fn default() -> Self {
        Self {
            granularity: Granularity::Month,
            anchor: today(),
            new_category: None,
            new_amount: Money::from_dollars(100.0),
            edits: HashMap::new(),
        }
    }
}

/// Progress bar with colour *and* a text label for budget status.
pub fn budget_bar(ui: &mut Ui, line: &BudgetLine, palette: &Palette, width: f32) {
    let f = line.fraction();
    let (color, label) = if line.kind == CategoryKind::Income {
        (palette.income, format!("{:.0}% of target", f * 100.0))
    } else {
        let h = Health::from_fraction(f);
        (palette.health(h), format!("{:.0}%  {}", f * 100.0, h.label()))
    };
    labeled_bar(ui, f, color, &label, (width - 150.0).max(60.0));
}

impl BudgetsPage {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>) {
        heading(ui, "Budgets", tips::BUDGET);
        let period = Period::containing(self.anchor, self.granularity);
        ui.horizontal(|ui| {
            enum_combo(ui, "budget-gran", &mut self.granularity);
            if ui.button("◀").on_hover_text("Previous period").clicked() {
                self.anchor = period.prev().start;
            }
            ui.label(RichText::new(period.label()).strong().size(16.0));
            if ui.button("▶").on_hover_text("Next period").clicked() {
                self.anchor = period.next().start;
            }
            if ui.button("Current").clicked() {
                self.anchor = today();
            }
        });
        let data = cx.data;
        let lines = budget_report(&data.budgets, &data.transactions, &data.categories, period);
        let expense: Vec<&BudgetLine> = lines.iter().filter(|l| l.kind != CategoryKind::Income).collect();
        let budgeted: Money = expense.iter().map(|l| l.budget).sum();
        let actual: Money = expense.iter().map(|l| l.actual).sum();
        ui.horizontal_wrapped(|ui| {
            stat_tile(ui, "Budgeted spending", &budgeted.to_whole(), &period.label(), "");
            stat_tile(ui, "Actual spending (budgeted categories)", &actual.to_whole(), "", "");
            stat_tile(ui, "Remaining", &(budgeted - actual).to_whole(), "", "");
            let over = expense.iter().filter(|l| l.fraction() > 1.0).count();
            stat_tile(ui, "Over budget", &over.to_string(), "categories", "");
        });
        ui.separator();
        if lines.is_empty() {
            empty_state(ui, "No budgets yet. Pick a category below and set a monthly amount.");
        } else {
            self.table(ui, cx, &lines);
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Add budget:");
            category_combo(ui, Id::new("budget-new-cat"), &mut self.new_category, &cx.data.categories, "Choose category…", 220.0);
            ui.label("per month");
            money_drag(ui, &mut self.new_amount);
            if let Some(c) = self.new_category
                && ui.small_button("Suggest").on_hover_text(tips::BUDGET_SUGGEST).clicked()
            {
                self.new_amount = average_monthly(&cx.data.transactions, &cx.data.categories, c, 3, today());
            }
            if ui.add_enabled(self.new_category.is_some(), egui::Button::new("➕ Add")).clicked() {
                let c = self.new_category.take().unwrap_or_default();
                cx.out.report(cx.db.set_budget(c, self.new_amount), |_| "Budget added".into());
            }
        });
    }

    fn table(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, lines: &[BudgetLine]) {
        let factor = self.granularity.months();
        egui::Grid::new("budgets").striped(true).num_columns(7).spacing([14.0, 6.0]).show(ui, |ui| {
            for h in ["Category", "Monthly budget", "This period", "Actual", "Remaining", "Progress", ""] {
                ui.strong(h);
            }
            ui.end_row();
            for line in lines {
                ui.label(RichText::new(&line.label).strong());
                let monthly = cx.data.budgets.iter().find(|b| b.id == line.budget_id).map_or(Money::ZERO, |b| b.monthly_amount);
                let value = self.edits.entry(line.budget_id).or_insert(monthly);
                let r = money_drag(ui, value);
                if r.drag_stopped() || r.lost_focus() {
                    let v = *value;
                    self.edits.remove(&line.budget_id);
                    cx.out.report(cx.db.set_budget(line.category_id, v), |_| "Budget updated".into());
                } else if !r.dragged() && !r.has_focus() {
                    self.edits.remove(&line.budget_id);
                }
                ui.label(RichText::new(line.budget.to_whole()).monospace());
                ui.label(RichText::new(line.actual.to_string()).monospace());
                ui.label(RichText::new(line.remaining().to_string()).monospace());
                budget_bar(ui, line, &cx.palette, 240.0);
                ui.horizontal(|ui| {
                    if ui.small_button("Suggest").on_hover_text(tips::BUDGET_SUGGEST).clicked() {
                        let avg = average_monthly(&cx.data.transactions, &cx.data.categories, line.category_id, 3, today());
                        cx.out.report(cx.db.set_budget(line.category_id, avg), |_| format!("Budget set to {avg} / month"));
                    }
                    if confirm_button(ui, Id::new(("del-budget", line.budget_id)), "🗑", "Remove?") {
                        cx.out.report(cx.db.delete_budget(line.budget_id), |_| "Budget removed".into());
                    }
                });
                ui.end_row();
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Period budgets are monthly × {factor:.2}.")).weak().small());
            help(ui, tips::BUDGET);
        });
    }
}
