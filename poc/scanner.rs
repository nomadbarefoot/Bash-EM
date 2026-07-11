//! scanner.rs — walks the tree, sniffs for text, counts dashes.
//!
//! Runs on a worker thread; streams results back over an mpsc channel so the
//! TUI stays responsive and counters tick up live. This producer/consumer
//! split is *the* canonical Rust TUI pattern.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use crate::replacer::{fix_content, Counts, LineChange};

/// Skip these directory names anywhere in the tree. Touching .git objects
/// or node_modules would be slow at best, destructive at worst.
const SKIP_DIRS: &[&str] = &[
    ".git", ".svn", ".hg", "node_modules", "target", "__pycache__",
    ".cache", ".venv", "venv", ".idea", ".vscode", "dist", "build",
];

/// Files bigger than this get skipped (giant logs, datasets). 10 MB.
const MAX_SIZE: u64 = 10 * 1024 * 1024;

/// How many before/after line previews we keep per file.
pub const PREVIEW_CAP: usize = 8;

/// Everything we remember about one dirty file.
#[derive(Debug, Clone)]
pub struct FileReport {
    pub path: PathBuf,
    pub counts: Counts,
    pub lines_changed: usize,
    pub previews: Vec<LineChange>,
}

/// Events streamed from worker -> UI. An enum-as-message-protocol:
/// the channel carries "things that happened", the UI folds them into state.
pub enum ScanEvent {
    /// A file containing dashes was found.
    Dirty(FileReport),
    /// Progress heartbeat: (files_scanned, files_skipped_binary_or_big).
    Progress(usize, usize),
    /// Walk finished. Payload = (total_scanned, total_skipped, elapsed_ms).
    Done(usize, usize, u128),
}

/// Sniff: valid UTF-8 with no NUL bytes = text. Cheap and surprisingly robust —
/// automatically rejects images, executables, archives, sqlite files, etc.
fn looks_like_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

/// Recursive walk. `tx` is the channel's sending half — cloned freely,
/// closed automatically when the last clone drops.
fn walk(dir: &Path, tx: &Sender<ScanEvent>, scanned: &mut usize, skipped: &mut usize) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // permission denied etc. — skip silently, keep going
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // symlink_metadata does NOT follow links — prevents infinite loops
        // and escaping the chosen tree via a stray symlink.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let ftype = match fs::symlink_metadata(&path) {
            Ok(m) => m.file_type(),
            Err(_) => continue,
        };
        if ftype.is_symlink() {
            continue;
        }

        if meta.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if SKIP_DIRS.iter().any(|d| *d == name) || name.starts_with('.') && name != "." {
                continue;
            }
            walk(&path, tx, scanned, skipped);
            continue;
        }

        if !meta.is_file() {
            continue;
        }
        if meta.len() > MAX_SIZE || meta.len() == 0 {
            *skipped += 1;
            continue;
        }

        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => {
                *skipped += 1;
                continue;
            }
        };
        if !looks_like_text(&bytes) {
            *skipped += 1;
            continue;
        }
        // Safe: looks_like_text already validated UTF-8.
        let content = String::from_utf8(bytes).unwrap();
        *scanned += 1;

        let result = fix_content(&content, PREVIEW_CAP);
        if result.counts.total() > 0 {
            // send() fails only if the receiver hung up (UI quit) — then we
            // just stop caring. `let _ =` explicitly discards the Result.
            let _ = tx.send(ScanEvent::Dirty(FileReport {
                path: path.clone(),
                counts: result.counts,
                lines_changed: result.lines_changed,
                previews: result.changes,
            }));
        }

        // Heartbeat every 64 files so counters animate without flooding the channel.
        if *scanned % 64 == 0 {
            let _ = tx.send(ScanEvent::Progress(*scanned, *skipped));
        }
    }
}

/// Entry point for the worker thread.
pub fn scan(root: PathBuf, tx: Sender<ScanEvent>) {
    let start = std::time::Instant::now();
    let mut scanned = 0usize;
    let mut skipped = 0usize;
    walk(&root, &tx, &mut scanned, &mut skipped);
    let _ = tx.send(ScanEvent::Done(scanned, skipped, start.elapsed().as_millis()));
}

/// Apply phase: re-read, re-transform, atomic write (temp file + rename in
/// the same directory, so a crash never leaves a half-written file).
/// Returns Ok(counts) for the file, Err on I/O trouble.
pub fn apply_file(path: &Path) -> std::io::Result<Counts> {
    let content = fs::read_to_string(path)?;
    let result = fix_content(&content, 0); // no previews needed here
    if result.counts.total() == 0 {
        return Ok(result.counts);
    }
    // Temp file lives next to the target => same filesystem => rename is atomic.
    let tmp = path.with_extension("bashm_tmp");
    fs::write(&tmp, &result.new_content)?;
    fs::rename(&tmp, path)?;
    Ok(result.counts)
}
