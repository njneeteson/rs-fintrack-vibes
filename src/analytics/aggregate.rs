//! Period × bucket aggregation for charts and pivot tables.

use super::flow::{BucketKey, Bucketing, Flow, Lookup, bucket_of, flow_of, flow_value};
use super::period::{Granularity, Period, periods_covering};
use super::range::DateRange;
use crate::model::{CategoryTree, Money, Transaction};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct BucketSeries {
    pub key: BucketKey,
    pub label: String,
    /// One value per period of the owning [`Breakdown`].
    pub values: Vec<Money>,
    pub total: Money,
}

/// Values per bucket per period, buckets sorted by total (largest first).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Breakdown {
    pub periods: Vec<Period>,
    pub buckets: Vec<BucketSeries>,
}

impl Breakdown {
    pub fn period_totals(&self) -> Vec<Money> {
        (0..self.periods.len()).map(|i| self.buckets.iter().map(|b| b.values[i]).sum()).collect()
    }

    pub fn grand_total(&self) -> Money {
        self.buckets.iter().map(|b| b.total).sum()
    }

    /// Keep the `n` largest buckets and fold the rest into one "Other" series.
    pub fn fold_top(mut self, n: usize) -> Breakdown {
        if self.buckets.len() <= n {
            return self;
        }
        let rest = self.buckets.split_off(n);
        let mut other = BucketSeries {
            key: BucketKey::Other,
            label: super::flow::OTHER.into(),
            values: vec![Money::ZERO; self.periods.len()],
            total: Money::ZERO,
        };
        for b in rest {
            for (o, v) in other.values.iter_mut().zip(b.values) {
                *o += v;
            }
            other.total += b.total;
        }
        self.buckets.push(other);
        self
    }
}

fn period_index(periods: &[Period]) -> HashMap<chrono::NaiveDate, usize> {
    periods.iter().enumerate().map(|(i, p)| (p.start, i)).collect()
}

/// Aggregate one flow over `range` by period and bucket.
pub fn breakdown(
    txns: &[Transaction],
    range: DateRange,
    granularity: Granularity,
    flow: Flow,
    bucketing: Bucketing,
    lookup: Lookup<'_>,
) -> Breakdown {
    let periods = periods_covering(range.start, range.end, granularity);
    let index = period_index(&periods);
    let mut map: HashMap<BucketKey, Vec<Money>> = HashMap::new();
    for t in txns.iter().filter(|t| range.contains(t.date)) {
        if flow_of(t, lookup.categories) != Some(flow) {
            continue;
        }
        let Some(&i) = index.get(&Period::containing(t.date, granularity).start) else { continue };
        let key = bucket_of(t, bucketing, lookup.categories);
        map.entry(key).or_insert_with(|| vec![Money::ZERO; periods.len()])[i] += flow_value(t, flow);
    }
    let mut buckets: Vec<BucketSeries> = map
        .into_iter()
        .map(|(key, values)| BucketSeries { label: lookup.bucket_label(&key), total: values.iter().copied().sum(), key, values })
        .collect();
    buckets.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.label.cmp(&b.label)));
    Breakdown { periods, buckets }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeriodTotals {
    pub period: Period,
    pub income: Money,
    pub expense: Money,
}

impl PeriodTotals {
    pub fn net(&self) -> Money {
        self.income - self.expense
    }

    /// Share of income kept, or `None` without income.
    pub fn savings_rate(&self) -> Option<f64> {
        (self.income.cents() > 0).then(|| self.net().dollars() / self.income.dollars())
    }
}

/// Income and spending totals per period.
pub fn income_vs_expense(
    txns: &[Transaction],
    range: DateRange,
    granularity: Granularity,
    cats: &CategoryTree,
) -> Vec<PeriodTotals> {
    let periods = periods_covering(range.start, range.end, granularity);
    let index = period_index(&periods);
    let mut out: Vec<PeriodTotals> =
        periods.iter().map(|&period| PeriodTotals { period, income: Money::ZERO, expense: Money::ZERO }).collect();
    for t in txns.iter().filter(|t| range.contains(t.date)) {
        let Some(flow) = flow_of(t, cats) else { continue };
        let Some(&i) = index.get(&Period::containing(t.date, granularity).start) else { continue };
        match flow {
            Flow::Income => out[i].income += flow_value(t, flow),
            Flow::Expense => out[i].expense += flow_value(t, flow),
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::category::tests::sample_tree;
    use chrono::NaiveDate;

    pub fn txn(id: i64, date: (i32, u32, u32), cents: i64, cat: Option<i64>) -> Transaction {
        Transaction {
            id,
            account_id: 1,
            date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
            amount: Money(cents),
            description: format!("Payee {id}"),
            memo: String::new(),
            balance: None,
            auto_category_id: cat,
            rule_id: None,
            manual_category_id: None,
            note: String::new(),
            import_id: None,
        }
    }

    #[test]
    fn breakdown_and_totals() {
        let cats = sample_tree();
        let txns = vec![
            txn(1, (2026, 1, 3), -5000, Some(2)),
            txn(2, (2026, 1, 20), 1000, Some(2)), // refund
            txn(3, (2026, 2, 1), -3000, Some(1)),
            txn(4, (2026, 2, 2), 200_000, Some(4)),
            txn(5, (2026, 2, 3), -100_000, Some(5)), // transfer: excluded
            txn(6, (2026, 2, 4), -700, None),
        ];
        let range = DateRange::new(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        let lookup = Lookup { categories: &cats, accounts: &[] };
        let top = breakdown(&txns, range, Granularity::Month, Flow::Expense, Bucketing::TopCategory, lookup);
        assert_eq!(top.periods.len(), 2);
        assert_eq!(top.buckets[0].label, "Groceries");
        assert_eq!(top.buckets[0].values, vec![Money(4000), Money(3000)]);
        assert_eq!(top.grand_total(), Money(7700));
        let sub = breakdown(&txns, range, Granularity::Month, Flow::Expense, Bucketing::SubCategory, lookup);
        assert_eq!(sub.buckets.len(), 3);
        assert_eq!(sub.clone().fold_top(1).buckets.len(), 2);
        assert_eq!(sub.fold_top(1).grand_total(), Money(7700));

        let ie = income_vs_expense(&txns, range, Granularity::Month, &cats);
        assert_eq!(ie[1].income, Money(200_000));
        assert_eq!(ie[1].expense, Money(3700));
        assert!((ie[1].savings_rate().unwrap() - 0.9815).abs() < 1e-3);
        assert_eq!(ie[0].savings_rate(), None);
    }
}
