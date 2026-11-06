//! Mortgage results: progress today, baseline vs scenario, balance chart and schedule.

use crate::model::{Money, Mortgage};
use crate::mortgage::{Scenario, Schedule, build_schedule, compare, progress_at, scenario_for, yearly_summary};
use crate::ui::charts::year_line_chart;
use crate::ui::filters::today;
use crate::ui::state::PageCtx;
use crate::ui::widgets::labeled_bar;
use crate::ui::widgets::{card, pct, stat_tile};
use chrono::{Datelike, NaiveDate};
use egui::{RichText, Ui};

fn year_frac(d: NaiveDate) -> f64 {
    f64::from(d.year()) + f64::from(d.ordinal0()) / 365.25
}

fn balance_points(m: &Mortgage, s: &Schedule) -> Vec<[f64; 2]> {
    std::iter::once([year_frac(m.start_date), m.principal.dollars()])
        .chain(s.rows.iter().map(|r| [year_frac(r.date), r.balance.dollars()]))
        .collect()
}

fn duration(months: i64) -> String {
    format!("{} yr {} mo", months / 12, months % 12)
}

pub fn show(ui: &mut Ui, cx: &mut PageCtx<'_>, m: &Mortgage, extra: Money, full_schedule: &mut bool) {
    let data = cx.data;
    let rates: Vec<_> = data.rate_changes.iter().filter(|r| r.mortgage_id == m.id).cloned().collect();
    let prepays: Vec<_> = data.prepayments.iter().filter(|p| p.mortgage_id == m.id).cloned().collect();
    let baseline = build_schedule(m, &scenario_for(&rates, &[], Money::ZERO));
    let scenario_inputs: Scenario = scenario_for(&rates, &prepays, extra);
    let has_scenario = !scenario_inputs.prepayments.is_empty() || extra.cents() > 0;
    let scenario = build_schedule(m, &scenario_inputs);
    let today = today();
    let progress = progress_at(m, &scenario, today);

    if scenario.negative_amortization {
        ui.colored_label(
            cx.palette.critical,
            "⚠ The payment doesn't cover the interest at some point — increase the payment or check the rate changes.",
        );
    }

    card(
        ui,
        "Progress today",
        "Where the mortgage stands today according to the schedule (including enabled prepayments made so far).",
        |ui| {
            ui.horizontal_wrapped(|ui| {
                stat_tile(ui, "Balance today", &progress.balance.to_whole(), &format!("of {}", m.principal.to_whole()), "");
                stat_tile(ui, "Principal repaid", &progress.principal_paid.to_whole(), &pct(progress.fraction_repaid), "");
                stat_tile(ui, "Interest paid", &progress.interest_paid.to_whole(), "", "");
                stat_tile(
                    ui,
                    "Payments",
                    &format!("{} made", progress.payments_made),
                    &format!("{} remaining", progress.payments_remaining),
                    "",
                );
                stat_tile(
                    ui,
                    "Regular payment",
                    &scenario.first_payment().unwrap_or_default().to_string(),
                    m.frequency.label(),
                    "",
                );
            });
            let label = format!("{} of principal repaid", pct(progress.fraction_repaid));
            labeled_bar(ui, progress.fraction_repaid, cx.palette.series(0), &label, (ui.available_width() - 200.0).max(80.0));
        },
    );
    ui.add_space(6.0);

    let cmp = compare(&baseline, &scenario);
    card(
        ui,
        "Projection",
        "Baseline: rate changes only. With prepayments: enabled prepayments plus the what-if extra payment.",
        |ui| {
            egui::Grid::new("projection-cmp").striped(true).num_columns(3).spacing([24.0, 4.0]).show(ui, |ui| {
                ui.strong("");
                ui.strong("Baseline");
                ui.strong(if has_scenario { "With prepayments" } else { "With prepayments (none yet)" });
                ui.end_row();
                let payoff = |s: &Schedule| s.payoff_date().map_or("never".to_string(), |d| d.format("%b %Y").to_string());
                ui.label("Paid off");
                ui.label(payoff(&baseline));
                ui.label(RichText::new(payoff(&scenario)).strong());
                ui.end_row();
                ui.label("Total interest");
                ui.label(baseline.total_interest.to_whole());
                ui.label(RichText::new(scenario.total_interest.to_whole()).strong());
                ui.end_row();
                ui.label("Total paid");
                ui.label(baseline.total_paid.to_whole());
                ui.label(scenario.total_paid.to_whole());
                ui.end_row();
                ui.label("Number of payments");
                ui.label(baseline.rows.len().to_string());
                ui.label(scenario.rows.len().to_string());
                ui.end_row();
            });
            if has_scenario {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "✔ Saves {} in interest and pays off {} sooner.",
                        cmp.interest_saved.to_whole(),
                        duration(cmp.months_saved)
                    ))
                    .size(15.0)
                    .strong(),
                );
            }
        },
    );
    ui.add_space(6.0);

    card(ui, "Balance over time", "Hover the lines for exact values. The dashed line marks today.", |ui| {
        let mut series = vec![("Baseline".to_string(), balance_points(m, &baseline), cx.palette.other)];
        if has_scenario {
            series.push(("With prepayments".to_string(), balance_points(m, &scenario), cx.palette.series(0)));
        }
        year_line_chart(ui, "mortgage-balance", &series, Some(year_frac(today)), 280.0);
    });
    ui.add_space(6.0);

    card(ui, "Schedule", "", |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(full_schedule, false, "By year");
            ui.selectable_value(full_schedule, true, "Every payment");
        });
        if *full_schedule {
            payment_table(ui, &scenario, today);
        } else {
            year_table(ui, &scenario, today);
        }
    });
}

