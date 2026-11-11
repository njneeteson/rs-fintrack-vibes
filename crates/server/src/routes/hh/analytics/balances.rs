//! Account balances over time and net worth.

use super::controls::Controls;
use crate::routes::hh::data::{Data, today};
use crate::views::charts::{Chart, Color, MAX_SERIES, Series, figure};
use crate::views::components::{card, empty, stat_tile};
use crate::views::format::{money, money_whole};
use fintrack_core::analytics::DateRange;
use fintrack_core::analytics::balances::{balance_on, balance_series};
use fintrack_core::analytics::period::periods_covering;
use fintrack_core::help::tips;
use fintrack_core::model::Money;
use maud::{Markup, html};

pub fn view(data: &Data, c: &Controls, range: DateRange) -> Markup {
    let accounts: Vec<_> = data.accounts.iter().filter(|a| !a.archived).collect();
    if accounts.is_empty() {
        return empty("No accounts yet.");
    }
    let periods = periods_covering(range.start, range.end, c.granularity());
    let labels: Vec<String> = periods.iter().map(|p| p.short_label()).collect();
    let per_account: Vec<Vec<Money>> = accounts
        .iter()
        .map(|a| balance_series(a, &data.txns, &periods))
        .collect();
    let net: Vec<Money> = (0..periods.len())
        .map(|i| per_account.iter().map(|s| s[i]).sum())
        .collect();
    let now: Vec<Money> = accounts
        .iter()
        .map(|a| balance_on(a, &data.txns, today()))
        .collect();
    let net_chart = Chart {
        labels: labels.clone(),
        lines: vec![Series::from_money("Net worth", &net, Color::nth(0))],
        ..Default::default()
    };
    let by_account = Chart {
        labels,
        lines: accounts
            .iter()
            .zip(&per_account)
            .take(MAX_SERIES)
            .enumerate()
            .map(|(i, (a, s))| Series::from_money(&a.name, s, Color::nth(i)))
            .collect(),
        ..Default::default()
    };
    html! {
        div class="tiles" {
            (stat_tile("Net worth (tracked accounts)", money_whole(now.iter().copied().sum()),
                "sum of account balances today", tips::OPENING_BALANCE))
        }
        (card("Net worth", "End-of-period sum of all non-archived account balances.",
            figure(&net_chart, "Net worth")))
        (card("Balance by account", "Opening balance plus all imported transactions.", html! {
            (figure(&by_account, "Balance by account"))
            table {
                @for (a, b) in accounts.iter().zip(&now) {
                    tr { td { (a.name) } td class="num" { (money(*b)) } }
                }
            }
        }))
    }
}
