//! Two-level category hierarchy (e.g. `Groceries / Save on Foods`).

use std::collections::HashMap;

str_enum! {
    /// What a top-level category represents. Subcategories inherit the kind of
    /// their parent. Transfers (credit-card payments, moving money between your
    /// own accounts) are excluded from income and spending analytics.
    pub enum CategoryKind {
        Expense => ("expense", "Expense"),
        Income => ("income", "Income"),
        Transfer => ("transfer", "Transfer"),
    }
}

/// Separator used when displaying a full category path.
pub const PATH_SEPARATOR: &str = " / ";

#[derive(Clone, Debug, PartialEq)]
pub struct Category {
    pub id: i64,
    pub name: String,
    /// `None` for top-level categories.
    pub parent_id: Option<i64>,
    pub kind: CategoryKind,
}

impl Category {
    pub fn is_top_level(&self) -> bool {
        self.parent_id.is_none()
    }
}

/// Read-only, indexed view over all categories.
#[derive(Clone, Debug, Default)]
pub struct CategoryTree {
    cats: Vec<Category>,
    index: HashMap<i64, usize>,
}

impl CategoryTree {
    pub fn new(mut cats: Vec<Category>) -> Self {
        cats.sort_by_key(|c| c.name.to_lowercase());
        let index = cats.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
        Self { cats, index }
    }

    pub fn get(&self, id: i64) -> Option<&Category> {
        self.index.get(&id).map(|&i| &self.cats[i])
    }

    pub fn len(&self) -> usize {
        self.cats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cats.is_empty()
    }

    pub fn top_level(&self) -> impl Iterator<Item = &Category> {
        self.cats.iter().filter(|c| c.parent_id.is_none())
    }

    pub fn children(&self, parent: i64) -> impl Iterator<Item = &Category> {
        self.cats.iter().filter(move |c| c.parent_id == Some(parent))
    }

    /// Top-level ancestor of `id` (itself if already top-level).
    pub fn top_id(&self, id: i64) -> i64 {
        self.get(id).and_then(|c| c.parent_id).unwrap_or(id)
    }

    /// Effective kind of a category (taken from its top-level parent).
    pub fn kind(&self, id: i64) -> Option<CategoryKind> {
        self.get(self.top_id(id)).map(|c| c.kind)
    }

    /// `Parent / Child` or just `Parent`.
    pub fn full_name(&self, id: i64) -> String {
        match self.get(id) {
            Some(c) => match c.parent_id.and_then(|p| self.get(p)) {
                Some(p) => format!("{}{PATH_SEPARATOR}{}", p.name, c.name),
                None => c.name.clone(),
            },
            None => "(deleted category)".to_string(),
        }
    }

    pub fn top_name(&self, id: i64) -> String {
        self.full_name(self.top_id(id))
    }

    /// True when `id` is `ancestor` or a child of `ancestor`.
    pub fn is_under(&self, id: i64, ancestor: i64) -> bool {
        id == ancestor || self.get(id).and_then(|c| c.parent_id) == Some(ancestor)
    }

    /// Categories ordered parent-first, each followed by its children.
    pub fn ordered(&self) -> Vec<&Category> {
        let mut out = Vec::with_capacity(self.cats.len());
        for top in self.top_level() {
            out.push(top);
            out.extend(self.children(top.id));
        }
        out
    }

    /// Case-insensitive lookup by name path.
    pub fn find(&self, top: &str, sub: Option<&str>) -> Option<i64> {
        let top_cat = self.top_level().find(|c| c.name.eq_ignore_ascii_case(top.trim()))?;
        match sub {
            None => Some(top_cat.id),
            Some(sub) => self.children(top_cat.id).find(|c| c.name.eq_ignore_ascii_case(sub.trim())).map(|c| c.id),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn sample_tree() -> CategoryTree {
        CategoryTree::new(vec![
            Category { id: 1, name: "Groceries".into(), parent_id: None, kind: CategoryKind::Expense },
            Category { id: 2, name: "Save on Foods".into(), parent_id: Some(1), kind: CategoryKind::Expense },
            Category { id: 3, name: "Income".into(), parent_id: None, kind: CategoryKind::Income },
            Category { id: 4, name: "Mayfair".into(), parent_id: Some(3), kind: CategoryKind::Income },
            Category { id: 5, name: "Transfers".into(), parent_id: None, kind: CategoryKind::Transfer },
        ])
    }

    #[test]
    fn names_and_hierarchy() {
        let t = sample_tree();
        assert_eq!(t.full_name(2), "Groceries / Save on Foods");
        assert_eq!(t.full_name(1), "Groceries");
        assert_eq!(t.top_id(4), 3);
        assert_eq!(t.kind(4), Some(CategoryKind::Income));
        assert!(t.is_under(2, 1));
        assert!(!t.is_under(4, 1));
        assert_eq!(t.find("groceries", Some("save on foods")), Some(2));
        let order: Vec<i64> = t.ordered().iter().map(|c| c.id).collect();
        assert_eq!(order, vec![1, 2, 3, 4, 5]);
    }
}
