use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    Browse,
    Scan,
    Backups,
    Profiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPhase {
    Idle,
    Scanning,
    Review,
    Applying,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    MissionControl,
    BrowseList,
    OffenderList,
    DiffPreview,
    RunsTable,
    ProfileRules,
}

pub struct ProfileRule {
    pub key: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
}

pub const PROFILE_RULES: &[ProfileRule] = &[
    ProfileRule {
        key: "em_dash",
        label: "Em dash",
        detail: "Replace em dashes with a spaced hyphen.",
    },
    ProfileRule {
        key: "en_dash",
        label: "En dash",
        detail: "Replace en dashes with a plain hyphen.",
    },
    ProfileRule {
        key: "horizontal_bar",
        label: "Horizontal bar",
        detail: "Replace horizontal bars with a spaced hyphen.",
    },
    ProfileRule {
        key: "html_dash_entities",
        label: "HTML dash entities",
        detail: "Replace encoded dash entities such as &mdash;.",
    },
    ProfileRule {
        key: "curly_quotes",
        label: "Curly quotes",
        detail: "Normalize typographic single and double quotes.",
    },
    ProfileRule {
        key: "ellipsis",
        label: "Ellipsis",
        detail: "Replace the single ellipsis glyph with three dots.",
    },
    ProfileRule {
        key: "zero_width",
        label: "Zero-width junk",
        detail: "Remove zero-width and bidi control characters.",
    },
    ProfileRule {
        key: "llm_boilerplate",
        label: "LLM boilerplate",
        detail: "Flag common LLM phrases in health results; no automatic rewrite.",
    },
];

pub enum ConfirmDialog {
    None,
    RestoreRun {
        run_id: String,
        root: PathBuf,
        file_count: usize,
        selected_button: u8,
    },
    ApplyConfirm {
        file_count: usize,
        selected_button: u8,
    },
}

#[derive(Clone)]
pub struct ScanFile {
    pub path: PathBuf,
    pub counts: engine::Counts,
    pub lines_changed: usize,
    pub changes: Vec<engine::LineChange>,
}

#[derive(Default)]
pub struct ScanStats {
    pub scanned: usize,
    pub skipped: usize,
    pub scan_ms: u128,
    pub progress: String,
}

#[derive(Default)]
pub struct ApplyStats {
    pub applied_files: usize,
    pub applied_counts: engine::Counts,
    pub errors: Vec<String>,
    pub backup_run_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
    pub pruned_runs: usize,
}

pub struct RunRow {
    pub run_id: String,
    pub timestamp: String,
    pub when_relative: String,
    pub file_count: usize,
    pub root: PathBuf,
    pub profile_name: String,
}

pub struct BrowseEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub category: String,
}

pub struct App {
    pub screen: Screen,
    pub focused_pane: Pane,
    pub root: PathBuf,
    pub browse_cwd: PathBuf,
    pub show_hidden: bool,
    pub profile: config::Profile,
    pub profile_path: PathBuf,
    pub profile_explicit: bool,
    pub profile_persisted: bool,
    pub tick: u64,
    pub flash: String,
    pub flash_color: ratatui::style::Color,
    pub should_quit: bool,
    pub theme: Theme,

    pub health: Option<engine::health::HealthReport>,
    pub scan_phase: ScanPhase,
    pub scan_report: Option<workflow::ScanReport>,
    pub pending_scan: Option<PathBuf>,
    pub finish_scan_as_done: bool,
    pub files: Vec<ScanFile>,
    pub excluded: HashSet<PathBuf>,
    pub list_selected: usize,
    pub diff_scroll: usize,
    pub scan_stats: ScanStats,
    pub apply_stats: ApplyStats,
    pub pending_apply: bool,

    pub runs: Vec<RunRow>,
    pub runs_selected: usize,
    pub confirm: ConfirmDialog,
    pub pending_restore: Option<(String, PathBuf)>,
    pub restore_in_progress: bool,

    pub profile_selected: usize,
    pub profile_dirty: bool,
    pub preserve_next_scan_flash: bool,
    pub reload_backups: bool,

    pub browse_entries: Vec<BrowseEntry>,
    pub browse_selected: usize,
}

