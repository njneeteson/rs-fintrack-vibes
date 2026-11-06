//! Classifying transactions as income, spending, or transfers, and bucketing them.

use crate::categorize::suggest_pattern;
use crate::model::{Account, CategoryKind, CategoryTree, Money, Transaction};

str_enum! {
    pub enum Flow {
        Expense => ("expense", "Spending"),
        Income => ("income", "Income"),
    }
}

str_enum! {
    /// How transactions are grouped into series.
    pub enum Bucketing {
        TopCategory => ("top", "Top-level category"),
        SubCategory => ("sub", "Subcategory"),
        Account => ("account", "Account"),
        Payee => ("payee", "Payee"),
    }
}

pub const UNCATEGORIZED: &str = "Uncategorized";
pub const OTHER: &str = "Other";

/// Identity of a bucket (used for drill-down and stable colors).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BucketKey {
    Category(i64),
    Uncategorized,
    Account(i64),
    Payee(String),
    Other,
}

/// Everything needed to turn ids into labels.
#[derive(Clone, Copy)]
pub struct Lookup<'a> {
    pub categories: &'a CategoryTree,
    pub accounts: &'a [Account],
}

impl Lookup<'_> {
    pub fn account_name(&self, id: i64) -> String {
        self.accounts.iter().find(|a| a.id == id).map_or_else(|| "(deleted account)".into(), |a| a.name.clone())
    }

    pub fn bucket_label(&self, key: &BucketKey) -> String {
        match key {
            BucketKey::Category(id) => self.categories.full_name(*id),
            BucketKey::Uncategorized => UNCATEGORIZED.into(),
            BucketKey::Account(id) => self.account_name(*id),
            BucketKey::Payee(p) => p.clone(),
            BucketKey::Other => OTHER.into(),
        }
    }
}

/// Income or spending; `None` for transfers (excluded from flow analytics).
/// Uncategorized transactions are classified by sign.
pub fn flow_of(t: &Transaction, cats: &CategoryTree) -> Option<Flow> {
    match t.category_id().and_then(|c| cats.kind(c)) {
        Some(CategoryKind::Transfer) => None,
        Some(CategoryKind::Expense) => Some(Flow::Expense),
        Some(CategoryKind::Income) => Some(Flow::Income),
        None if t.amount.is_negative() => Some(Flow::Expense),
        None => Some(Flow::Income),
    }
}

/// Contribution of `t` to a flow total: spending is positive for money out,
/// so a refund in an expense category reduces spending.
pub fn flow_value(t: &Transaction, flow: Flow) -> Money {
    match flow {
        Flow::Expense => -t.amount,
        Flow::Income => t.amount,
    }
}

pub fn bucket_of(t: &Transaction, bucketing: Bucketing, cats: &CategoryTree) -> BucketKey {
    match (bucketing, t.category_id()) {
        (Bucketing::Account, _) => BucketKey::Account(t.account_id),
        (Bucketing::Payee, _) => BucketKey::Payee(suggest_pattern(&t.description)),
        (_, None) => BucketKey::Uncategorized,
        (Bucketing::TopCategory, Some(c)) => BucketKey::Category(cats.top_id(c)),
        (Bucketing::SubCategory, Some(c)) => BucketKey::Category(c),
    }
}
