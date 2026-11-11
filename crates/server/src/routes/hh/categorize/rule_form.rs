//! The rule editor's fields as strings, and conversion to a [`Rule`].

use crate::error::AppResult;
use crate::form::{key, number, opt_id, opt_money, text};
use fintrack_core::model::Rule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuleForm {
    /// `"1"` once submitted (otherwise start from the saved rule or the query's draft).
    pub edited: String,
    pub field: String,
    pub match_kind: String,
    pub pattern: String,
    pub direction: String,
    pub min: String,
    pub max: String,
    pub account: String,
    pub category: String,
    pub priority: String,
    pub enabled: String,
}

impl RuleForm {
    pub fn from_rule(r: &Rule) -> Self {
        let amount = |m: Option<fintrack_core::model::Money>| {
            m.map(|m| m.abs().to_plain()).unwrap_or_default()
        };
        Self {
            edited: "1".into(),
            field: r.field.key().into(),
            match_kind: r.match_kind.key().into(),
            pattern: r.pattern.clone(),
            direction: r.direction.key().into(),
            min: amount(r.min_amount),
            max: amount(r.max_amount),
            account: r.account_id.map(|a| a.to_string()).unwrap_or_default(),
            category: if r.category_id == 0 {
                String::new()
            } else {
                r.category_id.to_string()
            },
            priority: r.priority.to_string(),
            enabled: if r.enabled { "1".into() } else { String::new() },
        }
    }

    /// The rule described by the form. Fails with a user-facing message.
    pub fn to_rule(&self, id: i64) -> AppResult<Rule> {
        Ok(Rule {
            id,
            field: key(&self.field, "rule field")?,
            match_kind: key(&self.match_kind, "match type")?,
            pattern: text(&self.pattern, "The text to match")?,
            direction: key(&self.direction, "direction")?,
            min_amount: opt_money(&self.min, "Minimum amount")?.map(|m| m.abs()),
            max_amount: opt_money(&self.max, "Maximum amount")?.map(|m| m.abs()),
            account_id: opt_id(&self.account),
            category_id: opt_id(&self.category).unwrap_or(0),
            priority: if self.priority.trim().is_empty() {
                100
            } else {
                number(&self.priority, "Priority")?
            },
            enabled: !self.enabled.is_empty(),
        })
    }

    /// The rule for previewing matches: like [`to_rule`](Self::to_rule) but lenient.
    pub fn draft(&self) -> Rule {
        self.to_rule(0).unwrap_or_else(|_| Rule {
            pattern: self.pattern.clone(),
            ..Default::default()
        })
    }

    pub fn to_query(&self) -> String {
        format!("?{}", serde_html_form::to_string(self).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fintrack_core::model::{Direction, MatchKind, Money};

    #[test]
    fn round_trip() {
        let r = Rule {
            id: 4,
            pattern: "COSTCO".into(),
            match_kind: MatchKind::StartsWith,
            direction: Direction::Outflow,
            min_amount: Some(Money::from_dollars(300.0)),
            category_id: 9,
            priority: 10,
            ..Default::default()
        };
        assert_eq!(RuleForm::from_rule(&r).to_rule(4).unwrap(), r);
        let bad = RuleForm {
            pattern: " ".into(),
            ..RuleForm::from_rule(&r)
        };
        assert!(bad.to_rule(4).is_err());
    }
}
