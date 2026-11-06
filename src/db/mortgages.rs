use super::Database;
use super::sql::get_enum;
use crate::model::{Mortgage, Prepayment, RateChange};
use anyhow::ensure;
use rusqlite::params;

impl Database {
    pub fn mortgages(&self) -> anyhow::Result<Vec<Mortgage>> {
        self.query_all(
            "SELECT id, name, principal, annual_rate, amortization_months, start_date, frequency, payment_override,
                    term_months, notes FROM mortgages ORDER BY name COLLATE NOCASE",
            [],
            |r| {
                Ok(Mortgage {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    principal: r.get(2)?,
                    annual_rate_pct: r.get(3)?,
                    amortization_months: r.get(4)?,
                    start_date: r.get(5)?,
                    frequency: get_enum(r, 6)?,
                    payment_override: r.get(7)?,
                    term_months: r.get(8)?,
                    notes: r.get(9)?,
                })
            },
        )
    }

    pub fn save_mortgage(&self, m: &Mortgage) -> anyhow::Result<i64> {
        ensure!(!m.name.trim().is_empty(), "mortgage name cannot be empty");
        ensure!(m.amortization_months > 0, "amortization must be at least one month");
        let p = params![
            m.name.trim(),
            m.principal,
            m.annual_rate_pct,
            m.amortization_months,
            m.start_date,
            m.frequency.key(),
            m.payment_override,
            m.term_months,
            m.notes,
            m.id
        ];
        if m.id == 0 {
            self.conn.execute(
                "INSERT INTO mortgages (name, principal, annual_rate, amortization_months, start_date, frequency,
                 payment_override, term_months, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                &p[..9],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE mortgages SET name=?1, principal=?2, annual_rate=?3, amortization_months=?4, start_date=?5,
                 frequency=?6, payment_override=?7, term_months=?8, notes=?9 WHERE id=?10",
                p,
            )?;
            Ok(m.id)
        }
    }

    pub fn delete_mortgage(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM mortgages WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn prepayments(&self, mortgage_id: i64) -> anyhow::Result<Vec<Prepayment>> {
        self.query_all(
            "SELECT id, mortgage_id, date, amount, note, enabled FROM mortgage_prepayments WHERE mortgage_id=?1 ORDER BY date",
            [mortgage_id],
            |r| {
                Ok(Prepayment {
                    id: r.get(0)?,
                    mortgage_id: r.get(1)?,
                    date: r.get(2)?,
                    amount: r.get(3)?,
                    note: r.get(4)?,
                    enabled: r.get(5)?,
                })
            },
        )
    }

    pub fn save_prepayment(&self, p: &Prepayment) -> anyhow::Result<i64> {
        ensure!(p.amount.cents() > 0, "prepayment amount must be positive");
        if p.id == 0 {
            self.conn.execute(
                "INSERT INTO mortgage_prepayments (mortgage_id, date, amount, note, enabled) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![p.mortgage_id, p.date, p.amount, p.note, p.enabled],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE mortgage_prepayments SET date=?1, amount=?2, note=?3, enabled=?4 WHERE id=?5",
                params![p.date, p.amount, p.note, p.enabled, p.id],
            )?;
            Ok(p.id)
        }
    }

    pub fn delete_prepayment(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM mortgage_prepayments WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn rate_changes(&self, mortgage_id: i64) -> anyhow::Result<Vec<RateChange>> {
        self.query_all(
            "SELECT id, mortgage_id, effective_date, annual_rate FROM mortgage_rate_changes WHERE mortgage_id=?1
             ORDER BY effective_date",
            [mortgage_id],
            |r| Ok(RateChange { id: r.get(0)?, mortgage_id: r.get(1)?, effective_date: r.get(2)?, annual_rate_pct: r.get(3)? }),
        )
    }

    pub fn add_rate_change(&self, r: &RateChange) -> anyhow::Result<i64> {
        ensure!(r.annual_rate_pct >= 0.0, "rate cannot be negative");
        self.conn.execute(
            "INSERT INTO mortgage_rate_changes (mortgage_id, effective_date, annual_rate) VALUES (?1, ?2, ?3)",
            params![r.mortgage_id, r.effective_date, r.annual_rate_pct],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_rate_change(&self, id: i64) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM mortgage_rate_changes WHERE id=?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::tests::temp_db;
    use crate::model::{Money, Mortgage, Prepayment, RateChange};
    use chrono::NaiveDate;

    #[test]
    fn mortgage_crud() {
        let (_d, db) = temp_db();
        let id = db.save_mortgage(&Mortgage::default()).unwrap();
        let date = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        db.save_prepayment(&Prepayment {
            id: 0,
            mortgage_id: id,
            date,
            amount: Money(1_000_000),
            note: "bonus".into(),
            enabled: true,
        })
        .unwrap();
        db.add_rate_change(&RateChange { id: 0, mortgage_id: id, effective_date: date, annual_rate_pct: 3.9 }).unwrap();
        assert_eq!(db.prepayments(id).unwrap()[0].note, "bonus");
        assert_eq!(db.rate_changes(id).unwrap()[0].annual_rate_pct, 3.9);
        db.delete_mortgage(id).unwrap();
        assert!(db.prepayments(id).unwrap().is_empty());
    }
}
