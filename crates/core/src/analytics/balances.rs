//! Account balances and net worth over time.

use super::period::Period;
use crate::model::{Account, Money, Transaction};

/// Opening balance plus every transaction up to and including `date`.
pub fn balance_on(account: &Account, txns: &[Transaction], date: chrono::NaiveDate) -> Money {
    account.opening_balance
        + txns
            .iter()
            .filter(|t| t.account_id == account.id && t.date <= date)
            .map(|t| t.amount)
            .sum::<Money>()
}

/// Balance at the end of each period for one account.
pub fn balance_series(account: &Account, txns: &[Transaction], periods: &[Period]) -> Vec<Money> {
    let mut own: Vec<&Transaction> = txns.iter().filter(|t| t.account_id == account.id).collect();
    own.sort_by_key(|t| t.date);
    let mut running = account.opening_balance;
    let mut i = 0;
    periods
        .iter()
        .map(|p| {
            let end = p.last_day();
            while i < own.len() && own[i].date <= end {
                running += own[i].amount;
                i += 1;
            }
            running
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::aggregate::tests::txn;
    use crate::analytics::period::{Granularity, periods_covering};
    use chrono::NaiveDate;

    #[test]
    fn running_balances() {
        let acct = Account {
            id: 1,
            opening_balance: Money(10_000),
            ..Default::default()
        };
        let txns = vec![
            txn(1, (2026, 1, 5), -2_000, None),
            txn(2, (2026, 3, 1), 5_000, None),
        ];
        let ps = periods_covering(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            Granularity::Month,
        );
        assert_eq!(
            balance_series(&acct, &txns, &ps),
            vec![Money(8_000), Money(8_000), Money(13_000)]
        );
        assert_eq!(
            balance_on(&acct, &txns, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap()),
            Money(8_000)
        );
    }
}
