// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use fintrack::ui::{FinTrackApp, LaunchOptions};

fn main() -> eframe::Result<()> {
    let launch = LaunchOptions::parse(std::env::args().skip(1)).unwrap_or_else(|msg| {
        eprintln!("{msg}");
        std::process::exit(2);
    });
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FinTrack")
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([960.0, 600.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("FinTrack", options, Box::new(move |cc| Ok(Box::new(FinTrackApp::new(cc, launch)))))
}
