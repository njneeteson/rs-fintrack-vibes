//! Request extractors that enforce who may see and change what.
//!
//! * [`Ctx`]: a signed-in user (every page behind sign-in).
//! * [`HhCtx`]: a signed-in user with access to the household named in the URL
//!   (`/h/{hh}/…`). Households the user can't access are reported as 404.
//! * [`EditCtx`]: like `HhCtx`, but the user must have edit rights (403 otherwise).
//!   Every handler that changes household data takes this.
//! * [`AdminCtx`]: an admin.

use crate::auth::CurrentUser;
use crate::auth::store::{Access, Household, Perm, User};
use crate::error::{AppError, AppResult};
use crate::flash::{Flash, FlashSlot};
use crate::households::SharedDb;
use crate::state::Shared;
use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use fintrack_core::db::Database;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub struct Ctx {
    pub st: Shared,
    pub user: Arc<User>,
    pub session: Arc<str>,
    /// Households this user can open.
    pub access: Vec<Access>,
    flash: FlashSlot,
}

impl Ctx {
    /// URL under the base path.
    pub fn url(&self, path: &str) -> String {
        self.st.url(path)
    }

    /// The pending flash message (marks it as shown). Call once, from the layout.
    pub fn take_flash(&self) -> Option<Flash> {
        self.flash.take()
    }
}

impl FromRequestParts<Shared> for Ctx {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &Shared) -> AppResult<Self> {
        let cu = parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or(AppError::Forbidden("Please sign in."))?;
        let access = st.auth.access_for(&cu.user)?;
        let flash = parts
            .extensions
            .get::<FlashSlot>()
            .cloned()
            .unwrap_or_default();
        Ok(Self {
            st: st.clone(),
            user: cu.user,
            session: cu.session,
            access,
            flash,
        })
    }
}

/// An open household and the user's permission on it.
#[derive(Clone)]
pub struct Hh {
    pub info: Household,
    pub perm: Perm,
    db: SharedDb,
}

impl Hh {
    pub fn can_edit(&self) -> bool {
        self.perm == Perm::Edit
    }

    /// Run blocking database work on a worker thread, with the household locked.
    pub async fn run<T, F>(&self, f: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Database) -> anyhow::Result<T> + Send + 'static,
    {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let guard = db.lock().unwrap_or_else(|e| e.into_inner());
            f(&guard)
        })
        .await
        .map_err(|e| AppError::Internal(e.into()))?
        .map_err(AppError::Internal)
    }
}

#[derive(Clone)]
pub struct HhCtx {
    pub cx: Ctx,
    pub hh: Hh,
}

impl Deref for HhCtx {
    type Target = Ctx;
    fn deref(&self) -> &Ctx {
        &self.cx
    }
}

impl HhCtx {
    /// URL inside this household: `at("/transactions")` → `/finances/h/home/transactions`.
    pub fn at(&self, path: &str) -> String {
        self.cx.url(&format!("/h/{}{path}", self.hh.info.slug))
    }
}

impl FromRequestParts<Shared> for HhCtx {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &Shared) -> AppResult<Self> {
        let cx = Ctx::from_request_parts(parts, st).await?;
        let Path(params) = Path::<HashMap<String, String>>::from_request_parts(parts, st)
            .await
            .map_err(|_| AppError::NotFound)?;
        let slug = params.get("hh").ok_or(AppError::NotFound)?;
        let access = cx
            .access
            .iter()
            .find(|a| &a.household.slug == slug)
            .ok_or(AppError::NotFound)?
            .clone();
        let db = st.households.get(slug).map_err(|e| {
            tracing::error!("opening household {slug}: {e:#}");
            AppError::Internal(e)
        })?;
        Ok(Self {
            cx,
            hh: Hh {
                info: access.household,
                perm: access.perm,
                db,
            },
        })
    }
}

/// A household the user may change.
#[derive(Clone)]
pub struct EditCtx(pub HhCtx);

impl Deref for EditCtx {
    type Target = HhCtx;
    fn deref(&self) -> &HhCtx {
        &self.0
    }
}

impl FromRequestParts<Shared> for EditCtx {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &Shared) -> AppResult<Self> {
        let h = HhCtx::from_request_parts(parts, st).await?;
        if !h.hh.can_edit() {
            return Err(AppError::Forbidden(
                "You have view-only access to this household. Ask an admin for edit access.",
            ));
        }
        Ok(Self(h))
    }
}

#[derive(Clone)]
pub struct AdminCtx(pub Ctx);

impl Deref for AdminCtx {
    type Target = Ctx;
    fn deref(&self) -> &Ctx {
        &self.0
    }
}

impl FromRequestParts<Shared> for AdminCtx {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, st: &Shared) -> AppResult<Self> {
        let cx = Ctx::from_request_parts(parts, st).await?;
        if !cx.user.is_admin() {
            return Err(AppError::Forbidden(
                "Only admins can manage users and households.",
            ));
        }
        Ok(Self(cx))
    }
}
