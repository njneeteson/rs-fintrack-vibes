//! Create or edit a rule, with a live preview of the transactions it matches.

use super::rule_form::RuleForm;
use crate::error::{AppError, AppResult};
use crate::extract::EditCtx;
use crate::flash;
use crate::form::opt_id;
use crate::routes::hh::data::Data;
use crate::views::components::{card, heading, notice, tip};
use crate::views::format::{date, money};
use crate::views::forms::{
    Live, LiveTarget, account_select, category_select, checkbox, enum_select, field, live_form,
    number_input, text_input,
};
use crate::views::{Nav, Page, hh_page};
use axum::Form;
use axum::extract::{Path, Query};
use axum::response::Response;
use fintrack_core::categorize::{CompiledRule, MatchInput};
use fintrack_core::enum_macro::KeyEnum;
use fintrack_core::help::tips;
use fintrack_core::model::{Direction, MatchKind, Rule, RuleField, Transaction};
use maud::{Markup, html};

const PREVIEW_ROWS: usize = 50;
/// Id of the preview that refreshes as the rule changes.
const PREVIEW: &str = "rule-preview";
const PRIORITY_NOTE: &str =
    "A rule with a lower priority number that also matches will win; see the Rules tab.";

/// An enum picker selected by key (the form holds keys), defaulting to the first value.
fn key_select<T: KeyEnum + PartialEq>(name: &str, selected: &str) -> Markup {
    enum_select(name, T::from_key(selected).unwrap_or(T::all()[0]), false)
}

fn form_fields(f: &RuleForm, data: &Data) -> Markup {
    let account = opt_id(&f.account);
    let category = opt_id(&f.category);
    html! {
        div class="form-grid" {
            (field("When", tips::RULE_MATCH, key_select::<RuleField>("field", &f.field)))
            (field("…", "", key_select::<MatchKind>("match_kind", &f.match_kind)))
            (field("Text", "", html! {
                input type="text" name="pattern" value=(f.pattern) placeholder="SAVE ON FOODS"
                    required;
            }))
            (field("Direction", "", key_select::<Direction>("direction", &f.direction)))
            (field("Amount at least", tips::RULE_AMOUNT, text_input("min", &f.min)))
            (field("Amount at most", tips::RULE_AMOUNT, text_input("max", &f.max)))
            (field("Only in account", "",
                account_select("account", &data.accounts, account, Some("Any account"), false)))
            (field("Assign category", "",
                category_select("category", &data.cats, category, Some("Choose…"), false)))
            (field("Priority", tips::RULE_PRIORITY,
                number_input("priority", &f.priority, ("0", "10000"), "1")))
            (checkbox("enabled", !f.enabled.is_empty(), html! { "Enabled" }))
        }
    }
}

fn preview(rule: &Rule, data: &Data) -> Markup {
    let compiled = match CompiledRule::compile(rule) {
        Ok(c) => c,
        Err(e) => return notice("error", html! { (e) }),
    };
    let matches: Vec<_> = data
        .txns
        .iter()
        .filter(|t| compiled.matches(MatchInput::from(*t)))
        .collect();
    let unmapped = matches.iter().filter(|t| t.is_uncategorized()).count();
    let manual = matches.iter().filter(|t| t.is_manual()).count();
    let category = |t: &Transaction| {
        t.category_id()
            .map(|c| data.cats.full_name(c))
            .unwrap_or_default()
    };
    html! {
        p {
            strong { "Matches " (matches.len()) " transactions" }
            " · " (unmapped) " currently uncategorized · "
            (manual) " with manual overrides (kept) "
            (tip(PRIORITY_NOTE))
        }
        div class="table-wrap" {
            table {
                tbody {
                    @for t in matches.iter().take(PREVIEW_ROWS) {
                        tr {
                            td class="date" { (date(t.date)) }
                            td { (t.description) }
                            td class="num" { (money(t.amount)) }
                            td class="muted" { (category(t)) }
                        }
                    }
                }
            }
        }
    }
}

