use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Paragraph};

use crate::app::{App, ConfirmDialog, Pane};
use crate::event::LayoutCache;

pub fn draw_backups(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let has_confirm = matches!(app.confirm, ConfirmDialog::RestoreRun { .. });

    let main_rows = if has_confirm {
        Layout::vertical([
            Constraint::Min(8),
            Constraint::Length(6),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Min(8),
            Constraint::Length(0),
        ])
        .split(area)
    };

    let cols = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(main_rows[0]);

    draw_runs_table(frame, app, cols[0], layout_cache);
    draw_profile_yaml(frame, app, cols[1]);

    if let ConfirmDialog::RestoreRun {
        run_id,
        file_count,
        selected_button,
    } = &app.confirm
    {
        draw_restore_dialog(frame, app, main_rows[1], run_id, *file_count, *selected_button);
    }
}

fn draw_runs_table(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let theme = &app.theme;

    let title = if app.runs.is_empty() {
        " RUNS ".to_string()
    } else {
        format!(
            " RUNS {:>width$} ",
            format!("{} OF {}", app.runs_selected + 1, app.runs.len()),
            width = 20
        )
    };

    let runs_block = theme.panel_block(&title, app.focused_pane == Pane::RunsTable);
    let runs_inner = runs_block.inner(area);
    frame.render_widget(runs_block, area);
    layout_cache.runs_area = Some(runs_inner);

    if app.runs.is_empty() {
        let lines = vec![
            Line::default(),
            Line::from(vec![Span::styled(
                "no backup runs found",
                Style::new().fg(theme.muted),
            )]),
            Line::default(),
            Line::from(vec![Span::styled(
                "originals are content-addressed.",
                Style::new().fg(theme.muted),
            )]),
            Line::from(vec![Span::styled(
                "restore is boring on purpose.",
                Style::new().fg(theme.muted),
            )]),
        ];
        let paragraph = Paragraph::new(lines).style(Style::new().bg(theme.panel));
        frame.render_widget(paragraph, runs_inner);
        return;
    }

    // Header row
    let header_area = Rect::new(runs_inner.x, runs_inner.y, runs_inner.width, 1);
    let header = Paragraph::new(Line::from(vec![Span::styled(
        format!(
            " {:<10} {:<10} {:<6} {}",
            "run", "when", "files", "profile"
        ),
        Style::new().fg(theme.muted),
    )]))
    .style(Style::new().bg(theme.panel));
    frame.render_widget(header, header_area);

    // Separator
    let sep_area = Rect::new(runs_inner.x, runs_inner.y + 1, runs_inner.width, 1);
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme.panel)),
        sep_area,
    );

    let list_area = Rect::new(
        runs_inner.x,
        runs_inner.y + 2,
        runs_inner.width,
        runs_inner.height.saturating_sub(4),
    );

    let items: Vec<ListItem> = app
        .runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let selected = i == app.runs_selected;
            let bg = if selected {
                theme.highlight_bg
            } else {
                theme.panel
            };
            let id_short = &run.run_id[..8.min(run.run_id.len())];

            let id_style = if selected {
                Style::new()
                    .fg(theme.count)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.count)
            };

            let line = Line::from(vec![
                Span::styled(format!(" {:<10}", id_short), id_style),
                Span::styled(
                    format!(" {:<10}", run.when_relative),
                    Style::new().fg(theme.text),
                ),
                Span::styled(
                    format!(" {:<6}", run.file_count),
                    Style::new().fg(theme.text),
                ),
                Span::styled(
                    "typographic",
                    Style::new().fg(theme.muted),
                ),
            ]);
            ListItem::new(line).style(Style::new().bg(bg))
        })
        .collect();

    let list = List::new(items).style(Style::new().bg(theme.panel));
    frame.render_widget(list, list_area);

    // Footer note
    let note_y = runs_inner.y + runs_inner.height.saturating_sub(1);
    let note_area = Rect::new(runs_inner.x, note_y, runs_inner.width, 1);
    let note = Paragraph::new(Line::from(vec![Span::styled(
        "originals are content-addressed. restore is boring on purpose.",
        Style::new().fg(theme.muted),
    )]))
    .style(Style::new().bg(theme.panel));
    frame.render_widget(note, note_area);
}

