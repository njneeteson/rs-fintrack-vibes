//! Built-in demo data: ~18 months of realistic synthetic household finances.
//!
//! [`generate`] creates `<folder>/Demo Household` (a FinTrack database) and
//! `<folder>/sample-csvs/` with the bank exports that were imported — including
//! two overlapping chequing exports to demonstrate duplicate handling.
//! Everything goes through the same import pipeline as the app. Used by the
//! server's "demo household" option and by `cargo run --example demo_data`.

use crate::db::Database;
use crate::import::detect::detect_format;
use crate::import::parse::parse_all;
use crate::import::reader::RawCsv;
use crate::import::{assign_accounts, group_by_account};
use crate::model::*;
use anyhow::Context;
use chrono::{Datelike, Days, Local, NaiveDate};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Tiny deterministic PRNG so the demo is reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() * items.len() as f64) as usize % items.len()]
    }
}

struct Txn {
    date: NaiveDate,
    amount: f64,
    desc: String,
}

fn chequing(start: NaiveDate, end: NaiveDate, rng: &mut Rng) -> Vec<Txn> {
    let mut v = Vec::new();
    let mut d = start;
    while d <= end {
        let day = d.day();
        if (d - start).num_days() % 14 == 4 {
            v.push(Txn {
                date: d,
                amount: 3_650.00 + rng.range(-20.0, 20.0),
                desc: "PAYROLL DEP MAYFAIR".into(),
            });
        }
        if day == 1 {
            v.push(Txn {
                date: d,
                amount: -2_908.02,
                desc: "MORTGAGE PAYMENT SCOTIA".into(),
            });
            v.push(Txn {
                date: d,
                amount: -95.00,
                desc: "TELUS MOBILITY PREAUTH".into(),
            });
        }
        if day == 5 {
            v.push(Txn {
                date: d,
                amount: -85.00 - rng.range(0.0, 60.0),
                desc: "FORTISBC GAS".into(),
            });
            v.push(Txn {
                date: d,
                amount: -110.00,
                desc: "SHAW CABLE INTERNET".into(),
            });
        }
        if day == 12 && d.month().is_multiple_of(2) {
            v.push(Txn {
                date: d,
                amount: -140.00 - rng.range(0.0, 80.0),
                desc: "BC HYDRO".into(),
            });
        }
        if day == 20 {
            v.push(Txn {
                date: d,
                amount: -2_350.00,
                desc: "PAYMENT BMO MASTERCARD".into(),
            });
            v.push(Txn {
                date: d,
                amount: -500.00,
                desc: "TFSA CONTRIBUTION".into(),
            });
        }
        if day == 15 && d.month() == 6 {
            v.push(Txn {
                date: d,
                amount: 1_250.00,
                desc: "CRA TAX REFUND".into(),
            });
        }
        if rng.next() < 0.06 {
            v.push(Txn {
                date: d,
                amount: -rng.range(40.0, 200.0),
                desc: "ATM WITHDRAWAL".into(),
            });
        }
        d = d + Days::new(1);
    }
    v
}

