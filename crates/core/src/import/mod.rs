//! CSV import pipeline: read → detect format → parse → group by account →
//! detect account → de-duplicate → commit (see [`crate::db`]).

pub mod builtin;
pub mod dedup;
pub mod detect;
pub mod parse;
pub mod reader;

use crate::model::Account;
use chrono::NaiveDate;
use parse::ParsedRow;

/// Rows of one import that belong to one account.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportGroup {
    /// Account number from the CSV, if the format has such a column.
    pub account_number: Option<String>,
    pub rows: Vec<ParsedRow>,
    /// Account the rows will be imported into (detected or chosen by the user).
    pub account_id: Option<i64>,
}

impl ImportGroup {
    pub fn date_range(&self) -> Option<(NaiveDate, NaiveDate)> {
        let min = self.rows.iter().map(|r| r.date).min()?;
        let max = self.rows.iter().map(|r| r.date).max()?;
        Some((min, max))
    }

    pub fn fingerprints(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| dedup::fingerprint(r.date, r.amount, &r.description))
            .collect()
    }
}

/// Split parsed rows by their account-number column (one group when there is none).
pub fn group_by_account(rows: Vec<ParsedRow>) -> Vec<ImportGroup> {
    let mut groups: Vec<ImportGroup> = Vec::new();
    for row in rows {
        match groups
            .iter_mut()
            .find(|g| g.account_number == row.account_number)
        {
            Some(g) => g.rows.push(row),
            None => groups.push(ImportGroup {
                account_number: row.account_number.clone(),
                rows: vec![row],
                account_id: None,
            }),
        }
    }
    groups
}

/// Work out which account a group of rows belongs to.
///
/// Signals, strongest first: account number in the CSV, a filename hint, and
/// finally the single account that normally uses this CSV format. A signal is
/// only used when it identifies exactly one account.
pub fn detect_account(
    accounts: &[Account],
    account_number: Option<&str>,
    file_name: &str,
    format_id: Option<i64>,
) -> Option<i64> {
    let active: Vec<&Account> = accounts.iter().filter(|a| !a.archived).collect();
    let unique = |pred: &dyn Fn(&Account) -> bool| {
        let hits: Vec<&&Account> = active.iter().filter(|a| pred(a)).collect();
        (hits.len() == 1).then(|| hits[0].id)
    };
    account_number
        .and_then(|n| unique(&|a| a.number_matches(n)))
        .or_else(|| unique(&|a| a.filename_matches(file_name)))
        .or_else(|| format_id.and_then(|f| unique(&|a| a.format_id == Some(f))))
}

/// Assign detected accounts to every group.
pub fn assign_accounts(
    groups: &mut [ImportGroup],
    accounts: &[Account],
    file_name: &str,
    format_id: Option<i64>,
) {
    for g in groups {
        g.account_id = detect_account(accounts, g.account_number.as_deref(), file_name, format_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Money;

    fn row(acct: Option<&str>) -> ParsedRow {
        ParsedRow {
            line: 1,
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            amount: Money(-100),
            description: "X".into(),
            memo: String::new(),
            balance: None,
            account_number: acct.map(String::from),
        }
    }

    #[test]
    fn groups_and_detects() {
        let accounts = vec![
            Account {
                id: 1,
                name: "Chequing".into(),
                account_number: "1234".into(),
                format_id: Some(9),
                ..Default::default()
            },
            Account {
                id: 2,
                name: "Visa".into(),
                filename_hint: "visa".into(),
                ..Default::default()
            },
            Account {
                id: 3,
                name: "Old".into(),
                account_number: "5555".into(),
                archived: true,
                ..Default::default()
            },
        ];
        let mut groups = group_by_account(vec![
            row(Some("****1234")),
            row(Some("9999")),
            row(Some("****1234")),
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].rows.len(), 2);
        assert_eq!(
            groups[0].fingerprints()[0],
            dedup::fingerprint(groups[0].rows[0].date, Money(-100), "x")
        );
        assign_accounts(&mut groups, &accounts, "export.csv", None);
        assert_eq!(groups[0].account_id, Some(1));
        assert_eq!(groups[1].account_id, None);
        assert_eq!(detect_account(&accounts, None, "MyVisa.csv", None), Some(2));
        assert_eq!(detect_account(&accounts, None, "x.csv", Some(9)), Some(1));
        assert_eq!(detect_account(&accounts, Some("5555"), "x.csv", None), None);
    }
}
