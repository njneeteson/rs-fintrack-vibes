//! Parsing of submitted form values.

use crate::error::AppError;
use chrono::NaiveDate;
use fintrack_core::enum_macro::KeyEnum;
use fintrack_core::model::Money;

/// An optional id (`""` → `None`).
pub fn opt_id(value: &str) -> Option<i64> {
    value.trim().parse().ok()
}

/// A ticked checkbox arrives as any value; an unticked one is absent.
pub fn checked(value: &Option<String>) -> bool {
    value.is_some()
}

/// A required amount, e.g. `"1,234.50"` or `"$-20"`.
pub fn money(value: &str, what: &str) -> Result<Money, AppError> {
    Money::parse(value)
        .ok_or_else(|| AppError::bad_request(format!("{what}: “{value}” isn't an amount.")))
}

/// An optional amount (`""` → `None`).
pub fn opt_money(value: &str, what: &str) -> Result<Option<Money>, AppError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        money(value, what).map(Some)
    }
}

pub fn date(value: &str, what: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| {
        AppError::bad_request(format!(
            "{what}: “{value}” isn't a date \
            (YYYY-MM-DD)."
        ))
    })
}

pub fn opt_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

/// A `str_enum!` value by key.
pub fn key<T: KeyEnum>(value: &str, what: &str) -> Result<T, AppError> {
    T::from_key(value.trim())
        .ok_or_else(|| AppError::bad_request(format!("Unknown {what} “{value}”.")))
}

/// A required, trimmed, non-empty text value.
pub fn text(value: &str, what: &str) -> Result<String, AppError> {
    let t = value.trim();
    if t.is_empty() {
        return Err(AppError::bad_request(format!("{what} can't be empty.")));
    }
    Ok(t.to_string())
}

pub fn number<T: std::str::FromStr>(value: &str, what: &str) -> Result<T, AppError> {
    value
        .trim()
        .parse()
        .map_err(|_| AppError::bad_request(format!("{what}: “{value}” isn't a number.")))
}
