use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Gauge, List, ListItem, Paragraph};

use crate::app::{App, ConfirmDialog, Pane, ScanPhase};
use crate::event::LayoutCache;

pub fn draw_scan(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    match app.scan_phase {
        ScanPhase::Idle | ScanPhase::Scanning => draw_scanning(frame, app, area),
        ScanPhase::Review => draw_review(frame, app, area, layout_cache),
        ScanPhase::Applying => draw_applying(frame, app, area),
        ScanPhase::Done => draw_done(frame, app, area),
    }
}

fn draw_scanning(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = theme.panel_block(" SCANNING ", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let spinner = ["\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}"];
    let ch = spinner[(app.tick as usize) % spinner.len()];
    let lines = vec![
        Line::default(),
        Line::from(vec![
            Span::raw(ch),
            Span::raw(" "),
            Span::styled(
                "scanning directory\u{2026}",
                Style::new().fg(theme.focus),
            ),
        ]),
        Line::from(vec![Span::styled(
            format!("{} files found", app.files.len()),
            Style::new().fg(theme.count),
        )]),
    ];
    let paragraph = Paragraph::new(lines).style(Style::new().bg(theme.panel));
    frame.render_widget(paragraph, inner);
}

fn draw_review(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let main_rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(6),
        Constraint::Length(3),
        Constraint::Length(4),
    ])
    .split(area);

    // ── stats panel (bordered, 2 content rows) ──
    draw_stats_panel(frame, app, main_rows[0]);

    // ── offender list + diff preview ──
    let cols = Layout::horizontal([
        Constraint::Percentage(42),
        Constraint::Percentage(58),
    ])
    .split(main_rows[1]);

    draw_offender_list(frame, app, cols[0], layout_cache);
    draw_diff_preview(frame, app, cols[1], layout_cache);

    // ── rules toggles ──
    draw_rules_row(frame, app, main_rows[2]);

    // ── progress / backup section ──
    draw_progress_section(frame, app, main_rows[3]);

    // ── confirm overlay ──
    if let ConfirmDialog::ApplyConfirm {
        file_count,
        selected_button,
    } = &app.confirm
    {
        draw_confirm_overlay(
            frame,
            app,
            area,
            &format!("Apply changes to {} files?", file_count),
            &["Confirm", "Cancel"],
            *selected_button,
        );
    }
}