fn editor_page(h: &EditCtx, id: Option<i64>, f: &RuleForm, data: &Data) -> Page {
    let action = h.at(&id.map_or_else(
        || "/categorize/rules".to_string(),
        |id| format!("/categorize/rules/{id}"),
    ));
    let preview_at = h.at(&id.map_or_else(
        || "/categorize/rules/new".to_string(),
        |id| format!("/categorize/rules/{id}"),
    ));
    let title = if id.is_some() {
        "Edit mapping rule"
    } else {
        "New mapping rule"
    };
    // With JavaScript the preview follows the form as you type.
    let form = live_form(
        &preview_at,
        "stack",
        LiveTarget {
            id: PREVIEW,
            also: &[],
        },
        Live::Preview,
        html! {
            input type="hidden" name="edited" value="1";
            (form_fields(f, data))
            div class="toolbar" {
                noscript { button type="submit" { "Preview matches" } }
                button type="submit" class="primary" formmethod="post" formaction=(action) {
                    "Save rule"
                }
            }
        },
    );
    let body = html! {
        (heading(title, tips::RULES, html! {
            a class="btn" href=(h.at("/categorize/rules")) { "← Rules" }
        }))
        (card("", "", html! {
            (form)
            p class="muted" {
                "Need a new category? "
                a href=(h.at("/categories")) { "Add it on the Categories page" } "."
            }
        }))
        div id=(PREVIEW) class="refreshing" { (card("Preview", "", preview(&f.draft(), data))) }
    };
    hh_page(h, Nav::Categorize, title, body)
}

/// `/categorize/rules/new?pattern=…&category=…` (or a submitted preview).
pub async fn new(h: EditCtx, Query(q): Query<RuleForm>) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let f = if q.edited.is_empty() {
        RuleForm {
            pattern: q.pattern,
            category: q.category,
            ..RuleForm::from_rule(&Rule::default())
        }
    } else {
        q
    };
    Ok(editor_page(&h, None, &f, &data))
}

pub async fn edit(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Query(q): Query<RuleForm>,
) -> AppResult<Page> {
    let data = h.hh.run(Data::load).await?;
    let rule = data
        .rules
        .iter()
        .find(|r| r.id == id)
        .ok_or(AppError::NotFound)?;
    let f = if q.edited.is_empty() {
        RuleForm::from_rule(rule)
    } else {
        q
    };
    Ok(editor_page(&h, Some(id), &f, &data))
}

/// Validate and save, then re-apply all rules. Errors go back to the form.
async fn save(h: &EditCtx, id: i64, f: RuleForm, retry: String) -> AppResult<Response> {
    let rule = match f.to_rule(id) {
        Ok(r) => r,
        Err(AppError::BadRequest(msg)) => return Ok(flash::error(&h.st, &retry, msg)),
        Err(e) => return Err(e),
    };
    if rule.category_id == 0 {
        return Ok(flash::error(
            &h.st,
            &retry,
            "Choose the category this rule assigns.",
        ));
    }
    if let Err(e) = CompiledRule::compile(&rule) {
        return Ok(flash::error(&h.st, &retry, e));
    }
    let changed =
        h.hh.run(move |db| db.save_rule(&rule).and_then(|_| db.apply_rules()))
            .await?;
    Ok(flash::info(
        &h.st,
        &h.at("/categorize/rules"),
        format!("Rule saved: {changed} transactions recategorized."),
    ))
}

pub async fn create(h: EditCtx, Form(f): Form<RuleForm>) -> AppResult<Response> {
    let retry = h.at(&format!("/categorize/rules/new{}", f.to_query()));
    save(&h, 0, f, retry).await
}

pub async fn update(
    h: EditCtx,
    Path((_, id)): Path<(String, i64)>,
    Form(f): Form<RuleForm>,
) -> AppResult<Response> {
    let retry = h.at(&format!("/categorize/rules/{id}{}", f.to_query()));
    save(&h, id, f, retry).await
}