impl App {
    pub fn new(root: PathBuf, profile: config::Profile) -> Self {
        let document = config::ProfileDocument {
            profile,
            path: config::project_profile_path(&root),
            explicit: false,
            persisted: false,
        };
        Self::with_profile_document(root, document)
    }

    pub fn with_profile_document(root: PathBuf, document: config::ProfileDocument) -> Self {
        let theme = Theme::default();
        let mut app = Self {
            screen: Screen::Welcome,
            focused_pane: Pane::MissionControl,
            browse_cwd: root.clone(),
            root,
            show_hidden: false,
            profile: document.profile,
            profile_path: document.path,
            profile_explicit: document.explicit,
            profile_persisted: document.persisted,
            tick: 0,
            flash: "choose a directory, then scan".to_string(),
            flash_color: theme.clean,
            should_quit: false,
            theme,
            health: None,
            scan_phase: ScanPhase::Idle,
            scan_report: None,
            pending_scan: None,
            finish_scan_as_done: false,
            files: Vec::new(),
            excluded: HashSet::new(),
            list_selected: 0,
            diff_scroll: 0,
            scan_stats: ScanStats::default(),
            apply_stats: ApplyStats::default(),
            pending_apply: false,
            runs: Vec::new(),
            runs_selected: 0,
            confirm: ConfirmDialog::None,
            pending_restore: None,
            restore_in_progress: false,
            profile_selected: 0,
            profile_dirty: false,
            preserve_next_scan_flash: false,
            reload_backups: false,
            browse_entries: Vec::new(),
            browse_selected: 0,
        };
        let _ = app.reload_browse();
        app
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.focused_pane = match screen {
            Screen::Welcome => Pane::MissionControl,
            Screen::Browse => Pane::BrowseList,
            Screen::Scan => Pane::OffenderList,
            Screen::Backups => Pane::RunsTable,
            Screen::Profiles => Pane::ProfileRules,
        };
    }

    pub fn request_scan(&mut self, root: PathBuf) {
        self.root = root;
        self.pending_scan = Some(self.root.clone());
        self.scan_phase = ScanPhase::Scanning;
        self.scan_stats.progress = "starting…".to_string();
        self.files.clear();
        self.excluded.clear();
        self.list_selected = 0;
        self.diff_scroll = 0;
        self.health = None;
        self.scan_report = None;
        self.apply_stats = ApplyStats::default();
        self.preserve_next_scan_flash = false;
        self.switch_screen(Screen::Scan);
    }

    pub fn request_scan_root(&mut self, root: PathBuf) -> Result<(), String> {
        if root != self.root && !self.profile_explicit {
            if self.profile_dirty {
                return Err(
                    "profile has unsaved changes; save or reload it before changing roots"
                        .to_string(),
                );
            }
            let document = config::resolve_profile(&root, None)?;
            self.install_profile_document(document);
        }
        self.request_scan(root);
        Ok(())
    }

    pub fn accept_scan_report(&mut self, report: workflow::ScanReport) {
        self.root = report.root.clone();
        self.scan_stats.scanned = report.stats.scanned;
        self.scan_stats.skipped = report.stats.skipped;
        self.scan_stats.scan_ms = report.scan_ms;
        self.health = Some(report.health.clone());
        self.files = report
            .files
            .iter()
            .map(|file| ScanFile {
                path: file.path.clone(),
                counts: file.counts,
                lines_changed: file.lines_changed,
                changes: file.changes.clone(),
            })
            .collect();
        self.scan_report = Some(report);
        self.excluded.clear();
        self.list_selected = 0;
        self.diff_scroll = 0;
        self.scan_phase = if self.finish_scan_as_done {
            self.finish_scan_as_done = false;
            ScanPhase::Done
        } else if self.files.is_empty() {
            ScanPhase::Done
        } else {
            ScanPhase::Review
        };
    }

    pub fn selected_file(&self) -> Option<&ScanFile> {
        self.files.get(self.list_selected)
    }

    pub fn toggle_selected(&mut self) {
        let Some(path) = self.selected_file().map(|file| file.path.clone()) else {
            return;
        };
        if !self.excluded.remove(&path) {
            self.excluded.insert(path);
        }
    }