fn draw_stats_panel(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = Block::bordered()
        .border_style(Style::new().fg(theme.border))
        .border_type(BorderType::Plain)
        .style(Style::new().bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let total_counts = app
        .files
        .iter()
        .fold(engine::Counts::default(), |mut acc, f| {
            acc.add(f.counts);
            acc
        });
    let active = app.apply_set().len();

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(inner);

    // Line 1: counts
    let stats_line = Line::from(vec![
        Span::styled("scanned ", Style::new().fg(theme.muted)),
        Span::styled(
            app.scan_stats.scanned.to_string(),
            Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  skipped ", Style::new().fg(theme.muted)),
        Span::styled(
            app.scan_stats.skipped.to_string(),
            Style::new().fg(theme.muted),
        ),
        Span::styled("  dirty ", Style::new().fg(theme.muted)),
        Span::styled(
            app.files.len().to_string(),
            Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  \u{25cf} ", Style::new().fg(theme.guilty)),
        Span::styled(
            format!("em {}", total_counts.em),
            Style::new().fg(theme.guilty),
        ),
        Span::styled("  \u{25cf} ", Style::new().fg(theme.en)),
        Span::styled(
            format!("en {}", total_counts.en),
            Style::new().fg(theme.en),
        ),
        Span::styled("  \u{03a3} ", Style::new().fg(theme.count)),
        Span::styled(
            total_counts.total().to_string(),
            Style::new()
                .fg(theme.count)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(stats_line).style(Style::new().bg(theme.panel)),
        rows[0],
    );

    // Line 2: throughput / quip
    let speed = if app.scan_stats.scan_ms > 0 {
        (app.scan_stats.scanned as f64 / (app.scan_stats.scan_ms as f64 / 1000.0)) as usize
    } else {
        app.scan_stats.scanned
    };
    let quip = if total_counts.total() > 500 {
        "the dashes are sweating"
    } else if total_counts.total() > 100 {
        "they know what\u{2019}s coming"
    } else if total_counts.total() > 0 {
        "a few stragglers"
    } else {
        "clean run"
    };

    let speed_line = Line::from(vec![
        Span::styled(
            format!(
                "{} files/s \u{00b7} {}  \u{00b7}  {} active",
                speed, quip, active
            ),
            Style::new().fg(theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(speed_line)
            .alignment(Alignment::Right)
            .style(Style::new().bg(theme.panel)),
        rows[1],
    );
}

fn draw_offender_list(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let theme = &app.theme;
    let list_title = format!(
        " OFFENDERS ({})   CLICK \u{00b7} SPACE TOGGLES ",
        app.files.len()
    );
    let list_block = theme.panel_block(&list_title, app.focused_pane == Pane::OffenderList);
    let list_inner = list_block.inner(area);
    frame.render_widget(list_block, area);
    layout_cache.list_area = Some(list_inner);

    let viewport_h = list_inner.height as usize;
    let offset = if app.list_selected >= app.list_offset + viewport_h {
        app.list_selected + 1 - viewport_h
    } else if app.list_selected < app.list_offset {
        app.list_selected
    } else {
        app.list_offset
    };

    let visible: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .skip(offset)
        .take(viewport_h)
        .map(|(i, f)| {
            let excluded = app.excluded.contains(&i);
            let selected = i == app.list_selected;
            let marker = if excluded { "\u{2717}" } else { "\u{25cf}" };
            let marker_color = if excluded { theme.muted } else { theme.guilty };

            let rel_path = f
                .path
                .strip_prefix(&app.root)
                .unwrap_or(&f.path)
                .display()
                .to_string();
            let count_str = format!("{}", f.counts.total());

            let bg = if selected {
                theme.highlight_bg
            } else {
                theme.panel
            };
            let name_style = if selected {
                Style::new()
                    .fg(theme.text)
                    .add_modifier(Modifier::BOLD)
            } else if excluded {
                Style::new().fg(theme.muted)
            } else {
                Style::new().fg(theme.text)
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", marker),
                    Style::new().fg(marker_color),
                ),
                Span::styled(
                    format!("{:>4} ", count_str),
                    Style::new().fg(theme.count),
                ),
                Span::styled(rel_path, name_style),
            ]);
            ListItem::new(line).style(Style::new().bg(bg))
        })
        .collect();

    let list = List::new(visible).style(Style::new().bg(theme.panel));
    frame.render_widget(list, list_inner);
}

fn draw_diff_preview(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let theme = &app.theme;
    let diff_title = if let Some(file) = app.selected_file() {
        let rel = file
            .path
            .strip_prefix(&app.root)
            .unwrap_or(&file.path)
            .display()
            .to_string()
            .to_uppercase();
        format!(" PREVIEW \u{00b7} {}   TEXT ADAPTER ", rel)
    } else {
        " PREVIEW ".to_string()
    };
    let diff_block = theme.panel_block(&diff_title, app.focused_pane == Pane::DiffPreview);
    let diff_inner = diff_block.inner(area);
    frame.render_widget(diff_block, area);
    layout_cache.diff_area = Some(diff_inner);

    let Some(file) = app.selected_file() else {
        let paragraph = Paragraph::new(Line::from(vec![Span::styled(
            "no file selected",
            Style::new().fg(theme.muted),
        )]))
        .style(Style::new().bg(theme.panel));
        frame.render_widget(paragraph, diff_inner);
        return;
    };

    let meta = Line::from(vec![Span::styled(
        format!(
            "em:{} en:{} bar:{} entities:{} \u{00b7} {} lines affected",
            file.counts.em,
            file.counts.en,
            file.counts.bar,
            file.counts.entities,
            file.lines_changed
        ),
        Style::new().fg(theme.count),
    )]);

    let viewport_h = diff_inner.height.saturating_sub(3) as usize;
    let mut diff_lines = vec![meta, Line::default()];
    let total_changes = file.changes.len();
    let shown = file
        .changes
        .iter()
        .skip(app.diff_scroll)
        .take(viewport_h / 3)
        .count();

    for change in file.changes.iter().skip(app.diff_scroll).take(viewport_h / 3) {
        diff_lines.push(Line::from(vec![Span::styled(
            format!("L{}", change.line_no),
            Style::new().fg(theme.muted),
        )]));
        diff_lines.push(Line::from(vec![
            Span::styled("\u{2212} ", Style::new().fg(theme.guilty)),
            Span::styled(
                change.before.clone(),
                Style::new().fg(theme.guilty),
            ),
        ]));
        diff_lines.push(Line::from(vec![
            Span::styled("+ ", Style::new().fg(theme.clean)),
            Span::styled(
                change.after.clone(),
                Style::new().fg(theme.clean),
            ),
        ]));
    }

    let remaining = total_changes.saturating_sub(app.diff_scroll + shown);
    if remaining > 0 {
        diff_lines.push(Line::default());
        diff_lines.push(Line::from(vec![Span::styled(
            format!("\u{2026} and {} more lines", remaining),
            Style::new().fg(theme.muted),
        )]));
    }

    let paragraph = Paragraph::new(diff_lines).style(Style::new().bg(theme.panel));
    frame.render_widget(paragraph, diff_inner);
}

fn draw_rules_row(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let n = app.rule_toggles.len().max(1);
    let constraints: Vec<Constraint> = (0..n)
        .map(|_| Constraint::Ratio(1, n as u32))
        .collect();

    let cols = Layout::horizontal(constraints).split(area);

    for (i, (name, enabled)) in app.rule_toggles.iter().enumerate() {
        if i >= cols.len() {
            break;
        }
        let col = cols[i];

        let (border_color, label_fg, state_text, state_fg, bg) = if *enabled {
            (
                theme.clean,
                theme.text,
                "ON",
                theme.clean,
                theme.rule_on_bg,
            )
        } else {
            (theme.border, theme.muted, "OFF", theme.muted, theme.panel)
        };

        let cell_block = Block::bordered()
            .border_style(Style::new().fg(border_color))
            .border_type(BorderType::Plain)
            .style(Style::new().bg(bg));
        let cell_inner = cell_block.inner(col);
        frame.render_widget(cell_block, col);

        if cell_inner.height >= 1 && cell_inner.width >= 4 {
            let max_name = (cell_inner.width as usize).saturating_sub(5);
            let truncated: String = if name.len() > max_name {
                name.chars().take(max_name).collect()
            } else {
                name.clone()
            };
            let pad = (cell_inner.width as usize)
                .saturating_sub(truncated.len() + state_text.len());
            let line = Line::from(vec![
                Span::styled(truncated, Style::new().fg(label_fg)),
                Span::styled(
                    format!("{:>width$}", state_text, width = pad + state_text.len()),
                    Style::new()
                        .fg(state_fg)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            let p = Paragraph::new(line).style(Style::new().bg(bg));
            frame.render_widget(p, cell_inner);
        }
    }
}

fn draw_progress_section(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let title_right = if let Some(rid) = &app.apply_stats.backup_run_id {
        format!(
            "BACKUP SEALED \u{00b7} {}",
            &rid[..12.min(rid.len())]
        )
    } else {
        String::new()
    };

    let block = Block::bordered()
        .title(" EXTERMINATION PROGRESS ")
        .title_style(Style::new().fg(theme.muted).add_modifier(Modifier::BOLD))
        .border_style(Style::new().fg(theme.border))
        .border_type(BorderType::Plain)
        .style(Style::new().bg(theme.panel));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(inner);

    // Right-aligned backup label
    if !title_right.is_empty() {
        let label = Paragraph::new(Line::from(vec![Span::styled(
            title_right,
            Style::new()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Right)
        .style(Style::new().bg(theme.panel));
        frame.render_widget(label, rows[0]);
    }

    if rows[1].height >= 1 {
        let total = app.apply_set().len().max(1);
        let done = app.apply_stats.applied_files;
        let ratio = if done > 0 {
            (done as f64 / total as f64).min(1.0)
        } else {
            0.0
        };
        let purged = app.apply_stats.applied_counts.total();

        let gauge = Gauge::default()
            .ratio(ratio)
            .gauge_style(Style::new().fg(theme.clean).bg(theme.bar_track))
            .label(Span::raw(format!(
                "{} / {} files \u{00b7} {} dashes purged",
                done, total, purged
            )));
        frame.render_widget(gauge, rows[1]);
    }
}

fn draw_applying(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = theme.panel_block(" EXTERMINATION PROGRESS ", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let total = app.apply_set().len().max(1);
    let done = app.apply_stats.applied_files;
    let ratio = (done as f64 / total as f64).min(1.0);
    let purged = app.apply_stats.applied_counts.total();

    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(inner);

    let lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(
                "exterminating\u{2026}",
                Style::new()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}/{} files", done, total),
                Style::new().fg(theme.count),
            ),
        ]),
    ];
    let paragraph = Paragraph::new(lines).style(Style::new().bg(theme.panel));
    frame.render_widget(paragraph, rows[0]);

    let gauge = Gauge::default()
        .ratio(ratio)
        .gauge_style(Style::new().fg(theme.clean).bg(theme.bar_track))
        .label(Span::raw(format!(
            "{} / {} files \u{00b7} {} dashes purged",
            done, total, purged
        )));
    frame.render_widget(gauge, rows[1]);

    if let Some(rid) = &app.apply_stats.backup_run_id {
        let backup_line = Paragraph::new(Line::from(vec![
            Span::styled("backup sealed \u{00b7} ", Style::new().fg(theme.muted)),
            Span::styled(
                &rid[..8.min(rid.len())],
                Style::new().fg(theme.clean),
            ),
        ]))
        .style(Style::new().bg(theme.panel));
        frame.render_widget(backup_line, rows[2]);
    }
}

fn draw_done(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = theme.panel_block(" DONE ", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let stats = &app.apply_stats;
    let mut lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled("\u{2713} ", Style::new().fg(theme.clean)),
            Span::styled(
                format!("{} files cleaned", stats.applied_files),
                Style::new()
                    .fg(theme.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            format!("  {} replacements", stats.applied_counts.total()),
            Style::new().fg(theme.count),
        )]),
    ];
    if stats.errors > 0 {
        lines.push(Line::from(vec![Span::styled(
            format!("  {} errors", stats.errors),
            Style::new().fg(theme.guilty),
        )]));
    }
    if let Some(rid) = &stats.backup_run_id {
        lines.push(Line::from(vec![
            Span::styled("  backup: ", Style::new().fg(theme.muted)),
            Span::styled(rid.as_str(), Style::new().fg(theme.focus)),
        ]));
        lines.push(Line::from(vec![Span::styled(
            format!("  restore with: bash-em restore {}", rid),
            Style::new().fg(theme.muted),
        )]));
    }
    let paragraph = Paragraph::new(lines).style(Style::new().bg(theme.panel));
    frame.render_widget(paragraph, inner);
}

fn draw_confirm_overlay(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    message: &str,
    buttons: &[&str],
    selected: u8,
) {
    let theme = &app.theme;
    let w = 50u16.min(area.width.saturating_sub(4));
    let h = 6u16.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup_area = Rect::new(x, y, w, h);

    let block = Block::bordered()
        .border_style(Style::new().fg(theme.accent))
        .style(Style::new().bg(theme.panel));
    let inner = block.inner(popup_area);
    frame.render_widget(ratatui::widgets::Clear, popup_area);
    frame.render_widget(block, popup_area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(inner);

    let msg = Paragraph::new(Line::from(vec![Span::styled(
        message,
        Style::new().fg(theme.text),
    )]));
    frame.render_widget(msg, rows[0]);

    let flash = Paragraph::new(Line::from(vec![Span::styled(
        "backup first. then we get messy.",
        Style::new().fg(theme.clean),
    )]));
    frame.render_widget(flash, rows[1]);

    let mut btn_spans = Vec::new();
    for (i, label) in buttons.iter().enumerate() {
        let style = if i as u8 == selected {
            Style::new()
                .fg(theme.bg)
                .bg(theme.focus)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.text)
        };
        btn_spans.push(Span::styled(format!("  {}  ", label), style));
        btn_spans.push(Span::raw("  "));
    }
    let btns = Paragraph::new(Line::from(btn_spans));
    frame.render_widget(btns, rows[3]);
}
