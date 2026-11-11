//! Everything most household pages need, loaded in one go.

use fintrack_core::analytics::{DateRange, Lookup};
use fintrack_core::db::Database;
use fintrack_core::model::{Account, Budget, CategoryTree, Rule, Transaction};

pub struct Data {
    /// Newest first.
    pub txns: Vec<Transaction>,
    pub cats: CategoryTree,
    pub accounts: Vec<Account>,
    pub rules: Vec<Rule>,
    pub budgets: Vec<Budget>,
}

impl Data {
    pub fn load(db: &Database) -> anyhow::Result<Self> {
        Ok(Self {
            txns: db.transactions()?,
            cats: CategoryTree::new(db.categories()?),
            accounts: db.accounts()?,
            rules: db.rules()?,
            budgets: db.budgets()?,
        })
    }

    pub fn lookup(&self) -> Lookup<'_> {
        Lookup {
            categories: &self.cats,
            accounts: &self.accounts,
        }
    }

    pub fn account_name(&self, id: i64) -> String {
        self.lookup().account_name(id)
    }

    /// First to last transaction date.
    pub fn bounds(&self) -> Option<DateRange> {
        let min = self.txns.iter().map(|t| t.date).min()?;
        let max = self.txns.iter().map(|t| t.date).max()?;
        Some(DateRange::new(min, max))
    }

    pub fn uncategorized(&self) -> usize {
        self.txns.iter().filter(|t| t.is_uncategorized()).count()
    }
}

pub fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}
