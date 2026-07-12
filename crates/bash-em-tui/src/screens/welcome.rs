use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Gauge, List, ListItem, Paragraph};

use crate::app::{App, Pane, Screen};
use crate::event::LayoutCache;

pub fn draw_welcome(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let hchunks = Layout::horizontal([
        Constraint::Percentage(60),
        Constraint::Percentage(40),
    ])
    .split(area);

    let vchunks = Layout::vertical([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(hchunks[1]);

    draw_mission_control(frame, app, hchunks[0]);
    draw_navigate(frame, app, vchunks[0]);
    draw_addon_panel(frame, app, vchunks[1], layout_cache);
}

fn draw_mission_control(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = theme.panel_block(
        " MISSION CONTROL ",
        app.focused_pane == Pane::MissionControl,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.health_scanning && app.health.is_none() {
        let spinner = ["\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}"];
        let ch = spinner[(app.tick as usize) % spinner.len()];
        let lines = vec![
            Line::default(),
            Line::from(vec![
                Span::styled(
                    "Bash",
                    Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
                Span::styled("\u{2014}", Style::new().fg(theme.guilty)),
                Span::styled(
                    "EM",
                    Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![Span::styled(
                "the em-dashes know. they are not ready.",
                Style::new().fg(theme.accent),
            )]),
            Line::default(),
            Line::from(vec![
                Span::raw(ch),
                Span::raw(" "),
                Span::styled("scanning\u{2026}", Style::new().fg(theme.focus)),
            ]),
        ];
        let paragraph = Paragraph::new(lines).style(Style::new().bg(theme.panel));
        frame.render_widget(paragraph, inner);
        return;
    }

    let Some(health) = &app.health else {
        return;
    };

    let score = health.score;
    let total = health.corruption.total();

    let vchunks = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(inner);

    // ── header: branding + stats ──
    let score_str = format!("{}%", score);
    let total_str = format!("{}", total);
    let header_lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(
                "Bash",
                Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "\u{2014}",
                Style::new().fg(theme.guilty).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "EM",
                Style::new().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            "the em-dashes know. they are not ready.",
            Style::new().fg(theme.accent),
        )]),
        Line::default(),
        Line::from(vec![
            Span::styled("directory", Style::new().fg(theme.text)),
            Span::styled("              ", Style::new().fg(theme.panel)),
            Span::styled(
                &score_str,
                Style::new().fg(theme.guilty).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" \u{00b7} {}", total_str),
                Style::new().fg(theme.guilty).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let header = Paragraph::new(header_lines).style(Style::new().bg(theme.panel));
    frame.render_widget(header, vchunks[0]);

    // ── corruption label ──
    let corr_line = Paragraph::new(Line::from(vec![
        Span::styled("corruption", Style::new().fg(theme.text)),
        Span::styled("              ", Style::new().fg(theme.panel)),
        Span::styled(
            "artifacts",
            Style::new().fg(theme.guilty).add_modifier(Modifier::BOLD),
        ),
    ]))
    .style(Style::new().bg(theme.panel));
    frame.render_widget(corr_line, vchunks[1]);

    // ── gauge bar ──
    let gauge = Gauge::default()
        .ratio(score as f64 / 100.0)
        .gauge_style(Style::new().fg(theme.guilty).bg(theme.bar_track))
        .label(Span::raw(format!("{}%", score)));
    frame.render_widget(gauge, vchunks[2]);

    // ── bottom area: category grid + buttons + status ──
    let bottom = vchunks[3];
    let bottom_rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(8),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(0),
    ])
    .split(bottom);

    // spacer
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme.panel)),
        bottom_rows[0],
    );

    // ── category grid 2x3 ──
    draw_category_grid(frame, app, health, bottom_rows[1]);

    // spacer
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme.panel)),
        bottom_rows[2],
    );

    // ── action buttons ──
    let btn_line = Line::from(vec![
        Span::styled(
            " \u{2318} Scan & bash ",
            Style::new()
                .fg(theme.bg)
                .bg(theme.clean)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::new().bg(theme.panel)),
        Span::styled(
            " Browse\u{2026} ",
            Style::new().fg(theme.focus).bg(theme.highlight_bg),
        ),
        Span::styled("  ", Style::new().bg(theme.panel)),
        Span::styled(
            " Backups ",
            Style::new().fg(theme.focus).bg(theme.highlight_bg),
        ),
    ]);
    let btns = Paragraph::new(btn_line).style(Style::new().bg(theme.panel));
    frame.render_widget(btns, bottom_rows[3]);

    // ── status line ──
    if bottom_rows[4].height > 0 {
        let mut status_spans = Vec::new();
        if let Some(rid) = &app.apply_stats.backup_run_id {
            status_spans.push(Span::styled(
                format!(
                    "last run: {} \u{00b7} {} dashes purged \u{00b7} restore available",
                    relative_time_short(rid),
                    app.apply_stats.applied_counts.total()
                ),
                Style::new().fg(theme.muted),
            ));
        }
        let status = Paragraph::new(Line::from(status_spans)).style(Style::new().bg(theme.panel));
        frame.render_widget(status, bottom_rows[4]);
    }
}