fn credit_card(start: NaiveDate, end: NaiveDate, rng: &mut Rng) -> Vec<Txn> {
    let mut v = Vec::new();
    let mut d = start;
    while d <= end {
        let wd = d.weekday().num_days_from_monday();
        if wd == 5 {
            let store = rng.pick(&[
                "SAVE ON FOODS #2134 VANCOUVER",
                "SAFEWAY #4410 BURNABY",
                "SAVE ON FOODS #988 BURNABY",
            ]);
            v.push(Txn {
                date: d,
                amount: rng.range(90.0, 230.0),
                desc: store.into(),
            });
        }
        if wd == 2 && rng.next() < 0.5 {
            v.push(Txn {
                date: d,
                amount: rng.range(120.0, 380.0),
                desc: "COSTCO WHOLESALE W53".into(),
            });
        }
        if rng.next() < 0.35 {
            let coffee = rng.pick(&["STARBUCKS #12345", "TIM HORTONS #3321", "JJ BEAN COFFEE"]);
            v.push(Txn {
                date: d,
                amount: rng.range(4.0, 9.0),
                desc: coffee.into(),
            });
        }
        if rng.next() < 0.12 {
            let r = rng.pick(&[
                "THE KEG STEAKHOUSE",
                "CACTUS CLUB CAFE",
                "SUSHI TOWN",
                "DOORDASH*ORDER",
            ]);
            v.push(Txn {
                date: d,
                amount: rng.range(25.0, 140.0),
                desc: r.into(),
            });
        }
        if rng.next() < 0.1 {
            let f = rng.pick(&["SHELL C12345", "CHEVRON 00123", "PETRO-CANADA 4432"]);
            v.push(Txn {
                date: d,
                amount: rng.range(50.0, 95.0),
                desc: f.into(),
            });
        }
        if rng.next() < 0.08 {
            v.push(Txn {
                date: d,
                amount: rng.range(15.0, 160.0),
                desc: "AMAZON.CA*1A2B3C".into(),
            });
        }
        if rng.next() < 0.05 {
            v.push(Txn {
                date: d,
                amount: rng.range(10.0, 60.0),
                desc: "LONDON DRUGS 12".into(),
            });
        }
        if d.day() == 3 {
            v.push(Txn {
                date: d,
                amount: 16.49,
                desc: "NETFLIX.COM".into(),
            });
            v.push(Txn {
                date: d,
                amount: 11.99,
                desc: "SPOTIFY P1A2B3".into(),
            });
        }
        if d.day() == 10 {
            v.push(Txn {
                date: d,
                amount: 54.00,
                desc: "GOODLIFE FITNESS".into(),
            });
        }
        if wd == 0 && rng.next() < 0.5 {
            v.push(Txn {
                date: d,
                amount: 3.15,
                desc: "COMPASS ACCOUNT".into(),
            });
        }
        if d.day() == 21 {
            v.push(Txn {
                date: d,
                amount: -2_350.00,
                desc: "PAYMENT RECEIVED - THANK YOU".into(),
            });
        }
        d = d + Days::new(1);
    }
    v
}

/// Scotiabank classic (no header): `M/D/YYYY,amount,-,description,sub-description`
fn write_scotia(path: &Path, txns: &[&Txn]) -> anyhow::Result<()> {
    let mut s = String::new();
    for t in txns {
        writeln!(
            s,
            "{},{:.2},-,{},",
            t.date.format("%-m/%-d/%Y"),
            t.amount,
            t.desc
        )?;
    }
    std::fs::write(path, s)?;
    Ok(())
}

/// BMO credit card: preamble + header, purchases positive.
fn write_bmo_card(path: &Path, txns: &[&Txn]) -> anyhow::Result<()> {
    let mut s = String::from("Following data is valid as of a recent date\n\n");
    s.push_str("Item #,Card #,Transaction Date,Posting Date,Transaction Amount,Description\n");
    for (i, t) in txns.iter().enumerate() {
        let posted = t.date + Days::new(1);
        writeln!(
            s,
            "{},'5191230000004821',{},{},{:.2},{}",
            i + 1,
            t.date.format("%Y%m%d"),
            posted.format("%Y%m%d"),
            t.amount,
            t.desc
        )?;
    }
    std::fs::write(path, s)?;
    Ok(())
}

fn import(db: &Database, path: &Path) -> anyhow::Result<()> {
    let raw = RawCsv::from_path(path)?;
    let formats = db.formats()?;
    let f = &formats[detect_format(&raw, &formats)
        .context("format not detected")?
        .format_index];
    let out = parse_all(&raw, &f.spec).map_err(anyhow::Error::msg)?;
    let mut groups = group_by_account(out.rows);
    assign_accounts(&mut groups, &db.accounts()?, &raw.file_name, Some(f.id));
    db.commit_import(
        &raw.file_name,
        &db.archive_csv(path)?.display().to_string(),
        &f.name,
        &groups,
    )?;
    for g in &groups {
        if let Some(a) = g.account_id {
            db.learn_account_identity(a, g.account_number.as_deref(), Some(f.id))?;
        }
    }

    Ok(())
}

