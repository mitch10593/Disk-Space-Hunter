use eframe::egui;

use crate::gui::formatting::format_size;
use crate::state::AppState;

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    if state.duplicates.is_empty() {
        ui.centered_and_justified(|ui| {
            if state.detect_duplicates {
                ui.label("No duplicates found (or scan not yet complete).");
            } else {
                ui.label("Enable 'Detect duplicates' in the toolbar, then scan.");
            }
        });
        return;
    }

    let total_wasted: u64 = state.duplicates.iter().map(|g| g.wasted_space).sum();
    ui.heading(format!(
        "\u{26A0}\u{FE0F} {} duplicate groups — {} reclaimable",
        state.duplicates.len(),
        format_size(total_wasted)
    ));
    ui.separator();

    use egui_extras::{Column, TableBuilder};
    let total_groups = state.duplicates.len();

    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(100.0).at_least(60.0)) // File Size
        .column(Column::initial(60.0).at_least(40.0)) // Count
        .column(Column::initial(100.0).at_least(60.0)) // Wasted
        .column(Column::remainder().at_least(200.0)) // Paths
        .header(22.0, |mut header| {
            header.col(|ui| { ui.strong("File Size"); });
            header.col(|ui| { ui.strong("Count"); });
            header.col(|ui| { ui.strong("Wasted"); });
            header.col(|ui| { ui.strong("Paths"); });
        })
        .body(|body| {
            body.rows(20.0, total_groups, |mut row| {
                let group = &state.duplicates[row.index()];
                row.col(|ui| {
                    ui.label(format_size(group.file_size));
                });
                row.col(|ui| {
                    ui.label(format!("{}", group.paths.len()));
                });
                row.col(|ui| {
                    ui.label(format_size(group.wasted_space));
                });
                row.col(|ui| {
                    // Show first path, tooltip with all
                    let first = group.paths[0].display().to_string();
                    let resp = ui.label(&first);
                    if group.paths.len() > 1 {
                        resp.on_hover_ui(|ui| {
                            for p in &group.paths {
                                ui.label(p.display().to_string());
                            }
                        });
                    }
                });
            });
        });
}
