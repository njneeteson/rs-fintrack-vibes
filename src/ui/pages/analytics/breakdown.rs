use super::{Controls, drill_down};
use crate::analytics::{Breakdown, DateRange, breakdown};
use crate::help::tips;
use crate::model::Money;
use crate::ui::charts::{bar_list, stacked_chart};
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, empty_state};
use egui::{RichText, Sense, Ui};

pub fn show(ui: &mut Ui, cx: &mut PageCtx<'_>, c: &Controls, range: DateRange) {
    let data = cx.data;
    let full = breakdown(&data.transactions, range, c.granularity, c.flow, c.bucketing, data.lookup());
    if full.buckets.is_empty() {
        empty_state(ui, "No transactions in this range.");
        return;
    }
    let total = full.grand_total();
    let title = format!(
        "{} by {} ({}) — total {}",
        c.flow.label(),
        c.bucketing.label().to_lowercase(),
        c.granularity.label().to_lowercase(),
        total.to_whole()
    );
    card(ui, &title, tips::BUCKETING, |ui| {
        stacked_chart(ui, "breakdown-stack", &full.clone().fold_top(c.top_n), &cx.palette, 320.0);
    });
    ui.add_space(6.0);
    card(ui, "Ranking for the whole range", "Click a row to see its transactions.", |ui| {
        let items: Vec<(String, Money)> = full.buckets.iter().take(15).map(|b| (b.label.clone(), b.total)).collect();
        if let Some(i) = bar_list(ui, &items, cx.palette.series(0), total) {
            drill_down(cx, &full.buckets[i].key, c.bucketing, range);
        }
    });
    ui.add_space(6.0);
    card(ui, "Table", "Rows are buckets, columns are periods. Click a label to see its transactions.", |ui| {
        pivot_table(ui, cx, &full, c, range);
    });
}

fn pivot_table(ui: &mut Ui, cx: &mut PageCtx<'_>, b: &Breakdown, c: &Controls, range: DateRange) {
    let n = b.periods.len().max(1) as i64;
    let mut clicked = None;
    egui::ScrollArea::horizontal().id_salt("pivot-scroll").show(ui, |ui| {
        egui::Grid::new("pivot").striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
            ui.strong("");
            for p in &b.periods {
                ui.strong(p.short_label());
            }
            ui.strong("Total");
            ui.strong("Avg / period");
            ui.strong("Share");
            ui.end_row();
            let grand = b.grand_total();
            for (i, bucket) in b.buckets.iter().enumerate() {
                if ui
                    .add(egui::Label::new(RichText::new(&bucket.label).strong()).sense(Sense::click()))
                    .on_hover_text("Show transactions")
                    .clicked()
                {
                    clicked = Some(i);
                }
                for v in &bucket.values {
                    ui.label(RichText::new(if v.is_zero() { "·".into() } else { v.to_whole() }).monospace());
                }
                ui.label(RichText::new(bucket.total.to_whole()).monospace().strong());
                ui.label(RichText::new(Money(bucket.total.cents() / n).to_whole()).monospace());
                let share = if grand.cents() != 0 { bucket.total.dollars() / grand.dollars() * 100.0 } else { 0.0 };
                ui.label(format!("{share:.1}%"));
                ui.end_row();
            }
            ui.strong("Total");
            for v in b.period_totals() {
                ui.label(RichText::new(v.to_whole()).monospace().strong());
            }
            ui.label(RichText::new(grand.to_whole()).monospace().strong());
            ui.label(RichText::new(Money(grand.cents() / n).to_whole()).monospace());
            ui.label("100%");
            ui.end_row();
        });
    });
    if let Some(i) = clicked {
        drill_down(cx, &b.buckets[i].key, c.bucketing, range);
    }
}