    pub fn included_paths(&self) -> HashSet<PathBuf> {
        self.files
            .iter()
            .filter(|file| !self.excluded.contains(&file.path))
            .map(|file| file.path.clone())
            .collect()
    }

    pub fn move_file_selection(&mut self, delta: i64) {
        if self.files.is_empty() {
            return;
        }
        let current = self.list_selected as i64;
        self.list_selected = (current + delta).rem_euclid(self.files.len() as i64) as usize;
        self.diff_scroll = 0;
    }

    pub fn move_browse_selection(&mut self, delta: i64) {
        if self.browse_entries.is_empty() {
            return;
        }
        let current = self.browse_selected as i64;
        self.browse_selected =
            (current + delta).rem_euclid(self.browse_entries.len() as i64) as usize;
    }

    pub fn move_profile_selection(&mut self, delta: i64) {
        let current = self.profile_selected as i64;
        self.profile_selected = (current + delta).rem_euclid(PROFILE_RULES.len() as i64) as usize;
    }

    pub fn toggle_selected_rule(&mut self) {
        let rule = &PROFILE_RULES[self.profile_selected];
        let enabled = !config::is_rule_enabled(&self.profile, rule.key);
        self.profile
            .rules
            .insert(rule.key.to_string(), config::RuleConfig { enabled });
        self.profile_dirty = true;
        self.invalidate_scan();
        self.flash = format!(
            "{} {} · press w to save · rescan required",
            rule.label,
            if enabled { "enabled" } else { "disabled" }
        );
        self.flash_color = if enabled {
            self.theme.clean
        } else {
            self.theme.muted
        };
    }

    pub fn save_profile(&mut self) -> Result<(), String> {
        config::save_profile(&self.profile_path, &self.profile)?;
        self.profile_dirty = false;
        self.profile_persisted = true;
        self.flash = format!("profile saved to {}", self.profile_path.display());
        self.flash_color = self.theme.clean;
        Ok(())
    }

    pub fn reload_profile(&mut self) -> Result<(), String> {
        let profile = config::load_profile(&self.profile_path)?;
        self.profile = profile;
        self.profile_dirty = false;
        self.profile_persisted = true;
        self.reload_backups = true;
        self.invalidate_scan();
        self.flash = format!("profile reloaded from {}", self.profile_path.display());
        self.flash_color = self.theme.clean;
        Ok(())
    }

    fn install_profile_document(&mut self, document: config::ProfileDocument) {
        self.profile = document.profile;
        self.profile_path = document.path;
        self.profile_explicit = document.explicit;
        self.profile_persisted = document.persisted;
        self.profile_dirty = false;
        self.reload_backups = true;
    }

    fn invalidate_scan(&mut self) {
        self.scan_phase = ScanPhase::Idle;
        self.pending_scan = None;
        self.finish_scan_as_done = false;
        self.preserve_next_scan_flash = false;
        self.health = None;
        self.scan_report = None;
        self.files.clear();
        self.excluded.clear();
        self.apply_stats = ApplyStats::default();
    }

