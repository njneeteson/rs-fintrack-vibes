//! SQL conversions for domain types.

use crate::enum_macro::KeyEnum;
use crate::model::Money;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Row, RowIndex};

impl ToSql for Money {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.cents()))
    }
}

impl FromSql for Money {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(Money::from_cents)
    }
}

/// Read a string-keyed enum column.
pub fn get_enum<T: KeyEnum, I: RowIndex>(row: &Row<'_>, idx: I) -> rusqlite::Result<T> {
    let key: String = row.get(idx)?;
    T::from_key(&key).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(FromSqlError::Other(format!("unknown value '{key}'").into())),
        )
    })
}
