//! CSV format definitions: how the columns of a bank export map onto transaction fields.

use serde::{Deserialize, Serialize};

/// How the amount is represented in the CSV.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum AmountColumns {
    /// One signed column. `negate` flips the sign (credit cards often export
    /// purchases as positive numbers).
    Single { column: usize, negate: bool },
    /// Separate debit (money out) and credit (money in) columns.
    DebitCredit { debit: usize, credit: usize },
}

/// A column whose text says whether the row is money out (e.g. `Debit`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectionColumn {
    pub column: usize,
    /// Case-insensitive text marking an outflow; other rows are inflows.
    pub outflow_marker: String,
}

/// Skip rows whose `column` contains `contains` (e.g. `Pending` status rows).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkipFilter {
    pub column: usize,
    pub contains: String,
}

/// Column mapping for one CSV layout. Column indexes are zero-based.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatSpec {
    /// The file has a header row (which may be preceded by preamble lines).
    pub has_header: bool,
    /// Normalized header cells used to recognise this format.
    pub header_signature: Vec<String>,
    /// Number of columns, used to recognise header-less formats.
    pub column_count: usize,
    pub date_column: usize,
    /// chrono format string, or empty for automatic detection.
    pub date_format: String,
    pub description_column: usize,
    pub memo_columns: Vec<usize>,
    pub amount: AmountColumns,
    pub direction: Option<DirectionColumn>,
    pub account_number_column: Option<usize>,
    pub balance_column: Option<usize>,
    pub skip_filter: Option<SkipFilter>,
}

impl Default for FormatSpec {
    fn default() -> Self {
        Self {
            has_header: true,
            header_signature: Vec::new(),
            column_count: 0,
            date_column: 0,
            date_format: String::new(),
            description_column: 1,
            memo_columns: Vec::new(),
            amount: AmountColumns::Single { column: 2, negate: false },
            direction: None,
            account_number_column: None,
            balance_column: None,
            skip_filter: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CsvFormat {
    pub id: i64,
    pub name: String,
    pub spec: FormatSpec,
    /// Shipped with the app (still editable).
    pub builtin: bool,
}
