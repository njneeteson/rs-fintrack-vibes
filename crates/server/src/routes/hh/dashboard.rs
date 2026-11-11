//! At-a-glance overview of the current month.

use super::analytics::cash_flow::cash_flow_chart;
use super::budgets::budget_meter;
use super::data::{Data, today};
use super::transactions::filter::TxnQuery;
use crate::error::AppResult;
use crate::extract::HhCtx;
use crate::views::charts::{BarItem, Color, bar_list, figure};
use crate::views::components::{card, empty, heading, link_button, notice, stat_tile};
use crate::views::format::{change, money, money_whole, pct, short_date, whole_plain};
use crate::views::{Nav, Page, hh_page};
use chrono::{Days, Months};
use fintrack_core::analytics::balances::balance_on;
use fintrack_core::analytics::budget::budget_report;
use fintrack_core::analytics::recurring::detect_recurring;
use fintrack_core::analytics::summary::summarize;
use fintrack_core::analytics::{
    Bucketing, DateRange, Flow, Granularity, Period, breakdown, income_vs_expense,
};
use fintrack_core::help::tips;
use fintrack_core::model::Money;
use maud::{Markup, html};

fn tiles(data: &Data, this: DateRange, last: DateRange) -> Markup {
    let s = summarize(&data.txns, this, &data.cats);
    let p = summarize(&data.txns, last, &data.cats);
    let rate = |r: Option<f64>| r.map_or_else(|| "—".to_string(), pct);
    let vs = |now, before| change(now, before, "last month");
    html! {
        div class="tiles" {
            (stat_tile("Income", money_whole(s.income), &vs(s.income, p.income), ""))
            (stat_tile("Spending", whole_plain(s.expense), &vs(s.expense, p.expense), tips::FLOW))
            (stat_tile("Net", money_whole(s.net()), &vs(s.net(), p.net()), ""))
            (stat_tile("Savings rate", html! { (rate(s.savings_rate())) },
                &format!("last month {}", rate(p.savings_rate())), tips::SAVINGS_RATE))
        }
    }
}

fn spending(h: &HhCtx, data: &Data, this: DateRange) -> Markup {
    let b = breakdown(
        &data.txns,
        this,
        Granularity::Month,
        Flow::Expense,
        Bucketing::TopCategory,
        data.lookup(),
    );
    let items: Vec<BarItem> = b
        .buckets
        .iter()
        .take(8)
        .enumerate()
        .map(|(i, x)| BarItem {
            label: x.label.clone(),
            value: x.total,
            href: TxnQuery::bucket_href(h, &x.key, Bucketing::TopCategory, this),
            color: Color::nth(i),
        })
        .collect();
    card(
        "Spending this month",
        "Top-level categories. Click one to see its transactions.",
        html! {
            @if items.is_empty() {
                (empty("No spending yet this month."))
            } @else {
                (bar_list(&items))
            }
        },
    )
}

fn budgets(h: &HhCtx, data: &Data, month: Period) -> Markup {
    let mut lines = budget_report(&data.budgets, &data.txns, &data.cats, month);
    lines.sort_by(|a, b| b.fraction().total_cmp(&a.fraction()));
    card(
        "Budgets this month",
        tips::BUDGET,
        html! {
            @if lines.is_empty() { (empty("No budgets set.")) }
            @for l in lines.iter().take(8) {
                div {
                    strong { (l.label) } " "
                    span class="muted" { (l.actual.to_whole()) " / " (l.budget.to_whole()) }
                }
                (budget_meter(l))
            }
            p { a href=(h.at("/budgets")) { "All budgets →" } }
        },
    )
}

fn cash_flow(h: &HhCtx, data: &Data, month: Period, today: chrono::NaiveDate) -> Markup {
    let range = DateRange::new(month.start - Months::new(11), today);
    let totals = income_vs_expense(&data.txns, range, Granularity::Month, &data.cats);
    let chart = cash_flow_chart(h, &totals, range);
    card(
        "Last 12 months",
        "Income and spending per month; the line is net.",
        figure(&chart, "Last 12 months"),
    )
}

fn accounts(data: &Data, today: chrono::NaiveDate) -> Markup {
    let active: Vec<_> = data.accounts.iter().filter(|a| !a.archived).collect();
    let balances: Vec<Money> = active
        .iter()
        .map(|a| balance_on(a, &data.txns, today))
        .collect();
    card(
        "Accounts",
        tips::OPENING_BALANCE,
        html! {
            table {
                @for (a, b) in active.iter().zip(&balances) {
                    tr { td { (a.name) } td class="num" { (money(*b)) } }
                }
                tr { th { "Total" } th class="num" { (money(balances.iter().copied().sum())) } }
            }
        },
    )
}

fn upcoming(data: &Data, today: chrono::NaiveDate) -> Markup {
    let horizon = today + Days::new(30);
    let soon: Vec<_> = detect_recurring(&data.txns, &data.cats, today)
        .into_iter()
        .filter(|r| r.next_expected <= horizon)
        .collect();
    card(
        "Upcoming recurring payments (next 30 days)",
        tips::RECURRING,
        html! {
            @if soon.is_empty() { (empty("Nothing detected.")) }
            table {
                @for r in soon.iter().take(8) {
                    tr {
                        td { (r.payee) }
                        td { (short_date(r.next_expected)) }
                        td class="num" { (r.typical_amount) }
                    }
                }
            }
        },
    )
}

fn onboarding(h: &HhCtx) -> Markup {
    card(
        "Welcome! Let's get your finances in.",
        "",
        html! {
            ol {
                li {
                    "Import a CSV export from your bank. "
                    "The format and account are detected automatically."
                }
                li {
                    "Categorize: create mapping rules so future imports are categorized "
                    "automatically."
                }
                li { "Explore the dashboard, analytics and budgets, and track your mortgage." }
            }
            div class="actions" {
                @if h.hh.can_edit() { (link_button(&h.at("/import"), "Import CSV", true)) }
                (link_button(&h.at("/help"), "Read the guide", false))
            }
        },
    )
}

pub async fn show(h: HhCtx) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let today = today();
    let month = Period::containing(today, Granularity::Month);
    let this = DateRange::new(month.start, today);
    let last = DateRange::new(month.prev().start, month.prev().last_day());
    let unmapped = data.uncategorized();
    let body = html! {
        (heading(&h.hh.info.name, &format!("This month: {}", month.label()), html! {}))
        @if data.txns.is_empty() {
            (onboarding(&h))
        } @else {
            (tiles(&data, this, last))
            @if unmapped > 0 {
                (notice("warn", html! {
                    (unmapped) " transactions are uncategorized. "
                    a href=(h.at("/categorize")) { "Categorize now" }
                }))
            }
            div class="grid" {
                (spending(&h, &data, this))
                (budgets(&h, &data, month))
            }
            (cash_flow(&h, &data, month, today))
            div class="grid" {
                (accounts(&data, today))
                (upcoming(&data, today))
            }
        }
    };
    Ok(hh_page(&h, Nav::Dashboard, "Dashboard", body))
}
