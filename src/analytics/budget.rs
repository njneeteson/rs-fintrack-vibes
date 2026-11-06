//! Budget vs. actual.

use super::flow::{Flow, flow_value};
use super::period::{Granularity, Period};
use crate::model::{Budget, CategoryKind, CategoryTree, Money, Transaction};
use chrono::{Months, NaiveDate};

#[derive(Clone, Debug, PartialEq)]
pub struct BudgetLine {
    pub budget_id: i64,
    pub category_id: i64,
    pub label: String,
    /// Budget scaled to the period length.
    pub budget: Money,
    pub actual: Money,
    pub kind: CategoryKind,
}

impl BudgetLine {
    /// Fraction of the budget used (1.0 = exactly on budget).
    pub fn fraction(&self) -> f64 {
        if self.budget.cents() == 0 { 0.0 } else { self.actual.dollars() / self.budget.dollars() }
    }

    pub fn remaining(&self) -> Money {
        self.budget - self.actual
    }
}

fn flow_for(kind: CategoryKind) -> Flow {
    if kind == CategoryKind::Income { Flow::Income } else { Flow::Expense }
}

/// Actual flow for a category (and its children) within `[start, end]`.
fn actual_for(txns: &[Transaction], cats: &CategoryTree, category_id: i64, start: NaiveDate, end: NaiveDate) -> Money {
    let flow = flow_for(cats.kind(category_id).unwrap_or(CategoryKind::Expense));
    txns.iter()
        .filter(|t| t.date >= start && t.date <= end)
        .filter(|t| t.category_id().is_some_and(|c| cats.is_under(c, category_id)))
        .map(|t| flow_value(t, flow))
        .sum()
}

pub fn budget_report(budgets: &[Budget], txns: &[Transaction], cats: &CategoryTree, period: Period) -> Vec<BudgetLine> {
    let factor = period.granularity.months();
    let mut lines: Vec<BudgetLine> = budgets
        .iter()
        .filter(|b| cats.get(b.category_id).is_some())
        .map(|b| BudgetLine {
            budget_id: b.id,
            category_id: b.category_id,
            label: cats.full_name(b.category_id),
            budget: Money::from_dollars(b.monthly_amount.dollars() * factor),
            actual: actual_for(txns, cats, b.category_id, period.start, period.last_day()),
            kind: cats.kind(b.category_id).unwrap_or(CategoryKind::Expense),
        })
        .collect();
    lines.sort_by(|a, b| a.label.cmp(&b.label));
    lines
}

/// Average monthly flow for a category over the `months` complete months before `today`'s month.
pub fn average_monthly(txns: &[Transaction], cats: &CategoryTree, category_id: i64, months: u32, today: NaiveDate) -> Money {
    let this_month = Period::containing(today, Granularity::Month).start;
    let start = this_month - Months::new(months);
    let end = this_month.pred_opt().unwrap_or(this_month);
    Money(actual_for(txns, cats, category_id, start, end).cents() / i64::from(months.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::aggregate::tests::txn;
    use crate::model::category::tests::sample_tree;

    #[test]
    fn report_scales_and_includes_children() {
        let cats = sample_tree();
        let txns = vec![txn(1, (2026, 1, 3), -30_000, Some(2)), txn(2, (2026, 2, 3), -10_000, Some(1))];
        let budgets = vec![Budget { id: 1, category_id: 1, monthly_amount: Money(20_000) }];
        let q = Period::containing(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), Granularity::Quarter);
        let r = budget_report(&budgets, &txns, &cats, q);
        assert_eq!(r[0].budget, Money(60_000));
        assert_eq!(r[0].actual, Money(40_000));
        assert!((r[0].fraction() - 0.6667).abs() < 1e-3);
        let avg = average_monthly(&txns, &cats, 1, 2, NaiveDate::from_ymd_opt(2026, 3, 15).unwrap());
        assert_eq!(avg, Money(20_000));
    }
}
