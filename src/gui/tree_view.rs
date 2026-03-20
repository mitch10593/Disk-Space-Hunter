use std::cell::RefCell;
use std::path::PathBuf;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::gui::formatting::{format_count, format_size};
use crate::scanner::file_category::FileCategory;
use crate::scanner::tree::DirNode;
use crate::state::{AppState, SortColumn};

struct FlatRow {
    depth: u16,
    index_path: Vec<usize>,
    is_expanded: bool,
    has_children: bool,
    name: String,
    path: PathBuf,
    total_size: u64,
    total_file_count: u32,
    avg_file_size: u64,
    parent_total_size: u64,
}

fn flatten_visible_tree(
    root: &DirNode,
    filter: Option<&[bool; FileCategory::COUNT]>,
) -> Vec<FlatRow> {
    let mut rows = Vec::with_capacity(1024);
    fn recurse(
        node: &DirNode,
        depth: u16,
        path: &mut Vec<usize>,
        parent_total: u64,
        filter: Option<&[bool; FileCategory::COUNT]>,
        rows: &mut Vec<FlatRow>,
    ) {
        let (total_size, total_file_count, avg_file_size) = match filter {
            Some(f) => (
                node.filtered_total_size(f),
                node.filtered_total_file_count(f),
                node.filtered_avg_file_size(f),
            ),
            None => (node.total_size, node.total_file_count, node.avg_file_size()),
        };
        // Hide directories with zero matching files when a filter is active
        if filter.is_some() && total_file_count == 0 {
            return;
        }
        let has_visible_children = match filter {
            Some(f) => node
                .children
                .iter()
                .any(|c| c.filtered_total_file_count(f) > 0),
            None => !node.children.is_empty(),
        };
        rows.push(FlatRow {
            depth,
            index_path: path.clone(),
            is_expanded: node.expanded,
            has_children: has_visible_children,
            name: node.name.clone(),
            path: node.path.clone(),
            total_size,
            total_file_count,
            avg_file_size,
            parent_total_size: parent_total,
        });
        if node.expanded {
            let node_total = total_size;
            for (i, child) in node.children.iter().enumerate() {
                path.push(i);
                recurse(child, depth + 1, path, node_total, filter, rows);
                path.pop();
            }
        }
    }
    let root_total = match filter {
        Some(f) => root.filtered_total_size(f),
        None => root.total_size,
    };
    let mut path = Vec::new();
    recurse(root, 0, &mut path, root_total, filter, &mut rows);
    rows
}

fn toggle_node_at(root: &mut DirNode, index_path: &[usize]) {
    let mut node = root;
    for &idx in index_path {
        node = &mut node.children[idx];
    }
    node.expanded = !node.expanded;
}

fn sort_tree(
    node: &mut DirNode,
    column: &SortColumn,
    ascending: bool,
    filter: Option<&[bool; FileCategory::COUNT]>,
) {
    match column {
        SortColumn::Name => {
            node.children
                .sort_by_cached_key(|child| child.name.to_lowercase());
            if !ascending {
                node.children.reverse();
            }
        }
        _ => {
            node.children.sort_by(|a, b| {
                let ord = match (column, filter) {
                    (SortColumn::TotalSize, Some(f)) => {
                        a.filtered_total_size(f).cmp(&b.filtered_total_size(f))
                    }
                    (SortColumn::TotalSize, None) => a.total_size.cmp(&b.total_size),
                    (SortColumn::FileCount, Some(f)) => a
                        .filtered_total_file_count(f)
                        .cmp(&b.filtered_total_file_count(f)),
                    (SortColumn::FileCount, None) => a.total_file_count.cmp(&b.total_file_count),
                    (SortColumn::AvgFileSize, Some(f)) => a
                        .filtered_avg_file_size(f)
                        .cmp(&b.filtered_avg_file_size(f)),
                    (SortColumn::AvgFileSize, None) => a.avg_file_size().cmp(&b.avg_file_size()),
                    (SortColumn::Name, _) => unreachable!(),
                };
                if ascending {
                    ord
                } else {
                    ord.reverse()
                }
            });
        }
    }
    for child in &mut node.children {
        sort_tree(child, column, ascending, filter);
    }
}

