use std::path::PathBuf;

use super::file_category::FileCategory;

#[derive(Clone, Default)]
pub struct CategoryStats {
    pub own_sizes: [u64; FileCategory::COUNT],
    pub own_counts: [u32; FileCategory::COUNT],
    pub total_sizes: [u64; FileCategory::COUNT],
    pub total_counts: [u32; FileCategory::COUNT],
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

#[cfg(test)]
impl DirNode {
    /// Test helper: creates a DirNode with pre-filled category stats.
    fn with_categories(
        sizes: [u64; FileCategory::COUNT],
        counts: [u32; FileCategory::COUNT],
    ) -> Self {
        let mut node = DirNode::new(std::path::PathBuf::from("/test"));
        node.category_stats.total_sizes = sizes;
        node.category_stats.total_counts = counts;
        let total_count: u32 = counts.iter().sum();
        let total_size: u64 = sizes.iter().sum();
        node.total_file_count = total_count;
        node.total_size = total_size;
        node
    }
}

pub struct DuplicateCandidate {
    pub file_size: u64,
    pub paths: Vec<PathBuf>,
    pub status: DuplicateStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── DirNode::new ─────────────────────────────────────────

    #[test]
    fn given_path_with_filename_when_new_then_name_is_filename() {
        // GIVEN
        let path = PathBuf::from("/foo/bar");
        // WHEN
        let node = DirNode::new(path);
        // THEN
        assert_eq!(node.name, "bar");
    }

    #[test]
    fn given_root_path_when_new_then_name_is_full_path() {
        // GIVEN
        let path = PathBuf::from("/");
        // WHEN
        let node = DirNode::new(path.clone());
        // THEN — file_name() returns None for root, so full path is used
        assert_eq!(node.name, path.to_string_lossy().to_string());
    }

    // ── avg_file_size ────────────────────────────────────────

    #[test]
    fn given_no_files_when_avg_file_size_then_returns_zero() {
        // GIVEN
        let node = DirNode::new(PathBuf::from("/empty"));
        // WHEN
        let avg = node.avg_file_size();
        // THEN
        assert_eq!(avg, 0);
    }

    #[test]
    fn given_files_when_avg_file_size_then_returns_total_div_count() {
        // GIVEN
        let mut node = DirNode::new(PathBuf::from("/data"));
        node.total_size = 3000;
        node.total_file_count = 3;
        // WHEN
        let avg = node.avg_file_size();
        // THEN
        assert_eq!(avg, 1000);
    }

    // ── filtered_total_size ──────────────────────────────────

    #[test]
    fn given_all_filters_false_when_filtered_total_size_then_returns_zero() {
        // GIVEN
        let node =
            DirNode::with_categories([100, 200, 300, 0, 0, 0, 0, 0], [1, 2, 3, 0, 0, 0, 0, 0]);
        let filter = [false; FileCategory::COUNT];
        // WHEN
        let size = node.filtered_total_size(&filter);
        // THEN
        assert_eq!(size, 0);
    }

    #[test]
    fn given_single_filter_active_when_filtered_total_size_then_returns_that_category() {
        // GIVEN — Video (index 0) = 500 bytes
        let node =
            DirNode::with_categories([500, 200, 300, 0, 0, 0, 0, 0], [5, 2, 3, 0, 0, 0, 0, 0]);
        let mut filter = [false; FileCategory::COUNT];
        filter[FileCategory::Video.index()] = true;
        // WHEN
        let size = node.filtered_total_size(&filter);
        // THEN
        assert_eq!(size, 500);
    }

    #[test]
    fn given_multiple_filters_active_when_filtered_total_size_then_returns_sum() {
        // GIVEN — Video = 500, Music = 200
        let node =
            DirNode::with_categories([500, 200, 300, 0, 0, 0, 0, 0], [5, 2, 3, 0, 0, 0, 0, 0]);
        let mut filter = [false; FileCategory::COUNT];
        filter[FileCategory::Video.index()] = true;
        filter[FileCategory::Music.index()] = true;
        // WHEN
        let size = node.filtered_total_size(&filter);
        // THEN
        assert_eq!(size, 700);
    }

    // ── filtered_total_file_count ────────────────────────────

    #[test]
    fn given_no_filters_when_filtered_total_file_count_then_returns_zero() {
        // GIVEN
        let node =
            DirNode::with_categories([100, 200, 0, 0, 0, 0, 0, 0], [10, 20, 0, 0, 0, 0, 0, 0]);
        let filter = [false; FileCategory::COUNT];
        // WHEN
        let count = node.filtered_total_file_count(&filter);
        // THEN
        assert_eq!(count, 0);
    }

    #[test]
    fn given_some_filters_when_filtered_total_file_count_then_returns_matching_sum() {
        // GIVEN — Video = 10 files, Music = 20 files
        let node =
            DirNode::with_categories([100, 200, 0, 0, 0, 0, 0, 0], [10, 20, 0, 0, 0, 0, 0, 0]);
        let mut filter = [false; FileCategory::COUNT];
        filter[FileCategory::Video.index()] = true;
        filter[FileCategory::Music.index()] = true;
        // WHEN
        let count = node.filtered_total_file_count(&filter);
        // THEN
        assert_eq!(count, 30);
    }

    // ── filtered_avg_file_size ───────────────────────────────

    #[test]
    fn given_no_filters_when_filtered_avg_file_size_then_returns_zero() {
        // GIVEN
        let node = DirNode::with_categories([1000, 0, 0, 0, 0, 0, 0, 0], [10, 0, 0, 0, 0, 0, 0, 0]);
        let filter = [false; FileCategory::COUNT];
        // WHEN
        let avg = node.filtered_avg_file_size(&filter);
        // THEN
        assert_eq!(avg, 0);
    }

    #[test]
    fn given_filter_with_no_matching_files_when_filtered_avg_file_size_then_returns_zero() {
        // GIVEN — Music filter active but Music has 0 files
        let node = DirNode::with_categories([1000, 0, 0, 0, 0, 0, 0, 0], [10, 0, 0, 0, 0, 0, 0, 0]);
        let mut filter = [false; FileCategory::COUNT];
        filter[FileCategory::Music.index()] = true;
        // WHEN
        let avg = node.filtered_avg_file_size(&filter);
        // THEN
        assert_eq!(avg, 0);
    }

    #[test]
    fn given_filter_with_files_when_filtered_avg_file_size_then_returns_correct_avg() {
        // GIVEN — Video: 1000 bytes, 10 files => avg = 100
        let node = DirNode::with_categories([1000, 0, 0, 0, 0, 0, 0, 0], [10, 0, 0, 0, 0, 0, 0, 0]);
        let mut filter = [false; FileCategory::COUNT];
        filter[FileCategory::Video.index()] = true;
        // WHEN
        let avg = node.filtered_avg_file_size(&filter);
        // THEN
        assert_eq!(avg, 100);
    }
}
