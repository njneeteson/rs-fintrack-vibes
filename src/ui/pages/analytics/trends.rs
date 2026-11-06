use super::{Controls, drill_down};
use crate::analytics::{BucketKey, DateRange, breakdown};
use crate::model::Money;
use crate::ui::charts::line_chart;
use crate::ui::state::PageCtx;
use crate::ui::widgets::{card, empty_state, stat_tile};
use egui::{ComboBox, Ui};

#[derive(Default)]
pub struct TrendsState {
    selected: Option<BucketKey>,
}

impl TrendsState {
    pub fn show(&mut self, ui: &mut Ui, cx: &mut PageCtx<'_>, c: &Controls, range: DateRange) {
        let data = cx.data;
        let b = breakdown(&data.transactions, range, c.granularity, c.flow, c.bucketing, data.lookup());
        if b.buckets.is_empty() {
            empty_state(ui, "No transactions in this range.");
            return;
        }
        if self.selected.as_ref().is_none_or(|k| !b.buckets.iter().any(|x| &x.key == k)) {
            self.selected = Some(b.buckets[0].key.clone());
        }
        let current = self.selected.clone().expect("set above");
        let series = b.buckets.iter().find(|x| x.key == current).expect("exists");
        ui.horizontal(|ui| {
            ui.label("Bucket:");
            ComboBox::from_id_salt("trend-bucket").selected_text(&series.label).width(260.0).height(400.0).show_ui(ui, |ui| {
                for x in &b.buckets {
                    ui.selectable_value(
                        &mut self.selected,
                        Some(x.key.clone()),
                        format!("{}  ({})", x.label, x.total.to_whole()),
                    );
                }
            });
            if ui.button("Show transactions").clicked() {
                drill_down(cx, &current, c.bucketing, range);
            }
        });
        let n = series.values.len().max(1) as i64;
        let avg = Money(series.total.cents() / n);
        let (min_i, max_i) = (
            (0..series.values.len()).min_by_key(|&i| series.values[i]).unwrap_or(0),
            (0..series.values.len()).max_by_key(|&i| series.values[i]).unwrap_or(0),
        );
        ui.horizontal_wrapped(|ui| {
            stat_tile(ui, "Total", &series.total.to_whole(), "", "");
            stat_tile(ui, "Average per period", &avg.to_whole(), "", "");
            stat_tile(ui, "Highest", &series.values[max_i].to_whole(), &b.periods[max_i].label(), "");
            stat_tile(ui, "Lowest", &series.values[min_i].to_whole(), &b.periods[min_i].label(), "");
        });
        card(ui, &format!("{} over time", series.label), "", |ui| {
            let labels: Vec<String> = b.periods.iter().map(|p| p.short_label()).collect();
            let values: Vec<f64> = series.values.iter().map(|v| v.dollars()).collect();
            let avg_line = vec![avg.dollars(); values.len()];
            line_chart(
                ui,
                "trend-line",
                labels,
                &[(series.label.clone(), values, cx.palette.series(0)), ("Average".into(), avg_line, cx.palette.other)],
                300.0,
            );
        });
    }
}
