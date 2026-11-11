//! Top payees by total.

use super::controls::Controls;
use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::routes::hh::transactions::filter::TxnQuery;
use crate::views::components::{Num, Text, card, empty, thead};
use crate::views::format::{date, money_plain};
use fintrack_core::analytics::summary::top_payees;
use fintrack_core::analytics::{BucketKey, Bucketing, DateRange};
use maud::{Markup, html};

const LIMIT: usize = 100;
const TIP: &str = "Payees are grouped by the leading words of the description. \
    Click a name to see its transactions.";

pub fn view(h: &HhCtx, data: &Data, c: &Controls, range: DateRange) -> Markup {
    let payees = top_payees(&data.txns, range, c.flow(), &data.cats, LIMIT);
    if payees.is_empty() {
        return empty("No transactions in this range.");
    }
    let total: f64 = payees.iter().map(|p| p.total.dollars()).sum();
    let share = |v: f64| if total == 0.0 { 0.0 } else { v / total * 100.0 };
    let title = format!("Top payees: {}", c.flow().label().to_lowercase());
    let href = |payee: &str| {
        TxnQuery::bucket_href(h, &BucketKey::Payee(payee.into()), Bucketing::Payee, range)
    };
    card(
        &title,
        TIP,
        html! {
            div class="table-wrap" {
                table {
                    (thead(&[
                        ("Payee", Text), ("Transactions", Num), ("Total", Num), ("Average", Num),
                        ("Share", Num), ("Last", Text),
                    ]))
                    tbody {
                        @for p in &payees {
                            tr {
                                th {
                                    @if let Some(href) = href(&p.payee) {
                                        a href=(href) { (p.payee) }
                                    }
                                }
                                td class="num" { (p.count) }
                                td class="num" { (money_plain(p.total)) }
                                td class="num" { (money_plain(p.average())) }
                                td class="num" { (format!("{:.1}%", share(p.total.dollars()))) }
                                td class="date" { (date(p.last_date)) }
                            }
                        }
                    }
                }
            }
        },
    )
}
