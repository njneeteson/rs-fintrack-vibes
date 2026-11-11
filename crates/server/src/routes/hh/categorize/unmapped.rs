//! Uncategorized transactions grouped by payee: create a rule, or assign once.

use super::{UNMAPPED, header};
use crate::error::AppResult;
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::form::opt_id;
use crate::routes::hh::data::Data;
use crate::routes::hh::transactions::filter::TxnQuery;
use crate::views::components::{Num, Text, card, empty, thead, tip};
use crate::views::format::{date, money};
use crate::views::forms::category_select;
use crate::views::{Nav, Page, hh_page};
use axum::Form;
use axum::extract::Query;
use axum::response::Response;
use chrono::NaiveDate;
use fintrack_core::categorize::suggest_pattern;
use fintrack_core::help::tips;
use fintrack_core::model::{Money, Transaction};
use maud::{Markup, html};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};

const RULE_TITLE: &str = "Map every transaction like this, now and in future imports";
const ASSIGN_TITLE: &str = "Set a manual category on these transactions only";

struct Group {
    payee: String,
    count: usize,
    total: Money,
    first: NaiveDate,
    last: NaiveDate,
    samples: BTreeSet<String>,
}

/// Uncategorized transactions by suggested payee, biggest groups first.
fn groups(txns: &[Transaction]) -> Vec<Group> {
    let mut map: HashMap<String, Group> = HashMap::new();
    for t in txns.iter().filter(|t| t.is_uncategorized()) {
        let payee = suggest_pattern(&t.description);
        let g = map.entry(payee.clone()).or_insert_with(|| Group {
            payee,
            count: 0,
            total: Money::ZERO,
            first: t.date,
            last: t.date,
            samples: BTreeSet::new(),
        });
        g.count += 1;
        g.total += t.amount;
        g.first = g.first.min(t.date);
        g.last = g.last.max(t.date);
        if g.samples.len() < 4 {
            g.samples.insert(t.description.clone());
        }
    }
    let mut v: Vec<Group> = map.into_values().collect();
    v.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| b.total.abs().cmp(&a.total.abs()))
    });
    v
}

fn group_row(h: &HhCtx, g: &Group, data: &Data) -> Markup {
    let rule = format!(
        "/categorize/rules/new?pattern={}",
        utf8_percent_encode(&g.payee, NON_ALPHANUMERIC)
    );
    let examples = g.samples.iter().cloned().collect::<Vec<_>>().join("\n");
    html! {
        tr {
            td { strong { (g.payee) } " " (tip(&format!("Examples:\n{examples}"))) }
            td class="num" { (g.count) }
            td class="num" { (money(g.total)) }
            td class="muted date" { (date(g.first)) " – " (date(g.last)) }
            td {
                @if h.hh.can_edit() {
                    a class="btn small primary" href=(h.at(&rule)) title=(RULE_TITLE) {
                        "Create rule"
                    }
                    " "
                    form method="post" action=(h.at("/categorize/assign")) class="inline" {
                        input type="hidden" name="payee" value=(g.payee);
                        (category_select("category", &data.cats, None, Some("Pick…"), false))
                        " "
                        button type="submit" class="small" title=(ASSIGN_TITLE) { "Assign once" }
                    }
                }
            }
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Filter {
    q: String,
}

pub async fn show(h: HhCtx, Query(f): Query<Filter>) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let needle = f.q.trim().to_lowercase();
    let all = groups(&data.txns);
    let shown: Vec<&Group> = all
        .iter()
        .filter(|g| needle.is_empty() || g.payee.to_lowercase().contains(&needle))
        .collect();
    let as_list = TxnQuery::uncategorized().href(&h);
    let body = html! {
        (header(&h, &data, UNMAPPED))
        form method="get" action=(h.at("/categorize")) class="toolbar" {
            input type="search" name="q" value=(f.q) placeholder="Filter payees…"
                aria-label="Filter payees";
            button type="submit" { "Filter" }
            a class="btn" href=(as_list) { "Open as a list in Transactions" }
        }
        (card("Uncategorized transactions, grouped by payee", tips::UNMAPPED, html! {
            @if all.is_empty() {
                (empty("🎉 Every transaction has a category."))
            } @else {
                div class="table-wrap" {
                    table {
                        (thead(&[
                            ("Payee", Text), ("Count", Num), ("Total", Num), ("Dates", Text),
                            ("Categorize", Text),
                        ]))
                        tbody { @for g in shown { (group_row(&h, g, &data)) } }
                    }
                }
            }
        }))
    };
    Ok(hh_page(&h, Nav::Categorize, "Categorize", body))
}

#[derive(Deserialize)]
pub struct AssignForm {
    payee: String,
    category: String,
}

/// Set a manual category on every uncategorized transaction of one payee group.
pub async fn assign(h: EditCtx, Form(f): Form<AssignForm>) -> AppResult<Response> {
    let back = h.at("/categorize");
    let Some(category) = opt_id(&f.category) else {
        return Ok(flash::error(&h.st, &back, "Pick a category first."));
    };
    let payee = f.payee.clone();
    let n =
        h.hh.run(move |db| {
            let ids: Vec<i64> = db
                .transactions()?
                .iter()
                .filter(|t| t.is_uncategorized() && suggest_pattern(&t.description) == payee)
                .map(|t| t.id)
                .collect();
            db.set_manual_category(&ids, Some(category))?;
            Ok(ids.len())
        })
        .await?;
    Ok(flash::info(
        &h.st,
        &back,
        format!("Categorized {n} transactions from {}.", f.payee),
    ))
}
