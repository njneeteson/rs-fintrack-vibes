//! Exact money arithmetic using integer cents.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// A signed amount of money in integer cents.
///
/// Convention used throughout the app: **positive = money in**, **negative = money out**.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Money(pub i64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub fn from_cents(cents: i64) -> Self {
        Money(cents)
    }

    pub fn cents(self) -> i64 {
        self.0
    }

    pub fn from_dollars(dollars: f64) -> Self {
        Money((dollars * 100.0).round() as i64)
    }

    pub fn dollars(self) -> f64 {
        self.0 as f64 / 100.0
    }

    pub fn abs(self) -> Self {
        Money(self.0.abs())
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Parse amounts as they appear in bank exports and user input.
    ///
    /// Accepts `1234.5`, `-1,234.56`, `$12.00`, `(12.00)` (negative),
    /// `12.00-` (negative), `+5`, `CAD 3.10`. Returns `None` for empty or
    /// non-numeric input.
    pub fn parse(input: &str) -> Option<Money> {
        let mut s = input.trim();
        let mut negative = false;
        if s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
            negative = true;
            s = &s[1..s.len() - 1];
        }
        let mut t: String = s
            .chars()
            .filter(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
            .collect();
        if t.ends_with('-') {
            negative = !negative;
            t.pop();
        }
        let mut body = t.as_str();
        if let Some(rest) = body.strip_prefix('-') {
            negative = !negative;
            body = rest;
        } else if let Some(rest) = body.strip_prefix('+') {
            body = rest;
        }
        let cents = parse_decimal_cents(body)?;
        Some(Money(if negative { -cents } else { cents }))
    }

    /// Plain decimal representation without currency symbol or grouping (`-1234.50`).
    pub fn to_plain(self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        format!("{sign}{}.{:02}", abs / 100, abs % 100)
    }

    /// Whole-dollar display (`$1,235`) for compact tiles and axes.
    pub fn to_whole(self) -> String {
        let dollars = (self.0 as f64 / 100.0).round() as i64;
        let sign = if dollars < 0 { "-" } else { "" };
        format!("{sign}${}", group_thousands(dollars.unsigned_abs()))
    }
}

fn parse_decimal_cents(body: &str) -> Option<i64> {
    if body.is_empty() || body.chars().any(|c| !c.is_ascii_digit() && c != '.') {
        return None;
    }
    let mut parts = body.split('.');
    let int_part = parts.next().unwrap_or("");
    let frac_part = parts.next().unwrap_or("");
    if parts.next().is_some() || (int_part.is_empty() && frac_part.is_empty()) {
        return None;
    }
    let int: i64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().ok()?
    };
    let mut frac_digits = frac_part.chars().map(|c| c as i64 - '0' as i64);
    let tens = frac_digits.next().unwrap_or(0);
    let ones = frac_digits.next().unwrap_or(0);
    let round_up = frac_digits.next().is_some_and(|d| d >= 5);
    int.checked_mul(100)?
        .checked_add(tens * 10 + ones + i64::from(round_up))
}

/// Insert thousands separators: `1234567` -> `1,234,567`.
pub fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Compact axis label for a dollar value: `$950`, `$1.2k`, `$3.4M`.
pub fn compact_dollars(value: f64) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let v = value.abs();
    if v >= 1_000_000.0 {
        format!("{sign}${:.1}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{sign}${:.1}k", v / 1_000.0)
    } else {
        format!("{sign}${v:.0}")
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{sign}${}.{:02}", group_thousands(abs / 100), abs % 100)
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        self.0 -= rhs.0;
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        Money(iter.map(|m| m.0).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bank_formats() {
        assert_eq!(Money::parse("12.34"), Some(Money(1234)));
        assert_eq!(Money::parse("-1,234.56"), Some(Money(-123456)));
        assert_eq!(Money::parse("$5"), Some(Money(500)));
        assert_eq!(Money::parse("(7.10)"), Some(Money(-710)));
        assert_eq!(Money::parse("7.10-"), Some(Money(-710)));
        assert_eq!(Money::parse("+0.5"), Some(Money(50)));
        assert_eq!(Money::parse(".99"), Some(Money(99)));
        assert_eq!(Money::parse("CAD 3.10"), Some(Money(310)));
        assert_eq!(Money::parse("1.005"), Some(Money(101)));
        assert_eq!(Money::parse(""), None);
        assert_eq!(Money::parse("abc"), None);
        assert_eq!(Money::parse("1.2.3"), None);
    }

    #[test]
    fn formats() {
        assert_eq!(Money(-123456).to_string(), "-$1,234.56");
        assert_eq!(Money(5).to_string(), "$0.05");
        assert_eq!(Money(-50).to_plain(), "-0.50");
        assert_eq!(Money(123_449).to_whole(), "$1,234");
        assert_eq!(compact_dollars(1500.0), "$1.5k");
        assert_eq!(group_thousands(1_000_000), "1,000,000");
    }
}
