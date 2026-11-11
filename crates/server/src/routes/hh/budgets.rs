//! Budgets: monthly amounts per category, compared with actual spending for a period.

use super::data::{Data, today};
use crate::error::{AppError, AppResult};
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::form::{money, opt_date, opt_id};
use crate::state::Shared;
use crate::views::components::{card, empty, heading, meter, stat_tile, tip};
use crate::views::format::{money as money_cell, money_plain, pct};
use crate::views::forms::{action_button, category_select, enum_select, field, text_input};
use crate::views::{Nav, Page, hh_page};
use axum::extract::{Path, Query};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Router};
use fintrack_core::analytics::budget::{BudgetLine, average_monthly, budget_report};
use fintrack_core::analytics::{Granularity, Period};
use fintrack_core::help::tips;
use fintrack_core::model::{CategoryKind, Money};
use maud::{Markup, html};
use serde::Deserialize;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/budgets", get(show).post(add))
        .route("/budgets/{id}", post(update))
        .route("/budgets/{id}/suggest", post(suggest))
        .route("/budgets/{id}/delete", post(delete))
}

/// Progress bar plus a text status (never colour alone).
pub fn budget_meter(line: &BudgetLine) -> Markup {
    let f = line.fraction();
    if line.kind == CategoryKind::Income {
        return meter(f, "", &format!("{} of target", pct(f)));
    }
    let (class, status) = match f {
        f if f > 1.0 => ("crit", "⛔ Over budget"),
        f if f >= 0.85 => ("warn", "⚠ Close to limit"),
        _ => ("", "✔ On track"),
    };
    meter(f, class, &format!("{} · {status}", pct(f)))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PeriodQuery {
    g: String,
    at: String,
}

impl PeriodQuery {
    fn period(&self) -> Period {
        let g = Granularity::from_key(&self.g).unwrap_or(Granularity::Month);
        Period::containing(opt_date(&self.at).unwrap_or_else(today), g)
    }
}

fn period_nav(h: &HhCtx, p: Period) -> Markup {
    let link = |q: Period| {
        format!(
            "{}?g={}&at={}",
            h.at("/budgets"),
            q.granularity.key(),
            q.start
        )
    };
    html! {
        form method="get" action=(h.at("/budgets")) class="toolbar" {
            (field("Period", tips::GRANULARITY, enum_select("g", p.granularity, true)))
            input type="hidden" name="at" value=(p.start.to_string());
            a class="btn" href=(link(p.prev())) aria-label="Previous period" { "◀" }
            strong { (p.label()) }
            a class="btn" href=(link(p.next())) aria-label="Next period" { "▶" }
            a class="btn" href={ (h.at("/budgets")) "?g=" (p.granularity.key()) } { "Current" }
            noscript { button type="submit" { "Show" } }
        }
    }
}

fn summary(lines: &[BudgetLine], p: Period) -> Markup {
    let spend: Vec<&BudgetLine> = lines
        .iter()
        .filter(|l| l.kind != CategoryKind::Income)
        .collect();
    let budgeted: Money = spend.iter().map(|l| l.budget).sum();
    let actual: Money = spend.iter().map(|l| l.actual).sum();
    let over = spend.iter().filter(|l| l.fraction() > 1.0).count();
    html! {
        div class="tiles" {
            (stat_tile("Budgeted spending", money_plain(budgeted), &p.label(), ""))
            (stat_tile("Actual (budgeted categories)", money_plain(actual), "", ""))
            (stat_tile("Remaining", money_cell(budgeted - actual), "", ""))
            (stat_tile("Over budget", html! { (over) }, "categories", ""))
        }
    }
}

fn line_row(h: &HhCtx, l: &BudgetLine, monthly: Money) -> Markup {
    let base = format!("/budgets/{}", l.budget_id);
    html! {
        tr {
            td { strong { (l.label) } }
            td {
                @if h.hh.can_edit() {
                    form method="post" action=(h.at(&base)) class="inline" {
                        input type="text" name="amount" value=(monthly.to_plain()) size="8"
                            aria-label="Monthly budget";
                        " " button type="submit" class="small" { "Save" }
                    }
                } @else { (money_plain(monthly)) }
            }
            td class="num" { (money_plain(l.budget)) }
            td class="num" { (money_plain(l.actual)) }
            td class="num" { (money_cell(l.remaining())) }
            td { (budget_meter(l)) }
            td {
                @if h.hh.can_edit() {
                    form method="post" action=(h.at(&format!("{base}/suggest"))) class="inline" {
                        button type="submit" class="small"
                            title=(tips::BUDGET_SUGGEST) { "Suggest" }
                    }
                    " " (action_button(&h.at(&format!("{base}/delete")), "Remove", "small danger",
                        Some("Remove this budget?")))
                }
            }
        }
    }
}

fn add_form(h: &HhCtx, data: &Data) -> Markup {
    html! {
        form method="post" action=(h.at("/budgets")) class="toolbar" {
            (field("Category", "A top-level budget includes all its subcategories.",
                category_select("category", &data.cats, None, Some("Choose…"), false)))
            (field("Per month", "", text_input("amount", "100.00")))
            button type="submit" class="primary" { "Add budget" }
            button type="submit" name="suggest" value="1"
                title=(tips::BUDGET_SUGGEST) { "Add with suggested amount" }
        }
    }
}

async fn show(h: HhCtx, Query(q): Query<PeriodQuery>) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let p = q.period();
    let lines = budget_report(&data.budgets, &data.txns, &data.cats, p);
    let monthly = |id: i64| {
        data.budgets
            .iter()
            .find(|b| b.id == id)
            .map_or(Money::ZERO, |b| b.monthly_amount)
    };
    let body = html! {
        (heading("Budgets", tips::BUDGET, html! {}))
        (period_nav(&h, p))
        (summary(&lines, p))
        (card("Budgets", "", html! {
            @if lines.is_empty() {
                (empty("No budgets yet."))
            } @else {
                div class="table-wrap" {
                    table {
                        thead { tr {
                            th { "Category" } th { "Monthly budget" } th
                                class="num" { "This period " (tip(tips::BUDGET)) }
                            th class="num" { "Actual" } th
                                class="num" { "Remaining" } th { "Progress" } th {}
                        } }
                        tbody { @for l in &lines { (line_row(&h, l, monthly(l.budget_id))) } }
                    }
                }
            }
            @if h.hh.can_edit() { h3 { "Add a budget" } (add_form(&h, &data)) }
        }))
    };
    Ok(hh_page(&h, Nav::Budgets, "Budgets", body))
}

