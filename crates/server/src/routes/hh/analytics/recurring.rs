//! Detected subscriptions and bills.

use crate::routes::hh::data::{Data, today};
use crate::views::components::{Num, Text, card, empty, stat_tile, thead};
use crate::views::format::{date, money_plain, money_whole};
use fintrack_core::analytics::recurring::detect_recurring;
use fintrack_core::help::tips;
use fintrack_core::model::Money;
use maud::{Markup, html};

pub fn view(data: &Data) -> Markup {
    let found = detect_recurring(&data.txns, &data.cats, today());
    if found.is_empty() {
        return empty(
            "No recurring payments detected yet. At least three regular charges are needed \
             (two for annual ones).",
        );
    }
    let monthly: Money = found.iter().map(|r| r.monthly_cost).sum();
    let yearly = format!("{} per year", Money(monthly.cents() * 12).to_whole());
    let category =
        |id: Option<i64>| id.map_or_else(|| "Uncategorized".into(), |c| data.cats.full_name(c));
    html! {
        div class="tiles" {
            (stat_tile("Recurring payments", html! { (found.len()) }, "", tips::RECURRING))
            (stat_tile("Monthly cost", money_whole(monthly), &yearly, ""))
        }
        (card("Detected subscriptions and bills", tips::RECURRING, html! {
            div class="table-wrap" {
                table {
                    (thead(&[
                        ("Payee", Text), ("Cadence", Text), ("Typical amount", Num),
                        ("Per month", Num), ("Last charge", Text), ("Next expected", Text),
                        ("Category", Text),
                    ]))
                    tbody {
                        @for r in &found {
                            tr {
                                th title={ (r.occurrences) " charges seen" } { (r.payee) }
                                td { (r.cadence.label()) }
                                td class="num" { (money_plain(r.typical_amount)) }
                                td class="num" { (money_plain(r.monthly_cost)) }
                                td class="date" { (date(r.last_date)) }
                                td class="date" { (date(r.next_expected)) }
                                td { (category(r.category_id)) }
                            }
                        }
                    }
                }
            }
        }))
    }
}
