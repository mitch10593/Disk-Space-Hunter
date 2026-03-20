use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Receiver;

use crate::scanner::file_category::FileCategory;
use crate::scanner::tree::{DirNode, DuplicateCandidate, DuplicateGroup};

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
        bytes_read: u64,
        bytes_total: u64,
    },
    DuplicateCandidates(Vec<DuplicateCandidate>),
    DuplicatesComplete(Vec<DuplicateGroup>),
    #[allow(dead_code)]
    Error(String),
}

#[derive(Debug, PartialEq, Clone)]
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
    pub dup_candidates: Vec<DuplicateCandidate>,
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
    pub dup_bytes_read: u64,
    pub dup_bytes_total: u64,
    pub dup_phase_start: Option<Instant>,
    pub detect_duplicates: bool,
    pub min_dup_size: u64,
    pub cancel_flag: Arc<AtomicBool>,
    pub category_filter: [bool; FileCategory::COUNT],
    pub any_filter_active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scan_path: String::new(),
            scan_status: ScanStatus::Idle,
            root_node: None,
            duplicates: Vec::new(),
            dup_candidates: Vec::new(),
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
            dup_bytes_read: 0,
            dup_bytes_total: 0,
            dup_phase_start: None,
            detect_duplicates: false,
            min_dup_size: 1024 * 1024, // 1 MB
            cancel_flag: Arc::new(AtomicBool::new(false)),
            category_filter: [false; FileCategory::COUNT],
            any_filter_active: false,
        }
    }
}

impl AppState {
    pub fn toggle_category(&mut self, cat: FileCategory) {
        let idx = cat.index();
        self.category_filter[idx] = !self.category_filter[idx];
        self.any_filter_active = self.category_filter.iter().any(|&v| v);
    }

    pub fn active_filter(&self) -> Option<[bool; FileCategory::COUNT]> {
        if self.any_filter_active {
            Some(self.category_filter)
        } else {
            None
        }
    }

