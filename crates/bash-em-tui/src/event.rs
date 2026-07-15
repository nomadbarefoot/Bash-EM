use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::app::{App, ConfirmDialog, Pane, ScanPhase, Screen};

pub struct LayoutCache {
    pub tab_rects: Vec<(Rect, Screen)>,
    pub browse_area: Option<Rect>,
    pub browse_offset: usize,
    pub list_area: Option<Rect>,
    pub list_offset: usize,
    pub diff_area: Option<Rect>,
    pub runs_area: Option<Rect>,
    pub runs_offset: usize,
    pub profile_area: Option<Rect>,
    pub profile_offset: usize,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            tab_rects: Vec::new(),
            browse_area: None,
            browse_offset: 0,
            list_area: None,
            list_offset: 0,
            diff_area: None,
            runs_area: None,
            runs_offset: 0,
            profile_area: None,
            profile_offset: 0,
        }
    }

    pub fn reset(&mut self) {
        self.tab_rects.clear();
        self.browse_area = None;
        self.list_area = None;
        self.diff_area = None;
        self.runs_area = None;
        self.profile_area = None;
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent, layout: &LayoutCache) {
    if key.code == KeyCode::Char('q') {
        handle_global_key(app, key);
        return;
    }
    if !matches!(app.confirm, ConfirmDialog::None) {
        handle_confirm_key(app, key);
        return;
    }
    if handle_global_key(app, key) {
        return;
    }

    match app.screen {
        Screen::Welcome => handle_welcome_key(app, key),
        Screen::Browse => handle_browse_key(app, key, layout),
        Screen::Scan => handle_scan_key(app, key, layout),
        Screen::Backups => handle_backups_key(app, key, layout),
        Screen::Profiles => handle_profiles_key(app, key, layout),
    }
}

fn handle_global_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => {
            if app.scan_phase != ScanPhase::Applying && !app.restore_in_progress {
                app.should_quit = true;
            }
            true
        }
        KeyCode::Esc => {
            if app.scan_phase == ScanPhase::Applying || app.restore_in_progress {
                return true;
            }
            if app.screen == Screen::Browse {
                if app.browse_cwd.parent().is_some() {
                    if let Err(error) = app.browse_parent() {
                        set_error(app, error);
                    }
                } else {
                    app.switch_screen(Screen::Welcome);
                }
            } else if app.screen != Screen::Welcome {
                app.switch_screen(Screen::Welcome);
            }
            true
        }
        KeyCode::Char('1') => {
            app.switch_screen(Screen::Welcome);
            true
        }
        KeyCode::Char('2') => {
            app.switch_screen(Screen::Browse);
            true
        }
        KeyCode::Char('3') => {
            app.switch_screen(Screen::Scan);
            true
        }
        KeyCode::Char('4') => {
            app.switch_screen(Screen::Backups);
            true
        }
        KeyCode::Char('5') => {
            app.switch_screen(Screen::Profiles);
            true
        }
        KeyCode::Tab if app.screen == Screen::Scan => {
            app.focused_pane = if app.focused_pane == Pane::OffenderList {
                Pane::DiffPreview
            } else {
                Pane::OffenderList
            };
            true
        }
        _ => false,
    }
}

fn handle_welcome_key(app: &mut App, key: KeyEvent) {
    if app.scan_phase == ScanPhase::Applying || app.restore_in_progress {
        return;
    }
    match key.code {
        KeyCode::Enter => app.request_scan(app.root.clone()),
        KeyCode::Char('b') => app.switch_screen(Screen::Browse),
        _ => {}
    }
}

fn handle_browse_key(app: &mut App, key: KeyEvent, layout: &LayoutCache) {
    if app.scan_phase == ScanPhase::Applying || app.restore_in_progress {
        return;
    }
    let result = match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.move_browse_selection(1);
            return;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.move_browse_selection(-1);
            return;
        }
        KeyCode::PageDown => {
            app.move_browse_selection(viewport_height(layout.browse_area) as i64);
            return;
        }
        KeyCode::PageUp => {
            app.move_browse_selection(-(viewport_height(layout.browse_area) as i64));
            return;
        }
        KeyCode::Enter | KeyCode::Char('l') => app.open_selected_directory(),
        KeyCode::Backspace | KeyCode::Char('h') => app.browse_parent(),
        KeyCode::Char('.') => {
            app.show_hidden = !app.show_hidden;
            app.reload_browse()
        }
        KeyCode::Char('~') => app.browse_home(),
        KeyCode::Char('r') => app.browse_root(),
        KeyCode::Char('s') => {
            if let Err(error) = app.request_scan_root(app.browse_cwd.clone()) {
                set_error(app, error);
            }
            return;
        }
        _ => return,
    };
    if let Err(error) = result {
        set_error(app, error);
    }
}

