use std::path::PathBuf;

use super::file_category::FileCategory;

#[derive(Clone)]
pub struct CategoryStats {
    pub own_sizes: [u64; FileCategory::COUNT],
    pub own_counts: [u32; FileCategory::COUNT],
    pub total_sizes: [u64; FileCategory::COUNT],
    pub total_counts: [u32; FileCategory::COUNT],
}

impl Default for CategoryStats {
    fn default() -> Self {
        Self {
            own_sizes: [0; FileCategory::COUNT],
            own_counts: [0; FileCategory::COUNT],
            total_sizes: [0; FileCategory::COUNT],
            total_counts: [0; FileCategory::COUNT],
        }
    }
}

#[derive(Clone)]
pub struct DirNode {
    pub name: String,
    pub path: PathBuf,
    pub own_size: u64,
    pub total_size: u64,
    pub file_count: u32,
    pub total_file_count: u32,
    pub children: Vec<DirNode>,
    pub expanded: bool,
    pub errors: Vec<String>,
    pub category_stats: CategoryStats,
}

impl DirNode {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        Self {
            name,
            path,
            own_size: 0,
            total_size: 0,
            file_count: 0,
            total_file_count: 0,
            children: Vec::new(),
            expanded: false,
            errors: Vec::new(),
            category_stats: CategoryStats::default(),
        }
    }

    pub fn avg_file_size(&self) -> u64 {
        if self.total_file_count == 0 {
            0
        } else {
            self.total_size / self.total_file_count as u64
        }
    }

    pub fn filtered_total_size(&self, filter: &[bool; FileCategory::COUNT]) -> u64 {
        self.category_stats
            .total_sizes
            .iter()
            .zip(filter.iter())
            .filter(|(_, &active)| active)
            .map(|(&size, _)| size)
            .sum()
    }

    pub fn filtered_total_file_count(&self, filter: &[bool; FileCategory::COUNT]) -> u32 {
        self.category_stats
            .total_counts
            .iter()
            .zip(filter.iter())
            .filter(|(_, &active)| active)
            .map(|(&count, _)| count)
            .sum()
    }

    pub fn filtered_avg_file_size(&self, filter: &[bool; FileCategory::COUNT]) -> u64 {
        let count = self.filtered_total_file_count(filter);
        if count == 0 {
            0
        } else {
            self.filtered_total_size(filter) / count as u64
        }
    }
}

#[derive(Clone)]
pub struct DuplicateGroup {
    pub file_size: u64,
    pub paths: Vec<PathBuf>,
    pub wasted_space: u64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DuplicateStatus {
    SameSize,
    SamePartialHash,
    Confirmed,
}

#[derive(Clone)]
pub struct DuplicateCandidate {
    pub file_size: u64,
    pub paths: Vec<PathBuf>,
    pub status: DuplicateStatus,
}
