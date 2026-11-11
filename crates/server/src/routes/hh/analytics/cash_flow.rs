//! Income vs spending per period.

use super::controls::Controls;
use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::routes::hh::range::RangeQuery;
use crate::routes::hh::transactions::filter::TxnQuery;
use crate::views::charts::{Chart, Color, Series, figure};
use crate::views::components::{Num, Text, card, stat_tile, thead};
use crate::views::format::{money, money_plain, money_whole, pct, whole_plain};
use fintrack_core::analytics::summary::summarize;
use fintrack_core::analytics::{DateRange, PeriodTotals, income_vs_expense};
use fintrack_core::help::tips;
use fintrack_core::model::Money;
use maud::{Markup, html};

/// Income and spending bars with a net line (also used by the dashboard).
/// Clicking a period opens its transactions (within `range`).
pub fn cash_flow_chart(h: &HhCtx, totals: &[PeriodTotals], range: DateRange) -> Chart {
    let col = |f: fn(&PeriodTotals) -> Money| totals.iter().map(f).collect::<Vec<_>>();
    let links = || {
        let period = |t: &PeriodTotals| {
            let span = DateRange::new(
                t.period.start.max(range.start),
                t.period.last_day().min(range.end),
            );
            Some(
                TxnQuery {
                    range: RangeQuery::custom(span),
                    ..Default::default()
                }
                .href(h),
            )
        };
        totals.iter().map(period).collect::<Vec<_>>()
    };
    let series = |name, values: &[Money], i| Series::from_money(name, values, Color::nth(i));
    Chart {
        labels: totals.iter().map(|t| t.period.short_label()).collect(),
        bars: vec![
            series("Income", &col(|t| t.income), 0).with_links(links()),
            series("Spending", &col(|t| t.expense), 1).with_links(links()),
        ],
        stacked: false,
        lines: vec![series("Net", &col(|t| t.net()), 2).with_links(links())],
    }
}

fn rate(r: Option<f64>) -> String {
    r.map_or_else(|| "—".into(), pct)
}

pub fn view(h: &HhCtx, data: &Data, c: &Controls, range: DateRange) -> Markup {
    let s = summarize(&data.txns, range, &data.cats);
    let totals = income_vs_expense(&data.txns, range, c.granularity(), &data.cats);
    let n = totals.len().max(1) as i64;
    let avg = |m: Money| format!("avg {} / period", Money(m.cents() / n).to_whole());
    let title = format!(
        "Income vs spending ({})",
        c.granularity().label().to_lowercase()
    );
    html! {
        div class="tiles" {
            (stat_tile("Income", money_whole(s.income), &avg(s.income), ""))
            (stat_tile("Spending", whole_plain(s.expense), &avg(s.expense), ""))
            (stat_tile("Net", money_whole(s.net()), "income − spending", ""))
            (stat_tile("Savings rate", html! { (rate(s.savings_rate())) }, "", tips::SAVINGS_RATE))
        }
        (card(&title, tips::FLOW, figure(&cash_flow_chart(h, &totals, range), &title)))
        (card("By period", "", html! {
            div class="table-wrap" {
                table {
                    (thead(&[
                        ("Period", Text), ("Income", Num), ("Spending", Num), ("Net", Num),
                        ("Savings rate", Num),
                    ]))
                    tbody {
                        @for t in totals.iter().rev() {
                            tr {
                                td { (t.period.label()) }
                                td class="num" { (money(t.income)) }
                                td class="num" { (money_plain(t.expense)) }
                                td class="num" { (money(t.net())) }
                                td class="num" { (rate(t.savings_rate())) }
                            }
                        }
                        tr {
                            th { "Total" }
                            th class="num" { (money(s.income)) }
                            th class="num" { (money_plain(s.expense)) }
                            th class="num" { (money(s.net())) }
                            th class="num" { (rate(s.savings_rate())) }
                        }
                    }
                }
            }
        }))
    }
}