    pub fn clear_filters(&mut self) {
        self.category_filter = [false; FileCategory::COUNT];
        self.any_filter_active = false;
    }

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
                    bytes_read,
                    bytes_total,
                } => {
                    if self.dup_progress_phase != phase {
                        self.dup_phase_start = Some(Instant::now());
                    }
                    self.dup_progress_phase = phase;
                    self.dup_progress_checked = checked;
                    self.dup_progress_total = total;
                    self.dup_bytes_read = bytes_read;
                    self.dup_bytes_total = bytes_total;
                }
                ScanMessage::DuplicateCandidates(candidates) => {
                    self.dup_candidates = candidates;
                }
                ScanMessage::DuplicatesComplete(groups) => {
                    self.duplicates = groups;
                    self.dup_candidates.clear();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    // ── default ──────────────────────────────────────────────

    #[test]
    fn given_nothing_when_default_then_initial_state_is_correct() {
        // GIVEN / WHEN
        let state = AppState::default();
        // THEN
        assert_eq!(state.scan_status, ScanStatus::Idle);
        assert_eq!(state.min_dup_size, 1024 * 1024);
        assert!(!state.any_filter_active);
        assert!(!state.detect_duplicates);
        assert!(state.category_filter.iter().all(|&v| !v));
    }

    // ── toggle_category ──────────────────────────────────────

    #[test]
    fn given_no_filters_when_toggle_category_then_that_category_is_active() {
        // GIVEN
        let mut state = AppState::default();
        // WHEN
        state.toggle_category(FileCategory::Video);
        // THEN
        assert!(state.category_filter[FileCategory::Video.index()]);
        assert!(state.any_filter_active);
    }

    #[test]
    fn given_active_filter_when_toggle_same_category_then_deactivates() {
        // GIVEN
        let mut state = AppState::default();
        state.toggle_category(FileCategory::Video);
        // WHEN
        state.toggle_category(FileCategory::Video);
        // THEN
        assert!(!state.category_filter[FileCategory::Video.index()]);
        assert!(!state.any_filter_active);
    }

    #[test]
    fn given_two_active_filters_when_toggle_one_off_then_other_remains() {
        // GIVEN
        let mut state = AppState::default();
        state.toggle_category(FileCategory::Video);
        state.toggle_category(FileCategory::Music);
        // WHEN
        state.toggle_category(FileCategory::Video);
        // THEN
        assert!(!state.category_filter[FileCategory::Video.index()]);
        assert!(state.category_filter[FileCategory::Music.index()]);
        assert!(state.any_filter_active);
    }

    // ── active_filter ────────────────────────────────────────

    #[test]
    fn given_no_filters_active_when_active_filter_then_returns_none() {
        // GIVEN
        let state = AppState::default();
        // WHEN
        let result = state.active_filter();
        // THEN
        assert!(result.is_none());
    }

    #[test]
    fn given_filter_active_when_active_filter_then_returns_some_with_array() {
        // GIVEN
        let mut state = AppState::default();
        state.toggle_category(FileCategory::Video);
        // WHEN
        let result = state.active_filter();
        // THEN
        let filter = result.expect("should be Some");
        assert!(filter[FileCategory::Video.index()]);
    }

    // ── clear_filters ────────────────────────────────────────

    #[test]
    fn given_active_filters_when_clear_filters_then_all_false() {
        // GIVEN
        let mut state = AppState::default();
        state.toggle_category(FileCategory::Video);
        state.toggle_category(FileCategory::Music);
        // WHEN
        state.clear_filters();
        // THEN
        assert!(state.category_filter.iter().all(|&v| !v));
        assert!(!state.any_filter_active);
    }

    // ── cancel_scan ──────────────────────────────────────────

    #[test]
    fn given_scanning_state_when_cancel_scan_then_flag_set_and_status_cancelled() {
        // GIVEN
        let mut state = AppState::default();
        state.scan_status = ScanStatus::ScanningTree;
        // WHEN
        state.cancel_scan();
        // THEN
        assert!(state.cancel_flag.load(Ordering::Relaxed));
        assert_eq!(state.scan_status, ScanStatus::Cancelled);
        assert!(state.scan_rx.is_none());
    }

    // ── process_scan_messages ────────────────────────────────

    #[test]
    fn given_no_receiver_when_process_scan_messages_then_noop() {
        // GIVEN
        let mut state = AppState::default();
        // WHEN — no panic expected
        state.process_scan_messages();
        // THEN
        assert_eq!(state.scan_status, ScanStatus::Idle);
    }

    #[test]
    fn given_progress_message_when_process_then_updates_counts() {
        // GIVEN
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut state = AppState::default();
        state.scan_rx = Some(rx);
        tx.send(ScanMessage::Progress {
            dirs_scanned: 10,
            files_scanned: 50,
            current_path: "/foo/bar".to_string(),
        })
        .unwrap();
        // WHEN
        state.process_scan_messages();
        // THEN
        assert_eq!(state.dirs_scanned, 10);
        assert_eq!(state.files_scanned, 50);
        assert_eq!(state.current_scan_path, "/foo/bar");
    }

    #[test]
    fn given_tree_complete_message_when_process_then_root_expanded_and_status_complete() {
        // GIVEN
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut state = AppState::default();
        state.scan_status = ScanStatus::ScanningTree;
        state.scan_rx = Some(rx);
        state.detect_duplicates = false;

        let node = DirNode::new(std::path::PathBuf::from("/root"));
        tx.send(ScanMessage::TreeComplete(Box::new(node))).unwrap();
        // WHEN
        state.process_scan_messages();
        // THEN
        let root = state.root_node.as_ref().expect("root should be set");
        assert!(root.expanded);
        assert_eq!(state.scan_status, ScanStatus::Complete);
        assert!(state.scan_rx.is_none());
    }
}
