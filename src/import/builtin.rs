//! Built-in CSV format templates, seeded into every new database.
//!
//! Bank export layouts change from time to time, so these are starting points:
//! every built-in format can be edited in the import screen, and any unknown
//! layout can be mapped by hand and saved as a new format.

use crate::model::{AmountColumns, DirectionColumn, FormatSpec, SkipFilter};

fn sig(cells: &[&str]) -> Vec<String> {
    cells.iter().map(|c| c.to_string()).collect()
}

/// `(name, spec)` pairs for the formats shipped with the app.
pub fn builtin_formats() -> Vec<(String, FormatSpec)> {
    vec![
        (
            "Scotiabank — bank account (classic, no header)".into(),
            FormatSpec {
                has_header: false,
                column_count: 5,
                date_column: 0,
                date_format: "%m/%d/%Y".into(),
                amount: AmountColumns::Single { column: 1, negate: false },
                description_column: 3,
                memo_columns: vec![4],
                ..Default::default()
            },
        ),
        (
            "Scotiabank — credit card (classic, no header)".into(),
            FormatSpec {
                has_header: false,
                column_count: 3,
                date_column: 0,
                date_format: "%m/%d/%Y".into(),
                description_column: 1,
                amount: AmountColumns::Single { column: 2, negate: false },
                ..Default::default()
            },
        ),
        (
            "Scotiabank — online banking export (with header)".into(),
            FormatSpec {
                has_header: true,
                header_signature: sig(&[
                    "filter",
                    "date",
                    "description",
                    "sub-description",
                    "status",
                    "type of transaction",
                    "amount",
                ]),
                column_count: 7,
                date_column: 1,
                date_format: String::new(),
                description_column: 2,
                memo_columns: vec![3],
                amount: AmountColumns::Single { column: 6, negate: false },
                direction: Some(DirectionColumn { column: 5, outflow_marker: "debit".into() }),
                skip_filter: Some(SkipFilter { column: 4, contains: "pending".into() }),
                ..Default::default()
            },
        ),
        (
            "BMO — bank account".into(),
            FormatSpec {
                has_header: true,
                header_signature: sig(&[
                    "first bank card",
                    "transaction type",
                    "date posted",
                    "transaction amount",
                    "description",
                ]),
                column_count: 5,
                date_column: 2,
                date_format: "%Y%m%d".into(),
                description_column: 4,
                amount: AmountColumns::Single { column: 3, negate: false },
                account_number_column: Some(0),
                ..Default::default()
            },
        ),
        (
            "BMO — credit card".into(),
            FormatSpec {
                has_header: true,
                header_signature: sig(&[
                    "item #",
                    "card #",
                    "transaction date",
                    "posting date",
                    "transaction amount",
                    "description",
                ]),
                column_count: 6,
                date_column: 2,
                date_format: "%Y%m%d".into(),
                description_column: 5,
                amount: AmountColumns::Single { column: 4, negate: true },
                account_number_column: Some(1),
                ..Default::default()
            },
        ),
    ]
}
