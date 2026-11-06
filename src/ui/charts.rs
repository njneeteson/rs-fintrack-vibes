//! Chart builders (egui_plot) and a lightweight ranked bar list.
//!
//! Conventions: one y-axis in dollars, thin bars with a surface-coloured gap
//! between stacked segments, hover tooltips on every mark, and a legend for
//! every multi-series chart.

use super::theme::Palette;
use crate::analytics::{Breakdown, PeriodTotals};
use crate::model::Money;
use crate::model::money::compact_dollars;
use egui::{Color32, CornerRadius, Rect, RichText, Sense, Stroke, Ui, vec2};
use egui_plot::{Bar, BarChart, Corner, GridMark, Legend, Line, Plot, PlotPoints};
use std::ops::RangeInclusive;

/// Base plot: no zoom/drag (charts are for reading, not exploring), dollar y-axis.
fn base_plot(id: &str, height: f32) -> Plot<'static> {
    Plot::new(id.to_string())
        .height(height)
        .allow_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .allow_double_click_reset(false)
        .show_grid([false, true])
        .y_axis_formatter(|m: GridMark, _r: &RangeInclusive<f64>| compact_dollars(m.value))
        .legend(Legend::default().position(Corner::LeftTop))
}

/// X axis showing a label at each integer index.
fn index_axis(labels: Vec<String>) -> impl Fn(GridMark, &RangeInclusive<f64>) -> String {
    move |m, _| {
        let i = m.value.round();
        if (m.value - i).abs() < 1e-6 && i >= 0.0 { labels.get(i as usize).cloned().unwrap_or_default() } else { String::new() }
    }
}

type BarFormatter = Box<dyn Fn(&Bar, &BarChart) -> String>;

fn tooltip(series: String) -> BarFormatter {
    Box::new(move |bar, _| format!("{}\n{series}: {}", bar.name, Money::from_dollars(bar.value)))
}

/// Income vs spending bars per period, with a net line.
pub fn cash_flow_chart(ui: &mut Ui, id: &str, totals: &[PeriodTotals], palette: &Palette, height: f32) {
    let labels: Vec<String> = totals.iter().map(|t| t.period.short_label()).collect();
    let gap = Stroke::new(1.0_f32, ui.visuals().extreme_bg_color);
    let bars = |f: fn(&PeriodTotals) -> Money, offset: f64, fill: Color32| -> Vec<Bar> {
        totals
            .iter()
            .enumerate()
            .map(|(i, t)| Bar::new(i as f64 + offset, f(t).dollars()).width(0.36).name(t.period.label()).fill(fill).stroke(gap))
            .collect()
    };
    let income = BarChart::new(bars(|t| t.income, -0.19, palette.income))
        .name("Income")
        .color(palette.income)
        .element_formatter(tooltip("Income".into()));
    let spending = BarChart::new(bars(|t| t.expense, 0.19, palette.spending))
        .name("Spending")
        .color(palette.spending)
        .element_formatter(tooltip("Spending".into()));
    let net: PlotPoints = totals.iter().enumerate().map(|(i, t)| [i as f64, t.net().dollars()]).collect();
    let ink = ui.visuals().text_color();
    let net_labels: Vec<String> = totals.iter().map(|t| t.period.label()).collect();
    base_plot(id, height)
        .x_axis_formatter(index_axis(labels))
        .label_formatter(move |name, p| {
            let i = p.x.round().max(0.0) as usize;
            if name == "Net" {
                format!("{}\nNet: {}", net_labels.get(i).cloned().unwrap_or_default(), Money::from_dollars(p.y))
            } else {
                String::new()
            }
        })
        .show(ui, |plot| {
            plot.bar_chart(income);
            plot.bar_chart(spending);
            plot.line(Line::new(net).name("Net").color(ink).width(2.0_f32));
        });
}

