use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, ConfirmDialog, Pane, ScanPhase};
use crate::event::LayoutCache;

pub fn draw_scan(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let has_confirm = matches!(app.confirm, ConfirmDialog::ApplyConfirm { .. });
    let rows = Layout::vertical([
        Constraint::Min(7),
        Constraint::Length(if has_confirm { 5 } else { 0 }),
    ])
    .split(area);
    match app.scan_phase {
        ScanPhase::Idle => draw_center(
            frame,
            app,
            rows[0],
            "no scan yet",
            "press r to scan the selected root",
        ),
        ScanPhase::Scanning => {
            draw_center(frame, app, rows[0], "scanning…", &app.scan_stats.progress)
        }
        ScanPhase::Applying => draw_center(
            frame,
            app,
            rows[0],
            "backup sealed. applying…",
            "writes are atomic; quit is locked until completion",
        ),
        ScanPhase::Review => draw_review(frame, app, rows[0], cache),
        ScanPhase::Done => draw_done(frame, app, rows[0]),
    }
    if let ConfirmDialog::ApplyConfirm {
        file_count,
        selected_button,
    } = app.confirm
    {
        draw_confirm(frame, app, rows[1], file_count, selected_button);
    }
}

fn draw_center(frame: &mut Frame, app: &App, area: Rect, title: &str, detail: &str) {
    let block = app.theme.panel_block(" SCAN ", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::default(),
            Line::from(Span::styled(
                title,
                Style::new()
                    .fg(app.theme.focus)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::default(),
            Line::from(Span::styled(detail, Style::new().fg(app.theme.muted))),
            Line::default(),
            Line::from(Span::styled(
                app.root.display().to_string(),
                Style::new().fg(app.theme.text),
            )),
        ])
        .alignment(Alignment::Center)
        .style(Style::new().bg(app.theme.panel)),
        inner,
    );
}

fn draw_review(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(5)]).split(area);
    let included = app.files.len().saturating_sub(app.excluded.len());
    frame.render_widget(
        Paragraph::new(format!(
            " {} dirty files · {} included · {} scanned · {} skipped · {}ms",
            app.files.len(),
            included,
            app.scan_stats.scanned,
            app.scan_stats.skipped,
            app.scan_stats.scan_ms
        ))
        .style(Style::new().fg(app.theme.muted).bg(app.theme.bg)),
        rows[0],
    );
    let columns =
        Layout::horizontal([Constraint::Percentage(44), Constraint::Percentage(56)]).split(rows[1]);
    draw_files(frame, app, columns[0], cache);
    draw_diff(frame, app, columns[1], cache);
}

fn draw_files(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let block = app
        .theme
        .panel_block(" OFFENDERS ", app.focused_pane == Pane::OffenderList);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    cache.list_area = Some(inner);
    let offset = visible_offset(app.list_selected, app.files.len(), inner.height as usize);
    cache.list_offset = offset;
    let items = app
        .files
        .iter()
        .enumerate()
        .skip(offset)
        .take(inner.height as usize)
        .map(|(index, file)| {
            let selected = index == app.list_selected;
            let excluded = app.excluded.contains(&file.path);
            let relative = file.path.strip_prefix(&app.root).unwrap_or(&file.path);
            ListItem::new(Line::from(vec![
                Span::styled(
                    if excluded { "○ " } else { "● " },
                    Style::new().fg(if excluded {
                        app.theme.muted
                    } else {
                        app.theme.guilty
                    }),
                ),
                Span::styled(
                    relative.display().to_string(),
                    Style::new().fg(if excluded {
                        app.theme.muted
                    } else {
                        app.theme.text
                    }),
                ),
                Span::styled(
                    format!("  {}", file.counts.total()),
                    Style::new()
                        .fg(app.theme.count)
                        .add_modifier(Modifier::BOLD),
                ),
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

fn draw_diff(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let block = app
        .theme
        .panel_block(" DIFF PREVIEW ", app.focused_pane == Pane::DiffPreview);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    cache.diff_area = Some(inner);
    let Some(file) = app.selected_file() else {
        frame.render_widget(
            Paragraph::new("no file selected")
                .style(Style::new().fg(app.theme.muted).bg(app.theme.panel)),
            inner,
        );
        return;
    };
    let mut lines = vec![
        Line::from(Span::styled(
            file.path
                .strip_prefix(&app.root)
                .unwrap_or(&file.path)
                .display()
                .to_string(),
            Style::new()
                .fg(app.theme.focus)
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
    ];
    for change in file.changes.iter().skip(app.diff_scroll) {
        if lines.len() + 3 > inner.height as usize {
            break;
        }
        lines.push(Line::from(Span::styled(
            format!("L{}", change.line_no),
            Style::new().fg(app.theme.muted),
        )));
        lines.push(Line::from(vec![
            Span::styled("− ", Style::new().fg(app.theme.guilty)),
            Span::styled(change.before.clone(), Style::new().fg(app.theme.guilty)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("+ ", Style::new().fg(app.theme.clean)),
            Span::styled(change.after.clone(), Style::new().fg(app.theme.clean)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::new().bg(app.theme.panel)),
        inner,
    );
}

fn draw_done(frame: &mut Frame, app: &App, area: Rect) {
    let block = app.theme.panel_block(" DONE ", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let (title, detail, color) = if let Some(run_id) = &app.apply_stats.backup_run_id {
        (
            format!("{} files cleaned", app.apply_stats.applied_files),
            format!(
                "backup {} retained in {}{}",
                run_id,
                app.apply_stats
                    .backup_dir
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
                if app.apply_stats.errors.is_empty() {
                    ""
                } else {
                    " · partial apply; restore available"
                }
            ),
            if app.apply_stats.errors.is_empty() {
                app.theme.clean
            } else {
                app.theme.guilty
            },
        )
    } else if app.files.is_empty()
        && app
            .health
            .as_ref()
            .map_or(0, |health| health.corruption.total())
            == 0
    {
        (
            "clean tree".to_string(),
            "no offenders found · r rescans".to_string(),
            app.theme.clean,
        )
    } else if app.files.is_empty() {
        let flags = app
            .health
            .as_ref()
            .map_or(0, |health| health.corruption.llm_flags);
        (
            "review-only flags found".to_string(),
            format!("{flags} LLM boilerplate flags · no automatic rewrite"),
            app.theme.count,
        )
    } else {
        (
            "scan complete".to_string(),
            format!("{} offenders remain · r rescans", app.files.len()),
            app.theme.count,
        )
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::default(),
            Line::from(Span::styled(
                title,
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::default(),
            Line::from(Span::styled(detail, Style::new().fg(app.theme.muted))),
        ])
        .alignment(Alignment::Center)
        .style(Style::new().bg(app.theme.panel)),
        inner,
    );
}

fn draw_confirm(frame: &mut Frame, app: &App, area: Rect, file_count: usize, selected: u8) {
    let block = Block::bordered()
        .border_type(BorderType::Double)
        .border_style(Style::new().fg(app.theme.accent))
        .style(Style::new().bg(app.theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("Apply to {file_count} files?  "),
                Style::new().fg(app.theme.text),
            ),
            button(" APPLY ", selected == 0, app),
            Span::raw("  "),
            button(" CANCEL ", selected == 1, app),
        ]))
        .alignment(Alignment::Center)
        .style(Style::new().bg(app.theme.panel)),
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
