//! The list of mapping rules, plus a tester ("which rule would catch this?").

use super::{RULES, header};
use crate::error::{AppError, AppResult};
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::form::opt_id;
use crate::routes::hh::data::Data;
use crate::views::components::{card, empty, notice, tip};
use crate::views::forms::{account_select, action_button, field};
use crate::views::{Nav, Page, hh_page};
use axum::extract::{Path, Query};
use axum::response::Response;
use fintrack_core::categorize::{MatchInput, RuleEngine};
use fintrack_core::help::tips;
use fintrack_core::model::{Money, Rule};
use maud::{Markup, html};
use serde::Deserialize;
use std::collections::HashMap;

const APPLIED_TIP: &str = "Transactions whose automatic category comes from this rule.";

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct RulesQuery {
    q: String,
    test: String,
    amount: String,
    account: String,
}

fn tester(h: &HhCtx, data: &Data, engine: &RuleEngine, q: &RulesQuery) -> Markup {
    let amount = Money::parse(&q.amount).unwrap_or_default();
    let input = MatchInput {
        account_id: opt_id(&q.account).unwrap_or(0),
        amount,
        description: &q.test,
        memo: "",
    };
    let result = (!q.test.trim().is_empty()).then(|| engine.first_match(input));
    html! {
        details open[!q.test.is_empty()] {
            summary { "Test a description" }
            form method="get" action=(h.at("/categorize/rules")) class="toolbar" {
                (field("Description", "As it appears in the bank's CSV.", html! {
                    input type="text" name="test" value=(q.test) placeholder="SAVE ON FOODS #123";
                }))
                (field("Amount", "Negative for money out.", html! {
                    input type="text" name="amount" value=(q.amount) placeholder="-45.20" size="9";
                }))
                (field("Account", "", account_select("account", &data.accounts, opt_id(&q.account),
                    Some("Any account"), false)))
                button type="submit" { "Test" }
            }
            @match result {
                Some(Some(r)) => p {
                    "→ " strong { (data.cats.full_name(r.category_id)) }
                    " (rule: " (r.describe()) ")"
                },
                Some(None) => p class="muted" { "No rule matches." },
                None => {},
            }
        }
    }
}

fn rule_row(h: &HhCtx, r: &Rule, data: &Data, hits: usize) -> Markup {
    let base = format!("/categorize/rules/{}", r.id);
    html! {
        tr {
            td {
                @if h.hh.can_edit() {
                    form method="post" action=(h.at(&format!("{base}/toggle"))) class="inline" {
                        input type="checkbox" checked[r.enabled] data-autosubmit
                            aria-label="Enabled";
                        noscript { button type="submit" class="small" { "Toggle" } }
                    }
                } @else { @if r.enabled { "on" } @else { "off" } }
            }
            td class="num" { (r.priority) }
            td { (r.describe()) }
            td { (r.account_id.map_or_else(|| "any".to_string(), |a| data.account_name(a))) }
            td { strong { (data.cats.full_name(r.category_id)) } }
            td class="num" { (hits) }
            td {
                @if h.hh.can_edit() {
                    a class="btn small" href=(h.at(&base)) { "Edit" }
                    " " (action_button(&h.at(&format!("{base}/delete")), "Delete", "small danger",
                        Some("Delete this rule?")))
                }
            }
        }
    }
}

pub async fn show(h: HhCtx, Query(q): Query<RulesQuery>) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let engine = RuleEngine::new(&data.rules);
    let mut hits: HashMap<i64, usize> = HashMap::new();
    for t in &data.txns {
        if let Some(r) = t.rule_id {
            *hits.entry(r).or_default() += 1;
        }
    }
    let hits_for = |id: i64| hits.get(&id).copied().unwrap_or(0);
    let needle = q.q.trim().to_lowercase();
    let shown: Vec<&Rule> = data
        .rules
        .iter()
        .filter(|r| {
            needle.is_empty()
                || r.pattern.to_lowercase().contains(&needle)
                || data
                    .cats
                    .full_name(r.category_id)
                    .to_lowercase()
                    .contains(&needle)
        })
        .collect();
    let body = html! {
        (header(&h, &data, RULES))
        @for (id, err) in &engine.errors {
            (notice("error", html! { "Rule #" (id) " is skipped because it's invalid: " (err) }))
        }
        (tester(&h, &data, &engine, &q))
        (card("Rules", tips::RULE_PRIORITY, html! {
            form method="get" action=(h.at("/categorize/rules")) class="toolbar" {
                input type="search" name="q" value=(q.q) placeholder="Filter rules…"
                    aria-label="Filter rules";
                button type="submit" { "Filter" }
            }
            @if data.rules.is_empty() {
                (empty("No rules yet. Create one from the Unmapped tab or with “New rule”."))
            } @else {
                div class="table-wrap" {
                    table {
                        thead {
                            tr {
                                th { "On" }
                                th class="num" { "Priority" }
                                th { "Rule " (tip(tips::RULE_MATCH)) }
                                th { "Account" }
                                th { "Category" }
                                th class="num" { "Applied to " (tip(APPLIED_TIP)) }
                                th {}
                            }
                        }
                        tbody {
                            @for r in shown { (rule_row(&h, r, &data, hits_for(r.id))) }
                        }
                    }
                }
            }
        }))
    };
    Ok(hh_page(&h, Nav::Categorize, "Rules", body))
}

async fn rule(h: &EditCtx, id: i64) -> AppResult<Rule> {
    let rules = h.hh.run(|db| db.rules()).await?;
    rules
        .into_iter()
        .find(|r| r.id == id)
        .ok_or(AppError::NotFound)
}

pub async fn toggle(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    let r = rule(&h, id).await?;
    let updated = Rule {
        enabled: !r.enabled,
        ..r
    };
    let changed =
        h.hh.run(move |db| db.save_rule(&updated).and_then(|_| db.apply_rules()))
            .await?;
    Ok(flash::info(
        &h.st,
        &h.at("/categorize/rules"),
        format!("Rule updated: {changed} transactions changed."),
    ))
}

pub async fn delete(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    let changed =
        h.hh.run(move |db| db.delete_rule(id).and_then(|()| db.apply_rules()))
            .await?;
    Ok(flash::info(
        &h.st,
        &h.at("/categorize/rules"),
        format!("Rule deleted: {changed} transactions changed."),
    ))
}
