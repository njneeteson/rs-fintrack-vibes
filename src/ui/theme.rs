//! Chart and status colours (validated categorical palette, light and dark steps).

use egui::Color32;

const SERIES_LIGHT: [Color32; 8] = [
    Color32::from_rgb(0x2a, 0x78, 0xd6), // blue
    Color32::from_rgb(0xeb, 0x68, 0x34), // orange
    Color32::from_rgb(0x1b, 0xaf, 0x7a), // aqua
    Color32::from_rgb(0xed, 0xa1, 0x00), // yellow
    Color32::from_rgb(0xe8, 0x7b, 0xa4), // magenta
    Color32::from_rgb(0x00, 0x83, 0x00), // green
    Color32::from_rgb(0x62, 0x50, 0xd6), // violet
    Color32::from_rgb(0xe3, 0x49, 0x48), // red
];

const SERIES_DARK: [Color32; 8] = [
    Color32::from_rgb(0x39, 0x87, 0xe5),
    Color32::from_rgb(0xd9, 0x59, 0x26),
    Color32::from_rgb(0x19, 0x9e, 0x70),
    Color32::from_rgb(0xc9, 0x85, 0x00),
    Color32::from_rgb(0xd5, 0x51, 0x81),
    Color32::from_rgb(0x00, 0x83, 0x00),
    Color32::from_rgb(0x90, 0x85, 0xe9),
    Color32::from_rgb(0xe6, 0x67, 0x67),
];

/// Maximum distinct series colours; anything beyond folds into "Other".
pub const MAX_SERIES: usize = SERIES_LIGHT.len();

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    series: [Color32; 8],
    /// Neutral grey for "Other" and baselines.
    pub other: Color32,
    pub income: Color32,
    pub spending: Color32,
    /// Text colour for positive amounts.
    pub positive_text: Color32,
    pub good: Color32,
    pub warning: Color32,
    pub critical: Color32,
}

/// Budget/progress status (always shown with a text label, never colour alone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Good,
    Warning,
    Critical,
}

impl Health {
    pub fn from_fraction(f: f64) -> Self {
        if f > 1.0 {
            Health::Critical
        } else if f >= 0.85 {
            Health::Warning
        } else {
            Health::Good
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Health::Good => "✔ On track",
            Health::Warning => "⚠ Close to limit",
            Health::Critical => "⛔ Over budget",
        }
    }
}

impl Palette {
    pub fn for_mode(dark: bool) -> Self {
        let series = if dark { SERIES_DARK } else { SERIES_LIGHT };
        Self {
            dark,
            series,
            other: Color32::from_rgb(0x89, 0x87, 0x81),
            income: series[0],
            spending: series[1],
            positive_text: if dark { Color32::from_rgb(0x0c, 0xa3, 0x0c) } else { Color32::from_rgb(0x00, 0x63, 0x00) },
            good: Color32::from_rgb(0x0c, 0xa3, 0x0c),
            warning: Color32::from_rgb(0xfa, 0xb2, 0x19),
            critical: Color32::from_rgb(0xd0, 0x3b, 0x3b),
        }
    }

    pub fn series(&self, i: usize) -> Color32 {
        self.series.get(i).copied().unwrap_or(self.other)
    }

    pub fn health(&self, h: Health) -> Color32 {
        match h {
            Health::Good => self.good,
            Health::Warning => self.warning,
            Health::Critical => self.critical,
        }
    }
}
