//! Step 3: choose the account for each group of rows, see duplicates, commit.

mod view;

use super::session::{Done, PendingImport};
use super::{begin, lock, pending};
use crate::error::{AppError, AppResult};
use crate::extract::EditCtx;
use crate::flash;
use crate::form::{key, opt_id, text};
use crate::views::{Nav, hh_page};
use axum::Form;
use axum::extract::Path;
use axum::response::{IntoResponse, Response};
use fintrack_core::import::ImportGroup;
use fintrack_core::model::Account;
use serde::Deserialize;
use view::{Finished, Snapshot, done_page, review_page};

pub async fn show(h: EditCtx, Path((_, id)): Path<(String, String)>) -> AppResult<Response> {
    let p = pending(&h, &id)?;
    let done = {
        let g = lock(&p);
        g.done
            .clone()
            .map(|d| (g.raw.file_name.clone(), d, g.queue.len()))
    };
    if let Some((file, done, queued)) = done {
        let unmapped =
            h.hh.run(|db| {
                Ok(db
                    .transactions()?
                    .iter()
                    .filter(|t| t.is_uncategorized())
                    .count())
            })
            .await?;
        let finished = Finished {
            file: &file,
            done: &done,
            queued,
            unmapped,
        };
        let body = done_page(&h, &id, &finished);
        return Ok(hh_page(&h, Nav::Import, "Import complete", body).into_response());
    }
    let Some(s) = Snapshot::of(&lock(&p)) else {
        return Ok(flash::redirect(&h.at(&format!("/import/{id}/mapping"))));
    };
    let groups = s.groups.clone();
    let (accounts, statuses) =
        h.hh.run(move |db| {
            let statuses = groups
                .iter()
                .map(|g| {
                    g.account_id
                        .map_or(Ok(Vec::new()), |a| db.preview_duplicates(a, g))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((db.accounts()?, statuses))
        })
        .await?;
    Ok(hh_page(
        &h,
        Nav::Import,
        "Review import",
        review_page(&h, &id, &s, &accounts, &statuses),
    )
    .into_response())
}

fn group_mut<T>(
    p: &mut PendingImport,
    i: usize,
    f: impl FnOnce(&mut ImportGroup) -> T,
) -> AppResult<T> {
    let g = p
        .parsed
        .as_mut()
        .and_then(|x| x.groups.get_mut(i))
        .ok_or(AppError::NotFound)?;
    Ok(f(g))
}

#[derive(Deserialize)]
pub struct AccountForm {
    account: String,
}

pub async fn set_account(
    h: EditCtx,
    Path((_, id, i)): Path<(String, String, usize)>,
    Form(f): Form<AccountForm>,
) -> AppResult<Response> {
    let p = pending(&h, &id)?;
    group_mut(&mut lock(&p), i, |g| g.account_id = opt_id(&f.account))?;
    Ok(flash::redirect(&h.at(&format!("/import/{id}"))))
}

#[derive(Deserialize)]
pub struct NewAccountForm {
    name: String,
    #[serde(default)]
    institution: String,
    kind: String,
    #[serde(default)]
    account_number: String,
}

pub async fn new_account(
    h: EditCtx,
    Path((_, id, i)): Path<(String, String, usize)>,
    Form(f): Form<NewAccountForm>,
) -> AppResult<Response> {
    let p = pending(&h, &id)?;
    let account = Account {
        name: text(&f.name, "Account name")?,
        institution: f.institution.trim().to_string(),
        kind: key(&f.kind, "account type")?,
        account_number: f.account_number.trim().to_string(),
        ..Default::default()
    };
    let new_id = h.hh.run(move |db| db.save_account(&account)).await?;
    group_mut(&mut lock(&p), i, |g| g.account_id = Some(new_id))?;
    Ok(flash::info(
        &h.st,
        &h.at(&format!("/import/{id}")),
        "Account created.",
    ))
}

pub async fn commit(h: EditCtx, Path((_, id)): Path<(String, String)>) -> AppResult<Response> {
    let p = pending(&h, &id)?;
    let p2 = p.clone();
    h.hh.run(move |db| {
        let mut g = lock(&p2);
        let (Some(format), Some(parsed)) = (g.format.clone(), g.parsed.as_ref()) else {
            anyhow::bail!("this import isn't ready yet");
        };
        let groups = parsed.groups.clone();
        let archived = db.archive_bytes(&g.upload.file_name, &g.upload.bytes)?;
        let summary = db.commit_import(
            &g.raw.file_name,
            &archived.display().to_string(),
            &format.name,
            &groups,
        )?;
        for grp in &groups {
            if let Some(a) = grp.account_id {
                db.learn_account_identity(a, grp.account_number.as_deref(), Some(format.id))?;
            }
        }
        let categorized = db.apply_rules()?;
        let unassigned_rows = groups
            .iter()
            .filter(|x| x.account_id.is_none())
            .map(|x| x.rows.len())
            .sum();
        g.done = Some(Done {
            summary,
            categorized,
            unassigned_rows,
        });
        Ok(())
    })
    .await?;
    Ok(flash::redirect(&h.at(&format!("/import/{id}"))))
}

pub async fn next(h: EditCtx, Path((_, id)): Path<(String, String)>) -> AppResult<Response> {
    let p = pending(&h, &id)?;
    let started = h.hh.run(move |db| lock(&p).next_file(db)).await?;
    h.st.pending.remove(&id);
    Ok(match started {
        Some(s) => begin(&h, s),
        None => flash::redirect(&h.at("/import")),
    })
}

pub async fn cancel(h: EditCtx, Path((_, id)): Path<(String, String)>) -> AppResult<Response> {
    pending(&h, &id)?;
    h.st.pending.remove(&id);
    Ok(flash::info(&h.st, &h.at("/import"), "Import cancelled."))
}
