//! Headline numbers and payee rankings.

use super::flow::{Flow, flow_of, flow_value};
use super::range::DateRange;
use crate::categorize::suggest_pattern;
use crate::model::{CategoryTree, Money, Transaction};
use chrono::NaiveDate;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summary {
    pub income: Money,
    pub expense: Money,
    pub count: usize,
    pub uncategorized: usize,
}

impl Summary {
    pub fn net(&self) -> Money {
        self.income - self.expense
    }

    pub fn savings_rate(&self) -> Option<f64> {
        (self.income.cents() > 0).then(|| self.net().dollars() / self.income.dollars())
    }
}

pub fn summarize(txns: &[Transaction], range: DateRange, cats: &CategoryTree) -> Summary {
    let mut s = Summary::default();
    for t in txns.iter().filter(|t| range.contains(t.date)) {
        s.count += 1;
        s.uncategorized += usize::from(t.is_uncategorized());
        match flow_of(t, cats) {
            Some(Flow::Income) => s.income += flow_value(t, Flow::Income),
            Some(Flow::Expense) => s.expense += flow_value(t, Flow::Expense),
            None => {}
        }
    }
    s
}

#[derive(Clone, Debug, PartialEq)]
pub struct PayeeStat {
    pub payee: String,
    pub count: usize,
    pub total: Money,
    pub last_date: NaiveDate,
}

impl PayeeStat {
    pub fn average(&self) -> Money {
        Money(self.total.cents() / self.count.max(1) as i64)
    }
}

/// Largest payees for one flow within `range`.
pub fn top_payees(
    txns: &[Transaction],
    range: DateRange,
    flow: Flow,
    cats: &CategoryTree,
    limit: usize,
) -> Vec<PayeeStat> {
    let mut map: HashMap<String, PayeeStat> = HashMap::new();
    for t in txns
        .iter()
        .filter(|t| range.contains(t.date) && flow_of(t, cats) == Some(flow))
    {
        let payee = suggest_pattern(&t.description);
        let e = map.entry(payee.clone()).or_insert(PayeeStat {
            payee,
            count: 0,
            total: Money::ZERO,
            last_date: t.date,
        });
        e.count += 1;
        e.total += flow_value(t, flow);
        e.last_date = e.last_date.max(t.date);
    }
    let mut v: Vec<PayeeStat> = map.into_values().collect();
    v.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.payee.cmp(&b.payee)));
    v.truncate(limit);
    v
}
