//! Create (empty, demo, or from an uploaded database), rename, back up and delete households.

use super::{HOUSEHOLDS, admin_tabs};
use crate::auth::store::Household;
use crate::error::{AppError, AppResult};
use crate::extract::AdminCtx;
use crate::flash;
use crate::routes::attachment;
use crate::state::Shared;
use crate::views::components::{Num, Text, card, empty, heading, thead};
use crate::views::forms::{action_button, field};
use crate::views::{Page, page};
use axum::extract::{Multipart, Path};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Form, Router};
use maud::{Markup, html};
use serde::Deserialize;

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/admin/households", get(list).post(create))
        .route("/admin/households/upload", post(upload))
        .route("/admin/households/{id}/rename", post(rename))
        .route("/admin/households/{id}/backup", get(backup))
        .route("/admin/households/{id}/delete", post(delete))
}

const DELETE_CONFIRM: &str =
    "Delete this household? Its folder is moved to households/.trash on the server.";
const DEMO_LABEL: &str =
    "Fill with demo data (18 months of sample transactions, budgets and a mortgage)";
const UPLOAD_TIP: &str =
    "Upload a fintrack.sqlite from the desktop app, or a backup downloaded here.";
const MEMBERS_TIP: &str =
    "Members counts people with explicit access; admins can open every household.";

fn household_row(cx: &AdminCtx, h: &Household, members: usize) -> Markup {
    let at = |action: &str| cx.url(&format!("/admin/households/{}/{action}", h.id));
    html! {
        tr {
            td { a href=(cx.url(&format!("/h/{}", h.slug))) { (h.name) } }
            td { code { "households/" (h.slug) } }
            td class="num" { (members) }
            td class="muted" { (h.created_at) }
            td {
                form method="post" action=(at("rename")) class="inline" {
                    input type="text" name="name" value=(h.name) aria-label="New name" size="14"
                        required;
                    " " button type="submit" class="small" { "Rename" }
                }
            }
            td {
                a class="btn small" href=(at("backup")) hx-boost="false" { "Download backup" }
                " "
                (action_button(&at("delete"), "Delete", "small danger", Some(DELETE_CONFIRM)))
            }
        }
    }
}

fn create_forms(cx: &AdminCtx) -> Markup {
    let name_field = field(
        "Name",
        "",
        html! { input type="text" name="name" required; },
    );
    html! {
        div class="grid" {
            (card("New household", "Starts with FinTrack's default categories.", html! {
                form method="post" action=(cx.url("/admin/households")) class="form-stack" {
                    (name_field)
                    label class="check" {
                        input type="checkbox" name="demo" value="1";
                        (DEMO_LABEL)
                    }
                    div { button type="submit" class="primary" { "Create household" } }
                }
            }))
            (card("Restore or migrate a database", UPLOAD_TIP, html! {
                form method="post" action=(cx.url("/admin/households/upload"))
                    enctype="multipart/form-data" class="form-stack" {
                    (name_field)
                    (field("Database file", "", html! {
                        input type="file" name="file" required
                            accept=".sqlite,.db,application/octet-stream";
                    }))
                    div { button type="submit" { "Upload" } }
                }
            }))
        }
    }
}

async fn list(cx: AdminCtx) -> AppResult<Page> {
    let households = cx.st.auth.households()?;
    let memberships = cx.st.auth.memberships()?;
    let members = |id: i64| memberships.iter().filter(|(_, h, _)| *h == id).count();
    let intro = "Each household is a separate FinTrack database folder on the server.";
    let body = html! {
        (heading("Admin", intro, html! {}))
        (admin_tabs(&cx, HOUSEHOLDS))
        (card("Households", MEMBERS_TIP, html! {
            @if households.is_empty() {
                (empty("No households yet. Create one below."))
            } @else {
                div class="table-wrap" {
                    table {
                        (thead(&[
                            ("Name", Text), ("Folder", Text), ("Members", Num), ("Created", Text),
                            ("", Text), ("", Text),
                        ]))
                        tbody {
                            @for h in &households { (household_row(&cx, h, members(h.id))) }
                        }
                    }
                }
            }
        }))
        (create_forms(&cx))
    };
    Ok(page(&cx, "Households", body))
}

