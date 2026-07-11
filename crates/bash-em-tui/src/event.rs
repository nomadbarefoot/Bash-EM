use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::Rect;
use crate::app::{App, Screen, ScanPhase, Pane, ConfirmDialog};

pub struct LayoutCache {
    pub tab_rects: Vec<(Rect, Screen)>,
    pub list_area: Option<Rect>,
    pub diff_area: Option<Rect>,
    pub runs_area: Option<Rect>,
    pub addon_area: Option<Rect>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            tab_rects: Vec::new(),
            list_area: None,
            diff_area: None,
            runs_area: None,
            addon_area: None,
        }
    }

    pub fn reset(&mut self) {
        self.tab_rects.clear();
        self.list_area = None;
        self.diff_area = None;
        self.runs_area = None;
        self.addon_area = None;
    }
}

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if matches!(app.confirm, ConfirmDialog::RestoreRun { .. } | ConfirmDialog::ApplyConfirm { .. }) {
        handle_confirm_key(app, key);
        return;
    }

    if handle_global_key(app, key) {
        return;
    }

    match app.screen {
        Screen::Welcome => handle_welcome_key(app, key),
        Screen::Scan => handle_scan_key(app, key),
        Screen::Backups => handle_backups_key(app, key),
        _ => {}
    }
}

fn handle_global_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            if app.scan_phase == ScanPhase::Applying { return true; }
            app.should_quit = true;
            true
        }
        KeyCode::Tab => {
            app.cycle_pane();
            true
        }
        KeyCode::Char('1') => { app.switch_screen(Screen::Welcome); true }
        KeyCode::Char('2') => { app.switch_screen(Screen::Browse); true }
        KeyCode::Char('3') => { app.switch_screen(Screen::Scan); true }
        KeyCode::Char('4') => { app.switch_screen(Screen::Backups); true }
        KeyCode::Char('5') => { app.switch_screen(Screen::Profiles); true }
        _ => false,
    }
}

fn handle_welcome_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            app.switch_screen(Screen::Scan);
            app.scan_phase = ScanPhase::Scanning;
        }
        KeyCode::Char('b') => app.switch_screen(Screen::Browse),
        KeyCode::Char('t') => {
            if app.addon.is_some() {
                app.addon_focused = !app.addon_focused;
                if app.addon_focused {
                    app.focused_pane = Pane::AddonPanel;
                } else {
                    app.focused_pane = Pane::MissionControl;
                }
            }
        }
        _ => {}
    }
}

fn handle_scan_key(app: &mut App, key: KeyEvent) {
    if app.scan_phase == ScanPhase::Applying { return; }

    match key.code {
        KeyCode::Char('j') | KeyCode::Down => app.move_list_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_list_selection(-1),
        KeyCode::Char(' ') => app.toggle_selected(),
        KeyCode::Char('a') => {
            if app.scan_phase == ScanPhase::Review && !app.files.is_empty() {
                let count = app.apply_set().len();
                if count > 0 {
                    app.confirm = ConfirmDialog::ApplyConfirm {
                        file_count: count,
                        selected_button: 0,
                    };
                }
            }
        }
        _ => {}
    }
}

fn handle_backups_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.runs.is_empty() {
                app.runs_selected = (app.runs_selected + 1) % app.runs.len();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if !app.runs.is_empty() {
                app.runs_selected = app.runs_selected.checked_sub(1).unwrap_or(app.runs.len() - 1);
            }
        }
        KeyCode::Char('r') => {
            if let Some(run) = app.runs.get(app.runs_selected) {
                app.confirm = ConfirmDialog::RestoreRun {
                    run_id: run.run_id.clone(),
                    file_count: run.file_count,
                    selected_button: 0,
                };
            }
        }
        _ => {}
    }
}

fn handle_confirm_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Left => {
            match &mut app.confirm {
                ConfirmDialog::RestoreRun { selected_button, .. } |
                ConfirmDialog::ApplyConfirm { selected_button, .. } => {
                    *selected_button = selected_button.saturating_sub(1);
                }
                _ => {}
            }
        }
        KeyCode::Right => {
            match &mut app.confirm {
                ConfirmDialog::RestoreRun { selected_button, .. } => {
                    *selected_button = (*selected_button + 1).min(2);
                }
                ConfirmDialog::ApplyConfirm { selected_button, .. } => {
                    *selected_button = (*selected_button + 1).min(1);
                }
                _ => {}
            }
        }
        KeyCode::Enter => {
            match &app.confirm {
                ConfirmDialog::RestoreRun { selected_button: 0, .. } => {
                }
                ConfirmDialog::ApplyConfirm { selected_button: 0, .. } => {
                    app.scan_phase = ScanPhase::Applying;
                }
                _ => {}
            }
            app.confirm = ConfirmDialog::None;
        }
        KeyCode::Esc => {
            app.confirm = ConfirmDialog::None;
        }
        _ => {}
    }
}

pub fn handle_mouse(app: &mut App, event: MouseEvent, layout: &LayoutCache) {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            let col = event.column;
            let row = event.row;

            for (rect, screen) in &layout.tab_rects {
                if rect.contains((col, row).into()) {
                    app.switch_screen(*screen);
                    return;
                }
            }

            if let Some(area) = layout.list_area {
                if area.contains((col, row).into()) {
                    let clicked_index = (row - area.y) as usize + app.list_offset;
                    if clicked_index < app.files.len() {
                        app.list_selected = clicked_index;
                    }
                    return;
                }
            }

            if let Some(area) = layout.runs_area {
                if area.contains((col, row).into()) {
                    let clicked_index = (row - area.y) as usize;
                    if clicked_index < app.runs.len() {
                        app.runs_selected = clicked_index;
                    }
                    return;
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if app.screen == Screen::Scan {
                if app.focused_pane == Pane::OffenderList {
                    app.move_list_selection(-3);
                } else if app.focused_pane == Pane::DiffPreview {
                    app.diff_scroll = app.diff_scroll.saturating_sub(3);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if app.screen == Screen::Scan {
                if app.focused_pane == Pane::OffenderList {
                    app.move_list_selection(3);
                } else if app.focused_pane == Pane::DiffPreview {
                    app.diff_scroll += 3;
                }
            }
        }
        _ => {}
    }
}
