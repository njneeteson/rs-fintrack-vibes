//! Two-level categories: add, rename, move, change kind, delete (with reassignment).

use super::data::Data;
use crate::error::{AppError, AppResult};
use crate::extract::{EditCtx, HhCtx};
use crate::flash;
use crate::form::{key, opt_id, text};
use crate::state::Shared;
use crate::views::components::{card, heading};
use crate::views::format::money_plain;
use crate::views::forms::{category_select, enum_select, field};
use crate::views::{Nav, Page, hh_page};
use axum::extract::Path;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Router};
use fintrack_core::help::tips;
use fintrack_core::model::{Category, CategoryKind, CategoryTree, Money};
use maud::{Markup, html};
use serde::Deserialize;
use std::collections::HashMap;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/categories", get(list).post(add))
        .route("/categories/{id}", get(edit_page).post(update))
        .route("/categories/{id}/delete", post(delete))
}

/// `(transactions, total)` per category, with subcategories rolled into their parent.
fn usage(data: &Data) -> HashMap<i64, (usize, Money)> {
    let mut used: HashMap<i64, (usize, Money)> = HashMap::new();
    for t in &data.txns {
        let Some(c) = t.category_id() else { continue };
        let top = data.cats.top_id(c);
        for id in if top == c { vec![c] } else { vec![c, top] } {
            let e = used.entry(id).or_default();
            e.0 += 1;
            e.1 += t.amount.abs();
        }
    }
    used
}

fn usage_text(used: &HashMap<i64, (usize, Money)>, id: i64) -> Markup {
    let (n, total) = used.get(&id).copied().unwrap_or_default();
    html! { span class="muted" { (n) " transactions · " (money_plain(total)) } }
}

fn top_card(
    h: &HhCtx,
    top: &Category,
    cats: &CategoryTree,
    used: &HashMap<i64, (usize, Money)>,
) -> Markup {
    let edit = |c: &Category| {
        let href = h.at(&format!("/categories/{}", c.id));
        html! { @if h.hh.can_edit() { a class="btn small" href=(href) { "Edit" } } }
    };
    html! {
        tr {
            th { (top.name) } td { (usage_text(used, top.id)) } td { (edit(top)) }
        }
        @for c in cats.children(top.id) {
            tr { td { "\u{2003}" (c.name) } td { (usage_text(used, c.id)) } td { (edit(c)) } }
        }
        @if h.hh.can_edit() {
            tr {
                td colspan="3" {
                    form method="post" action=(h.at("/categories")) class="inline" {
                        input type="hidden" name="parent" value=(top.id);
                        input type="hidden" name="kind" value=(top.kind.key());
                        "\u{2003}"
                        input type="text" name="name"
                            placeholder={ "New subcategory of " (top.name) } required
                            aria-label={ "New subcategory of " (top.name) };
                        " " button type="submit" class="small" { "Add" }
                    }
                }
            }
        }
    }
}

async fn list(h: HhCtx) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let used = usage(&data);
    let body = html! {
        (heading("Categories", tips::CATEGORY_KIND, html! {}))
        @if h.hh.can_edit() {
            (card("New top-level category", "", html! {
                form method="post" action=(h.at("/categories")) class="toolbar" {
                    (field("Name", "", html! { input type="text" name="name" required; }))
                    (field("Kind", tips::CATEGORY_KIND,
                        enum_select("kind", CategoryKind::Expense, false)))
                    button type="submit" class="primary" { "Add category" }
                }
            }))
        }
        @for kind in CategoryKind::ALL {
            (card(&format!("{} categories", kind.label()), "", html! {
                div class="table-wrap" {
                    table {
                        @for top in data.cats.top_level().filter(|c| c.kind == *kind) {
                            (top_card(&h, top, &data.cats, &used))
                        }
                    }
                }
            }))
        }
    };
    Ok(hh_page(&h, Nav::Categories, "Categories", body))
}

#[derive(Deserialize)]
pub struct CategoryForm {
    name: String,
    #[serde(default)]
    parent: String,
    #[serde(default)]
    kind: String,
}

impl CategoryForm {
    fn kind(&self) -> AppResult<CategoryKind> {
        if self.kind.is_empty() {
            Ok(CategoryKind::Expense)
        } else {
            key(&self.kind, "category kind")
        }
    }
}

