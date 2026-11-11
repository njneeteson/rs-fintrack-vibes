//! Analytics: one module per tab, sharing [`Controls`].

mod balances;
mod breakdown;
pub mod cash_flow;
mod controls;
mod payees;
mod recurring;
mod trends;

use super::data::Data;
use crate::error::{AppError, AppResult};
use crate::extract::HhCtx;
use crate::flash;
use crate::state::Shared;
use crate::views::components::{heading, tabs};
use crate::views::{Nav, Page, hh_page};
use axum::Router;
use axum::extract::{Path, Query};
use axum::response::Response;
use axum::routing::get;
use controls::Controls;
use fintrack_core::str_enum;
use maud::{Markup, html};

str_enum! {
    pub enum Tab {
        CashFlow => ("cash-flow", "Cash flow"),
        Breakdown => ("breakdown", "Breakdown"),
        Trends => ("trends", "Trends"),
        Payees => ("payees", "Payees"),
        Recurring => ("recurring", "Recurring"),
        Balances => ("balances", "Balances & net worth"),
    }
}

const INTRO: &str = "Where money comes from and where it goes. Transfers are excluded. \
    Click a bar or a name to see its transactions.";
/// Ids of the parts the controls refresh in place.
const CONTENT: &str = "analytics-content";
const TABS: &str = "analytics-tabs";

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/analytics", get(index))
        .route("/analytics/{tab}", get(show))
}

async fn index(h: HhCtx) -> Response {
    flash::redirect(&h.at(&format!("/analytics/{}", Tab::CashFlow.key())))
}

fn tab_bar(h: &HhCtx, active: Tab, c: &Controls) -> Markup {
    // Keep the range when switching tabs.
    let query = serde_html_form::to_string(&c.range).unwrap_or_default();
    let items: Vec<(&str, String)> = Tab::ALL
        .iter()
        .map(|t| (t.label(), h.at(&format!("/analytics/{}?{query}", t.key()))))
        .collect();
    html! { div id=(TABS) { (tabs(&items, active.label())) } }
}

async fn show(
    h: HhCtx,
    Path((_, tab)): Path<(String, String)>,
    Query(c): Query<Controls>,
) -> AppResult<Page> {
    let tab = Tab::from_key(&tab).ok_or(AppError::NotFound)?;
    let data = h.hh.run(Data::load).await?;
    let range = c.date_range(&data);
    let content = match tab {
        Tab::CashFlow => cash_flow::view(&h, &data, &c, range),
        Tab::Breakdown => breakdown::view(&h, &data, &c, range),
        Tab::Trends => trends::view(&h, &data, &c, range),
        Tab::Payees => payees::view(&h, &data, &c, range),
        Tab::Recurring => recurring::view(&data),
        Tab::Balances => balances::view(&data, &c, range),
    };
    let body = html! {
        (heading("Analytics", INTRO, html! {}))
        (tab_bar(&h, tab, &c))
        (c.form(&h, tab, &data))
        div id=(CONTENT) class="refreshing" { (content) }
    };
    Ok(hh_page(&h, Nav::Analytics, tab.label(), body))
}