fn rules(db: &Database) -> anyhow::Result<()> {
    let cats = CategoryTree::new(db.categories()?);
    let rules: &[(&str, &str, Option<&str>, Direction)] = &[
        ("SAVE ON FOODS", "Groceries", None, Direction::Any),
        ("SAFEWAY", "Groceries", None, Direction::Any),
        ("COSTCO", "Groceries", None, Direction::Any),
        ("PAYROLL", "Income", Some("Salary"), Direction::Inflow),
        (
            "CRA TAX REFUND",
            "Income",
            Some("Refunds & Rebates"),
            Direction::Inflow,
        ),
        (
            "MORTGAGE PAYMENT",
            "Housing",
            Some("Mortgage"),
            Direction::Outflow,
        ),
        ("TELUS", "Utilities", Some("Mobile Phone"), Direction::Any),
        ("SHAW", "Utilities", Some("Internet"), Direction::Any),
        ("FORTISBC", "Utilities", Some("Natural Gas"), Direction::Any),
        ("BC HYDRO", "Utilities", Some("Electricity"), Direction::Any),
        (
            "NETFLIX",
            "Entertainment",
            Some("Subscriptions"),
            Direction::Any,
        ),
        (
            "SPOTIFY",
            "Entertainment",
            Some("Subscriptions"),
            Direction::Any,
        ),
        ("GOODLIFE", "Health", Some("Fitness"), Direction::Any),
        ("STARBUCKS", "Dining", Some("Coffee"), Direction::Any),
        ("TIM HORTONS", "Dining", Some("Coffee"), Direction::Any),
        ("SHELL", "Transportation", Some("Fuel"), Direction::Any),
        ("CHEVRON", "Transportation", Some("Fuel"), Direction::Any),
        ("COMPASS", "Transportation", Some("Transit"), Direction::Any),
        (
            "PAYMENT BMO MASTERCARD",
            "Transfers",
            Some("Credit Card Payment"),
            Direction::Any,
        ),
        (
            "PAYMENT RECEIVED",
            "Transfers",
            Some("Credit Card Payment"),
            Direction::Any,
        ),
        (
            "TFSA CONTRIBUTION",
            "Transfers",
            Some("Savings & Investments"),
            Direction::Any,
        ),
    ];
    for (pattern, top, sub, direction) in rules {
        let category_id = cats
            .find(top, *sub)
            .with_context(|| format!("category {top}/{sub:?}"))?;
        db.save_rule(&Rule {
            pattern: pattern.to_string(),
            category_id,
            direction: *direction,
            ..Default::default()
        })?;
    }
    // A higher-priority rule: big Costco trips are household shopping, not groceries.
    db.save_rule(&Rule {
        pattern: "COSTCO".into(),
        min_amount: Some(Money::from_dollars(300.0)),
        category_id: cats
            .find("Shopping", Some("Household"))
            .context("household")?,
        priority: 10,
        ..Default::default()
    })?;
    db.apply_rules()?;
    for (top, sub, amount) in [
        ("Groceries", None, 900.0),
        ("Dining", None, 450.0),
        ("Utilities", None, 450.0),
        ("Entertainment", None, 40.0),
    ] {
        db.set_budget(
            cats.find(top, sub).context("budget category")?,
            Money::from_dollars(amount),
        )?;
    }
    Ok(())
}

/// Locations of a generated demo.
#[derive(Clone, Debug, PartialEq)]
pub struct Demo {
    /// The demo database folder.
    pub database: PathBuf,
    /// The sample bank exports that were imported (try re-importing them).
    pub sample_csvs: PathBuf,
}

/// Generate the demo database in `<out>/Demo Household` and the sample CSVs in `<out>/sample-csvs`.
pub fn generate(out: &Path) -> anyhow::Result<Demo> {
    generate_at(&out.join("Demo Household"), &out.join("sample-csvs"))
}

