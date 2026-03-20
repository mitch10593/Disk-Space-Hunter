use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Sender;

use crate::scanner::tree::{DuplicateCandidate, DuplicateGroup, DuplicateStatus};
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
        bytes_read: 0,
        bytes_total: 0,
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

    // Send size-based candidates
    let mut dup_candidates: Vec<DuplicateCandidate> = size_groups
        .iter()
        .map(|(size, paths)| DuplicateCandidate {
            file_size: *size,
            paths: paths.clone(),
            status: DuplicateStatus::SameSize,
        })
        .collect();
    dup_candidates.sort_by(|a, b| {
        (b.file_size * b.paths.len() as u64).cmp(&(a.file_size * a.paths.len() as u64))
    });
    let _ = tx.send(ScanMessage::DuplicateCandidates(dup_candidates));
    repaint();

    let phase2_bytes_total: u64 = size_groups
        .iter()
        .map(|(size, paths)| paths.len() as u64 * (*size).min(4096))
        .sum();

    // Phase 2: Partial hash (first 4KB)
    let _ = tx.send(ScanMessage::DuplicateProgress {
        phase: "Partial hash (4KB)".to_string(),
        checked: 0,
        total: candidates as u32,
        bytes_read: 0,
        bytes_total: phase2_bytes_total,
    });
    repaint();

    let mut partial_groups: HashMap<(u64, [u8; 32]), Vec<PathBuf>> = HashMap::new();
    let mut checked: u32 = 0;
    let mut bytes_read: u64 = 0;
    let mut last_update = Instant::now();

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
            bytes_read += (*size).min(4096);
            if last_update.elapsed().as_millis() >= 250 {
                last_update = Instant::now();
                let _ = tx.send(ScanMessage::DuplicateProgress {
                    phase: "Partial hash (4KB)".to_string(),
                    checked,
                    total: candidates as u32,
                    bytes_read,
                    bytes_total: phase2_bytes_total,
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

    // Send partial-hash candidates
    let mut dup_candidates: Vec<DuplicateCandidate> = partial_groups
        .iter()
        .map(|((size, _), paths)| DuplicateCandidate {
            file_size: *size,
            paths: paths.clone(),
            status: DuplicateStatus::SamePartialHash,
        })
        .collect();
    dup_candidates.sort_by(|a, b| {
        (b.file_size * b.paths.len() as u64).cmp(&(a.file_size * a.paths.len() as u64))
    });
    let _ = tx.send(ScanMessage::DuplicateCandidates(dup_candidates));
    repaint();

    let phase3_bytes_total: u64 = partial_groups
        .iter()
        .map(|((size, _), paths)| *size * paths.len() as u64)
        .sum();

    // Phase 3: Full hash
    let _ = tx.send(ScanMessage::DuplicateProgress {
        phase: "Full hash".to_string(),
        checked: 0,
        total: candidates2 as u32,
        bytes_read: 0,
        bytes_total: phase3_bytes_total,
    });
    repaint();

    let mut full_groups: HashMap<(u64, [u8; 32]), Vec<PathBuf>> = HashMap::new();
    let mut checked: u32 = 0;
    let mut bytes_read: u64 = 0;
    let mut last_update = Instant::now();
    let mut last_candidates_update = Instant::now();

    let mut send_progress = |checked: u32, bytes_read: u64, last_update: &mut Instant| {
        *last_update = Instant::now();
        let _ = tx.send(ScanMessage::DuplicateProgress {
            phase: "Full hash".to_string(),
            checked,
            total: candidates2 as u32,
            bytes_read,
            bytes_total: phase3_bytes_total,
        });
        repaint();
    };

    let send_candidates = |full_groups: &HashMap<(u64, [u8; 32]), Vec<PathBuf>>,
                           remaining: &[((u64, [u8; 32]), Vec<PathBuf>)],
                           last_candidates_update: &mut Instant| {
        *last_candidates_update = Instant::now();
        let mut candidates: Vec<DuplicateCandidate> = Vec::new();
        // Confirmed duplicates
        for ((size, _), paths) in full_groups {
            if paths.len() >= 2 {
                candidates.push(DuplicateCandidate {
                    file_size: *size,
                    paths: paths.clone(),
                    status: DuplicateStatus::Confirmed,
                });
            }
        }
        // Remaining partial-hash candidates not yet fully hashed
        for ((size, _), paths) in remaining {
            candidates.push(DuplicateCandidate {
                file_size: *size,
                paths: paths.clone(),
                status: DuplicateStatus::SamePartialHash,
            });
        }
        candidates.sort_by(|a, b| {
            (b.file_size * b.paths.len() as u64).cmp(&(a.file_size * a.paths.len() as u64))
        });
        let _ = tx.send(ScanMessage::DuplicateCandidates(candidates));
        repaint();
    };

    for (group_idx, ((size, _), paths)) in partial_groups.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        for path in paths {
            if let Ok(hash) = full_hash_with_progress(
                path,
                &mut bytes_read,
                &mut last_update,
                checked,
                &mut send_progress,
            ) {
                full_groups
                    .entry((*size, hash))
                    .or_default()
                    .push(path.clone());
            }
            checked += 1;
            if last_update.elapsed().as_millis() >= 250 {
                send_progress(checked, bytes_read, &mut last_update);
            }
        }
        // Send candidate updates after each size group is fully hashed
        if last_candidates_update.elapsed().as_millis() >= 500 {
            let remaining = &partial_groups[group_idx + 1..];
            send_candidates(&full_groups, remaining, &mut last_candidates_update);
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

fn full_hash_with_progress(
    path: &Path,
    bytes_read: &mut u64,
    last_update: &mut Instant,
    checked: u32,
    send_progress: &mut impl FnMut(u32, u64, &mut Instant),
) -> std::io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        *bytes_read += n as u64;
        if last_update.elapsed().as_millis() >= 250 {
            send_progress(checked, *bytes_read, last_update);
        }
    }
    Ok(*hasher.finalize().as_bytes())
}
