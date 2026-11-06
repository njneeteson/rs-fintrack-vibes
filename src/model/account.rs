//! Bank accounts and the identifiers used to auto-detect them during import.

use super::Money;

str_enum! {
    pub enum AccountKind {
        Chequing => ("chequing", "Chequing"),
        Savings => ("savings", "Savings"),
        CreditCard => ("credit_card", "Credit card"),
        LineOfCredit => ("line_of_credit", "Line of credit"),
        Investment => ("investment", "Investment"),
        Cash => ("cash", "Cash"),
        Other => ("other", "Other"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub institution: String,
    pub kind: AccountKind,
    /// Full number or last 4+ digits; matched against an account-number column in CSVs.
    pub account_number: String,
    /// Case-insensitive text that appears in this account's export file names.
    pub filename_hint: String,
    /// CSV format usually used by this account (helps detection).
    pub format_id: Option<i64>,
    /// Balance before the first imported transaction.
    pub opening_balance: Money,
    pub archived: bool,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            institution: String::new(),
            kind: AccountKind::Chequing,
            account_number: String::new(),
            filename_hint: String::new(),
            format_id: None,
            opening_balance: Money::ZERO,
            archived: false,
        }
    }
}

impl Account {
    pub fn number_matches(&self, candidate: &str) -> bool {
        numbers_match(&self.account_number, candidate)
    }

    pub fn filename_matches(&self, file_name: &str) -> bool {
        let hint = self.filename_hint.trim().to_lowercase();
        !hint.is_empty() && file_name.to_lowercase().contains(&hint)
    }
}

fn trailing_digits(s: &str) -> String {
    let rev: String = s.chars().rev().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
    rev.chars().rev().collect()
}

/// Compare two account numbers, tolerating masking (`5191********1234`) and
/// partial numbers. Long unmasked numbers must match exactly; otherwise the
/// last four digits are compared.
pub fn numbers_match(a: &str, b: &str) -> bool {
    let digits = |s: &str| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    let masked = |s: &str| s.contains(['*', 'x', 'X']);
    let (da, db) = (digits(a), digits(b));
    if da.is_empty() || db.is_empty() {
        return false;
    }
    if !masked(a) && !masked(b) && da.len() >= 8 && db.len() >= 8 {
        return da == db;
    }
    let (ta, tb) = (trailing_digits(a), trailing_digits(b));
    ta.len() >= 4 && tb.len() >= 4 && ta[ta.len() - 4..] == tb[tb.len() - 4..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_number_matching() {
        assert!(numbers_match("1234", "5191********1234"));
        assert!(numbers_match("'5007661234567890'", "5007661234567890"));
        assert!(!numbers_match("5007661234567890", "5007669999567890"));
        assert!(numbers_match("4567890", "12-34567890"));
        assert!(!numbers_match("", "1234"));
        assert!(!numbers_match("1234", "5678"));
    }

    #[test]
    fn filename_hint() {
        let a = Account { filename_hint: "Visa".into(), ..Default::default() };
        assert!(a.filename_matches("my_visa_2026.csv"));
        assert!(!Account::default().filename_matches("anything.csv"));
    }
}
