use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Receiver;

use crate::scanner::tree::{DirNode, DuplicateGroup};

pub enum ScanMessage {
    Progress {
        dirs_scanned: u32,
        files_scanned: u32,
        current_path: String,
    },
    TreeComplete(Box<DirNode>),
    DuplicateProgress {
        phase: String,
        checked: u32,
        total: u32,
    },
    DuplicatesComplete(Vec<DuplicateGroup>),
    #[allow(dead_code)]
    Error(String),
}

#[derive(PartialEq, Clone)]
pub enum ScanStatus {
    Idle,
    ScanningTree,
    DetectingDuplicates,
    Complete,
    Cancelled,
    Error(String),
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    TreeView,
    Duplicates,
}

#[derive(PartialEq, Clone, Copy)]
pub enum SortColumn {
    Name,
    TotalSize,
    FileCount,
    AvgFileSize,
}

pub struct AppState {
    pub scan_path: String,
    pub scan_status: ScanStatus,
    pub root_node: Option<DirNode>,
    pub duplicates: Vec<DuplicateGroup>,
    pub active_tab: Tab,
    pub sort_column: SortColumn,
    pub sort_ascending: bool,
    pub scan_rx: Option<Receiver<ScanMessage>>,
    pub dirs_scanned: u32,
    pub files_scanned: u32,
    pub current_scan_path: String,
    pub dup_progress_phase: String,
    pub dup_progress_checked: u32,
    pub dup_progress_total: u32,
    pub detect_duplicates: bool,
    pub min_dup_size: u64,
    pub cancel_flag: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scan_path: String::new(),
            scan_status: ScanStatus::Idle,
            root_node: None,
            duplicates: Vec::new(),
            active_tab: Tab::TreeView,
            sort_column: SortColumn::TotalSize,
            sort_ascending: false,
            scan_rx: None,
            dirs_scanned: 0,
            files_scanned: 0,
            current_scan_path: String::new(),
            dup_progress_phase: String::new(),
            dup_progress_checked: 0,
            dup_progress_total: 0,
            detect_duplicates: false,
            min_dup_size: 1024 * 1024, // 1 MB
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl AppState {
    pub fn cancel_scan(&mut self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
        self.scan_status = ScanStatus::Cancelled;
        self.scan_rx = None;
    }

    pub fn process_scan_messages(&mut self) {
        let rx = match &self.scan_rx {
            Some(rx) => rx.clone(),
            None => return,
        };

        while let Ok(msg) = rx.try_recv() {
            match msg {
                ScanMessage::Progress {
                    dirs_scanned,
                    files_scanned,
                    current_path,
                } => {
                    self.dirs_scanned = dirs_scanned;
                    self.files_scanned = files_scanned;
                    self.current_scan_path = current_path;
                }
                ScanMessage::TreeComplete(node) => {
                    let mut node = *node;
                    node.expanded = true; // expand root by default
                    self.root_node = Some(node);
                    if self.detect_duplicates {
                        self.scan_status = ScanStatus::DetectingDuplicates;
                    } else {
                        self.scan_status = ScanStatus::Complete;
                        self.scan_rx = None;
                    }
                }
                ScanMessage::DuplicateProgress {
                    phase,
                    checked,
                    total,
                } => {
                    self.dup_progress_phase = phase;
                    self.dup_progress_checked = checked;
                    self.dup_progress_total = total;
                }
                ScanMessage::DuplicatesComplete(groups) => {
                    self.duplicates = groups;
                    self.scan_status = ScanStatus::Complete;
                    self.scan_rx = None;
                }
                ScanMessage::Error(e) => {
                    self.scan_status = ScanStatus::Error(e);
                    self.scan_rx = None;
                }
            }
        }
    }
}
