//! Form controls shared by many pages.

use super::components::tip;
use fintrack_core::enum_macro::KeyEnum;
use fintrack_core::model::{Account, CategoryTree};
use maud::{Markup, html};

/// A labelled form field with an optional tooltip.
pub fn field(label: &str, hint: &str, control: Markup) -> Markup {
    html! {
        label class="field" {
            span class="field-label" { (label) " " (tip(hint)) }
            (control)
        }
    }
}

/// How a [`live_form`] reacts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Live {
    /// Filters: every change (and Apply) refreshes the results and the address bar.
    Filter,
    /// Previews: changes refresh a preview; submit buttons still post the form normally.
    Preview,
}

/// Where a [`live_form`] puts its results.
pub struct LiveTarget<'a> {
    /// Id of the element to refresh.
    pub id: &'a str,
    /// Ids of other elements elsewhere on the page to refresh from the same response.
    pub also: &'a [&'a str],
}

/// A GET form that, with JavaScript, refreshes part of the page in place as its
/// controls change (typing is debounced) instead of reloading the page.
/// Without JavaScript it is an ordinary form.
pub fn live_form(
    action: &str,
    class: &str,
    target: LiveTarget<'_>,
    live: Live,
    body: Markup,
) -> Markup {
    let selector = format!("#{}", target.id);
    let also: Vec<String> = target.also.iter().map(|id| format!("#{id}")).collect();
    let (trigger, push_url) = match live {
        Live::Filter => ("submit, change, input delay:400ms", "true"),
        Live::Preview => ("change, input delay:500ms", "false"),
    };
    html! {
        form method="get" action=(action) class=(class)
            hx-get=(action) hx-trigger=(trigger) hx-target=(selector) hx-select=(selector)
            hx-select-oob=[(!also.is_empty()).then(|| also.join(","))]
            hx-swap="outerHTML" hx-push-url=(push_url) hx-sync="this:replace"
            hx-indicator=(selector) {
            (body)
        }
    }
}

/// A labelled checkbox that submits `value="1"` when ticked.
pub fn checkbox(name: &str, checked: bool, label: Markup) -> Markup {
    html! {
        label class="check" {
            input type="checkbox" name=(name) value="1" checked[checked];
            (label)
        }
    }
}

/// `<input type="text">`.
pub fn text_input(name: &str, value: &str) -> Markup {
    html! { input type="text" name=(name) value=(value); }
}

/// `<input type="date">` holding `YYYY-MM-DD` (empty for none).
pub fn date_input(name: &str, value: &str) -> Markup {
    html! { input type="date" name=(name) value=(value); }
}

/// `<input type="number">` with bounds and step.
pub fn number_input(
    name: &str,
    value: impl std::fmt::Display,
    range: (&str, &str),
    step: &str,
) -> Markup {
    html! {
        input type="number" name=(name) value=(value.to_string())
            min=(range.0) max=(range.1) step=(step);
    }
}

/// `<select>` over every value of a `str_enum!`; `autosubmit` submits on change.
pub fn enum_select<T: KeyEnum + PartialEq>(name: &str, selected: T, autosubmit: bool) -> Markup {
    html! {
        select name=(name) data-autosubmit[autosubmit] {
            @for v in T::all() {
                option value=(v.key()) selected[*v == selected] { (v.label()) }
            }
        }
    }
}

/// `<option>`s for every category in tree order, subcategories indented.
pub fn category_options(cats: &CategoryTree, selected: Option<i64>) -> Markup {
    html! {
        @for c in cats.ordered() {
            option value=(c.id) selected[selected == Some(c.id)] {
                @if c.parent_id.is_some() { "\u{2003}" }
                (c.name)
            }
        }
    }
}

/// Category picker. `none_label` adds an empty first option; `autosubmit`
/// submits the form when the choice changes.
pub fn category_select(
    name: &str,
    cats: &CategoryTree,
    selected: Option<i64>,
    none_label: Option<&str>,
    autosubmit: bool,
) -> Markup {
    html! {
        select name=(name) data-autosubmit[autosubmit] {
            @if let Some(label) = none_label {
                option value="" selected[selected.is_none()] { (label) }
            }
            (category_options(cats, selected))
        }
    }
}

/// `<option>`s for every account, archived ones last.
pub fn account_options(accounts: &[Account], selected: Option<i64>) -> Markup {
    let mut sorted: Vec<&Account> = accounts.iter().collect();
    sorted.sort_by_key(|a| (a.archived, a.name.to_lowercase()));
    html! {
        @for a in sorted {
            option value=(a.id) selected[selected == Some(a.id)] {
                (a.name) @if a.archived { " (archived)" }
            }
        }
    }
}

/// Account picker. `none_label` adds an empty first option; `autosubmit`
/// submits the form when the choice changes.
pub fn account_select(
    name: &str,
    accounts: &[Account],
    selected: Option<i64>,
    none_label: Option<&str>,
    autosubmit: bool,
) -> Markup {
    html! {
        select name=(name) data-autosubmit[autosubmit] {
            @if let Some(label) = none_label {
                option value="" selected[selected.is_none()] { (label) }
            }
            (account_options(accounts, selected))
        }
    }
}

/// A POST form holding a single button; `confirm` asks before submitting.
pub fn action_button(action: &str, label: &str, class: &str, confirm: Option<&str>) -> Markup {
    html! {
        form method="post" action=(action) class="inline" {
            button type="submit" class={ "btn " (class) } data-confirm=[confirm] { (label) }
        }
    }
}