/// Stacked bars per period, one colour per bucket (buckets beyond the palette fold into grey).
pub fn stacked_chart(ui: &mut Ui, id: &str, breakdown: &Breakdown, palette: &Palette, height: f32) {
    let labels: Vec<String> = breakdown.periods.iter().map(|p| p.short_label()).collect();
    let gap = Stroke::new(1.0_f32, ui.visuals().extreme_bg_color);
    let mut charts: Vec<BarChart> = Vec::new();
    for (j, bucket) in breakdown.buckets.iter().enumerate() {
        let color = if bucket.key == crate::analytics::BucketKey::Other { palette.other } else { palette.series(j) };
        let bars: Vec<Bar> = bucket
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                Bar::new(i as f64, v.dollars().max(0.0)).width(0.7).name(breakdown.periods[i].label()).fill(color).stroke(gap)
            })
            .collect();
        let refs: Vec<&BarChart> = charts.iter().collect();
        let chart =
            BarChart::new(bars).name(&bucket.label).color(color).element_formatter(tooltip(bucket.label.clone())).stack_on(&refs);
        charts.push(chart);
    }
    base_plot(id, height).x_axis_formatter(index_axis(labels)).show(ui, |plot| {
        for c in charts {
            plot.bar_chart(c);
        }
    });
}

/// Line chart over indexed x values.
pub fn line_chart(ui: &mut Ui, id: &str, x_labels: Vec<String>, series: &[(String, Vec<f64>, Color32)], height: f32) {
    let hover_labels = x_labels.clone();
    base_plot(id, height)
        .x_axis_formatter(index_axis(x_labels))
        .label_formatter(move |name, p| {
            if name.is_empty() {
                return String::new();
            }
            let i = p.x.round().max(0.0) as usize;
            format!("{name}\n{}: {}", hover_labels.get(i).cloned().unwrap_or_default(), Money::from_dollars(p.y))
        })
        .show(ui, |plot| {
            for (name, values, color) in series {
                let pts: PlotPoints = values.iter().enumerate().map(|(i, v)| [i as f64, *v]).collect();
                plot.line(Line::new(pts).name(name).color(*color).width(2.0_f32));
            }
        });
}

/// Line chart over fractional years (mortgage balances).
pub fn year_line_chart(ui: &mut Ui, id: &str, series: &[(String, Vec<[f64; 2]>, Color32)], marker_x: Option<f64>, height: f32) {
    base_plot(id, height)
        .x_axis_formatter(|m, _| format!("{:.0}", m.value))
        .label_formatter(|name, p| {
            if name.is_empty() { String::new() } else { format!("{name}\n{:.1}: {}", p.x, Money::from_dollars(p.y)) }
        })
        .show(ui, |plot| {
            for (name, pts, color) in series {
                plot.line(Line::new(PlotPoints::from(pts.clone())).name(name).color(*color).width(2.0_f32));
            }
            if let Some(x) = marker_x {
                plot.vline(
                    egui_plot::VLine::new(x).name("Today").color(Color32::GRAY).style(egui_plot::LineStyle::dashed_loose()),
                );
            }
        });
}

/// Ranked horizontal bars drawn with plain egui. Returns the index of a clicked row.
pub fn bar_list(ui: &mut Ui, items: &[(String, Money)], color: Color32, total: Money) -> Option<usize> {
    let max = items.iter().map(|i| i.1.cents()).max().unwrap_or(1).max(1);
    let mut clicked = None;
    for (i, (label, value)) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            let label_w = 170.0;
            let value_w = 120.0;
            let r = ui
                .allocate_ui_with_layout(vec2(label_w, 18.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.set_min_width(label_w);
                    ui.add(egui::Label::new(label.as_str()).truncate().sense(Sense::click()))
                })
                .inner;
            let bar_w = (ui.available_width() - value_w).max(40.0);
            let (rect, resp) = ui.allocate_exact_size(vec2(bar_w, 14.0), Sense::click());
            let w = (rect.width() * value.cents().max(0) as f32 / max as f32).max(2.0);
            ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(w, rect.height())), CornerRadius::same(3), color);
            let share = if total.cents() > 0 { value.dollars() / total.dollars() } else { 0.0 };
            ui.label(RichText::new(format!("{}  ({:.0}%)", value.to_whole(), share * 100.0)).monospace());
            let hover = format!("{label}: {value} — {:.1}% of total. Click to see transactions.", share * 100.0);
            if r.on_hover_text(&hover).clicked() | resp.on_hover_text(&hover).clicked() {
                clicked = Some(i);
            }
        });
    }
    clicked
}