fn draw_profile_yaml(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let yaml_title = format!(
        " PROFILE \u{00b7} {} ",
        app.profile.name.to_uppercase()
    );
    let yaml_block = theme.panel_block(&yaml_title, app.focused_pane == Pane::ProfileEditor);
    let yaml_inner = yaml_block.inner(area);
    frame.render_widget(yaml_block, area);

    // VALID YAML indicator on first row
    let valid_area = Rect::new(yaml_inner.x, yaml_inner.y, yaml_inner.width, 1);

    let valid_line = Line::from(vec![
        Span::styled(
            "# editable here or on disk",
            Style::new().fg(theme.muted),
        ),
        Span::styled(
            format!(
                "{:>width$}",
                "VALID YAML",
                width = yaml_inner
                    .width
                    .saturating_sub(26) as usize
            ),
            Style::new()
                .fg(theme.clean)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(valid_line).style(Style::new().bg(theme.panel)),
        valid_area,
    );

    let yaml_body = Rect::new(
        yaml_inner.x,
        yaml_inner.y + 1,
        yaml_inner.width,
        yaml_inner.height.saturating_sub(1),
    );

    let yaml_lines: Vec<Line> = app
        .profile_yaml
        .lines()
        .take(yaml_body.height as usize)
        .map(|line| {
            if line.starts_with('#') || line.starts_with("---") {
                Line::from(vec![Span::styled(line, Style::new().fg(theme.muted))])
            } else if let Some((key, val)) = line.split_once(':') {
                let val_trimmed = val.trim();
                let val_color = if val_trimmed == "true" {
                    theme.count
                } else if val_trimmed == "false" {
                    theme.muted
                } else {
                    theme.clean
                };
                Line::from(vec![
                    Span::styled(key, Style::new().fg(theme.focus)),
                    Span::styled(":", Style::new().fg(theme.border)),
                    Span::styled(val, Style::new().fg(val_color)),
                ])
            } else {
                Line::from(vec![Span::styled(
                    line,
                    Style::new().fg(theme.text),
                )])
            }
        })
        .collect();

    let yaml_paragraph = Paragraph::new(yaml_lines).style(Style::new().bg(theme.panel));
    frame.render_widget(yaml_paragraph, yaml_body);
}

fn draw_restore_dialog(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    run_id: &str,
    file_count: usize,
    selected: u8,
) {
    let theme = &app.theme;

    let block = Block::bordered()
        .border_style(Style::new().fg(theme.accent))
        .border_type(BorderType::Plain)
        .style(Style::new().bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(inner);

    // Title
    let id_short = &run_id[..8.min(run_id.len())];
    let msg = Paragraph::new(Line::from(vec![Span::styled(
        format!("restore {}?", id_short),
        Style::new()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )]))
    .style(Style::new().bg(theme.panel));
    frame.render_widget(msg, rows[0]);

    // Detail line
    let root_name = app
        .root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let detail = Paragraph::new(Line::from(vec![Span::styled(
        format!(
            "{} files \u{2190} {}/ \u{00b7} profile hash matched \u{00b7} no sarcasm in this dialog",
            file_count, root_name
        ),
        Style::new().fg(theme.muted),
    )]))
    .style(Style::new().bg(theme.panel));
    frame.render_widget(detail, rows[1]);

    // Buttons
    let buttons = ["Restore", "Cancel", "Open in $EDITOR"];
    let mut btn_spans = Vec::new();
    btn_spans.push(Span::styled(
        "                              ",
        Style::new().bg(theme.panel),
    ));
    for (i, label) in buttons.iter().enumerate() {
        let (fg, bg, _border_color) = if i as u8 == selected {
            if i == 0 {
                (theme.bg, theme.clean, theme.clean)
            } else {
                (theme.bg, theme.accent, theme.accent)
            }
        } else {
            (theme.accent, theme.panel, theme.accent)
        };
        btn_spans.push(Span::styled(
            format!("  {}  ", label),
            Style::new()
                .fg(fg)
                .bg(bg)
                .add_modifier(if i as u8 == selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        ));
        btn_spans.push(Span::styled("  ", Style::new().bg(theme.panel)));
    }
    let btns = Paragraph::new(Line::from(btn_spans))
        .alignment(Alignment::Right)
        .style(Style::new().bg(theme.panel));
    frame.render_widget(btns, rows[3]);
}
