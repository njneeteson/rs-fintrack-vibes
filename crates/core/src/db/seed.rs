//! Default data for new databases: a starter category set and the built-in CSV formats.

use super::Database;
use crate::import::builtin::builtin_formats;
use crate::model::{CategoryKind, CsvFormat};

/// `(top-level, kind, subcategories)`
const DEFAULT_CATEGORIES: &[(&str, CategoryKind, &[&str])] = &[
    (
        "Income",
        CategoryKind::Income,
        &[
            "Salary",
            "Bonus",
            "Interest & Dividends",
            "Government Benefits",
            "Refunds & Rebates",
            "Other Income",
        ],
    ),
    (
        "Housing",
        CategoryKind::Expense,
        &[
            "Mortgage",
            "Rent",
            "Property Tax",
            "Strata Fees",
            "Home Insurance",
            "Maintenance & Repairs",
        ],
    ),
    (
        "Utilities",
        CategoryKind::Expense,
        &[
            "Electricity",
            "Natural Gas",
            "Water & Sewer",
            "Internet",
            "Mobile Phone",
        ],
    ),
    ("Groceries", CategoryKind::Expense, &[]),
    (
        "Dining",
        CategoryKind::Expense,
        &["Restaurants", "Coffee", "Takeout & Delivery"],
    ),
    (
        "Transportation",
        CategoryKind::Expense,
        &[
            "Fuel",
            "Transit",
            "Parking",
            "Auto Insurance",
            "Vehicle Maintenance",
        ],
    ),
    (
        "Shopping",
        CategoryKind::Expense,
        &["Clothing", "Household", "Electronics", "Online Shopping"],
    ),
    (
        "Health",
        CategoryKind::Expense,
        &["Pharmacy", "Dental", "Medical", "Fitness"],
    ),
    (
        "Entertainment",
        CategoryKind::Expense,
        &["Subscriptions", "Events", "Hobbies"],
    ),
    ("Kids & Childcare", CategoryKind::Expense, &[]),
    ("Personal Care", CategoryKind::Expense, &[]),
    ("Travel", CategoryKind::Expense, &[]),
    ("Gifts & Donations", CategoryKind::Expense, &[]),
    (
        "Fees & Interest",
        CategoryKind::Expense,
        &["Bank Fees", "Interest Charges"],
    ),
    ("Taxes", CategoryKind::Expense, &[]),
    (
        "Transfers",
        CategoryKind::Transfer,
        &[
            "Credit Card Payment",
            "Between Accounts",
            "Savings & Investments",
        ],
    ),
];

impl Database {
    pub(super) fn seed_defaults(&self) -> anyhow::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (top, kind, subs) in DEFAULT_CATEGORIES {
            let parent = self.add_category(top, None, *kind)?;
            for sub in *subs {
                self.add_category(sub, Some(parent), *kind)?;
            }
        }
        for (name, spec) in builtin_formats() {
            self.save_format(&CsvFormat {
                id: 0,
                name,
                spec,
                builtin: true,
            })?;
        }
        tx.commit()?;
        Ok(())
    }
}
