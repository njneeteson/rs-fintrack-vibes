//! Turning raw CSV rows into transactions using a [`FormatSpec`].

use super::detect::find_header_row;
use super::reader::RawCsv;
use crate::model::{AmountColumns, FormatSpec, Money};
use chrono::NaiveDate;

/// Date formats tried by automatic detection, in order of preference.
pub const DATE_FORMATS: &[&str] = &[
    "%Y-%m-%d",
    "%Y/%m/%d",
    "%Y%m%d",
    "%m/%d/%Y",
    "%d/%m/%Y",
    "%m/%d/%y",
    "%d/%m/%y",
    "%m-%d-%Y",
    "%d-%m-%Y",
    "%d-%b-%Y",
    "%d %b %Y",
    "%b %d, %Y",
    "%b %d %Y",
    "%B %d, %Y",
];

/// Parse a date with `format`, or with any of [`DATE_FORMATS`] when `format` is empty.
/// A trailing time component (`2026-01-31 00:00:00`) is ignored.
pub fn parse_date(input: &str, format: &str) -> Option<NaiveDate> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let try_fmt = |f: &str| {
        NaiveDate::parse_from_str(s, f).ok().or_else(|| {
            let first = s.split(['T', ' ']).next()?;
            (first != s)
                .then(|| NaiveDate::parse_from_str(first, f).ok())
                .flatten()
        })
    };
    if format.is_empty() {
        DATE_FORMATS.iter().find_map(|f| try_fmt(f))
    } else {
        try_fmt(format)
    }
}

