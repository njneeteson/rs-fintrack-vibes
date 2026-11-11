//! Transactions: filter, browse, override categories, bulk actions, notes, CSV export.

mod actions;
mod detail;
mod export;
pub mod filter;
mod table;

use crate::error::AppResult;
use crate::extract::HhCtx;
use crate::routes::hh::data::Data;
use crate::state::Shared;
use crate::views::components::{heading, pager};
use crate::views::format::{money, plural};
use crate::views::forms::category_select;
use crate::views::{Nav, Page, hh_page};
use axum::Router;
use axum::extract::Query;
use axum::routing::{get, post};
use filter::TxnQuery;
use fintrack_core::help::tips;
use fintrack_core::model::Money;
use maud::{Markup, html};

const PAGE_SIZE: usize = 100;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/transactions", get(list))
        .route("/transactions.csv", get(export::csv))
        .route("/transactions/bulk", post(actions::bulk))
        .route("/transactions/{id}", get(detail::show))
        .route("/transactions/{id}/category", post(actions::set_category))
        .route("/transactions/{id}/note", post(actions::set_note))
}

fn totals(rows: &[&fintrack_core::model::Transaction]) -> (Money, Money) {
    rows.iter()
        .fold((Money::ZERO, Money::ZERO), |(inflow, outflow), t| {
            if t.amount.is_negative() {
                (inflow, outflow + t.amount)
            } else {
                (inflow + t.amount, outflow)
            }
        })
}

fn bulk_bar(h: &HhCtx, data: &Data, back: &str) -> Markup {
    html! {
        form id=(table::BULK_FORM) method="post" action=(h.at("/transactions/bulk"))
            class="toolbar" {
            input type="hidden" name="back" value=(back);
            span class="muted" { "With the ticked transactions:" }
            (category_select("category", &data.cats, None, Some("Choose category…"), false))
            button type="submit" name="action" value="set" { "Set category" }
            button type="submit" name="action" value="clear"
                title="Go back to the automatic (rule) category" {
                "Clear overrides"
            }
            button type="submit" name="action" value="delete" class="danger"
                data-confirm="Delete the ticked transactions? This can't be undone." { "Delete" }
        }
    }
}

async fn list(h: HhCtx, Query(q): Query<TxnQuery>) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let rows = q.apply(&data);
    let (inflow, outflow) = totals(&rows);
    let pages = rows.len().div_ceil(PAGE_SIZE).max(1);
    let page = q.page().min(pages);
    let shown = &rows[(page - 1) * PAGE_SIZE..rows.len().min(page * PAGE_SIZE)];
    let back = q.with_page(page).href(&h);
    let body = html! {
        (heading("Transactions", tips::OVERRIDE, html! {
            a class="btn" href={ (h.at("/transactions.csv")) (q.to_query()) } hx-boost="false" {
                "Export CSV"
            }
        }))
        (q.controls(&h, &data))
        // Replaced in place when the filters change (see TxnQuery::controls).
        div id=(filter::RESULTS) class="refreshing" {
            p {
                strong { (plural(rows.len(), "transaction", "transactions")) }
                " · In " (money(inflow))
                " · Out " (money(outflow))
                " · Net " (money(inflow + outflow))
            }
            @if h.hh.can_edit() && !shown.is_empty() { (bulk_bar(&h, &data, &back)) }
            (table::table(&h, shown, &data, &back))
            (pager(page, pages, |p| q.with_page(p).href(&h)))
        }
    };
    Ok(hh_page(&h, Nav::Transactions, "Transactions", body))
}
