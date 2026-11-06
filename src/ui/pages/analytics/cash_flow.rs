use super::Controls;
use crate::analytics::summary::summarize;
use crate::analytics::{DateRange, income_vs_expense};
use crate::help::tips;
use crate::model::Money;
use crate::ui::charts::cash_flow_chart;
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, money_text, pct, stat_tile};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, cx: &mut PageCtx<'_>, c: &Controls, range: DateRange) {
    let data = cx.data;
    let s = summarize(&data.transactions, range, &data.categories);
    let totals = income_vs_expense(&data.transactions, range, c.granularity, &data.categories);
    let n = totals.len().max(1) as i64;
    ui.horizontal_wrapped(|ui| {
        stat_tile(ui, "Income", &s.income.to_whole(), &format!("avg {} / period", Money(s.income.cents() / n).to_whole()), "");
        stat_tile(
            ui,
            "Spending",
            &s.expense.to_whole(),
            &format!("avg {} / period", Money(s.expense.cents() / n).to_whole()),
            "",
        );
        stat_tile(ui, "Net", &s.net().to_whole(), "income − spending", "");
        stat_tile(ui, "Savings rate", &s.savings_rate().map_or("—".into(), pct), "", tips::SAVINGS_RATE);
    });
    ui.add_space(6.0);
    card(ui, &format!("Income vs spending ({})", c.granularity.label().to_lowercase()), tips::FLOW, |ui| {
        cash_flow_chart(ui, "cash-flow", &totals, &cx.palette, 300.0);
    });
    ui.add_space(6.0);
    card(ui, "By period", "", |ui| {
        egui::Grid::new("cash-flow-table").striped(true).num_columns(5).spacing([20.0, 4.0]).show(ui, |ui| {
            for h in ["Period", "Income", "Spending", "Net", "Savings rate"] {
                ui.strong(h);
            }
            ui.end_row();
            for t in totals.iter().rev() {
                ui.label(t.period.label());
                ui.label(money_text(t.income, &cx.palette));
                ui.label(RichText::new(t.expense.to_string()).monospace());
                ui.label(money_text(t.net(), &cx.palette));
                ui.label(t.savings_rate().map_or("—".into(), pct));
                ui.end_row();
            }
            ui.strong("Total");
            ui.label(money_text(s.income, &cx.palette));
            ui.label(RichText::new(s.expense.to_string()).monospace().strong());
            ui.label(money_text(s.net(), &cx.palette));
            ui.label(s.savings_rate().map_or("—".into(), pct));
            ui.end_row();
        });
    });
}
