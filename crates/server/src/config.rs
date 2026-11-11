//! Server configuration, read from `FINTRACK_*` environment variables.

use anyhow::{Context, bail};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    /// URL prefix the app is served under, normalised to `""` (root) or
    /// `"/finances"` (no trailing slash).
    pub base_path: String,
    pub bind: SocketAddr,
    /// Holds `auth.sqlite` and `households/<slug>/`.
    pub data_dir: PathBuf,
    /// Mark cookies `Secure` (HTTPS only). Disable only when testing over plain HTTP.
    pub secure_cookies: bool,
    /// Lifetime of a "keep me signed in" session, renewed on activity.
    pub session_days: i64,
    /// Optional first admin, created at start-up when no admin exists.
    pub bootstrap_admin: Option<(String, String)>,
    /// Largest accepted upload (CSV or database file), in bytes.
    pub max_upload_bytes: usize,
}

pub const ENV_BASE_PATH: &str = "FINTRACK_BASE_PATH";
pub const ENV_BIND: &str = "FINTRACK_BIND";
pub const ENV_DATA_DIR: &str = "FINTRACK_DATA_DIR";
pub const ENV_SECURE_COOKIES: &str = "FINTRACK_SECURE_COOKIES";
pub const ENV_SESSION_DAYS: &str = "FINTRACK_SESSION_DAYS";
pub const ENV_ADMIN_USER: &str = "FINTRACK_ADMIN_USER";
pub const ENV_ADMIN_PASSWORD: &str = "FINTRACK_ADMIN_PASSWORD";
pub const ENV_MAX_UPLOAD_MB: &str = "FINTRACK_MAX_UPLOAD_MB";

/// `"/"`, `""`, `"finances"`, `"/finances/"` → `""` or `"/finances"`.
pub fn normalize_base_path(raw: &str) -> anyhow::Result<String> {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let ok = trimmed.split('/').all(|seg| {
        !seg.is_empty()
            && seg != "."
            && seg != ".."
            && seg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.~".contains(c))
    });
    if !ok {
        bail!(
            "{ENV_BASE_PATH} must be a URL path such as /finances (letters, digits, - _ . ~), got \
                {raw:?}"
        );
    }
    Ok(format!("/{trimmed}"))
}

fn parse_bool(name: &str, value: &str) -> anyhow::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => bail!("{name} must be true or false, got {other:?}"),
    }
}

impl Config {
    /// Defaults suitable for the container: `/data`, port 8080, served at the root.
    pub fn defaults() -> Self {
        Self {
            base_path: String::new(),
            bind: SocketAddr::from(([0, 0, 0, 0], 8080)),
            data_dir: PathBuf::from("/data"),
            secure_cookies: true,
            session_days: 30,
            bootstrap_admin: None,
            max_upload_bytes: 50 * 1024 * 1024,
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// Build from any key lookup (the environment in production, a map in tests).
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let mut c = Self::defaults();
        if let Some(v) = get(ENV_BASE_PATH) {
            c.base_path = normalize_base_path(&v)?;
        }
        if let Some(v) = get(ENV_BIND) {
            c.bind = v
                .parse()
                .with_context(|| format!("{ENV_BIND} must look like 0.0.0.0:8080, got {v:?}"))?;
        }
        if let Some(v) = get(ENV_DATA_DIR).filter(|v| !v.trim().is_empty()) {
            c.data_dir = PathBuf::from(v);
        }
        if let Some(v) = get(ENV_SECURE_COOKIES) {
            c.secure_cookies = parse_bool(ENV_SECURE_COOKIES, &v)?;
        }
        if let Some(v) = get(ENV_SESSION_DAYS) {
            c.session_days = v
                .trim()
                .parse()
                .ok()
                .filter(|d| (1..=365).contains(d))
                .with_context(|| {
                    format!(
                        "{ENV_SESSION_DAYS} must be a whole number of days between 1 and 365, got \
                    {v:?}"
                    )
                })?;
        }
        if let Some(v) = get(ENV_MAX_UPLOAD_MB) {
            let mb: usize = v
                .trim()
                .parse()
                .ok()
                .filter(|m| (1..=1024).contains(m))
                .with_context(|| {
                    format!("{ENV_MAX_UPLOAD_MB} must be between 1 and 1024, got {v:?}")
                })?;
            c.max_upload_bytes = mb * 1024 * 1024;
        }
        c.bootstrap_admin = match (get(ENV_ADMIN_USER), get(ENV_ADMIN_PASSWORD)) {
            (Some(u), Some(p)) if !u.trim().is_empty() && !p.is_empty() => {
                Some((u.trim().to_string(), p))
            }
            (Some(_), _) | (_, Some(_)) => {
                bail!("set both {ENV_ADMIN_USER} and {ENV_ADMIN_PASSWORD}, or neither")
            }
            _ => None,
        };
        Ok(c)
    }

    pub fn households_dir(&self) -> PathBuf {
        self.data_dir.join("households")
    }

    pub fn auth_db_path(&self) -> PathBuf {
        self.data_dir.join("auth.sqlite")
    }

    /// Cookie `Path` attribute: the base path, or `/` when served at the root.
    pub fn cookie_path(&self) -> &str {
        if self.base_path.is_empty() {
            "/"
        } else {
            &self.base_path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn base_path_normalisation() {
        assert_eq!(normalize_base_path("/").unwrap(), "");
        assert_eq!(normalize_base_path("").unwrap(), "");
        assert_eq!(normalize_base_path("finances").unwrap(), "/finances");
        assert_eq!(normalize_base_path("/finances/").unwrap(), "/finances");
        assert_eq!(normalize_base_path("/apps/money").unwrap(), "/apps/money");
        assert!(normalize_base_path("/../etc").is_err());
        assert!(normalize_base_path("/a b").is_err());
        assert!(normalize_base_path("/a//b").is_err());
    }

    #[test]
    fn reads_environment() {
        let env: HashMap<&str, &str> = [
            (ENV_BASE_PATH, "/finances/"),
            (ENV_BIND, "127.0.0.1:9000"),
            (ENV_SECURE_COOKIES, "false"),
            (ENV_SESSION_DAYS, "7"),
            (ENV_ADMIN_USER, "alice"),
            (ENV_ADMIN_PASSWORD, "correct horse"),
        ]
        .into();
        let c = Config::from_lookup(|k| env.get(k).map(|v| v.to_string())).unwrap();
        assert_eq!(c.base_path, "/finances");
        assert_eq!(c.cookie_path(), "/finances");
        assert_eq!(c.bind.port(), 9000);
        assert!(!c.secure_cookies);
        assert_eq!(c.session_days, 7);
        assert_eq!(
            c.bootstrap_admin,
            Some(("alice".into(), "correct horse".into()))
        );
        assert!(Config::from_lookup(|k| (k == ENV_ADMIN_USER).then(|| "bob".to_string())).is_err());
        assert!(Config::from_lookup(|k| (k == ENV_SESSION_DAYS).then(|| "0".to_string())).is_err());
    }
}
