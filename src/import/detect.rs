//! Recognising which known format a CSV uses, and guessing a mapping for unknown ones.

use super::parse::{guess_date_format, parse_date, parse_row};
use super::reader::RawCsv;
use crate::model::{AmountColumns, CsvFormat, FormatSpec, Money};

/// How many leading rows are searched for a header line.
const HEADER_SEARCH_ROWS: usize = 25;

/// Lowercase, trim and collapse whitespace of a header cell.
pub fn normalize_header(cell: &str) -> String {
    cell.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Normalized header cells of a row, with trailing empty cells removed.
pub fn header_signature(row: &[String]) -> Vec<String> {
    let mut sig: Vec<String> = row.iter().map(|c| normalize_header(c)).collect();
    while sig.last().is_some_and(String::is_empty) {
        sig.pop();
    }
    sig
}

pub fn find_header_row(raw: &RawCsv, signature: &[String]) -> Option<usize> {
    if signature.is_empty() {
        return None;
    }
    raw.rows.iter().take(HEADER_SEARCH_ROWS).position(|row| header_signature(row) == signature)
}

/// Result of matching a file against the known formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Detected {
    /// Index into the `formats` slice passed to [`detect_format`].
    pub format_index: usize,
}

/// Pick the best matching format. Header matches beat header-less shape
/// matches; user-defined formats beat built-ins on ties.
pub fn detect_format(raw: &RawCsv, formats: &[CsvFormat]) -> Option<Detected> {
    formats
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let base = if f.spec.has_header {
                find_header_row(raw, &f.spec.header_signature).map(|_| 100)
            } else {
                headerless_fit(raw, &f.spec).then_some(50)
            }?;
            Some((base + u32::from(!f.builtin), i))
        })
        .max_by_key(|&(score, i)| (score, std::cmp::Reverse(i)))
        .map(|(_, format_index)| Detected { format_index })
}

fn headerless_fit(raw: &RawCsv, spec: &FormatSpec) -> bool {
    let sample: Vec<_> = raw.rows.iter().take(10).collect();
    !sample.is_empty()
        && sample.iter().enumerate().all(|(i, r)| r.len() == spec.column_count && parse_row(r, spec, i + 1).is_ok())
}

/// Per-column statistics used for guessing.
struct ColumnStats {
    date_ratio: f32,
    money_ratio: f32,
    avg_text_len: f32,
}

fn column_stats(rows: &[&Vec<String>], col: usize) -> ColumnStats {
    let cells: Vec<&str> = rows.iter().map(|r| r.get(col).map(String::as_str).unwrap_or("")).filter(|c| !c.is_empty()).collect();
    let n = cells.len().max(1) as f32;
    let dates = cells.iter().filter(|c| parse_date(c, "").is_some()).count() as f32;
    let money = cells.iter().filter(|c| Money::parse(c).is_some()).count() as f32;
    let text: Vec<&&str> = cells.iter().filter(|c| Money::parse(c).is_none()).collect();
    let avg_text_len = text.iter().map(|c| c.len()).sum::<usize>() as f32 / text.len().max(1) as f32;
    ColumnStats { date_ratio: dates / n, money_ratio: money / n, avg_text_len: avg_text_len * (text.len() as f32 / n) }
}

/// A guessed mapping for an unknown file, plus the header row it assumed.
#[derive(Clone, Debug, PartialEq)]
pub struct Guess {
    pub spec: FormatSpec,
    /// Row index of the header, if any.
    pub header_row: Option<usize>,
}

