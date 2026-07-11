use std::fs;
use std::path::{Path, PathBuf};
use config::Prefs;
use crate::text::TextAdapter;

#[derive(Debug)]
pub struct FileCandidate {
    pub path: PathBuf,
    pub content: String,
}

pub struct WalkStats {
    pub scanned: usize,
    pub skipped: usize,
}

pub fn walk_tree(root: &Path, prefs: &Prefs) -> (Vec<FileCandidate>, WalkStats) {
    let mut candidates = Vec::new();
    let mut stats = WalkStats { scanned: 0, skipped: 0 };
    walk_recursive(root, prefs, &mut candidates, &mut stats);
    (candidates, stats)
}

fn walk_recursive(
    dir: &Path,
    prefs: &Prefs,
    candidates: &mut Vec<FileCandidate>,
    stats: &mut WalkStats,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        let ftype = match fs::symlink_metadata(&path) {
            Ok(m) => m.file_type(),
            Err(_) => continue,
        };
        if ftype.is_symlink() {
            continue;
        }

        if ftype.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if prefs.skip_dirs.iter().any(|d| d == name.as_ref())
                || (name.starts_with('.') && name.as_ref() != ".")
            {
                continue;
            }
            walk_recursive(&path, prefs, candidates, stats);
            continue;
        }

        if !ftype.is_file() {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.len() > prefs.max_file_bytes || meta.len() == 0 {
            stats.skipped += 1;
            continue;
        }

        match TextAdapter::read(&path) {
            Ok(content) => {
                stats.scanned += 1;
                candidates.push(FileCandidate { path, content });
            }
            Err(_) => {
                stats.skipped += 1;
            }
        }
    }
}
