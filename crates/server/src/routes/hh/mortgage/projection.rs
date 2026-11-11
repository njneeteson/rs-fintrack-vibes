//! Progress today, baseline vs prepayments, the balance chart and the schedule.

use super::{Loaded, ProjectionQuery};
use crate::extract::HhCtx;
use crate::routes::hh::data::today;
use crate::views::charts::{Chart, Color, Series, figure};
use crate::views::components::{Num, Text, card, meter, notice, stat_tile, tabs, thead};
use crate::views::format::{date, money_plain, pct, whole_plain};
use chrono::Datelike;
use fintrack_core::model::Money;
use fintrack_core::mortgage::{
    Schedule, build_schedule, compare, progress_at, scenario_for, yearly_summary,
};
use maud::{Markup, html};
use std::collections::{BTreeMap, BTreeSet};

const PROGRESS_TIP: &str = "Where the mortgage stands today according to the schedule, \
    including enabled prepayments made so far.";
const PROJECTION_TIP: &str = "Baseline: rate changes only. With prepayments: enabled \
    prepayments plus the what-if extra payment.";

fn duration(months: i64) -> String {
    format!("{} yr {} mo", months / 12, months % 12)
}

fn payoff(s: &Schedule) -> String {
    s.payoff_date()
        .map_or_else(|| "never".into(), |d| d.format("%b %Y").to_string())
}

/// End-of-year balances of both schedules on one set of year labels.
fn balance_chart(baseline: &Schedule, scenario: &Schedule, has_scenario: bool) -> Chart {
    let by_year = |s: &Schedule| -> BTreeMap<i32, Money> {
        yearly_summary(s)
            .into_iter()
            .map(|y| (y.year, y.end_balance))
            .collect()
    };
    let (b, s) = (by_year(baseline), by_year(scenario));
    let years: BTreeSet<i32> = b.keys().chain(s.keys()).copied().collect();
    let values = |m: &BTreeMap<i32, Money>| {
        years
            .iter()
            .map(|y| m.get(y).map_or(0.0, |v| v.dollars()))
            .collect()
    };
    let mut lines = vec![Series::new("Baseline", values(&b), Color::Other)];
    if has_scenario {
        lines.push(Series::new("With prepayments", values(&s), Color::nth(0)));
    }
    Chart {
        labels: years.iter().map(i32::to_string).collect(),
        lines,
        ..Default::default()
    }
}

fn comparison(baseline: &Schedule, scenario: &Schedule, has_scenario: bool) -> Markup {
    let cmp = compare(baseline, scenario);
    let with = if has_scenario {
        "With prepayments"
    } else {
        "With prepayments (none yet)"
    };
    html! {
        table {
            (thead(&[("", Text), ("Baseline", Text), (with, Text)]))
            tbody {
                tr {
                    th { "Paid off" }
                    td { (payoff(baseline)) }
                    td { strong { (payoff(scenario)) } }
                }
                tr {
                    th { "Total interest" }
                    td { (baseline.total_interest.to_whole()) }
                    td { strong { (scenario.total_interest.to_whole()) } }
                }
                tr {
                    th { "Total paid" }
                    td { (baseline.total_paid.to_whole()) }
                    td { (scenario.total_paid.to_whole()) }
                }
                tr {
                    th { "Number of payments" }
                    td { (baseline.rows.len()) }
                    td { (scenario.rows.len()) }
                }
            }
        }
        @if has_scenario {
            p {
                strong {
                    "✔ Saves " (cmp.interest_saved.to_whole()) " in interest and pays off "
                    (duration(cmp.months_saved)) " sooner."
                }
            }
        }
    }
}

