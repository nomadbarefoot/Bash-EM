//! app.rs — the single source of truth for the TUI.
//!
//! The event loop mutates this struct; ui.rs only *reads* it to draw.
//! Keeping state and rendering separate is what makes ratatui apps sane.

use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;

use crate::replacer::Counts;
use crate::scanner::FileReport;

/// How many top offenders the list shows. Static for v1, as agreed.
pub const TOP_N: usize = 20;

/// The app's lifecycle. Rust enums make illegal states unrepresentable:
/// you literally cannot be "applying" and "scanning" at once.
#[derive(PartialEq, Clone, Copy)]
pub enum Phase {
    Scanning,
    Review,
    Applying,
    Done,
}

pub struct App {
    pub root: PathBuf,
    pub phase: Phase,

    /// ALL dirty files (needed for apply + log), sorted by dash count desc.
    pub files: Vec<FileReport>,
    /// Indices into `files` that the user has excluded from apply.
    pub excluded: HashSet<usize>,
    /// Cursor position within the displayed top-N list.
    pub selected: usize,

    // live scan stats
    pub scanned: usize,
    pub skipped: usize,
    pub scan_ms: u128,

    // apply stats
    pub applied_files: usize,
    pub applied_counts: Counts,
    pub apply_errors: usize,

    /// Monotonic tick for the spinner animation.
    pub tick: u64,
    /// One-line status message (log saved, errors, etc.)
    pub flash: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            phase: Phase::Scanning,
            files: Vec::new(),
            excluded: HashSet::new(),
            selected: 0,
            scanned: 0,
            skipped: 0,
            scan_ms: 0,
            applied_files: 0,
            applied_counts: Counts::default(),
            apply_errors: 0,
            tick: 0,
            flash: String::new(),
            should_quit: false,
        }
    }

    /// Fold a newly-discovered dirty file into state, keeping the list
    /// sorted by total dash count (descending). Binary search keeps this
    /// O(log n) to find the slot instead of re-sorting every event.
    pub fn add_file(&mut self, report: FileReport) {
        let key = report.counts.total();
        let pos = self
            .files
            .partition_point(|f| f.counts.total() >= key);
        self.files.insert(pos, report);
        // Inserting shifts indices; excluded uses indices, so remap.
        // (Cheap for our sizes; a HashSet<PathBuf> would avoid this but
        // indices keep toggle handling simple. Fine for v1.)
        self.excluded = self
            .excluded
            .iter()
            .map(|&i| if i >= pos { i + 1 } else { i })
            .collect();
    }

    pub fn total_counts(&self) -> Counts {
        let mut c = Counts::default();
        for f in &self.files {
            c.add(f.counts);
        }
        c
    }

    pub fn shown_len(&self) -> usize {
        self.files.len().min(TOP_N)
    }

    pub fn move_selection(&mut self, delta: i64) {
        let n = self.shown_len();
        if n == 0 {
            return;
        }
        let cur = self.selected as i64;
        self.selected = (cur + delta).rem_euclid(n as i64) as usize;
    }

    pub fn toggle_selected(&mut self) {
        if self.files.is_empty() {
            return;
        }
        if !self.excluded.remove(&self.selected) {
            self.excluded.insert(self.selected);
        }
    }

    /// Files that will actually be modified on apply.
    pub fn apply_set(&self) -> Vec<usize> {
        (0..self.files.len())
            .filter(|i| !self.excluded.contains(i))
            .collect()
    }

    /// Write a plain-text log of everything found (and what apply did).
    /// Returns the path written. No chrono dep — epoch seconds are unique enough.
    pub fn write_log(&self) -> std::io::Result<PathBuf> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let path = PathBuf::from(format!("Bash-EM-{stamp}.log"));
        let mut f = std::fs::File::create(&path)?;

        let t = self.total_counts();
        writeln!(f, "Bash-EM log")?;
        writeln!(f, "root: {}", self.root.display())?;
        writeln!(f, "files scanned: {}   skipped (binary/big): {}   scan time: {}ms", self.scanned, self.skipped, self.scan_ms)?;
        writeln!(f, "dirty files: {}   em: {}  en: {}  bar: {}  entities: {}  total: {}",
            self.files.len(), t.em, t.en, t.bar, t.entities, t.total())?;
        if self.phase == Phase::Done {
            writeln!(f, "applied: {} files, {} dashes replaced, {} errors",
                self.applied_files, self.applied_counts.total(), self.apply_errors)?;
        }
        writeln!(f)?;

        for (i, file) in self.files.iter().enumerate() {
            let mark = if self.excluded.contains(&i) { "[skipped]" } else { "" };
            writeln!(f, "== {} {}  (em:{} en:{} bar:{} ent:{} lines:{})",
                file.path.display(), mark,
                file.counts.em, file.counts.en, file.counts.bar,
                file.counts.entities, file.lines_changed)?;
            for ch in &file.previews {
                writeln!(f, "  L{}:", ch.line_no)?;
                writeln!(f, "    - {}", ch.before)?;
                writeln!(f, "    + {}", ch.after)?;
            }
            if file.lines_changed > file.previews.len() {
                writeln!(f, "  ... and {} more lines", file.lines_changed - file.previews.len())?;
            }
        }
        Ok(path)
    }
}
