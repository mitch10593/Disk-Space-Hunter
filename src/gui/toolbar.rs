use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eframe::egui;

use crate::scanner;
use crate::scanner::file_category::FileCategory;
use crate::state::{AppState, ScanStatus};

pub fn render(ui: &mut egui::Ui, state: &mut AppState, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.label("Path:");
        ui.add(egui::TextEdit::singleline(&mut state.scan_path).desired_width(400.0));

        if ui.button("\u{1F4C1} Browse").clicked() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                state.scan_path = folder.display().to_string();
            }
        }

        let is_scanning = matches!(
            state.scan_status,
            ScanStatus::ScanningTree | ScanStatus::DetectingDuplicates
        );

        if ui.add_enabled(!is_scanning, egui::Button::new("\u{1F50D} Scan")).clicked() {
            start_scan(state, ctx);
        }

        if is_scanning {
            if ui.button("\u{274C} Cancel").clicked() {
                state.cancel_scan();
            }
        }
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.detect_duplicates, "Detect duplicates");
        if state.detect_duplicates {
            ui.label("Min size:");
            let mut mb = (state.min_dup_size / (1024 * 1024)) as u32;
            if ui
                .add(egui::DragValue::new(&mut mb).range(0..=1024).suffix(" MB"))
                .changed()
            {
                state.min_dup_size = mb as u64 * 1024 * 1024;
            }
        }
    });

    if state.root_node.is_some() {
        ui.horizontal(|ui| {
            ui.label("Filter:");
            for &cat in &FileCategory::ALL {
                let selected = state.category_filter[cat.index()];
                let label = format!("{} {}", cat.emoji(), cat.label());
                if ui.selectable_label(selected, label).clicked() {
                    state.toggle_category(cat);
                }
            }
            if state.any_filter_active
                && ui.small_button("Clear").clicked()
            {
                state.clear_filters();
            }
        });
    }

    // Progress display
    match &state.scan_status {
        ScanStatus::ScanningTree => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "Scanning: {} dirs, {} files",
                    state.dirs_scanned, state.files_scanned
                ));
            });
            if !state.current_scan_path.is_empty() {
                ui.label(
                    egui::RichText::new(&state.current_scan_path)
                        .small()
                        .weak(),
                );
            }
        }
        ScanStatus::DetectingDuplicates => {
            ui.horizontal(|ui| {
                ui.spinner();
                if state.dup_progress_total > 0 {
                    let progress =
                        state.dup_progress_checked as f32 / state.dup_progress_total as f32;

                    // Speed and ETA based on bytes
                    let speed_eta = if let Some(start) = state.dup_phase_start {
                        let elapsed = start.elapsed().as_secs_f64();
                        if elapsed > 0.5 && state.dup_bytes_read > 0 {
                            let speed = state.dup_bytes_read as f64 / elapsed;
                            let speed_str = format!(
                                "{}/s",
                                crate::gui::formatting::format_size(speed as u64)
                            );
                            let remaining_bytes =
                                state.dup_bytes_total.saturating_sub(state.dup_bytes_read);
                            let eta_secs = (remaining_bytes as f64 / speed) as u64;
                            let eta_str = if eta_secs >= 60 {
                                format!("{}m {:02}s", eta_secs / 60, eta_secs % 60)
                            } else {
                                format!("{}s", eta_secs)
                            };
                            format!(" — {} — ETA {}", speed_str, eta_str)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    };

                    ui.label(format!(
                        "Duplicates: {} ({}/{}){}",
                        state.dup_progress_phase,
                        state.dup_progress_checked,
                        state.dup_progress_total,
                        speed_eta
                    ));
                    ui.add(egui::ProgressBar::new(progress).desired_width(200.0));
                } else {
                    ui.label(format!("Duplicates: {}", state.dup_progress_phase));
                }
            });
        }
        ScanStatus::Error(e) => {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", e));
        }
        ScanStatus::Complete => {
            if let Some(root) = &state.root_node {
                ui.label(format!(
                    "Scan complete: {} files, {}",
                    crate::gui::formatting::format_count(root.total_file_count),
                    crate::gui::formatting::format_size(root.total_size)
                ));
            }
        }
        ScanStatus::Cancelled => {
            ui.label("Scan cancelled.");
        }
        ScanStatus::Idle => {}
    }
}

fn start_scan(state: &mut AppState, ctx: &egui::Context) {
    let path = state.scan_path.clone();
    if path.is_empty() {
        return;
    }

    let (tx, rx) = crossbeam_channel::unbounded();
    state.scan_rx = Some(rx);
    state.scan_status = ScanStatus::ScanningTree;
    state.dirs_scanned = 0;
    state.files_scanned = 0;
    state.current_scan_path.clear();
    state.root_node = None;
    state.duplicates.clear();
    state.dup_candidates.clear();
    state.cancel_flag = Arc::new(AtomicBool::new(false));

    let repaint_ctx = ctx.clone();
    let detect_dupes = state.detect_duplicates;
    let min_size = state.min_dup_size;
    let cancel = state.cancel_flag.clone();

    std::thread::spawn(move || {
        let repaint = || repaint_ctx.request_repaint();
        scanner::walk::scan_directory(&path, &tx, &repaint, &cancel);

        if detect_dupes && !cancel.load(Ordering::Relaxed) {
            scanner::duplicates::detect(&path, min_size, &tx, &repaint, &cancel);
        }
    });
}
