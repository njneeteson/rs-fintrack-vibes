//! Formatting of money, percentages and dates.

use chrono::NaiveDate;
use fintrack_core::model::Money;
use maud::{Markup, html};

fn sign_class(m: Money) -> &'static str {
    if m.is_negative() {
        "num neg"
    } else if m.is_zero() {
        "num"
    } else {
        "num pos"
    }
}

/// `$1,234.56`, right-aligned digits, coloured by sign (the sign is also in the text).
pub fn money(m: Money) -> Markup {
    html! { span class=(sign_class(m)) { (m) } }
}

/// `$1,235`: whole dollars for tiles and summaries.
pub fn money_whole(m: Money) -> Markup {
    html! { span class=(sign_class(m)) { (m.to_whole()) } }
}

/// Whole dollars without sign colouring (e.g. spending, where "more" isn't "good").
pub fn whole_plain(m: Money) -> Markup {
    html! { span class="num" { (m.to_whole()) } }
}

/// Money without sign colouring (e.g. budget amounts, where "positive" isn't "good").
pub fn money_plain(m: Money) -> Markup {
    html! { span class="num" { (m) } }
}

pub fn pct(fraction: f64) -> String {
    format!("{:.0}%", fraction * 100.0)
}

pub fn date(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub fn short_date(d: NaiveDate) -> String {
    d.format("%b %d").to_string()
}

/// `+$120 vs last month` / `-$45 vs last month`.
pub fn change(now: Money, before: Money, what: &str) -> String {
    let d = now - before;
    let sign = if d.is_negative() { "" } else { "+" };
    format!("{sign}{} vs {what}", d.to_whole())
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}
