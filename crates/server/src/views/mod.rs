//! HTML building blocks (maud). Pages live in `routes`; this module holds
//! what they share.

pub mod charts;
pub mod components;
pub mod format;
pub mod forms;
pub mod layout;
pub mod nav;

pub use layout::{Page, bare_page, hh_page, page};
pub use nav::Nav;
