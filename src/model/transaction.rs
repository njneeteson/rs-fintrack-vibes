//! Transactions.

use super::Money;
use chrono::NaiveDate;

#[derive(Clone, Debug, PartialEq)]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub date: NaiveDate,
    /// Positive = money in, negative = money out.
    pub amount: Money,
    pub description: String,
    /// Secondary description columns from the CSV (sub-description, memo, ...).
    pub memo: String,
    /// Running balance reported by the bank, when the CSV has one.
    pub balance: Option<Money>,
    /// Category assigned by the rule engine.
    pub auto_category_id: Option<i64>,
    /// The rule that produced `auto_category_id`.
    pub rule_id: Option<i64>,
    /// Category chosen by the user; always wins over the automatic one.
    pub manual_category_id: Option<i64>,
    pub note: String,
    pub import_id: Option<i64>,
}

impl Transaction {
    /// Effective category: manual override first, then the rule result.
    pub fn category_id(&self) -> Option<i64> {
        self.manual_category_id.or(self.auto_category_id)
    }

    pub fn is_manual(&self) -> bool {
        self.manual_category_id.is_some()
    }

    pub fn is_uncategorized(&self) -> bool {
        self.category_id().is_none()
    }
}

/// Uppercase, trim and collapse internal whitespace. Used for matching and
/// duplicate detection so that cosmetic differences between exports don't matter.
pub fn normalize_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(normalize_text("  Save  on\tFoods "), "SAVE ON FOODS");
    }
}