/// Heuristically guess a mapping for a file no known format matched.
pub fn guess_spec(raw: &RawCsv) -> Guess {
    let first_data = raw.rows.iter().position(|row| row.iter().any(|c| parse_date(c, "").is_some())).unwrap_or(0);
    let header_row = (first_data > 0).then(|| first_data - 1);
    let headers: Vec<String> = header_row.map(|h| header_signature(&raw.rows[h])).unwrap_or_default();
    let data: Vec<&Vec<String>> = raw.rows[first_data..].iter().take(40).collect();
    let ncols = data.iter().map(|r| r.len()).max().unwrap_or(0);
    let stats: Vec<ColumnStats> = (0..ncols).map(|c| column_stats(&data, c)).collect();
    let header_has = |c: usize, words: &[&str]| headers.get(c).is_some_and(|h| words.iter().any(|w| h.contains(w)));
    let find_header = |words: &[&str]| (0..ncols).find(|&c| header_has(c, words));

    let date_column = (0..ncols)
        .filter(|&c| stats[c].date_ratio > 0.8)
        .max_by_key(|&c| (header_has(c, &["transaction date", "date posted"]), header_has(c, &["date"])))
        .unwrap_or(0);
    let balance_column = find_header(&["balance"]);
    let is_money = |c: usize| c != date_column && Some(c) != balance_column && stats[c].money_ratio > 0.6;
    let debit = (0..ncols).find(|&c| header_has(c, &["debit", "withdrawal", "money out", "paid out"]));
    let credit = (0..ncols).find(|&c| header_has(c, &["credit", "deposit", "money in", "paid in"]));
    let amount = match (debit, credit) {
        (Some(debit), Some(credit)) if debit != credit => AmountColumns::DebitCredit { debit, credit },
        _ => AmountColumns::Single {
            column: (0..ncols)
                .find(|&c| header_has(c, &["amount"]) && is_money(c))
                .or_else(|| (0..ncols).find(|&c| is_money(c)))
                .unwrap_or(ncols.saturating_sub(1)),
            negate: false,
        },
    };
    let amount_cols: Vec<usize> = match amount {
        AmountColumns::Single { column, .. } => vec![column],
        AmountColumns::DebitCredit { debit, credit } => vec![debit, credit],
    };
    let text_cols: Vec<usize> =
        (0..ncols).filter(|c| *c != date_column && !amount_cols.contains(c) && Some(*c) != balance_column).collect();
    let description_column = text_cols
        .iter()
        .copied()
        .find(|&c| header_has(c, &["description", "payee", "merchant", "details", "name"]))
        .or_else(|| text_cols.iter().copied().max_by(|&a, &b| stats[a].avg_text_len.total_cmp(&stats[b].avg_text_len)))
        .unwrap_or(1);
    let date_samples: Vec<&str> = data.iter().filter_map(|r| r.get(date_column).map(String::as_str)).collect();

    Guess {
        spec: FormatSpec {
            has_header: header_row.is_some(),
            header_signature: headers.clone(),
            column_count: ncols,
            date_column,
            date_format: guess_date_format(date_samples.iter().copied()).unwrap_or("").to_string(),
            description_column,
            memo_columns: Vec::new(),
            amount,
            direction: None,
            account_number_column: find_header(&["account", "card #", "card number"]).filter(|c| !amount_cols.contains(c)),
            balance_column,
            skip_filter: None,
        },
        header_row,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::builtin::builtin_formats;

    fn raw(lines: &str) -> RawCsv {
        RawCsv::from_bytes("t.csv", lines.as_bytes()).unwrap()
    }

    fn formats() -> Vec<CsvFormat> {
        builtin_formats()
            .into_iter()
            .enumerate()
            .map(|(i, (name, spec))| CsvFormat { id: i as i64 + 1, name, spec, builtin: true })
            .collect()
    }

    #[test]
    fn detects_builtin_formats() {
        let fs = formats();
        let bmo = raw(
            "Following data is valid as of 20260105\n\nFirst Bank Card,Transaction Type,Date Posted, Transaction Amount,Description\n'500766******1234',DEBIT,20260102,-45.67,[DN]SAVE ON FOODS\n",
        );
        let d = detect_format(&bmo, &fs).unwrap();
        assert!(fs[d.format_index].name.contains("BMO"));

        let scotia = raw("1/2/2026,-45.67,-,SAVE ON FOODS,VANCOUVER BC\n1/3/2026,2000.00,-,PAYROLL,MAYFAIR\n");
        let d = detect_format(&scotia, &fs).unwrap();
        assert!(fs[d.format_index].name.contains("Scotiabank"));

        assert!(detect_format(&raw("foo,bar\n1,2\n"), &fs).is_none());
    }

    #[test]
    fn guesses_unknown_layout() {
        let r = raw(
            "Account statement\nPosted Date,Payee,Withdrawals,Deposits,Balance\n2026-01-02,Coffee Shop,4.50,,95.50\n2026-01-03,Employer,,1000.00,1095.50\n",
        );
        let g = guess_spec(&r);
        assert_eq!(g.header_row, Some(1));
        assert_eq!(g.spec.date_column, 0);
        assert_eq!(g.spec.description_column, 1);
        assert_eq!(g.spec.amount, AmountColumns::DebitCredit { debit: 2, credit: 3 });
        assert_eq!(g.spec.balance_column, Some(4));
        assert_eq!(g.spec.date_format, "%Y-%m-%d");
    }
}
