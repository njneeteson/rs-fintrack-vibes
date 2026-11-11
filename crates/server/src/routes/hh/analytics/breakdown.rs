//! Spending or income split by category, account or payee, per period.

use super::controls::Controls;
use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::routes::hh::transactions::filter::TxnQuery;
use crate::views::charts::{BarItem, Chart, Color, Series, bar_list, figure};
use crate::views::components::{card, empty};
use fintrack_core::analytics::{Breakdown, BucketKey, BucketSeries, DateRange, breakdown};
use fintrack_core::help::tips;
use fintrack_core::model::Money;
use maud::{Markup, html};

const RANKED: usize = 15;

/// Colour of the `i`-th ranked bucket when `top_n` get their own colour.
fn color(key: &BucketKey, i: usize, top_n: usize) -> Color {
    if *key == BucketKey::Other || i >= top_n {
        Color::Other
    } else {
        Color::nth(i)
    }
}

/// Stacked bars per period; clicking a segment opens its transactions.
fn stacked(h: &HhCtx, b: &Breakdown, c: &Controls, range: DateRange) -> Chart {
    let series = |(i, s): (usize, &BucketSeries)| {
        let links = TxnQuery::period_hrefs(h, &s.key, c.bucketing(), &b.periods, range);
        Series::from_money(&s.label, &s.values, color(&s.key, i, c.top_n())).with_links(links)
    };
    Chart {
        labels: b.periods.iter().map(|p| p.short_label()).collect(),
        bars: b.buckets.iter().enumerate().map(series).collect(),
        stacked: true,
        lines: vec![],
    }
}

fn pivot(h: &HhCtx, b: &Breakdown, c: &Controls, range: DateRange) -> Markup {
    let n = b.periods.len().max(1) as i64;
    let grand = b.grand_total();
    let share = |m: Money| {
        if grand.is_zero() {
            0.0
        } else {
            m.dollars() / grand.dollars() * 100.0
        }
    };
    let cell = |m: &Money| {
        if m.is_zero() {
            "·".to_string()
        } else {
            m.to_whole()
        }
    };
    let avg = |m: Money| Money(m.cents() / n).to_whole();
    html! {
        div class="table-wrap" {
            table {
                thead {
                    tr {
                        th {}
                        @for p in &b.periods { th class="num" { (p.short_label()) } }
                        th class="num" { "Total" }
                        th class="num" { "Avg / period" }
                        th class="num" { "Share" }
                    }
                }
                tbody {
                    @for s in &b.buckets {
                        tr {
                            th {
                                @match TxnQuery::bucket_href(h, &s.key, c.bucketing(), range) {
                                    Some(href) => a href=(href) { (s.label) },
                                    None => (s.label),
                                }
                            }
                            @for v in &s.values { td class="num" { (cell(v)) } }
                            th class="num" { (s.total.to_whole()) }
                            td class="num" { (avg(s.total)) }
                            td class="num" { (format!("{:.1}%", share(s.total))) }
                        }
                    }
                    tr {
                        th { "Total" }
                        @for v in b.period_totals() { th class="num" { (v.to_whole()) } }
                        th class="num" { (grand.to_whole()) }
                        td class="num" { (avg(grand)) }
                        td class="num" { "100%" }
                    }
                }
            }
        }
    }
}

pub fn view(h: &HhCtx, data: &Data, c: &Controls, range: DateRange) -> Markup {
    let full = breakdown(
        &data.txns,
        range,
        c.granularity(),
        c.flow(),
        c.bucketing(),
        data.lookup(),
    );
    if full.buckets.is_empty() {
        return empty("No transactions in this range.");
    }
    let title = format!(
        "{} by {} ({}): total {}",
        c.flow().label(),
        c.bucketing().label().to_lowercase(),
        c.granularity().label().to_lowercase(),
        full.grand_total().to_whole()
    );
    let ranked: Vec<BarItem> = full
        .buckets
        .iter()
        .take(RANKED)
        .enumerate()
        .map(|(i, s)| BarItem {
            label: s.label.clone(),
            value: s.total,
            href: TxnQuery::bucket_href(h, &s.key, c.bucketing(), range),
            color: color(&s.key, i, c.top_n()),
        })
        .collect();
    let chart = stacked(h, &full.clone().fold_top(c.top_n()), c, range);
    let click = "Click a name to see its transactions.";
    html! {
        (card(&title, tips::BUCKETING, figure(&chart, &title)))
        (card("Ranking for the whole range", click, bar_list(&ranked)))
        (card("Table", &format!("Rows are groups, columns are periods. {click}"),
            pivot(h, &full, c, range)))
    }
}
