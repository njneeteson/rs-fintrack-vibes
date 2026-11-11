//! One group (category, account or payee) over time, against its average.

use super::controls::{Controls, decode_bucket, encode_bucket};
use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::routes::hh::transactions::filter::TxnQuery;
use crate::views::charts::{Chart, Color, Series, figure};
use crate::views::components::{card, empty, link_button, stat_tile};
use crate::views::format::money_whole;
use crate::views::forms::field;
use fintrack_core::analytics::{DateRange, breakdown};
use fintrack_core::model::Money;
use maud::{Markup, html};

pub fn view(h: &HhCtx, data: &Data, c: &Controls, range: DateRange) -> Markup {
    let b = breakdown(
        &data.txns,
        range,
        c.granularity(),
        c.flow(),
        c.bucketing(),
        data.lookup(),
    );
    let wanted = decode_bucket(&c.bucket);
    let Some(series) = b
        .buckets
        .iter()
        .find(|s| Some(&s.key) == wanted.as_ref())
        .or(b.buckets.first())
    else {
        return empty("No transactions in this range.");
    };
    let n = series.values.len().max(1) as i64;
    let avg = Money(series.total.cents() / n);
    let max_i = (0..series.values.len())
        .max_by_key(|&i| series.values[i])
        .unwrap_or(0);
    let min_i = (0..series.values.len())
        .min_by_key(|&i| series.values[i])
        .unwrap_or(0);
    let chart = Chart {
        labels: b.periods.iter().map(|p| p.short_label()).collect(),
        bars: vec![],
        stacked: false,
        lines: vec![
            Series::from_money(&series.label, &series.values, Color::nth(0)).with_links(
                TxnQuery::period_hrefs(h, &series.key, c.bucketing(), &b.periods, range),
            ),
            Series::new(
                "Average",
                vec![avg.dollars(); series.values.len()],
                Color::Other,
            ),
        ],
    };
    let title = format!("{} over time", series.label);
    // Keep the other controls when choosing a group.
    let keep = [
        ("range", &c.range.range),
        ("from", &c.range.from),
        ("to", &c.range.to),
        ("g", &c.g),
        ("flow", &c.flow),
        ("by", &c.by),
    ];
    html! {
        form method="get" action=(h.at("/analytics/trends")) class="toolbar" {
            @for (k, v) in keep { input type="hidden" name=(k) value=(v); }
            (field("Group", "", html! {
                select name="bucket" data-autosubmit {
                    @for s in &b.buckets {
                        option value=(encode_bucket(&s.key)) selected[s.key == series.key] {
                            (s.label) " (" (s.total.to_whole()) ")"
                        }
                    }
                }
            }))
            noscript { button type="submit" { "Show" } }
            @if let Some(href) = TxnQuery::bucket_href(h, &series.key, c.bucketing(), range) {
                (link_button(&href, "Show transactions", false))
            }
        }
        div class="tiles" {
            (stat_tile("Total", money_whole(series.total), "", ""))
            (stat_tile("Average per period", money_whole(avg), "", ""))
            (stat_tile("Highest", money_whole(series.values[max_i]), &b.periods[max_i].label(), ""))
            (stat_tile("Lowest", money_whole(series.values[min_i]), &b.periods[min_i].label(), ""))
        }
        (card(&title, "", figure(&chart, &title)))
    }
}