/// Generate the demo database at `database` (a new database folder) and write
/// the bank exports it was built from into `sample_csvs`.
pub fn generate_at(database: &Path, sample_csvs: &Path) -> anyhow::Result<Demo> {
    let csv_dir = sample_csvs.to_path_buf();
    std::fs::create_dir_all(&csv_dir)?;
    let end = Local::now().date_naive();
    let start = end - Days::new(540);
    let mut rng = Rng(42);

    let chq = chequing(start, end, &mut rng);
    let card = credit_card(start, end, &mut rng);
    let mid = start + Days::new(300);
    let overlap_start = mid - Days::new(45);
    let first: Vec<&Txn> = chq.iter().filter(|t| t.date <= mid).rev().collect();
    let second: Vec<&Txn> = chq
        .iter()
        .filter(|t| t.date >= overlap_start)
        .rev()
        .collect();
    write_scotia(&csv_dir.join("scotia-chequing-part1.csv"), &first)?;
    write_scotia(
        &csv_dir.join("scotia-chequing-part2-overlapping.csv"),
        &second,
    )?;
    write_bmo_card(
        &csv_dir.join("bmo-mastercard.csv"),
        &card.iter().collect::<Vec<_>>(),
    )?;

    let db = Database::create(database)?;

    db.save_account(&Account {
        name: "Scotia Chequing".into(),
        institution: "Scotiabank".into(),
        filename_hint: "scotia-chequing".into(),
        opening_balance: Money::from_dollars(6_500.0),
        ..Default::default()
    })?;
    db.save_account(&Account {
        name: "BMO Mastercard".into(),
        institution: "BMO".into(),
        kind: AccountKind::CreditCard,
        account_number: "4821".into(),
        opening_balance: Money::from_dollars(-1_200.0),
        ..Default::default()
    })?;
    for f in [
        "scotia-chequing-part1.csv",
        "scotia-chequing-part2-overlapping.csv",
        "bmo-mastercard.csv",
    ] {
        import(&db, &csv_dir.join(f))?;
    }
    rules(&db)?;

    let mortgage_start = NaiveDate::from_ymd_opt(end.year() - 3, 6, 1).context("date")?;
    let id = db.save_mortgage(&Mortgage {
        name: "Home mortgage".into(),
        principal: Money::from_dollars(540_000.0),
        annual_rate_pct: 4.89,
        amortization_months: 300,
        start_date: mortgage_start,
        term_months: 60,
        ..Default::default()
    })?;
    db.save_prepayment(&Prepayment {
        id: 0,
        mortgage_id: id,
        date: NaiveDate::from_ymd_opt(end.year() - 1, 12, 15).context("date")?,
        amount: Money::from_dollars(15_000.0),
        note: "Annual lump sum".into(),
        enabled: true,
    })?;
    db.save_prepayment(&Prepayment {
        id: 0,
        mortgage_id: id,
        date: NaiveDate::from_ymd_opt(end.year() + 1, 12, 15).context("date")?,
        amount: Money::from_dollars(20_000.0),
        note: "Planned bonus prepayment".into(),
        enabled: true,
    })?;
    db.add_rate_change(&RateChange {
        id: 0,
        mortgage_id: id,
        effective_date: mortgage_start + chrono::Months::new(60),
        annual_rate_pct: 4.19,
    })?;
    Ok(Demo {
        database: db.root().to_path_buf(),
        sample_csvs: csv_dir,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn generates_a_populated_database() {
        let dir = tempfile::tempdir().unwrap();
        let demo = super::generate(dir.path()).unwrap();
        let db = crate::db::Database::open(&demo.database).unwrap();
        let txns = db.transactions().unwrap();
        assert!(txns.len() > 500);
        assert!(txns.iter().filter(|t| t.category_id().is_some()).count() > txns.len() / 2);
        assert_eq!(db.accounts().unwrap().len(), 2);
        assert_eq!(db.mortgages().unwrap().len(), 1);
        assert!(demo.sample_csvs.join("bmo-mastercard.csv").is_file());
    }
}
