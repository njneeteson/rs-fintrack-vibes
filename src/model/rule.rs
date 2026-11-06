//! Categorization rules ("mappings" from transaction fields to categories).

use super::Money;

str_enum! {
    /// Which transaction text a rule looks at.
    pub enum RuleField {
        Description => ("description", "Description"),
        Memo => ("memo", "Memo / details"),
        Either => ("either", "Description or memo"),
    }
}

str_enum! {
    pub enum MatchKind {
        Contains => ("contains", "contains"),
        StartsWith => ("starts_with", "starts with"),
        Equals => ("equals", "equals"),
        Regex => ("regex", "matches regex"),
    }
}

str_enum! {
    /// Restrict a rule to money going out, coming in, or either.
    pub enum Direction {
        Any => ("any", "In or out"),
        Outflow => ("outflow", "Money out only"),
        Inflow => ("inflow", "Money in only"),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub id: i64,
    pub field: RuleField,
    pub match_kind: MatchKind,
    pub pattern: String,
    pub direction: Direction,
    /// Optional bounds on the absolute amount (inclusive).
    pub min_amount: Option<Money>,
    pub max_amount: Option<Money>,
    /// Only apply to transactions in this account.
    pub account_id: Option<i64>,
    pub category_id: i64,
    /// Lower numbers are evaluated first; the first matching rule wins.
    pub priority: i32,
    pub enabled: bool,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            id: 0,
            field: RuleField::Description,
            match_kind: MatchKind::Contains,
            pattern: String::new(),
            direction: Direction::Any,
            min_amount: None,
            max_amount: None,
            account_id: None,
            category_id: 0,
            priority: 100,
            enabled: true,
        }
    }
}

impl Rule {
    /// One-line description such as `Description contains "SAVE ON FOODS"`.
    pub fn describe(&self) -> String {
        let mut s = format!("{} {} \"{}\"", self.field, self.match_kind, self.pattern);
        if self.direction != Direction::Any {
            s.push_str(&format!(", {}", self.direction.label().to_lowercase()));
        }
        match (self.min_amount, self.max_amount) {
            (Some(lo), Some(hi)) => s.push_str(&format!(", {lo}–{hi}")),
            (Some(lo), None) => s.push_str(&format!(", ≥ {lo}")),
            (None, Some(hi)) => s.push_str(&format!(", ≤ {hi}")),
            (None, None) => {}
        }
        s
    }
}