fn year_table(ui: &mut Ui, s: &Schedule, today: NaiveDate) {
    egui::Grid::new("schedule-years").striped(true).num_columns(6).spacing([18.0, 4.0]).show(ui, |ui| {
        for h in ["Year", "Payments", "Interest", "Principal", "Prepayments", "End balance"] {
            ui.strong(h);
        }
        ui.end_row();
        for y in yearly_summary(s) {
            let text = |v: String| {
                let t = RichText::new(v).monospace();
                if y.year == today.year() { t.strong() } else { t }
            };
            ui.label(text(y.year.to_string()));
            ui.label(text(y.payments.to_whole()));
            ui.label(text(y.interest.to_whole()));
            ui.label(text(y.principal.to_whole()));
            ui.label(text(y.prepayments.to_whole()));
            ui.label(text(y.end_balance.to_whole()));
            ui.end_row();
        }
    });
}

fn payment_table(ui: &mut Ui, s: &Schedule, today: NaiveDate) {
    use egui_extras::{Column, TableBuilder};
    TableBuilder::new(ui)
        .id_salt("schedule-payments")
        .striped(true)
        .max_scroll_height(420.0)
        .columns(Column::auto().at_least(80.0), 7)
        .header(20.0, |mut h| {
            for t in ["#", "Date", "Rate", "Payment", "Interest", "Principal", "Prepay / balance"] {
                h.col(|ui| {
                    ui.strong(t);
                });
            }
        })
        .body(|body| {
            body.rows(20.0, s.rows.len(), |mut row| {
                let r = &s.rows[row.index()];
                let past = r.date <= today;
                let fmt = |v: String| if past { RichText::new(v).weak() } else { RichText::new(v) };
                row.col(|ui| {
                    ui.label(fmt(r.number.to_string()));
                });
                row.col(|ui| {
                    ui.label(fmt(r.date.to_string()));
                });
                row.col(|ui| {
                    ui.label(fmt(format!("{:.2}%", r.rate_pct)));
                });
                row.col(|ui| {
                    ui.label(fmt(r.payment.to_string()));
                });
                row.col(|ui| {
                    ui.label(fmt(r.interest.to_string()));
                });
                row.col(|ui| {
                    ui.label(fmt(r.principal.to_string()));
                });
                row.col(|ui| {
                    let pre = if r.prepayment.is_zero() { String::new() } else { format!("+{} · ", r.prepayment) };
                    ui.label(fmt(format!("{pre}{}", r.balance)));
                });
            });
        });
}
