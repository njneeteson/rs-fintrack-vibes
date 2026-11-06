use super::{Controls, drill_down};
use crate::analytics::summary::top_payees;
use crate::analytics::{BucketKey, Bucketing, DateRange};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, empty_state};
use egui::{RichText, Sense, Ui};

pub fn show(ui: &mut Ui, cx: &mut PageCtx<'_>, c: &Controls, range: DateRange) {
    let data = cx.data;
    let payees = top_payees(&data.transactions, range, c.flow, &data.categories, 100);
    if payees.is_empty() {
        empty_state(ui, "No transactions in this range.");
        return;
    }
    let total: f64 = payees.iter().map(|p| p.total.dollars()).sum();
    let mut clicked = None;
    card(
        ui,
        &format!("Top payees — {}", c.flow.label().to_lowercase()),
        "Payees are grouped by the leading words of the description. Click a name to see its transactions.",
        |ui| {
            egui::Grid::new("payees").striped(true).num_columns(6).spacing([18.0, 4.0]).show(ui, |ui| {
                for h in ["Payee", "Transactions", "Total", "Average", "Share", "Last"] {
                    ui.strong(h);
                }
                ui.end_row();
                for (i, p) in payees.iter().enumerate() {
                    if ui.add(egui::Label::new(RichText::new(&p.payee).strong()).sense(Sense::click())).clicked() {
                        clicked = Some(i);
                    }
                    ui.label(p.count.to_string());
                    ui.label(RichText::new(p.total.to_string()).monospace());
                    ui.label(RichText::new(p.average().to_string()).monospace());
                    ui.label(format!("{:.1}%", if total != 0.0 { p.total.dollars() / total * 100.0 } else { 0.0 }));
                    ui.label(p.last_date.to_string());
                    ui.end_row();
                }
            });
        },
    );
    if let Some(i) = clicked {
        drill_down(cx, &BucketKey::Payee(payees[i].payee.clone()), Bucketing::Payee, range);
    }
}
