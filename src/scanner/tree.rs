use std::path::PathBuf;

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
        }
    }

    pub fn avg_file_size(&self) -> u64 {
        if self.total_file_count == 0 {
            0
        } else {
            self.total_size / self.total_file_count as u64
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