/// Run household creation off the async runtime and report the outcome.
async fn create_and_report<F>(cx: &AdminCtx, build: F) -> AppResult<Response>
where
    F: FnOnce(&Shared) -> anyhow::Result<Household> + Send + 'static,
{
    let st = cx.st.clone();
    let result = tokio::task::spawn_blocking(move || build(&st))
        .await
        .map_err(anyhow::Error::from)?;
    let msg = result.map(|h| {
        tracing::info!("{} created household {:?}", cx.user.username, h.slug);
        format!(
            "Created “{}”. Give people access on the Access tab.",
            h.name
        )
    });
    Ok(flash::report(&cx.st, &cx.url("/admin/households"), msg))
}

#[derive(Deserialize)]
pub struct CreateForm {
    name: String,
    demo: Option<String>,
}

async fn create(cx: AdminCtx, Form(f): Form<CreateForm>) -> AppResult<Response> {
    let demo = f.demo.is_some();
    create_and_report(&cx, move |st| {
        if demo {
            st.households.create_demo(&st.auth, &f.name)
        } else {
            st.households.create_empty(&st.auth, &f.name)
        }
    })
    .await
}

async fn upload(cx: AdminCtx, mut form: Multipart) -> AppResult<Response> {
    let (mut name, mut bytes) = (String::new(), Vec::new());
    while let Some(part) = form
        .next_field()
        .await
        .map_err(|_| AppError::PayloadTooLarge)?
    {
        match part.name() {
            Some("name") => {
                name = part
                    .text()
                    .await
                    .map_err(|e| AppError::bad_request(e.to_string()))?
            }
            Some("file") => {
                bytes = part
                    .bytes()
                    .await
                    .map_err(|_| AppError::PayloadTooLarge)?
                    .to_vec()
            }
            _ => {}
        }
    }
    create_and_report(&cx, move |st| {
        st.households.create_from_upload(&st.auth, &name, &bytes)
    })
    .await
}

fn load(cx: &AdminCtx, id: i64) -> AppResult<Household> {
    cx.st.auth.household(id)?.ok_or(AppError::NotFound)
}

#[derive(Deserialize)]
pub struct RenameForm {
    name: String,
}

async fn rename(
    cx: AdminCtx,
    Path(id): Path<i64>,
    Form(f): Form<RenameForm>,
) -> AppResult<Response> {
    load(&cx, id)?;
    let result = crate::households::validate_name(&f.name)
        .and_then(|()| cx.st.auth.rename_household(id, &f.name))
        .map(|()| "Household renamed.");
    Ok(flash::report(&cx.st, &cx.url("/admin/households"), result))
}

async fn backup(cx: AdminCtx, Path(id): Path<i64>) -> AppResult<Response> {
    let h = load(&cx, id)?;
    let st = cx.st.clone();
    let slug = h.slug.clone();
    let bytes = tokio::task::spawn_blocking(move || st.households.backup(&slug))
        .await
        .map_err(anyhow::Error::from)??;
    let file = format!(
        "{}-{}.sqlite",
        h.slug,
        chrono::Local::now().format("%Y%m%d")
    );
    Ok(attachment(bytes, "application/vnd.sqlite3", &file))
}

async fn delete(cx: AdminCtx, Path(id): Path<i64>) -> AppResult<Response> {
    let h = load(&cx, id)?;
    let result = cx.st.households.delete(&cx.st.auth, &h).map(|()| {
        tracing::info!("{} deleted household {:?}", cx.user.username, h.slug);
        format!(
            "Deleted “{}”. Its folder was moved to households/.trash/ on the server.",
            h.name
        )
    });
    Ok(flash::report(&cx.st, &cx.url("/admin/households"), result))
}
