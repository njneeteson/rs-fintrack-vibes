//! Detection of recurring payments (subscriptions, bills, memberships).

use super::flow::{Flow, flow_of};
use crate::categorize::suggest_pattern;
use crate::model::{CategoryTree, Money, Transaction};
use chrono::{Days, NaiveDate};
use std::collections::HashMap;

str_enum! {
    pub enum Cadence {
        Weekly => ("weekly", "Weekly"),
        BiWeekly => ("biweekly", "Every 2 weeks"),
        Monthly => ("monthly", "Monthly"),
        Quarterly => ("quarterly", "Quarterly"),
        Annual => ("annual", "Annual"),
    }
}

impl Cadence {
    pub fn days(self) -> f64 {
        match self {
            Self::Weekly => 7.0,
            Self::BiWeekly => 14.0,
            Self::Monthly => 30.44,
            Self::Quarterly => 91.3,
            Self::Annual => 365.25,
        }
    }

    fn tolerance(self) -> f64 {
        match self {
            Self::Weekly => 1.5,
            Self::BiWeekly => 2.5,
            Self::Monthly => 4.5,
            Self::Quarterly => 12.0,
            Self::Annual => 20.0,
        }
    }

    fn min_occurrences(self) -> usize {
        if self == Self::Annual { 2 } else { 3 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recurring {
    pub payee: String,
    pub cadence: Cadence,
    pub typical_amount: Money,
    pub occurrences: usize,
    pub last_date: NaiveDate,
    pub next_expected: NaiveDate,
    /// Typical amount normalized to a monthly cost.
    pub monthly_cost: Money,
    pub category_id: Option<i64>,
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

/// Find payees charged at a regular cadence with a consistent amount and still active as of `today`.
pub fn detect_recurring(txns: &[Transaction], cats: &CategoryTree, today: NaiveDate) -> Vec<Recurring> {
    let mut groups: HashMap<String, Vec<&Transaction>> = HashMap::new();
    for t in txns.iter().filter(|t| flow_of(t, cats) == Some(Flow::Expense) && t.amount.is_negative()) {
        groups.entry(suggest_pattern(&t.description)).or_default().push(t);
    }
    let mut found: Vec<Recurring> = groups
        .into_iter()
        .filter_map(|(payee, mut ts)| {
            ts.sort_by_key(|t| t.date);
            analyse(payee, &ts, today)
        })
        .collect();
    found.sort_by_key(|r| std::cmp::Reverse(r.monthly_cost));
    found
}

fn analyse(payee: String, ts: &[&Transaction], today: NaiveDate) -> Option<Recurring> {
    if ts.len() < 2 {
        return None;
    }
    let gaps: Vec<f64> = ts.windows(2).map(|w| (w[1].date - w[0].date).num_days() as f64).collect();
    let typical_gap = median(gaps.clone());
    let cadence =
        *Cadence::ALL.iter().find(|c| (typical_gap - c.days()).abs() <= c.tolerance() && ts.len() >= c.min_occurrences())?;
    let regular = gaps.iter().filter(|g| (*g - cadence.days()).abs() <= cadence.tolerance()).count();
    if (regular as f64) < 0.7 * gaps.len() as f64 {
        return None;
    }
    let amounts: Vec<f64> = ts.iter().map(|t| t.amount.abs().dollars()).collect();
    let typical = median(amounts.clone());
    let consistent = amounts.iter().filter(|a| (*a - typical).abs() <= 0.2 * typical.max(1.0)).count();
    if (consistent as f64) < 0.7 * amounts.len() as f64 {
        return None;
    }
    let last = ts.last()?;
    let next_expected = last.date + Days::new(cadence.days().round() as u64);
    // Stop showing subscriptions that appear to have been cancelled.
    if (today - last.date).num_days() as f64 > cadence.days() * 1.6 + cadence.tolerance() {
        return None;
    }
    Some(Recurring {
        payee,
        cadence,
        typical_amount: Money::from_dollars(typical),
        occurrences: ts.len(),
        last_date: last.date,
        next_expected,
        monthly_cost: Money::from_dollars(typical * 30.44 / cadence.days()),
        category_id: last.category_id(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::aggregate::tests::txn;
    use crate::model::category::tests::sample_tree;

    #[test]
    fn finds_monthly_subscription() {
        let mut txns: Vec<Transaction> = (1..=5u32)
            .map(|m| {
                let mut t = txn(m as i64, (2026, m, 3), -1599, None);
                t.description = format!("NETFLIX.COM {m}");
                t
            })
            .collect();
        // Irregular groceries should not be detected.
        for (i, d) in [(1, 2), (1, 9), (1, 11), (2, 20), (3, 1)].into_iter().enumerate() {
            let mut t = txn(100 + i as i64, (2026, d.0, d.1), -(5000 + 700 * i as i64), None);
            t.description = "SAFEWAY #1".into();
            txns.push(t);
        }
        let today = NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let found = detect_recurring(&txns, &sample_tree(), today);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].payee, "NETFLIX.COM");
        assert_eq!(found[0].cadence, Cadence::Monthly);
        assert_eq!(found[0].typical_amount, Money(1599));
        // Long after the last charge it's considered cancelled.
        assert!(detect_recurring(&txns, &sample_tree(), NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()).is_empty());
    }
}