fn year_table(s: &Schedule) -> Markup {
    let this_year = today().year();
    html! {
        table {
            (thead(&[
                ("Year", Text), ("Payments", Num), ("Interest", Num), ("Principal", Num),
                ("Prepayments", Num), ("End balance", Num),
            ]))
            tbody {
                @for y in yearly_summary(s) {
                    tr class=[(y.year == this_year).then_some("current")] {
                        th { (y.year) }
                        td class="num" { (y.payments.to_whole()) }
                        td class="num" { (y.interest.to_whole()) }
                        td class="num" { (y.principal.to_whole()) }
                        td class="num" { (y.prepayments.to_whole()) }
                        td class="num" { (y.end_balance.to_whole()) }
                    }
                }
            }
        }
    }
}

fn payment_table(s: &Schedule) -> Markup {
    let today = today();
    html! {
        table {
            (thead(&[
                ("#", Num), ("Date", Text), ("Rate", Num), ("Payment", Num), ("Interest", Num),
                ("Principal", Num), ("Prepayment", Num), ("Balance", Num),
            ]))
            tbody {
                @for r in &s.rows {
                    tr class=[(r.date <= today).then_some("muted")] {
                        td class="num" { (r.number) }
                        td class="date" { (date(r.date)) }
                        td class="num" { (format!("{:.2}%", r.rate_pct)) }
                        td class="num" { (money_plain(r.payment)) }
                        td class="num" { (money_plain(r.interest)) }
                        td class="num" { (money_plain(r.principal)) }
                        td class="num" {
                            @if !r.prepayment.is_zero() { (money_plain(r.prepayment)) }
                        }
                        td class="num" { (money_plain(r.balance)) }
                    }
                }
            }
        }
    }
}

fn progress(m: &fintrack_core::model::Mortgage, scenario: &Schedule) -> Markup {
    let p = progress_at(m, scenario, today());
    let repaid = pct(p.fraction_repaid);
    let payment = money_plain(scenario.first_payment().unwrap_or_default());
    html! {
        div class="tiles" {
            (stat_tile("Balance today", whole_plain(p.balance),
                &format!("of {}", m.principal.to_whole()), ""))
            (stat_tile("Principal repaid", whole_plain(p.principal_paid), &repaid, ""))
            (stat_tile("Interest paid", whole_plain(p.interest_paid), "", ""))
            (stat_tile("Payments", html! { (p.payments_made) " made" },
                &format!("{} remaining", p.payments_remaining), ""))
            (stat_tile("Regular payment", payment, m.frequency.label(), ""))
        }
        (meter(p.fraction_repaid, "", &format!("{repaid} of principal repaid")))
    }
}

pub fn view(h: &HhCtx, l: &Loaded, q: &ProjectionQuery) -> Markup {
    let m = &l.m;
    let extra = Money::parse(&q.extra).map(|e| e.abs()).unwrap_or_default();
    let baseline = build_schedule(m, &scenario_for(&l.rates, &[], Money::ZERO));
    let inputs = scenario_for(&l.rates, &l.prepayments, extra);
    let has_scenario = !inputs.prepayments.is_empty() || extra.cents() > 0;
    let scenario = build_schedule(m, &inputs);
    let base = h.at(&format!("/mortgage/{}", m.id));
    let view_link = |v: &str| format!("{base}?extra={}&view={v}", q.extra);
    let every = q.view == "all";
    let schedule_tabs = tabs(
        &[
            ("By year", view_link("")),
            ("Every payment", view_link("all")),
        ],
        if every { "Every payment" } else { "By year" },
    );
    let chart = balance_chart(&baseline, &scenario, has_scenario);
    html! {
        @if scenario.negative_amortization {
            (notice("error", html! {
                "The payment doesn't cover the interest at some point. "
                "Increase the payment or check the rate changes."
            }))
        }
        (card("Progress today", PROGRESS_TIP, progress(m, &scenario)))
        (card("Projection", PROJECTION_TIP, comparison(&baseline, &scenario, has_scenario)))
        (card("Balance over time", "End-of-year balance.",
            figure(&chart, "Mortgage balance over time")))
        (card("Schedule", "", html! {
            (schedule_tabs)
            div class="table-wrap" {
                @if every { (payment_table(&scenario)) } @else { (year_table(&scenario)) }
            }
        }))
    }
}
