//! Date range selection shared by the Transactions, Analytics and Dashboard pages.

use super::data::DataCache;
use super::widgets::{date_edit, enum_combo};
use crate::analytics::{DateRange, RangePreset};
use chrono::{Months, NaiveDate};
use egui::{RichText, Ui};

pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeSelector {
    pub preset: RangePreset,
    pub custom: DateRange,
}

impl RangeSelector {
    pub fn new(preset: RangePreset) -> Self {
        let t = today();
        Self { preset, custom: DateRange::new(t - Months::new(1), t) }
    }

    pub fn resolve(&self, data: &DataCache) -> DateRange {
        self.preset.resolve(today(), data.data_range(), self.custom)
    }

    /// Preset combo plus date pickers for a custom range. Returns `true` on change.
    pub fn ui(&mut self, ui: &mut Ui, id: &str, data: &DataCache) -> bool {
        let before = self.clone();
        if enum_combo(ui, (id, "preset"), &mut self.preset) && self.preset == RangePreset::Custom {
            // Start the custom range from whatever was showing.
            self.custom = before.resolve(data);
        }
        if self.preset == RangePreset::Custom {
            let (mut start, mut end) = (self.custom.start, self.custom.end);
            date_edit(ui, &format!("{id}-start"), &mut start);
            ui.label("to");
            date_edit(ui, &format!("{id}-end"), &mut end);
            self.custom = DateRange::new(start, end);
        } else {
            ui.label(RichText::new(self.resolve(data).label()).weak().small());
        }
        *self != before
    }
}
