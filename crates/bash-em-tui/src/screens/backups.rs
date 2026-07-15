use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, ConfirmDialog, Pane};
use crate::event::LayoutCache;

pub fn draw_backups(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let has_confirm = matches!(app.confirm, ConfirmDialog::RestoreRun { .. });
    let rows = Layout::vertical([
        Constraint::Min(7),
        Constraint::Length(if has_confirm { 5 } else { 0 }),
    ])
    .split(area);
    let columns =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(rows[0]);
    draw_runs(frame, app, columns[0], cache);
    draw_details(frame, app, columns[1]);
    if let ConfirmDialog::RestoreRun {
        ref run_id,
        ref root,
        file_count,
        selected_button,
    } = app.confirm
    {
        draw_confirm(
            frame,
            app,
            rows[1],
            run_id,
            root,
            file_count,
            selected_button,
        );
    }
}

fn draw_runs(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let vault = config::resolve_backup_dir(&app.profile.prefs);
    let title = format!(" RUNS · {} ", vault.display());
    let block = app
        .theme
        .panel_block(&title, app.focused_pane == Pane::RunsTable);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    cache.runs_area = Some(inner);
    let offset = visible_offset(app.runs_selected, app.runs.len(), inner.height as usize);
    cache.runs_offset = offset;
    if app.runs.is_empty() {
        frame.render_widget(
            Paragraph::new("no backup runs found")
                .style(Style::new().fg(app.theme.muted).bg(app.theme.panel)),
            inner,
        );
        return;
    }
    let items = app
        .runs
        .iter()
        .enumerate()
        .skip(offset)
        .take(inner.height as usize)
        .map(|(index, run)| {
            let selected = index == app.runs_selected;
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}  ", &run.run_id[..8.min(run.run_id.len())]),
                    Style::new().fg(app.theme.count),
                ),
                Span::styled(
                    format!("{:<9}  ", run.when_relative),
                    Style::new().fg(app.theme.text),
                ),
                Span::styled(
                    format!("{:>4} files  ", run.file_count),
                    Style::new().fg(app.theme.text),
                ),
                Span::styled(run.profile_name.clone(), Style::new().fg(app.theme.muted)),
            ]))
            .style(Style::new().bg(if selected {
                app.theme.highlight_bg
            } else {
                app.theme.panel
            }))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).style(Style::new().bg(app.theme.panel)),
        inner,
    );
}

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = app.theme.panel_block(" RESTORE TARGET ", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let lines = if let Some(run) = app.runs.get(app.runs_selected) {
        vec![
            Line::from(vec![
                Span::styled("run      ", Style::new().fg(app.theme.muted)),
                Span::styled(run.run_id.clone(), Style::new().fg(app.theme.count)),
            ]),
            Line::from(vec![
                Span::styled("profile  ", Style::new().fg(app.theme.muted)),
                Span::styled(run.profile_name.clone(), Style::new().fg(app.theme.text)),
            ]),
            Line::from(vec![
                Span::styled("created  ", Style::new().fg(app.theme.muted)),
                Span::styled(run.timestamp.clone(), Style::new().fg(app.theme.text)),
            ]),
            Line::default(),
            Line::from(Span::styled(
                "files will be restored to:",
                Style::new().fg(app.theme.muted),
            )),
            Line::from(Span::styled(
                run.root.display().to_string(),
                Style::new().fg(app.theme.focus),
            )),
            Line::default(),
            Line::from(Span::styled(
                "the restore point is retained",
                Style::new().fg(app.theme.clean),
            )),
            Line::from(Span::styled(
                if app.profile.prefs.keep_last_n == 0 {
                    "retention: unlimited".to_string()
                } else {
                    format!(
                        "retention: last {} runs globally",
                        app.profile.prefs.keep_last_n
                    )
                },
                Style::new().fg(app.theme.muted),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "select a backup run",
            Style::new().fg(app.theme.muted),
        ))]
    };
    frame.render_widget(
        Paragraph::new(lines).style(Style::new().bg(app.theme.panel)),
        inner,
    );
}

fn draw_confirm(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    run_id: &str,
    root: &std::path::Path,
    file_count: usize,
    selected: u8,
) {
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(app.theme.accent))
        .style(Style::new().bg(app.theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let short = &run_id[..8.min(run_id.len())];
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!(
                "Restore {file_count} files from {short} to {}?",
                root.display()
            )),
            Line::from(vec![
                button(" RESTORE ", selected == 0, app),
                Span::raw("  "),
                button(" CANCEL ", selected == 1, app),
            ]),
        ])
        .alignment(Alignment::Center)
        .style(Style::new().fg(app.theme.text).bg(app.theme.panel)),
        inner,
    );
}

fn button<'a>(label: &'a str, selected: bool, app: &App) -> Span<'a> {
    Span::styled(
        label,
        if selected {
            Style::new()
                .fg(app.theme.bg)
                .bg(app.theme.clean)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(app.theme.muted).bg(app.theme.bar_track)
        },
    )
}

fn visible_offset(selected: usize, total: usize, height: usize) -> usize {
    if total <= height || height == 0 {
        0
    } else {
        selected.saturating_sub(height - 1).min(total - height)
    }
}