/// First date format that parses every non-empty sample.
pub fn guess_date_format<'a>(
    samples: impl IntoIterator<Item = &'a str> + Clone,
) -> Option<&'static str> {
    DATE_FORMATS.iter().copied().find(|f| {
        let mut any = false;
        let all = samples
            .clone()
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .all(|s| {
                any = true;
                parse_date(s, f).is_some()
            });
        any && all
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsedRow {
    /// 1-based line number in the file (for error messages).
    pub line: usize,
    pub date: NaiveDate,
    pub amount: Money,
    pub description: String,
    pub memo: String,
    pub balance: Option<Money>,
    pub account_number: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RowError {
    pub line: usize,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParseOutcome {
    pub rows: Vec<ParsedRow>,
    pub errors: Vec<RowError>,
    /// Rows deliberately skipped by the format's skip filter.
    pub skipped: usize,
}

/// Index of the first data row for `spec`.
pub fn data_start(raw: &RawCsv, spec: &FormatSpec) -> Result<usize, String> {
    if spec.has_header {
        find_header_row(raw, &spec.header_signature)
            .map(|h| h + 1)
            .ok_or_else(|| "the header row of this format was not found in the file".to_string())
    } else {
        Ok(0)
    }
}

/// Parse every data row of `raw`.
pub fn parse_all(raw: &RawCsv, spec: &FormatSpec) -> Result<ParseOutcome, String> {
    let start = data_start(raw, spec)?;
    let mut out = ParseOutcome::default();
    for (i, row) in raw.rows.iter().enumerate().skip(start) {
        let line = i + 1;
        if let Some(skip) = &spec.skip_filter {
            let cell = row.get(skip.column).map(String::as_str).unwrap_or("");
            if !skip.contains.is_empty()
                && cell.to_lowercase().contains(&skip.contains.to_lowercase())
            {
                out.skipped += 1;
                continue;
            }
        }
        match parse_row(row, spec, line) {
            Ok(r) => out.rows.push(r),
            Err(message) => out.errors.push(RowError { line, message }),
        }
    }
    Ok(out)
}

/// Parse one row.
pub fn parse_row(row: &[String], spec: &FormatSpec, line: usize) -> Result<ParsedRow, String> {
    let cell = |i: usize| row.get(i).map(String::as_str).unwrap_or("");
    let date_text = cell(spec.date_column);
    let date = parse_date(date_text, &spec.date_format)
        .ok_or_else(|| format!("unrecognized date '{date_text}'"))?;

    let mut amount = match spec.amount {
        AmountColumns::Single { column, negate } => {
            let a = Money::parse(cell(column))
                .ok_or_else(|| format!("unrecognized amount '{}'", cell(column)))?;
            if negate { -a } else { a }
        }
        AmountColumns::DebitCredit { debit, credit } => {
            let d = Money::parse(cell(debit));
            let c = Money::parse(cell(credit));
            if d.is_none() && c.is_none() {
                return Err("neither a debit nor a credit amount is present".into());
            }
            c.unwrap_or_default().abs() - d.unwrap_or_default().abs()
        }
    };
    if let Some(dir) = spec
        .direction
        .as_ref()
        .filter(|d| !d.outflow_marker.trim().is_empty())
    {
        let is_out = cell(dir.column)
            .to_lowercase()
            .contains(&dir.outflow_marker.trim().to_lowercase());
        amount = if is_out { -amount.abs() } else { amount.abs() };
    }

    let memo = spec
        .memo_columns
        .iter()
        .map(|&i| cell(i))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut description = cell(spec.description_column).to_string();
    if description.is_empty() {
        description = if memo.is_empty() {
            "(no description)".into()
        } else {
            memo.clone()
        };
    }
    Ok(ParsedRow {
        line,
        date,
        amount,
        description,
        memo,
        balance: spec.balance_column.and_then(|i| Money::parse(cell(i))),
        account_number: spec
            .account_number_column
            .map(cell)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DirectionColumn, SkipFilter};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn dates() {
        assert_eq!(parse_date("2026-01-31", ""), Some(d(2026, 1, 31)));
        assert_eq!(parse_date("20260131", ""), Some(d(2026, 1, 31)));
        assert_eq!(parse_date("1/31/2026", ""), Some(d(2026, 1, 31)));
        assert_eq!(parse_date("31/01/2026", ""), Some(d(2026, 1, 31)));
        assert_eq!(parse_date("2026-01-31 00:00:00", ""), Some(d(2026, 1, 31)));
        assert_eq!(parse_date("Jan 31, 2026", ""), Some(d(2026, 1, 31)));
        assert_eq!(parse_date("hello", ""), None);
        assert_eq!(parse_date("45.67", ""), None);
        assert_eq!(
            guess_date_format(["1/2/2026", "13/2/2026"]),
            Some("%d/%m/%Y")
        );
        assert_eq!(
            guess_date_format(["1/2/2026", "2/13/2026"]),
            Some("%m/%d/%Y")
        );
    }

    #[test]
    fn rows_with_debit_credit_and_direction() {
        let row: Vec<String> = ["2026-03-01", "COFFEE", "4.50", "", "100.00"]
            .map(String::from)
            .to_vec();
        let spec = FormatSpec {
            date_column: 0,
            description_column: 1,
            amount: AmountColumns::DebitCredit {
                debit: 2,
                credit: 3,
            },
            balance_column: Some(4),
            ..Default::default()
        };
        let r = parse_row(&row, &spec, 1).unwrap();
        assert_eq!(r.amount, Money(-450));
        assert_eq!(r.balance, Some(Money(10000)));

        let row: Vec<String> = ["2026-03-01", "X", "Debit", "12.00"]
            .map(String::from)
            .to_vec();
        let spec = FormatSpec {
            amount: AmountColumns::Single {
                column: 3,
                negate: false,
            },
            direction: Some(DirectionColumn {
                column: 2,
                outflow_marker: "debit".into(),
            }),
            ..Default::default()
        };
        assert_eq!(parse_row(&row, &spec, 1).unwrap().amount, Money(-1200));
    }

    #[test]
    fn parse_all_with_header_and_skip() {
        let raw = RawCsv {
            file_name: "x.csv".into(),
            rows: vec![
                vec!["Preamble".into()],
                vec![
                    "Date".into(),
                    "Desc".into(),
                    "Amount".into(),
                    "Status".into(),
                ],
                vec![
                    "2026-01-01".into(),
                    "A".into(),
                    "-1".into(),
                    "Posted".into(),
                ],
                vec!["bad".into(), "B".into(), "-1".into(), "Posted".into()],
                vec![
                    "2026-01-02".into(),
                    "C".into(),
                    "-1".into(),
                    "Pending".into(),
                ],
            ],
        };
        let spec = FormatSpec {
            header_signature: vec![
                "date".into(),
                "desc".into(),
                "amount".into(),
                "status".into(),
            ],
            skip_filter: Some(SkipFilter {
                column: 3,
                contains: "pending".into(),
            }),
            ..Default::default()
        };
        let out = parse_all(&raw, &spec).unwrap();
        assert_eq!(out.rows.len(), 1);
        assert_eq!(out.errors.len(), 1);
        assert_eq!(out.errors[0].line, 4);
        assert_eq!(out.skipped, 1);
    }
}