pub fn render(ui: &mut egui::Ui, state: &mut AppState) {
    if state.root_node.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label("Select a folder and click Scan to analyze disk usage.");
        });
        return;
    }

    let filter = state.active_filter();
    let flat_rows = flatten_visible_tree(state.root_node.as_ref().unwrap(), filter.as_ref());
    let total_rows = flat_rows.len();

    // Collect actions to apply after rendering
    let toggled: RefCell<Option<Vec<usize>>> = RefCell::new(None);
    let mut new_sort: Option<SortColumn> = None;

    let sort_col = state.sort_column;
    let sort_asc = state.sort_ascending;

    let table = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .sense(egui::Sense::click())
        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
        .column(Column::initial(350.0).at_least(150.0)) // Name
        .column(Column::initial(100.0).at_least(60.0)) // Size
        .column(Column::initial(80.0).at_least(50.0)) // Files
        .column(Column::initial(90.0).at_least(60.0)) // Avg Size
        .column(Column::initial(100.0).at_least(80.0)) // % bar
        .column(Column::remainder().at_least(90.0)); // Actions

    table
        .header(22.0, |mut header| {
            header.col(|ui| {
                if ui
                    .add(
                        egui::Label::new(sort_label_text(
                            "Name",
                            &SortColumn::Name,
                            sort_col,
                            sort_asc,
                        ))
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    new_sort = Some(SortColumn::Name);
                }
            });
            header.col(|ui| {
                if ui
                    .add(
                        egui::Label::new(sort_label_text(
                            "Size",
                            &SortColumn::TotalSize,
                            sort_col,
                            sort_asc,
                        ))
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    new_sort = Some(SortColumn::TotalSize);
                }
            });
            header.col(|ui| {
                if ui
                    .add(
                        egui::Label::new(sort_label_text(
                            "Files",
                            &SortColumn::FileCount,
                            sort_col,
                            sort_asc,
                        ))
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    new_sort = Some(SortColumn::FileCount);
                }
            });
            header.col(|ui| {
                if ui
                    .add(
                        egui::Label::new(sort_label_text(
                            "Avg Size",
                            &SortColumn::AvgFileSize,
                            sort_col,
                            sort_asc,
                        ))
                        .sense(egui::Sense::click()),
                    )
                    .clicked()
                {
                    new_sort = Some(SortColumn::AvgFileSize);
                }
            });
            header.col(|ui| {
                ui.strong("% of Parent");
            });
            header.col(|ui| {
                ui.strong("Actions");
            });
        })
        .body(|body| {
            body.rows(20.0, total_rows, |mut row| {
                let idx = row.index();
                let flat = &flat_rows[idx];

                row.col(|ui| {
                    ui.add_space(flat.depth as f32 * 20.0);
                    if flat.has_children {
                        let icon = if flat.is_expanded {
                            "\u{25BC}"
                        } else {
                            "\u{25B6}"
                        };
                        if ui.small_button(icon).clicked() {
                            *toggled.borrow_mut() = Some(flat.index_path.clone());
                        }
                        let folder_icon = if flat.is_expanded {
                            "\u{1F4C2}"
                        } else {
                            "\u{1F4C1}"
                        };
                        ui.label(format!("{} {}", folder_icon, &flat.name));
                    } else {
                        ui.add_space(20.0);
                        ui.label(format!("\u{1F4C4} {}", &flat.name));
                    }
                });
                row.col(|ui| {
                    ui.label(format_size(flat.total_size));
                });
                row.col(|ui| {
                    ui.label(format_count(flat.total_file_count));
                });
                row.col(|ui| {
                    ui.label(format_size(flat.avg_file_size));
                });
                row.col(|ui| {
                    let pct = if flat.parent_total_size > 0 {
                        flat.total_size as f32 / flat.parent_total_size as f32
                    } else {
                        0.0
                    };
                    ui.add(
                        egui::ProgressBar::new(pct)
                            .show_percentage()
                            .desired_width(ui.available_width()),
                    );
                });
                row.col(|ui| {
                    if ui.small_button("\u{1F4C2} Open").clicked() {
                        let _ = std::process::Command::new("explorer")
                            .arg(&flat.path)
                            .spawn();
                    }
                    if ui.small_button("\u{1F50D} Select").clicked() {
                        let _ = std::process::Command::new("explorer")
                            .arg("/select,")
                            .arg(&flat.path)
                            .spawn();
                    }
                });
            });
        });

    // Apply toggle
    if let Some(path) = toggled.into_inner() {
        if let Some(root) = &mut state.root_node {
            toggle_node_at(root, &path);
        }
    }

    // Apply sort
    if let Some(col) = new_sort {
        if col == state.sort_column {
            state.sort_ascending = !state.sort_ascending;
        } else {
            state.sort_column = col;
            state.sort_ascending = false;
        }
        let filter = state.active_filter();
        if let Some(root) = &mut state.root_node {
            sort_tree(
                root,
                &state.sort_column,
                state.sort_ascending,
                filter.as_ref(),
            );
        }
    }
}

fn sort_label_text(
    name: &str,
    col: &SortColumn,
    current: SortColumn,
    ascending: bool,
) -> egui::RichText {
    let arrow = if *col == current {
        if ascending {
            " \u{1F53C}"
        } else {
            " \u{1F53D}"
        }
    } else {
        ""
    };
    egui::RichText::new(format!("{}{}", name, arrow)).strong()
}
