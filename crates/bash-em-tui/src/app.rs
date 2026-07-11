use std::collections::HashSet;
use std::path::PathBuf;

use crate::addon::Addon;
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Welcome,
    Browse,
    Scan,
    Backups,
    Profiles,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ScanPhase {
    Idle,
    Scanning,
    Review,
    Applying,
    Done,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Pane {
    MissionControl,
    Navigate,
    AddonPanel,
    OffenderList,
    DiffPreview,
    RulesRow,
    RunsTable,
    ProfileEditor,
}

pub enum ConfirmDialog {
    None,
    RestoreRun { run_id: String, file_count: usize, selected_button: u8 },
    ApplyConfirm { file_count: usize, selected_button: u8 },
}

pub struct ScanFile {
    pub path: PathBuf,
    pub counts: engine::Counts,
    pub lines_changed: usize,
    pub changes: Vec<engine::LineChange>,
}

pub struct ScanStats {
    pub scanned: usize,
    pub skipped: usize,
    pub scan_ms: u128,
}

impl Default for ScanStats {
    fn default() -> Self {
        Self { scanned: 0, skipped: 0, scan_ms: 0 }
    }
}

pub struct ApplyStats {
    pub applied_files: usize,
    pub applied_counts: engine::Counts,
    pub errors: usize,
    pub backup_run_id: Option<String>,
}

impl Default for ApplyStats {
    fn default() -> Self {
        Self {
            applied_files: 0,
            applied_counts: engine::Counts::default(),
            errors: 0,
            backup_run_id: None,
        }
    }
}

pub struct RunRow {
    pub run_id: String,
    pub timestamp: String,
    pub when_relative: String,
    pub file_count: usize,
    pub root: PathBuf,
}

pub struct App {
    pub screen: Screen,
    pub focused_pane: Pane,

    pub root: PathBuf,
    pub profile: config::Profile,
    pub tick: u64,
    pub flash: String,
    pub flash_color: ratatui::style::Color,
    pub should_quit: bool,
    pub theme: Theme,

    pub health: Option<engine::health::HealthReport>,
    pub health_scanning: bool,

    pub scan_phase: ScanPhase,
    pub files: Vec<ScanFile>,
    pub excluded: HashSet<usize>,
    pub list_offset: usize,
    pub list_selected: usize,
    pub diff_scroll: usize,
    pub scan_stats: ScanStats,
    pub apply_stats: ApplyStats,
    pub rule_toggles: Vec<(String, bool)>,

    pub runs: Vec<RunRow>,
    pub runs_selected: usize,
    pub profile_yaml: String,
    pub confirm: ConfirmDialog,

    pub addon: Option<Box<dyn Addon>>,
    pub addon_focused: bool,
}

impl App {
    pub fn new(root: PathBuf, profile: config::Profile) -> Self {
        let rule_toggles: Vec<(String, bool)> = profile.rules.iter()
            .map(|(k, v)| (k.clone(), v.enabled))
            .collect();
        let profile_yaml = serde_yaml::to_string(&profile).unwrap_or_default();
        let theme = Theme::default();
        let flash_color = theme.clean;

        Self {
            screen: Screen::Welcome,
            focused_pane: Pane::MissionControl,
            root,
            profile,
            tick: 0,
            flash: String::new(),
            flash_color,
            should_quit: false,
            theme,
            health: None,
            health_scanning: false,
            scan_phase: ScanPhase::Idle,
            files: Vec::new(),
            excluded: HashSet::new(),
            list_offset: 0,
            list_selected: 0,
            diff_scroll: 0,
            scan_stats: ScanStats::default(),
            apply_stats: ApplyStats::default(),
            rule_toggles,
            runs: Vec::new(),
            runs_selected: 0,
            profile_yaml,
            confirm: ConfirmDialog::None,
            addon: None,
            addon_focused: false,
        }
    }

    pub fn add_scan_file(&mut self, file: ScanFile) {
        let key = file.counts.total();
        let pos = self.files.partition_point(|f| f.counts.total() >= key);
        self.files.insert(pos, file);
        self.excluded = self.excluded.iter()
            .map(|&i| if i >= pos { i + 1 } else { i })
            .collect();
    }

    pub fn selected_file(&self) -> Option<&ScanFile> {
        self.files.get(self.list_selected)
    }

    pub fn toggle_selected(&mut self) {
        if self.files.is_empty() { return; }
        if !self.excluded.remove(&self.list_selected) {
            self.excluded.insert(self.list_selected);
        }
    }

    pub fn apply_set(&self) -> Vec<usize> {
        (0..self.files.len())
            .filter(|i| !self.excluded.contains(i))
            .collect()
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.focused_pane = match screen {
            Screen::Welcome => Pane::MissionControl,
            Screen::Browse => Pane::MissionControl,
            Screen::Scan => Pane::OffenderList,
            Screen::Backups => Pane::RunsTable,
            Screen::Profiles => Pane::ProfileEditor,
        };
    }

    pub fn cycle_pane(&mut self) {
        let panes = match self.screen {
            Screen::Welcome => &[Pane::MissionControl, Pane::Navigate, Pane::AddonPanel][..],
            Screen::Scan => &[Pane::OffenderList, Pane::DiffPreview, Pane::RulesRow][..],
            Screen::Backups => &[Pane::RunsTable, Pane::ProfileEditor][..],
            _ => return,
        };
        if let Some(idx) = panes.iter().position(|p| *p == self.focused_pane) {
            self.focused_pane = panes[(idx + 1) % panes.len()];
        }
    }

    pub fn move_list_selection(&mut self, delta: i64) {
        let n = self.files.len();
        if n == 0 { return; }
        let cur = self.list_selected as i64;
        self.list_selected = (cur + delta).rem_euclid(n as i64) as usize;
    }
}