fn draw_category_grid(
    frame: &mut Frame,
    app: &App,
    health: &engine::health::HealthReport,
    area: Rect,
) {
    let theme = &app.theme;

    let mut categories: Vec<(&str, usize)> = Vec::new();
    let cat_order = ["text / docs", "code", "web assets", "office", "xlsx cells", "pdf (text)"];
    let cat_keys = ["text", "code", "web", "office", "xlsx", "pdf"];

    for (i, key) in cat_keys.iter().enumerate() {
        let count = health
            .by_category
            .get(*key)
            .map(|s| s.artifact_count)
            .unwrap_or(0);
        categories.push((cat_order[i], count));
    }

    let grid_rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4),
    ])
    .split(area);

    for (row_idx, row_area) in grid_rows.iter().enumerate() {
        let cols = Layout::horizontal([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(*row_area);

        for (col_idx, col_area) in cols.iter().enumerate() {
            let cat_idx = row_idx * 3 + col_idx;
            if cat_idx >= categories.len() {
                break;
            }
            let (label, count) = categories[cat_idx];

            let cell_block = Block::bordered()
                .border_style(Style::new().fg(theme.border))
                .border_type(BorderType::Plain)
                .style(Style::new().bg(theme.panel));
            let cell_inner = cell_block.inner(*col_area);
            frame.render_widget(cell_block, *col_area);

            if cell_inner.height >= 1 {
                let count_color = if count > 0 { theme.count } else { theme.muted };
                let lines = vec![Line::from(vec![
                    Span::styled(
                        format!("{}", count),
                        Style::new()
                            .fg(count_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])];
                let mut all_lines = lines;
                if cell_inner.height >= 2 {
                    all_lines.push(Line::from(vec![Span::styled(
                        label.to_string(),
                        Style::new().fg(theme.muted),
                    )]));
                }
                let p = Paragraph::new(all_lines).style(Style::new().bg(theme.panel));
                frame.render_widget(p, cell_inner);
            }
        }
    }
}

fn draw_navigate(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let block = theme.panel_block(" NAVIGATE ", app.focused_pane == Pane::Navigate);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items = [
        ("\u{2318}  Welcome / health", Some(Screen::Welcome)),
        ("Browse directory", Some(Screen::Browse)),
        ("Review offenders", Some(Screen::Scan)),
        ("Backup vault", Some(Screen::Backups)),
        ("Profiles (YAML)", Some(Screen::Profiles)),
        ("Stats museum", None),
    ];

    let list_items: Vec<ListItem> = items
        .iter()
        .map(|(label, screen)| {
            let active = screen.map(|s| s == app.screen).unwrap_or(false);
            let bg = if active {
                theme.highlight_bg
            } else {
                theme.panel
            };
            let indicator = if active {
                Span::styled(
                    "\u{2502}",
                    Style::new().fg(theme.focus).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(" ")
            };
            let label_style = if active {
                Style::new()
                    .fg(theme.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(theme.text)
            };
            let line = Line::from(vec![
                indicator,
                Span::styled(format!(" {}", label), label_style),
            ]);
            ListItem::new(line).style(Style::new().bg(bg))
        })
        .collect();

    let list = List::new(list_items).style(Style::new().bg(theme.panel));
    frame.render_widget(list, inner);
}

fn draw_addon_panel(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let theme = &app.theme;
    let title = if app.addon.is_some() {
        " ADD-ON \u{00b7} TETRIS "
    } else {
        " ADD-ON "
    };
    let block = theme.panel_block(title, app.focused_pane == Pane::AddonPanel);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    layout_cache.addon_area = Some(inner);

    if let Some(addon) = &app.addon {
        addon.draw(frame, inner);
    } else {
        let paragraph = Paragraph::new(Line::from(vec![Span::styled(
            "no add-ons loaded",
            Style::new().fg(theme.muted),
        )]))
        .style(Style::new().bg(theme.panel));
        frame.render_widget(paragraph, inner);
    }
}

fn relative_time_short(_rid: &str) -> &'static str {
    "recent"
}