    pub fn reload_browse(&mut self) -> Result<(), String> {
        let read_dir = fs::read_dir(&self.browse_cwd)
            .map_err(|error| format!("open {}: {error}", self.browse_cwd.display()))?;
        let mut entries = Vec::new();
        for entry in read_dir {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let name = entry.file_name().to_string_lossy().to_string();
            if !self.show_hidden && name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let is_symlink = metadata.file_type().is_symlink();
            let is_dir = metadata.file_type().is_dir();
            entries.push(BrowseEntry {
                category: categorize(&path).to_string(),
                path,
                is_dir,
                is_symlink,
            });
        }
        entries.sort_by(|left, right| {
            right.is_dir.cmp(&left.is_dir).then_with(|| {
                left.path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_lowercase())
                    .cmp(
                        &right
                            .path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_lowercase()),
                    )
            })
        });
        self.browse_entries = entries;
        self.browse_selected = 0;
        Ok(())
    }

    pub fn open_selected_directory(&mut self) -> Result<(), String> {
        let entry = self
            .browse_entries
            .get(self.browse_selected)
            .ok_or_else(|| "no directory selected".to_string())?;
        if entry.is_symlink {
            return Err("symlinks are shown but not followed".to_string());
        }
        if !entry.is_dir {
            return Err("select a directory".to_string());
        }
        self.browse_cwd = entry.path.clone();
        self.reload_browse()
    }

    pub fn browse_parent(&mut self) -> Result<(), String> {
        let Some(parent) = self.browse_cwd.parent() else {
            return Ok(());
        };
        self.browse_cwd = parent.to_path_buf();
        self.reload_browse()
    }

    pub fn browse_root(&mut self) -> Result<(), String> {
        if let Some(root) = self.browse_cwd.ancestors().last() {
            self.browse_cwd = root.to_path_buf();
        }
        self.reload_browse()
    }

    pub fn browse_home(&mut self) -> Result<(), String> {
        if let Some(home) = dirs::home_dir() {
            self.browse_cwd = home;
        }
        self.reload_browse()
    }
}

fn categorize(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("md" | "txt" | "rst") => "text",
        Some("rs" | "py" | "js" | "ts" | "go" | "rb" | "sh") => "code",
        Some("html" | "htm" | "css" | "astro") => "web",
        _ => "",
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
        let path = std::env::temp_dir().join(format!("bash-em-browser-{nonce}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn browser_hides_dot_entries_until_toggled() {
        let root = tempdir();
        fs::create_dir(root.join("visible")).unwrap();
        fs::create_dir(root.join(".hidden")).unwrap();
        let mut app = App::new(root.clone(), config::default_profile());
        assert_eq!(app.browse_entries.len(), 1);
        app.show_hidden = true;
        app.reload_browse().unwrap();
        assert_eq!(app.browse_entries.len(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn browser_descends_and_returns_to_parent() {
        let root = tempdir();
        fs::create_dir(root.join("child")).unwrap();
        let mut app = App::new(root.clone(), config::default_profile());
        app.open_selected_directory().unwrap();
        assert_eq!(app.browse_cwd, root.join("child"));
        app.browse_parent().unwrap();
        assert_eq!(app.browse_cwd, root);
        let _ = fs::remove_dir_all(app.browse_cwd.clone());
    }

    #[test]
    fn profile_toggle_changes_the_live_profile_and_invalidates_scan() {
        let root = tempdir();
        let mut app = App::new(root.clone(), config::default_profile());
        app.profile_selected = PROFILE_RULES
            .iter()
            .position(|rule| rule.key == "curly_quotes")
            .unwrap();
        app.scan_phase = ScanPhase::Review;

        app.toggle_selected_rule();

        assert!(config::is_rule_enabled(&app.profile, "curly_quotes"));
        assert!(app.profile_dirty);
        assert_eq!(app.scan_phase, ScanPhase::Idle);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn profile_save_and_reload_round_trip() {
        let root = tempdir();
        let mut app = App::new(root.clone(), config::default_profile());
        app.profile_selected = PROFILE_RULES
            .iter()
            .position(|rule| rule.key == "curly_quotes")
            .unwrap();
        app.toggle_selected_rule();
        app.save_profile().unwrap();
        assert!(app.profile_persisted);
        assert!(!app.profile_dirty);

        app.toggle_selected_rule();
        assert!(!config::is_rule_enabled(&app.profile, "curly_quotes"));
        app.reload_profile().unwrap();
        assert!(config::is_rule_enabled(&app.profile, "curly_quotes"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn automatic_profile_follows_a_new_scan_root() {
        let first = tempdir();
        let second = tempdir();
        let mut profile = config::default_profile();
        profile.name = "second-root".to_string();
        config::save_profile(&config::project_profile_path(&second), &profile).unwrap();
        let mut app = App::new(first.clone(), config::default_profile());

        app.request_scan_root(second.clone()).unwrap();

        assert_eq!(app.profile.name, "second-root");
        assert_eq!(app.profile_path, config::project_profile_path(&second));
        let _ = fs::remove_dir_all(first);
        let _ = fs::remove_dir_all(second);
    }
}
