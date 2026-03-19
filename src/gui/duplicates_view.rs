use eframe::egui;

use crate::gui::formatting::format_size;
use crate::scanner::tree::DuplicateStatus;
use crate::state::{AppState, ScanStatus};

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    let is_scanning = state.scan_status == ScanStatus::DetectingDuplicates;

    // Show candidates during scan
    if is_scanning && !state.dup_candidates.is_empty() {
        render_candidates(ui, state);
        return;
    }

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

fn render_candidates(ui: &mut egui::Ui, state: &AppState) {
    let confirmed = state.dup_candidates.iter().filter(|c| c.status == DuplicateStatus::Confirmed).count();
    let pending = state.dup_candidates.len() - confirmed;

    let total_wasted_est: u64 = state.dup_candidates.iter()
        .map(|c| c.file_size * (c.paths.len() as u64 - 1))
        .sum();

    ui.heading(format!(
        "\u{1F50D} Analysing duplicates — \u{2705} {} confirmed, \u{1F50E} {} pending — ~{} wasted",
        confirmed, pending, format_size(total_wasted_est)
    ));
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        for candidate in &state.dup_candidates {
            let emoji = match candidate.status {
                DuplicateStatus::SameSize => "\u{1F50D}",
                DuplicateStatus::SamePartialHash => "\u{1F50E}",
                DuplicateStatus::Confirmed => "\u{2705}",
            };
            let wasted = candidate.file_size * (candidate.paths.len() as u64 - 1);
            let header_text = format!(
                "{} {} — {} files — {} wasted",
                emoji,
                format_size(candidate.file_size),
                candidate.paths.len(),
                format_size(wasted)
            );

            egui::CollapsingHeader::new(header_text)
                .id_salt(candidate.paths.first().map(|p| p.display().to_string()).unwrap_or_default())
                .show(ui, |ui| {
                    for path in &candidate.paths {
                        ui.horizontal(|ui| {
                            if ui.small_button("\u{1F4C2}").on_hover_text("Show in Explorer").clicked() {
                                #[cfg(target_os = "windows")]
                                {
                                    let _ = std::process::Command::new("explorer")
                                        .arg("/select,")
                                        .arg(path.as_os_str())
                                        .spawn();
                                }
                            }
                            ui.label(path.display().to_string());
                        });
                    }
                });
        }
    });
}