fn handle_scan_key(app: &mut App, key: KeyEvent, layout: &LayoutCache) {
    if app.scan_phase == ScanPhase::Applying || app.restore_in_progress {
        return;
    }
    if key.code == KeyCode::Char('r') {
        app.request_scan(app.root.clone());
        return;
    }
    if app.scan_phase != ScanPhase::Review {
        return;
    }

    if app.focused_pane == Pane::DiffPreview {
        let max = app
            .selected_file()
            .map(|file| file.changes.len().saturating_sub(1))
            .unwrap_or(0);
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => app.diff_scroll = (app.diff_scroll + 1).min(max),
            KeyCode::Char('k') | KeyCode::Up => app.diff_scroll = app.diff_scroll.saturating_sub(1),
            KeyCode::PageDown => {
                app.diff_scroll = (app.diff_scroll + viewport_height(layout.diff_area)).min(max)
            }
            KeyCode::PageUp => {
                app.diff_scroll = app
                    .diff_scroll
                    .saturating_sub(viewport_height(layout.diff_area))
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.move_file_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_file_selection(-1),
        KeyCode::PageDown => app.move_file_selection(viewport_height(layout.list_area) as i64),
        KeyCode::PageUp => app.move_file_selection(-(viewport_height(layout.list_area) as i64)),
        KeyCode::Char(' ') => app.toggle_selected(),
        KeyCode::Char('a') => {
            let file_count = app.included_paths().len();
            if file_count > 0 {
                app.confirm = ConfirmDialog::ApplyConfirm {
                    file_count,
                    selected_button: 0,
                };
            }
        }
        _ => {}
    }
}

fn handle_backups_key(app: &mut App, key: KeyEvent, layout: &LayoutCache) {
    if app.scan_phase == ScanPhase::Applying || app.restore_in_progress {
        return;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => move_run_selection(app, 1),
        KeyCode::Char('k') | KeyCode::Up => move_run_selection(app, -1),
        KeyCode::PageDown => move_run_selection(app, viewport_height(layout.runs_area) as i64),
        KeyCode::PageUp => move_run_selection(app, -(viewport_height(layout.runs_area) as i64)),
        KeyCode::Char('r') | KeyCode::Enter => {
            if let Some(run) = app.runs.get(app.runs_selected) {
                app.confirm = ConfirmDialog::RestoreRun {
                    run_id: run.run_id.clone(),
                    root: run.root.clone(),
                    file_count: run.file_count,
                    selected_button: 0,
                };
            }
        }
        _ => {}
    }
}

fn handle_profiles_key(app: &mut App, key: KeyEvent, layout: &LayoutCache) {
    if matches!(app.scan_phase, ScanPhase::Scanning | ScanPhase::Applying)
        || app.restore_in_progress
    {
        return;
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.move_profile_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_profile_selection(-1),
        KeyCode::PageDown => {
            app.move_profile_selection(viewport_height(layout.profile_area) as i64)
        }
        KeyCode::PageUp => {
            app.move_profile_selection(-(viewport_height(layout.profile_area) as i64))
        }
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_selected_rule(),
        KeyCode::Char('s') => app.request_scan(app.root.clone()),
        KeyCode::Char('w') => {
            if let Err(error) = app.save_profile() {
                set_error(app, error);
            }
        }
        KeyCode::Char('l') => {
            if let Err(error) = app.reload_profile() {
                set_error(app, error);
            }
        }
        _ => {}
    }
}

fn handle_confirm_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Left | KeyCode::Right => match &mut app.confirm {
            ConfirmDialog::RestoreRun {
                selected_button, ..
            }
            | ConfirmDialog::ApplyConfirm {
                selected_button, ..
            } => {
                *selected_button = if *selected_button == 0 { 1 } else { 0 };
            }
            ConfirmDialog::None => {}
        },
        KeyCode::Enter => {
            match &app.confirm {
                ConfirmDialog::RestoreRun {
                    run_id,
                    root,
                    selected_button: 0,
                    ..
                } => app.pending_restore = Some((run_id.clone(), root.clone())),
                ConfirmDialog::ApplyConfirm {
                    selected_button: 0, ..
                } => {
                    app.pending_apply = true;
                    app.scan_phase = ScanPhase::Applying;
                }
                _ => {}
            }
            if app.pending_restore.is_some() {
                app.restore_in_progress = true;
                app.flash = "restoring…".to_string();
                app.flash_color = app.theme.count;
            }
            app.confirm = ConfirmDialog::None;
        }
        KeyCode::Esc => app.confirm = ConfirmDialog::None,
        _ => {}
    }
}

