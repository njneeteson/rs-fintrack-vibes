//! Navigation: which page is active and the household side menu.

use crate::extract::HhCtx;
use maud::{Markup, html};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Dashboard,
    Transactions,
    Import,
    Categorize,
    Categories,
    Accounts,
    Analytics,
    Budgets,
    Mortgage,
}

struct Item {
    nav: Nav,
    label: &'static str,
    path: &'static str,
    /// Hidden from view-only members.
    edit_only: bool,
}

const fn item(nav: Nav, label: &'static str, path: &'static str, edit_only: bool) -> Item {
    Item {
        nav,
        label,
        path,
        edit_only,
    }
}

/// Side-menu groups for a household: `(section title, items)`.
const SECTIONS: &[(&str, &[Item])] = &[
    (
        "Overview",
        &[
            item(Nav::Dashboard, "Dashboard", "", false),
            item(Nav::Transactions, "Transactions", "/transactions", false),
            item(Nav::Analytics, "Analytics", "/analytics", false),
            item(Nav::Budgets, "Budgets", "/budgets", false),
            item(Nav::Mortgage, "Mortgage", "/mortgage", false),
        ],
    ),
    (
        "Organize",
        &[
            item(Nav::Import, "Import CSV", "/import", true),
            item(Nav::Categorize, "Categorize", "/categorize", false),
            item(Nav::Categories, "Categories", "/categories", false),
            item(Nav::Accounts, "Accounts", "/accounts", false),
        ],
    ),
];

pub fn household_menu(h: &HhCtx, active: Nav) -> Markup {
    html! {
        nav class="sidenav" aria-label="Household" {
            @for (title, items) in SECTIONS {
                div class="section" { (title) }
                @for it in items.iter().filter(|it| h.hh.can_edit() || !it.edit_only) {
                    a href=(h.at(it.path)) aria-current=[(it.nav == active).then_some("page")] {
                        (it.label)
                    }
                }
            }
        }
    }
}