fn suggested(data: &Data, category_id: i64) -> Money {
    average_monthly(&data.txns, &data.cats, category_id, 3, today())
}

#[derive(Deserialize)]
pub struct AddForm {
    category: String,
    amount: String,
    suggest: Option<String>,
}

async fn add(h: EditCtx, Form(f): Form<AddForm>) -> AppResult<Response> {
    let back = h.at("/budgets");
    let Some(category) = opt_id(&f.category) else {
        return Ok(flash::error(
            &h.st,
            &back,
            "Choose a category for the budget.",
        ));
    };
    let typed = if f.suggest.is_some() {
        None
    } else {
        Some(money(&f.amount, "Monthly amount")?)
    };
    let amount =
        h.hh.run(move |db| {
            let amount = match typed {
                Some(a) => a,
                None => suggested(&Data::load(db)?, category),
            };
            db.set_budget(category, amount.abs()).map(|()| amount.abs())
        })
        .await?;
    Ok(flash::info(
        &h.st,
        &back,
        format!("Budget set to {amount} per month."),
    ))
}

/// The category a budget belongs to.
async fn budget_category(h: &EditCtx, id: i64) -> AppResult<i64> {
    let budgets = h.hh.run(|db| db.budgets()).await?;
    budgets
        .iter()
        .find(|b| b.id == id)
        .map(|b| b.category_id)
        .ok_or(AppError::NotFound)
}

#[derive(Deserialize)]
pub struct AmountForm {
    amount: String,
}

async fn update(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<AmountForm>,
) -> AppResult<Response> {
    let category = budget_category(&h, id).await?;
    let amount = money(&f.amount, "Monthly amount")?.abs();
    h.hh.run(move |db| db.set_budget(category, amount)).await?;
    Ok(flash::info(&h.st, &h.at("/budgets"), "Budget updated."))
}

async fn suggest(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    let category = budget_category(&h, id).await?;
    let amount =
        h.hh.run(move |db| {
            let amount = suggested(&Data::load(db)?, category).abs();
            db.set_budget(category, amount).map(|()| amount)
        })
        .await?;
    Ok(flash::info(
        &h.st,
        &h.at("/budgets"),
        format!("Budget set to the 3-month average, {amount}."),
    ))
}

async fn delete(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Response> {
    h.hh.run(move |db| db.delete_budget(id)).await?;
    Ok(flash::info(&h.st, &h.at("/budgets"), "Budget removed."))
}
