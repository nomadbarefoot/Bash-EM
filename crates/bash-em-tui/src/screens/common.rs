use std::path::Path;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, ScanPhase, Screen};
use crate::event::LayoutCache;

pub fn draw_titlebar(frame: &mut Frame, app: &App, area: Rect) {
    let subtitle = match app.screen {
        Screen::Welcome => env!("CARGO_PKG_VERSION"),
        Screen::Browse => "filesystem browser",
        Screen::Scan => match app.scan_phase {
            ScanPhase::Idle => "ready",
            ScanPhase::Scanning => "scanning…",
            ScanPhase::Review => "review targets",
            ScanPhase::Applying => "applying…",
            ScanPhase::Done => "done",
        },
        Screen::Backups => "backup vault",
        Screen::Profiles => "profile rules",
    };
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(app.theme.bg)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "Bash",
                Style::new().fg(app.theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "—",
                Style::new()
                    .fg(app.theme.guilty)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "EM",
                Style::new().fg(app.theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ·  ", Style::new().fg(app.theme.muted)),
            Span::styled(subtitle, Style::new().fg(app.theme.muted)),
        ]))
        .alignment(Alignment::Center)
        .style(Style::new().bg(app.theme.bg)),
        rows[1],
    );
}

pub fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(app.theme.bg)),
        rows[0],
    );
    let tabs = [
        ("Welcome", Screen::Welcome),
        ("Browse", Screen::Browse),
        ("Scan", Screen::Scan),
        ("Backups", Screen::Backups),
        ("Profiles", Screen::Profiles),
    ];
    let root = shorten_path(&app.root);
    let tabs_width =
        tabs.iter().map(|(label, _)| label.len() + 2).sum::<usize>() + (tabs.len() - 1) * 2;
    let chunks = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Length((tabs_width as u16).min(area.width.saturating_sub(2))),
        Constraint::Min(0),
    ])
    .split(rows[1]);
    let mut spans = Vec::new();
    let mut x = chunks[1].x;
    for (index, (label, screen)) in tabs.iter().enumerate() {
        let text = format!(" {label} ");
        let width = text.len() as u16;
        spans.push(Span::styled(
            text,
            if app.screen == *screen {
                Style::new()
                    .fg(app.theme.bg)
                    .bg(app.theme.focus)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(app.theme.muted)
            },
        ));
        cache
            .tab_rects
            .push((Rect::new(x, chunks[1].y, width, 1), *screen));
        x += width;
        if index + 1 < tabs.len() {
            spans.push(Span::raw("  "));
            x += 2;
        }
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(app.theme.bg)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(root)
            .alignment(Alignment::Right)
            .style(Style::new().fg(app.theme.muted).bg(app.theme.bg)),
        chunks[2],
    );
}

pub fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
    let keys: &[(&str, &str)] = match app.screen {
        Screen::Welcome => &[("enter", "scan"), ("b", "browse"), ("1-5", "tabs")],
        Screen::Browse => &[
            ("enter", "open"),
            ("esc/backspace", "up"),
            ("s", "scan here"),
            (".", "hidden"),
            ("~", "home"),
            ("r", "root"),
        ],
        Screen::Scan => &[
            ("jk", "move"),
            ("space", "include"),
            ("tab", "pane"),
            ("a", "apply"),
            ("r", "rescan"),
            ("esc", "welcome"),
        ],
        Screen::Backups => &[("jk", "select"), ("r/enter", "restore"), ("esc", "welcome")],
        Screen::Profiles => &[
            ("jk", "move"),
            ("space", "toggle"),
            ("s", "scan"),
            ("w", "save"),
            ("l", "load"),
            ("esc", "home"),
        ],
    };
    let mut spans = Vec::new();
    for (index, (key, label)) in keys.iter().enumerate() {
        spans.push(Span::styled(
            format!(" {key} "),
            Style::new()
                .fg(app.theme.text)
                .bg(Color::Rgb(27, 32, 48))
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::new().fg(app.theme.muted),
        ));
        if index + 1 < keys.len() {
            spans.push(Span::raw("  "));
        }
    }
    let footer_columns =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(9)]).split(rows[0]);
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(app.theme.bg)),
        footer_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " q ",
                Style::new()
                    .fg(app.theme.bg)
                    .bg(app.theme.guilty)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" quit", Style::new().fg(app.theme.guilty)),
        ]))
        .alignment(Alignment::Right)
        .style(Style::new().bg(app.theme.bg)),
        footer_columns[1],
    );
    frame.render_widget(
        Paragraph::new(app.flash.as_str())
            .alignment(Alignment::Right)
            .style(Style::new().fg(app.flash_color).bg(app.theme.bg)),
        rows[1],
    );
}

fn shorten_path(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}
