use crate::analytics::recurring::detect_recurring;
use crate::help::tips;
use crate::model::Money;
use crate::ui::filters::today;
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, empty_state, stat_tile};
use egui::{RichText, Ui};

pub fn show(ui: &mut Ui, cx: &mut PageCtx<'_>) {
    let data = cx.data;
    let found = detect_recurring(&data.transactions, &data.categories, today());
    if found.is_empty() {
        empty_state(ui, "No recurring payments detected yet. At least three regular charges are needed (two for annual ones).");
        return;
    }
    let monthly: Money = found.iter().map(|r| r.monthly_cost).sum();
    ui.horizontal_wrapped(|ui| {
        stat_tile(ui, "Recurring payments", &found.len().to_string(), "", tips::RECURRING);
        stat_tile(ui, "Monthly cost", &monthly.to_whole(), &format!("{} per year", Money(monthly.cents() * 12).to_whole()), "");
    });
    card(ui, "Detected subscriptions and bills", tips::RECURRING, |ui| {
        egui::Grid::new("recurring").striped(true).num_columns(7).spacing([16.0, 4.0]).show(ui, |ui| {
            for h in ["Payee", "Cadence", "Typical amount", "Per month", "Last charge", "Next expected", "Category"] {
                ui.strong(h);
            }
            ui.end_row();
            for r in &found {
                ui.label(RichText::new(&r.payee).strong()).on_hover_text(format!("{} charges seen", r.occurrences));
                ui.label(r.cadence.label());
                ui.label(RichText::new(r.typical_amount.to_string()).monospace());
                ui.label(RichText::new(r.monthly_cost.to_string()).monospace());
                ui.label(r.last_date.to_string());
                ui.label(r.next_expected.to_string());
                ui.label(r.category_id.map(|c| data.categories.full_name(c)).unwrap_or_else(|| "Uncategorized".into()));
                ui.end_row();
            }
        });
    });
}
