#![windows_subsystem = "windows"]

mod app;
mod gui;
mod scanner;
mod state;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("TreeSize Rust"),
        ..Default::default()
    };

    eframe::run_native(
        "TreeSize Rust",
        options,
        Box::new(|_cc| Ok(Box::new(app::TreeSizeApp::new()))),
    )
}