async fn add(h: EditCtx, Form(f): Form<CategoryForm>) -> AppResult<Response> {
    let name = text(&f.name, "Category name")?;
    let (parent, kind) = (opt_id(&f.parent), f.kind()?);
    let result =
        h.hh.run(move |db| Ok(db.add_category(&name, parent, kind)))
            .await?;
    Ok(flash::report(
        &h.st,
        &h.at("/categories"),
        result.map(|_| "Category added."),
    ))
}

fn edit_form(h: &HhCtx, c: &Category, cats: &CategoryTree) -> Markup {
    let parents = CategoryTree::new(cats.top_level().filter(|p| p.id != c.id).cloned().collect());
    let has_children = cats.children(c.id).next().is_some();
    html! {
        form method="post" action=(h.at(&format!("/categories/{}", c.id))) class="form-stack" {
            (field("Name", "", html! { input type="text" name="name" value=(c.name) required; }))
            @if has_children {
                p class="muted" { "This category has subcategories, so it stays top-level." }
            } @else {
                (field("Parent", "Subcategories inherit their parent's kind.",
                    category_select("parent", &parents, c.parent_id, Some("(top level)"), false)))
            }
            (field("Kind (top level only)", tips::CATEGORY_KIND,
                enum_select("kind", c.kind, false)))
            div { button type="submit" class="primary" { "Save" } }
        }
    }
}

fn delete_form(h: &HhCtx, c: &Category, data: &Data) -> Markup {
    let affected: Vec<i64> = std::iter::once(c.id)
        .chain(data.cats.children(c.id).map(|x| x.id))
        .collect();
    let txns = data
        .txns
        .iter()
        .filter(|t| t.category_id().is_some_and(|x| affected.contains(&x)))
        .count();
    let rules = data
        .rules
        .iter()
        .filter(|r| affected.contains(&r.category_id))
        .count();
    let targets = CategoryTree::new(
        data.cats
            .ordered()
            .into_iter()
            .filter(|x| !affected.contains(&x.id))
            .cloned()
            .collect(),
    );
    let action = h.at(&format!("/categories/{}/delete", c.id));
    let none = Some("Nothing (leave uncategorized)");
    html! {
        form method="post" action=(action) class="form-stack" {
            @if affected.len() > 1 {
                p { "Its " (affected.len() - 1) " subcategories will be deleted too." }
            }
            p { (txns) " transactions and " (rules) " rules use it." }
            (field("Move them to", tips::CATEGORY_DELETE,
                category_select("reassign", &targets, None, none, false)))
            div {
                button type="submit" class="danger" data-confirm="Delete this category?" {
                    "Delete category"
                }
            }
        }
    }
}

async fn edit_page(h: EditCtx, Path((_, id)): Path<(String, i64)>) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let c = data.cats.get(id).ok_or(AppError::NotFound)?;
    let body = html! {
        (heading(&format!("Edit “{}”", data.cats.full_name(id)), "", html! {
            a class="btn" href=(h.at("/categories")) { "← Categories" }
        }))
        div class="grid" {
            (card("Category", "", edit_form(&h, c, &data.cats)))
            (card("Delete", tips::CATEGORY_DELETE, delete_form(&h, c, &data)))
        }
    };
    Ok(hh_page(&h, Nav::Categories, "Edit category", body))
}

async fn update(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<CategoryForm>,
) -> AppResult<Response> {
    let name = text(&f.name, "Category name")?;
    let (parent, kind) = (opt_id(&f.parent).filter(|p| *p != id), f.kind()?);
    let result =
        h.hh.run(move |db| Ok(db.update_category(id, &name, parent, kind)))
            .await?;
    Ok(flash::report(
        &h.st,
        &h.at("/categories"),
        result.map(|()| "Category updated."),
    ))
}

#[derive(Deserialize)]
pub struct DeleteForm {
    #[serde(default)]
    reassign: String,
}

async fn delete(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<DeleteForm>,
) -> AppResult<Response> {
    let reassign = opt_id(&f.reassign);
    let result =
        h.hh.run(move |db| Ok(db.delete_category(id, reassign)))
            .await?;
    Ok(flash::report(
        &h.st,
        &h.at("/categories"),
        result.map(|()| "Category deleted."),
    ))
}
