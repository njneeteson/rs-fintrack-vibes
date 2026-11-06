//! Plain domain types shared by every layer.

pub mod account;
pub mod category;
pub mod format;
pub mod money;
pub mod mortgage;
pub mod rule;
pub mod transaction;

pub use account::{Account, AccountKind};
pub use category::{Category, CategoryKind, CategoryTree};
pub use format::{AmountColumns, CsvFormat, DirectionColumn, FormatSpec, SkipFilter};
pub use money::Money;
pub use mortgage::{Mortgage, PaymentFrequency, Prepayment, RateChange};
pub use rule::{Direction, MatchKind, Rule, RuleField};
pub use transaction::{Transaction, normalize_text};

/// A monthly budget for a category (top-level budgets include their subcategories).
#[derive(Clone, Debug, PartialEq)]
pub struct Budget {
    pub id: i64,
    pub category_id: i64,
    pub monthly_amount: Money,
}