pub fn handle_mouse(app: &mut App, event: MouseEvent, layout: &LayoutCache) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let point = (event.column, event.row).into();
            for (rect, screen) in &layout.tab_rects {
                if rect.contains(point) {
                    app.switch_screen(*screen);
                    return;
                }
            }
            if let Some(area) = layout.browse_area.filter(|area| area.contains(point)) {
                let index = (event.row - area.y) as usize + layout.browse_offset;
                if index < app.browse_entries.len() {
                    app.browse_selected = index;
                }
                return;
            }
            if let Some(area) = layout.list_area.filter(|area| area.contains(point)) {
                let index = (event.row - area.y) as usize + layout.list_offset;
                if index < app.files.len() {
                    app.list_selected = index;
                    app.diff_scroll = 0;
                }
                return;
            }
            if let Some(area) = layout.runs_area.filter(|area| area.contains(point)) {
                let index = (event.row - area.y) as usize + layout.runs_offset;
                if index < app.runs.len() {
                    app.runs_selected = index;
                }
                return;
            }
            if let Some(area) = layout.profile_area.filter(|area| area.contains(point)) {
                let index = (event.row - area.y) as usize + layout.profile_offset;
                if index < crate::app::PROFILE_RULES.len() {
                    app.profile_selected = index;
                }
            }
        }
        MouseEventKind::ScrollUp => scroll(app, -3),
        MouseEventKind::ScrollDown => scroll(app, 3),
        _ => {}
    }
}

fn scroll(app: &mut App, delta: i64) {
    match app.screen {
        Screen::Browse => app.move_browse_selection(delta),
        Screen::Scan if app.focused_pane == Pane::DiffPreview => {
            let max = app
                .selected_file()
                .map(|file| file.changes.len().saturating_sub(1))
                .unwrap_or(0);
            app.diff_scroll = if delta < 0 {
                app.diff_scroll
                    .saturating_sub(delta.unsigned_abs() as usize)
            } else {
                (app.diff_scroll + delta as usize).min(max)
            };
        }
        Screen::Scan => app.move_file_selection(delta),
        Screen::Backups => move_run_selection(app, delta),
        Screen::Profiles => app.move_profile_selection(delta),
        Screen::Welcome => {}
    }
}

fn move_run_selection(app: &mut App, delta: i64) {
    if app.runs.is_empty() {
        return;
    }
    let current = app.runs_selected as i64;
    app.runs_selected = (current + delta).rem_euclid(app.runs.len() as i64) as usize;
}

fn viewport_height(area: Option<Rect>) -> usize {
    area.map(|area| area.height as usize).unwrap_or(1).max(1)
}

fn set_error(app: &mut App, error: String) {
    app.flash = error;
    app.flash_color = app.theme.guilty;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tempdir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("bash-em-event-{nonce}"));
        fs::create_dir_all(path.join("child")).unwrap();
        path
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn q_is_the_only_quit_key() {
        let root = tempdir();
        let mut app = App::new(root.clone(), config::default_profile());
        let layout = LayoutCache::new();

        handle_key(&mut app, key(KeyCode::Esc), &layout);
        assert!(!app.should_quit);
        handle_key(&mut app, key(KeyCode::Char('q')), &layout);
        assert!(app.should_quit);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn escape_goes_up_in_browse_then_home_from_other_screens() {
        let root = tempdir();
        let mut app = App::new(root.clone(), config::default_profile());
        let layout = LayoutCache::new();
        app.switch_screen(Screen::Browse);
        app.browse_cwd = root.join("child");

        handle_key(&mut app, key(KeyCode::Esc), &layout);
        assert_eq!(app.browse_cwd, root);
        assert_eq!(app.screen, Screen::Browse);
        app.switch_screen(Screen::Profiles);
        handle_key(&mut app, key(KeyCode::Esc), &layout);
        assert_eq!(app.screen, Screen::Welcome);
        assert!(!app.should_quit);
        let _ = fs::remove_dir_all(root);
    }
}
