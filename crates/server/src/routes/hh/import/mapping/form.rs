//! The column-mapping form and its conversion to and from a [`FormatSpec`].
//!
//! Every field is a string so a half-finished form round-trips through the
//! query string (the "Preview" button is a GET to the same page).

use fintrack_core::import::detect::header_signature;
use fintrack_core::import::reader::RawCsv;
use fintrack_core::model::{AmountColumns, DirectionColumn, FormatSpec, SkipFilter};
use serde::{Deserialize, Serialize};

pub const SINGLE: &str = "single";
pub const SPLIT: &str = "split";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MappingForm {
    /// `"1"` once the form has been submitted (otherwise start from the detected/guessed spec).
    pub edited: String,
    pub name: String,
    /// Id of the saved format being edited, if any.
    pub format_id: String,
    /// 0-based row index of the header; empty for "no header row".
    pub header_row: String,
    pub date_column: String,
    pub date_format: String,
    pub description_column: String,
    pub memo: Vec<String>,
    pub amount_mode: String,
    pub amount_column: String,
    pub negate: String,
    pub debit_column: String,
    pub credit_column: String,
    pub direction_column: String,
    pub direction_marker: String,
    pub account_column: String,
    pub balance_column: String,
    pub skip_column: String,
    pub skip_contains: String,
}

fn idx(s: &str) -> Option<usize> {
    s.trim().parse().ok()
}

fn opt(i: Option<usize>) -> String {
    i.map(|i| i.to_string()).unwrap_or_default()
}

impl MappingForm {
    pub fn from_spec(
        name: &str,
        format_id: Option<i64>,
        spec: &FormatSpec,
        header_row: Option<usize>,
    ) -> Self {
        let mut f = Self {
            edited: "1".into(),
            name: name.to_string(),
            format_id: format_id.map(|i| i.to_string()).unwrap_or_default(),
            header_row: opt(header_row),
            date_column: spec.date_column.to_string(),
            date_format: spec.date_format.clone(),
            description_column: spec.description_column.to_string(),
            memo: spec.memo_columns.iter().map(usize::to_string).collect(),
            account_column: opt(spec.account_number_column),
            balance_column: opt(spec.balance_column),
            ..Default::default()
        };
        match spec.amount {
            AmountColumns::Single { column, negate } => {
                f.amount_mode = SINGLE.into();
                f.amount_column = column.to_string();
                f.negate = if negate { "1".into() } else { String::new() };
            }
            AmountColumns::DebitCredit { debit, credit } => {
                f.amount_mode = SPLIT.into();
                f.debit_column = debit.to_string();
                f.credit_column = credit.to_string();
            }
        }
        if let Some(d) = &spec.direction {
            f.direction_column = d.column.to_string();
            f.direction_marker = d.outflow_marker.clone();
        }
        if let Some(s) = &spec.skip_filter {
            f.skip_column = s.column.to_string();
            f.skip_contains = s.contains.clone();
        }
        f
    }

    pub fn header_row(&self) -> Option<usize> {
        idx(&self.header_row)
    }

    pub fn memo_columns(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.memo.iter().filter_map(|m| idx(m)).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The spec described by the form, with header signature and column count read from `raw`.
    pub fn to_spec(&self, raw: &RawCsv) -> FormatSpec {
        let header = self.header_row().filter(|&h| h < raw.rows.len());
        let first_data = header.map_or(0, |h| h + 1);
        let amount = if self.amount_mode == SPLIT {
            AmountColumns::DebitCredit {
                debit: idx(&self.debit_column).unwrap_or(0),
                credit: idx(&self.credit_column).unwrap_or(0),
            }
        } else {
            AmountColumns::Single {
                column: idx(&self.amount_column).unwrap_or(0),
                negate: !self.negate.is_empty(),
            }
        };
        FormatSpec {
            has_header: header.is_some(),
            header_signature: header
                .map(|h| header_signature(&raw.rows[h]))
                .unwrap_or_default(),
            column_count: raw
                .rows
                .iter()
                .skip(first_data)
                .take(20)
                .map(Vec::len)
                .max()
                .unwrap_or(0),
            date_column: idx(&self.date_column).unwrap_or(0),
            date_format: self.date_format.trim().to_string(),
            description_column: idx(&self.description_column).unwrap_or(0),
            memo_columns: self.memo_columns(),
            amount,
            direction: idx(&self.direction_column)
                .filter(|_| !self.direction_marker.trim().is_empty())
                .map(|column| DirectionColumn {
                    column,
                    outflow_marker: self.direction_marker.trim().to_string(),
                }),
            account_number_column: idx(&self.account_column),
            balance_column: idx(&self.balance_column),
            skip_filter: idx(&self.skip_column)
                .filter(|_| !self.skip_contains.trim().is_empty())
                .map(|column| SkipFilter {
                    column,
                    contains: self.skip_contains.trim().to_string(),
                }),
        }
    }

    /// `?…` for re-opening the form in this state.
    pub fn to_query(&self) -> String {
        format!("?{}", serde_html_form::to_string(self).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_round_trip() {
        let raw = RawCsv {
            file_name: "x.csv".into(),
            rows: vec![
                vec![
                    "Date".into(),
                    "Payee".into(),
                    "Out".into(),
                    "In".into(),
                    "Note".into(),
                ],
                vec![
                    "2026-01-02".into(),
                    "Shop".into(),
                    "5.00".into(),
                    "".into(),
                    "n".into(),
                ],
            ],
        };
        let spec = FormatSpec {
            has_header: true,
            header_signature: vec![
                "date".into(),
                "payee".into(),
                "out".into(),
                "in".into(),
                "note".into(),
            ],
            column_count: 5,
            date_column: 0,
            date_format: "%Y-%m-%d".into(),
            description_column: 1,
            memo_columns: vec![4],
            amount: AmountColumns::DebitCredit {
                debit: 2,
                credit: 3,
            },
            direction: None,
            account_number_column: None,
            balance_column: None,
            skip_filter: Some(SkipFilter {
                column: 4,
                contains: "pending".into(),
            }),
        };
        let form = MappingForm::from_spec("Bank", Some(7), &spec, Some(0));
        assert_eq!(form.to_spec(&raw), spec);
        let q = form.to_query();
        let back: MappingForm = serde_html_form::from_str(&q[1..]).unwrap();
        assert_eq!(back, form);
    }
}
