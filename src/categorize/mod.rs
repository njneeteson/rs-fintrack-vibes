//! Rule engine: maps transaction fields to categories.
//!
//! Rules are evaluated in priority order (lowest number first, then oldest);
//! the first enabled rule that matches decides the transaction's *automatic*
//! category. A *manual* category set by the user always takes precedence and is
//! never touched by the engine.

use crate::model::{Direction, MatchKind, Money, Rule, RuleField, Transaction, normalize_text};
use regex::Regex;

/// The parts of a transaction rules look at.
#[derive(Clone, Copy, Debug)]
pub struct MatchInput<'a> {
    pub account_id: i64,
    pub amount: Money,
    pub description: &'a str,
    pub memo: &'a str,
}

impl<'a> From<&'a Transaction> for MatchInput<'a> {
    fn from(t: &'a Transaction) -> Self {
        Self { account_id: t.account_id, amount: t.amount, description: &t.description, memo: &t.memo }
    }
}

/// Input with normalized text, computed once per transaction.
struct Prepared<'a> {
    raw: MatchInput<'a>,
    description: String,
    memo: String,
}

enum Matcher {
    Text(String),
    Regex(Regex),
}

pub struct CompiledRule {
    pub rule: Rule,
    matcher: Matcher,
}

impl CompiledRule {
    pub fn compile(rule: &Rule) -> Result<Self, String> {
        let matcher = match rule.match_kind {
            MatchKind::Regex => {
                Matcher::Regex(Regex::new(&format!("(?i){}", rule.pattern)).map_err(|e| format!("invalid regex: {e}"))?)
            }
            _ => Matcher::Text(normalize_text(&rule.pattern)),
        };
        if rule.pattern.trim().is_empty() {
            return Err("pattern is empty".into());
        }
        Ok(Self { rule: rule.clone(), matcher })
    }

    pub fn matches(&self, input: MatchInput<'_>) -> bool {
        self.matches_prepared(&prepare(input))
    }

    fn matches_prepared(&self, input: &Prepared<'_>) -> bool {
        let r = &self.rule;
        let amount = input.raw.amount;
        let abs = amount.abs();
        let filters_ok = r.account_id.is_none_or(|a| a == input.raw.account_id)
            && match r.direction {
                Direction::Any => true,
                Direction::Outflow => amount.is_negative(),
                Direction::Inflow => !amount.is_negative(),
            }
            && r.min_amount.is_none_or(|m| abs >= m.abs())
            && r.max_amount.is_none_or(|m| abs <= m.abs());
        if !filters_ok {
            return false;
        }
        let texts: &[(&str, &str)] = &[(&input.description, input.raw.description), (&input.memo, input.raw.memo)];
        let candidates = match r.field {
            RuleField::Description => &texts[..1],
            RuleField::Memo => &texts[1..],
            RuleField::Either => texts,
        };
        candidates.iter().any(|(norm, raw)| match &self.matcher {
            Matcher::Regex(re) => re.is_match(raw),
            Matcher::Text(p) => match r.match_kind {
                MatchKind::Contains => norm.contains(p.as_str()),
                MatchKind::StartsWith => norm.starts_with(p.as_str()),
                MatchKind::Equals => *norm == p.as_str(),
                MatchKind::Regex => unreachable!("regex rules use the regex matcher"),
            },
        })
    }
}

