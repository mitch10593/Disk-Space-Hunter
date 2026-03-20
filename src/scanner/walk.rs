use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;

use crate::scanner::file_category::FileCategory;
use crate::scanner::tree::DirNode;
use crate::state::ScanMessage;

pub fn scan_directory(path: &str, tx: &Sender<ScanMessage>, repaint: &dyn Fn(), cancel: &Arc<AtomicBool>) {
    let root_path = PathBuf::from(path);

    let mut dirs: HashMap<PathBuf, DirNode> = HashMap::new();
    dirs.insert(root_path.clone(), DirNode::new(root_path.clone()));

    let mut dirs_scanned: u32 = 0;
    let mut files_scanned: u32 = 0;

    for entry in jwalk::WalkDir::new(&root_path)
        .skip_hidden(false)
        .follow_links(false)
        .into_iter()
    {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        match entry {
            Ok(entry) => {
                let entry_path = entry.path();

                if entry.file_type().is_dir() {
                    dirs_scanned += 1;
                    let parent = entry_path.parent().map(|p| p.to_path_buf());
                    dirs.entry(entry_path.clone())
                        .or_insert_with(|| DirNode::new(entry_path.clone()));

                    if let Some(parent_path) = &parent {
                        dirs.entry(parent_path.clone())
                            .or_insert_with(|| DirNode::new(parent_path.clone()));
                    }
                } else if entry.file_type().is_file() {
                    files_scanned += 1;
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                    let category = entry_path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(FileCategory::from_extension)
                        .unwrap_or(FileCategory::Other);
                    let cat_idx = category.index();

                    if let Some(parent_path) = entry_path.parent() {
                        let parent_node = dirs
                            .entry(parent_path.to_path_buf())
                            .or_insert_with(|| DirNode::new(parent_path.to_path_buf()));
                        parent_node.own_size += size;
                        parent_node.file_count += 1;
                        parent_node.category_stats.own_sizes[cat_idx] += size;
                        parent_node.category_stats.own_counts[cat_idx] += 1;
                    }
                }

                if (dirs_scanned + files_scanned) % 1000 == 0 {
                    let _ = tx.send(ScanMessage::Progress {
                        dirs_scanned,
                        files_scanned,
                        current_path: truncate_path(&entry_path, 60),
                    });
                    repaint();
                }
            }
            Err(e) => {
                let err_path = e.path().map(|p| p.to_path_buf());
                if let Some(err_path) = err_path {
                    if let Some(parent_path) = err_path.parent() {
                        let parent_node = dirs
                            .entry(parent_path.to_path_buf())
                            .or_insert_with(|| DirNode::new(parent_path.to_path_buf()));
                        parent_node.errors.push(format!("{}", e));
                    }
                }
            }
        }
    }

    // Build tree bottom-up: sort dirs by depth (deepest first)
    let mut dir_paths: Vec<PathBuf> = dirs.keys().cloned().collect();
    dir_paths.sort_by(|a, b| {
        let depth_a = a.components().count();
        let depth_b = b.components().count();
        depth_b.cmp(&depth_a)
    });

    for dir_path in &dir_paths {
        if *dir_path == root_path {
            continue;
        }

        if let Some(parent_path) = dir_path.parent() {
            if let Some(mut child) = dirs.remove(dir_path) {
                child.total_file_count = child.file_count
                    + child.children.iter().map(|c| c.total_file_count).sum::<u32>();
                child.total_size = child.own_size
                    + child.children.iter().map(|c| c.total_size).sum::<u64>();
                for i in 0..FileCategory::COUNT {
                    child.category_stats.total_sizes[i] = child.category_stats.own_sizes[i]
                        + child.children.iter().map(|c| c.category_stats.total_sizes[i]).sum::<u64>();
                    child.category_stats.total_counts[i] = child.category_stats.own_counts[i]
                        + child.children.iter().map(|c| c.category_stats.total_counts[i]).sum::<u32>();
                }
                child.children.sort_by(|a, b| b.total_size.cmp(&a.total_size));

                if let Some(parent) = dirs.get_mut(&parent_path.to_path_buf()) {
                    parent.children.push(child);
                }
            }
        }
    }

    // Finalize root
    if let Some(mut root) = dirs.remove(&root_path) {
        root.total_file_count = root.file_count
            + root.children.iter().map(|c| c.total_file_count).sum::<u32>();
        root.total_size = root.own_size
            + root.children.iter().map(|c| c.total_size).sum::<u64>();
        for i in 0..FileCategory::COUNT {
            root.category_stats.total_sizes[i] = root.category_stats.own_sizes[i]
                + root.children.iter().map(|c| c.category_stats.total_sizes[i]).sum::<u64>();
            root.category_stats.total_counts[i] = root.category_stats.own_counts[i]
                + root.children.iter().map(|c| c.category_stats.total_counts[i]).sum::<u32>();
        }
        root.children.sort_by(|a, b| b.total_size.cmp(&a.total_size));

        let _ = tx.send(ScanMessage::TreeComplete(Box::new(root)));
        repaint();
    }
}

fn truncate_path(path: &Path, max_len: usize) -> String {
    let s = path.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}
