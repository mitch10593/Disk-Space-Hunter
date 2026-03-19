use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::Sender;

use crate::scanner::tree::DuplicateGroup;
use crate::state::ScanMessage;

pub fn detect(
    path: &str,
    min_size: u64,
    tx: &Sender<ScanMessage>,
    repaint: &dyn Fn(),
    cancel: &Arc<AtomicBool>,
) {
    // Phase 1: Group files by size
    let _ = tx.send(ScanMessage::DuplicateProgress {
        phase: "Grouping by size".to_string(),
        checked: 0,
        total: 0,
    });
    repaint();

    let mut size_groups: HashMap<u64, Vec<PathBuf>> = HashMap::new();

    for entry in jwalk::WalkDir::new(path)
        .skip_hidden(false)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if entry.file_type().is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if size >= min_size {
                size_groups.entry(size).or_default().push(entry.path());
            }
        }
    }

    // Keep only groups with 2+ files
    let size_groups: Vec<(u64, Vec<PathBuf>)> = size_groups
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .collect();

    let candidates: usize = size_groups.iter().map(|(_, p)| p.len()).sum();

    // Phase 2: Partial hash (first 4KB)
    let _ = tx.send(ScanMessage::DuplicateProgress {
        phase: "Partial hash (4KB)".to_string(),
        checked: 0,
        total: candidates as u32,
    });
    repaint();

    let mut partial_groups: HashMap<(u64, [u8; 32]), Vec<PathBuf>> = HashMap::new();
    let mut checked: u32 = 0;

    for (size, paths) in &size_groups {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        for path in paths {
            if let Ok(hash) = partial_hash(path) {
                partial_groups
                    .entry((*size, hash))
                    .or_default()
                    .push(path.clone());
            }
            checked += 1;
            if checked % 100 == 0 {
                let _ = tx.send(ScanMessage::DuplicateProgress {
                    phase: "Partial hash (4KB)".to_string(),
                    checked,
                    total: candidates as u32,
                });
                repaint();
            }
        }
    }

    // Keep only groups with 2+ files
    let partial_groups: Vec<((u64, [u8; 32]), Vec<PathBuf>)> = partial_groups
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .collect();

    let candidates2: usize = partial_groups.iter().map(|(_, p)| p.len()).sum();

    // Phase 3: Full hash
    let _ = tx.send(ScanMessage::DuplicateProgress {
        phase: "Full hash".to_string(),
        checked: 0,
        total: candidates2 as u32,
    });
    repaint();

    let mut full_groups: HashMap<(u64, [u8; 32]), Vec<PathBuf>> = HashMap::new();
    let mut checked: u32 = 0;

    for ((size, _), paths) in &partial_groups {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        for path in paths {
            if let Ok(hash) = full_hash(path) {
                full_groups
                    .entry((*size, hash))
                    .or_default()
                    .push(path.clone());
            }
            checked += 1;
            if checked % 10 == 0 {
                let _ = tx.send(ScanMessage::DuplicateProgress {
                    phase: "Full hash".to_string(),
                    checked,
                    total: candidates2 as u32,
                });
                repaint();
            }
        }
    }

    // Build result
    let mut groups: Vec<DuplicateGroup> = full_groups
        .into_iter()
        .filter(|(_, paths)| paths.len() >= 2)
        .map(|((size, _), paths)| {
            let wasted = size * (paths.len() as u64 - 1);
            DuplicateGroup {
                file_size: size,
                paths,
                wasted_space: wasted,
            }
        })
        .collect();

    groups.sort_by(|a, b| b.wasted_space.cmp(&a.wasted_space));

    let _ = tx.send(ScanMessage::DuplicatesComplete(groups));
    repaint();
}

fn partial_hash(path: &Path) -> std::io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut buf = [0u8; 4096];
    let n = file.read(&mut buf)?;
    let hash = blake3::hash(&buf[..n]);
    Ok(*hash.as_bytes())
}

fn full_hash(path: &Path) -> std::io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}
