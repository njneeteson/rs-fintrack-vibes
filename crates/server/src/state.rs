//! Shared application state.

use crate::auth::password;
use crate::auth::ratelimit::RateLimiter;
use crate::auth::store::{AuthDb, Role, UserFields};
use crate::config::Config;
use crate::households::Households;
use crate::routes::hh::import::PendingImport;
use anyhow::Context;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type Shared = Arc<AppState>;

pub struct AppState {
    pub cfg: Config,
    pub auth: AuthDb,
    pub households: Households,
    pub pending: Pending<PendingImport>,
    pub limiter: RateLimiter,
    /// One-time token for creating the first admin in the browser; `None` once an admin exists.
    pub setup_token: Mutex<Option<String>>,
}

impl AppState {
    /// Open (or create) the data directory and prepare the server.
    pub fn new(cfg: Config) -> anyhow::Result<Shared> {
        std::fs::create_dir_all(&cfg.data_dir)
            .with_context(|| format!("creating data directory {}", cfg.data_dir.display()))?;
        let auth = AuthDb::open(&cfg.auth_db_path())?;
        let households = Households::new(cfg.households_dir())?;
        for h in households.adopt_folders(&auth)? {
            tracing::info!("registered household folder {:?} as “{}”", h.slug, h.name);
        }
        let purged = auth.purge_expired_sessions()?;
        if purged > 0 {
            tracing::debug!("removed {purged} expired sessions");
        }
        if auth.count_admins()? == 0
            && let Some((user, pass)) = &cfg.bootstrap_admin
        {
            password::validate_new(pass, pass)
                .map_err(anyhow::Error::msg)
                .context("FINTRACK_ADMIN_PASSWORD")?;
            let fields = UserFields {
                username: user.clone(),
                role: Some(Role::Admin),
                ..Default::default()
            };
            auth.create_user(&fields, &password::hash(pass)?, false)?;
            tracing::info!("created admin account {user:?} from the environment");
        }
        let setup_token = (auth.count_admins()? == 0).then(crate::auth::session::new_token);
        Ok(Arc::new(Self {
            cfg,
            auth,
            households,
            pending: Pending::new(Duration::from_secs(30 * 60)),
            limiter: RateLimiter::default(),
            setup_token: Mutex::new(setup_token),
        }))
    }

    /// An absolute URL path under the base path: `url("/login")` → `/finances/login`.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.cfg.base_path)
    }

    pub fn setup_token(&self) -> Option<String> {
        self.setup_token
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn clear_setup_token(&self) {
        *self.setup_token.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

type Entries<T> = HashMap<String, (Instant, Arc<Mutex<T>>)>;

/// Short-lived server-side state keyed by an unguessable id (e.g. an import in progress).
pub struct Pending<T> {
    ttl: Duration,
    items: Mutex<Entries<T>>,
}

impl<T> Pending<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            items: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Entries<T>> {
        self.items.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert(&self, item: T) -> String {
        let id = crate::auth::session::new_token()[..24].to_string();
        let now = Instant::now();
        let mut items = self.lock();
        items.retain(|_, (t, _)| now.duration_since(*t) < self.ttl);
        items.insert(id.clone(), (now, Arc::new(Mutex::new(item))));
        id
    }

    /// The item, if it exists and hasn't expired. Using it extends its life.
    pub fn get(&self, id: &str) -> Option<Arc<Mutex<T>>> {
        let now = Instant::now();
        let mut items = self.lock();
        let (t, item) = items.get_mut(id)?;
        if now.duration_since(*t) >= self.ttl {
            items.remove(id);
            return None;
        }
        *t = now;
        Some(item.clone())
    }

    pub fn remove(&self, id: &str) {
        self.lock().remove(id);
    }
}
