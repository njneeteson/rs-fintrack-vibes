//! In-memory snapshot of the open database, reloaded after every change.

use crate::analytics::{DateRange, Lookup};
use crate::db::{Database, ImportRecord};
use crate::model::{Account, Budget, CategoryTree, CsvFormat, Mortgage, Prepayment, RateChange, Rule, Transaction};

#[derive(Default)]
pub struct DataCache {
    /// Incremented on every reload so pages can invalidate derived caches.
    pub generation: u64,
    pub accounts: Vec<Account>,
    pub categories: CategoryTree,
    pub rules: Vec<Rule>,
    pub formats: Vec<CsvFormat>,
    /// Newest first.
    pub transactions: Vec<Transaction>,
    pub budgets: Vec<Budget>,
    pub mortgages: Vec<Mortgage>,
    pub prepayments: Vec<Prepayment>,
    pub rate_changes: Vec<RateChange>,
    pub imports: Vec<ImportRecord>,
}

impl DataCache {
    pub fn load(db: &Database, generation: u64) -> anyhow::Result<Self> {
        let mortgages = db.mortgages()?;
        let mut prepayments = Vec::new();
        let mut rate_changes = Vec::new();
        for m in &mortgages {
            prepayments.extend(db.prepayments(m.id)?);
            rate_changes.extend(db.rate_changes(m.id)?);
        }
        Ok(Self {
            generation,
            accounts: db.accounts()?,
            categories: CategoryTree::new(db.categories()?),
            rules: db.rules()?,
            formats: db.formats()?,
            transactions: db.transactions()?,
            budgets: db.budgets()?,
            mortgages,
            prepayments,
            rate_changes,
            imports: db.imports()?,
        })
    }

    pub fn lookup(&self) -> Lookup<'_> {
        Lookup { categories: &self.categories, accounts: &self.accounts }
    }

    pub fn account(&self, id: i64) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == id)
    }

    pub fn account_name(&self, id: i64) -> String {
        self.lookup().account_name(id)
    }

    /// First to last transaction date.
    pub fn data_range(&self) -> Option<DateRange> {
        let last = self.transactions.first()?.date;
        let first = self.transactions.last()?.date;
        Some(DateRange::new(first, last))
    }

    pub fn uncategorized_count(&self) -> usize {
        self.transactions.iter().filter(|t| t.is_uncategorized()).count()
    }
}
