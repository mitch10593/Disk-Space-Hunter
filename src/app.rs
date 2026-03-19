use eframe::egui;

use crate::gui;
use crate::state::{AppState, Tab};

pub struct TreeSizeApp {
    pub state: AppState,
}

impl TreeSizeApp {
    pub fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl eframe::App for TreeSizeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.process_scan_messages();

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            gui::toolbar::render(ui, &mut self.state, ctx);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.state.active_tab, Tab::TreeView, "Tree View");
                ui.selectable_value(&mut self.state.active_tab, Tab::Duplicates, "Duplicates");
            });
            ui.separator();

            match self.state.active_tab {
                Tab::TreeView => gui::tree_view::render(ui, &mut self.state),
                Tab::Duplicates => gui::duplicates_view::render(ui, &mut self.state),
            }
        });
    }
}