fn prepare(raw: MatchInput<'_>) -> Prepared<'_> {
    Prepared { raw, description: normalize_text(raw.description), memo: normalize_text(raw.memo) }
}

/// All enabled rules, compiled and sorted by priority.
pub struct RuleEngine {
    rules: Vec<CompiledRule>,
    /// Rules that failed to compile: `(rule id, message)`.
    pub errors: Vec<(i64, String)>,
}

impl RuleEngine {
    pub fn new(rules: &[Rule]) -> Self {
        let mut sorted: Vec<&Rule> = rules.iter().filter(|r| r.enabled).collect();
        sorted.sort_by_key(|r| (r.priority, r.id));
        let mut compiled = Vec::new();
        let mut errors = Vec::new();
        for r in sorted {
            match CompiledRule::compile(r) {
                Ok(c) => compiled.push(c),
                Err(e) => errors.push((r.id, e)),
            }
        }
        Self { rules: compiled, errors }
    }

    /// The first rule matching `input`, if any.
    pub fn first_match(&self, input: MatchInput<'_>) -> Option<&Rule> {
        let prepared = prepare(input);
        self.rules.iter().find(|r| r.matches_prepared(&prepared)).map(|r| &r.rule)
    }

    /// `(category_id, rule_id)` assigned to `input`.
    pub fn categorize(&self, input: MatchInput<'_>) -> (Option<i64>, Option<i64>) {
        self.first_match(input).map_or((None, None), |r| (Some(r.category_id), Some(r.id)))
    }
}

/// Suggest a rule pattern (and a payee grouping key) from a raw description by
/// keeping the leading words before store numbers, reference codes, etc.
///
/// `SAVE ON FOODS #2134 VANCOUVER BC` → `SAVE ON FOODS`.
pub fn suggest_pattern(description: &str) -> String {
    let norm = normalize_text(description);
    let mut words = Vec::new();
    for w in norm.split(' ') {
        let noisy = w.chars().any(|c| c.is_ascii_digit() || matches!(c, '#' | '*' | '/'));
        if noisy || words.len() >= 5 {
            break;
        }
        words.push(w);
    }
    if words.is_empty() { norm.split(' ').take(2).collect::<Vec<_>>().join(" ") } else { words.join(" ") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(desc: &'static str, amount: i64) -> MatchInput<'static> {
        MatchInput { account_id: 1, amount: Money(amount), description: desc, memo: "" }
    }

    #[test]
    fn priority_and_filters() {
        let rules = vec![
            Rule { id: 1, pattern: "save on".into(), category_id: 10, priority: 50, ..Default::default() },
            Rule { id: 2, pattern: "SAVE ON FOODS".into(), category_id: 20, priority: 10, ..Default::default() },
            Rule {
                id: 3,
                pattern: "^payroll".into(),
                match_kind: MatchKind::Regex,
                direction: Direction::Inflow,
                category_id: 30,
                ..Default::default()
            },
            Rule { id: 4, pattern: "x".into(), category_id: 40, enabled: false, ..Default::default() },
            Rule { id: 5, pattern: "(".into(), match_kind: MatchKind::Regex, category_id: 50, ..Default::default() },
        ];
        let e = RuleEngine::new(&rules);
        assert_eq!(e.errors.len(), 1);
        assert_eq!(e.categorize(input("Save  on Foods #12", -100)), (Some(20), Some(2)));
        assert_eq!(e.categorize(input("save on everything", -100)), (Some(10), Some(1)));
        assert_eq!(e.categorize(input("Payroll Mayfair", 500)), (Some(30), Some(3)));
        assert_eq!(e.categorize(input("Payroll reversal", -500)), (None, None));
        assert_eq!(e.categorize(input("x", -1)), (None, None));
    }

    #[test]
    fn amount_bounds_and_memo() {
        let r = CompiledRule::compile(&Rule {
            pattern: "netflix".into(),
            field: RuleField::Memo,
            min_amount: Some(Money(1000)),
            max_amount: Some(Money(2000)),
            ..Default::default()
        })
        .unwrap();
        let mut i = input("POS", -1599);
        i.memo = "NETFLIX.COM";
        assert!(r.matches(i));
        i.amount = Money(-2500);
        assert!(!r.matches(i));
    }

    #[test]
    fn suggestions() {
        assert_eq!(suggest_pattern("SAVE ON FOODS #2134 VANCOUVER BC"), "SAVE ON FOODS");
        assert_eq!(suggest_pattern("AMZN Mktp CA*2K3LL"), "AMZN MKTP");
        assert_eq!(suggest_pattern("12345 SOMETHING"), "12345 SOMETHING");
    }
}
