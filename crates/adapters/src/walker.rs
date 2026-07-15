use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use config::Prefs;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::text::TextAdapter;

#[derive(Debug)]
pub struct FileCandidate {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WalkStats {
    pub scanned: usize,
    pub skipped: usize,
    pub ignored: usize,
    pub hidden_dirs: usize,
    pub symlinks: usize,
    pub too_large: usize,
    pub binary: usize,
    pub empty: usize,
    pub unreadable: usize,
}

pub fn walk_tree(root: &Path, prefs: &Prefs) -> Result<(Vec<FileCandidate>, WalkStats), String> {
    walk_tree_with_ignores(root, prefs, &[])
}

pub fn walk_tree_with_ignores(
    root: &Path,
    prefs: &Prefs,
    ignore_patterns: &[String],
) -> Result<(Vec<FileCandidate>, WalkStats), String> {
    walk_tree_inner(root, prefs, ignore_patterns, None)
}

pub fn walk_tree_with_ignores_cancelled(
    root: &Path,
    prefs: &Prefs,
    ignore_patterns: &[String],
    cancelled: &AtomicBool,
) -> Result<(Vec<FileCandidate>, WalkStats), String> {
    walk_tree_inner(root, prefs, ignore_patterns, Some(cancelled))
}

fn walk_tree_inner(
    root: &Path,
    prefs: &Prefs,
    ignore_patterns: &[String],
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<FileCandidate>, WalkStats), String> {
    let mut candidates = Vec::new();
    let mut stats = WalkStats::default();
    let ignores = build_ignores(root, ignore_patterns)?;
    walk_recursive(
        root,
        root,
        prefs,
        &ignores,
        cancelled,
        &mut candidates,
        &mut stats,
    );
    Ok((candidates, stats))
}

fn build_ignores(root: &Path, patterns: &[String]) -> Result<Gitignore, String> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .map_err(|error| format!("invalid profile ignore pattern '{pattern}': {error}"))?;
    }
    let ignore_path = root.join(".bash-emignore");
    if ignore_path.exists() {
        if let Some(error) = builder.add(&ignore_path) {
            return Err(format!("invalid {}: {error}", ignore_path.display()));
        }
    }
    for internal in [".bash-emignore", config::PROJECT_PROFILE_FILE] {
        builder
            .add_line(None, internal)
            .map_err(|error| format!("invalid internal ignore pattern '{internal}': {error}"))?;
    }
    builder
        .build()
        .map_err(|error| format!("build ignore matcher: {error}"))
}

fn walk_recursive(
    root: &Path,
    dir: &Path,
    prefs: &Prefs,
    ignores: &Gitignore,
    cancelled: Option<&AtomicBool>,
    candidates: &mut Vec<FileCandidate>,
    stats: &mut WalkStats,
) {
    if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => {
            stats.unreadable += 1;
            stats.skipped += 1;
            return;
        }
    };

    for entry in entries.flatten() {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return;
        }
        let path = entry.path();

        let file_type = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata.file_type(),
            Err(_) => {
                stats.unreadable += 1;
                stats.skipped += 1;
                continue;
            }
        };
        if file_type.is_symlink() {
            stats.symlinks += 1;
            stats.skipped += 1;
            continue;
        }

        if ignores
            .matched_path_or_any_parents(&path, file_type.is_dir())
            .is_ignore()
        {
            stats.ignored += 1;
            stats.skipped += 1;
            continue;
        }

        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if prefs.skip_dirs.iter().any(|skip| skip == name.as_ref()) {
                stats.ignored += 1;
                stats.skipped += 1;
            } else if name.starts_with('.') && name.as_ref() != "." {
                stats.hidden_dirs += 1;
                stats.skipped += 1;
            } else {
                walk_recursive(root, &path, prefs, ignores, cancelled, candidates, stats);
            }
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                stats.unreadable += 1;
                stats.skipped += 1;
                continue;
            }
        };
        if metadata.len() == 0 {
            stats.empty += 1;
            stats.skipped += 1;
            continue;
        }
        if metadata.len() > prefs.max_file_bytes {
            stats.too_large += 1;
            stats.skipped += 1;
            continue;
        }

        match TextAdapter::read(&path) {
            Ok(content) => {
                stats.scanned += 1;
                candidates.push(FileCandidate { path, content });
            }
            Err(error) if error == "binary file" || error == "invalid utf-8" => {
                stats.binary += 1;
                stats.skipped += 1;
            }
            Err(_) => {
                stats.unreadable += 1;
                stats.skipped += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tempdir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("bash-em-ignore-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn project_ignore_supports_negation_and_hides_control_files() {
        let root = tempdir();
        fs::write(root.join("ignored.md"), "bad\u{2014}dash").unwrap();
        fs::write(root.join("keep.md"), "keep\u{2014}dash").unwrap();
        fs::write(root.join(".bash-emignore"), "*.md\n!keep.md\n").unwrap();
        fs::write(root.join(config::PROJECT_PROFILE_FILE), "name: test\n").unwrap();

        let (files, stats) = walk_tree(&root, &Prefs::default()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, root.join("keep.md"));
        assert_eq!(stats.ignored, 3);
        let _ = fs::remove_dir_all(root);
    }
}
