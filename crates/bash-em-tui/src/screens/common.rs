use std::env;
use std::path::PathBuf;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, ScanPhase, Screen};
use crate::event::LayoutCache;

pub fn draw_titlebar(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(area);

    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme.bg)),
        rows[0],
    );

    let subtitle = match app.screen {
        Screen::Welcome => "0.2.0-hub",
        Screen::Scan => match app.scan_phase {
            ScanPhase::Idle | ScanPhase::Scanning => "scanning\u{2026}",
            ScanPhase::Review => "review \u{2014} pick your targets",
            ScanPhase::Applying => "applying\u{2026}",
            ScanPhase::Done => "done",
        },
        Screen::Backups => "vault & preferences",
        Screen::Browse => "browse filesystem",
        Screen::Profiles => "profiles",
    };

    let line = Line::from(vec![
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
        Span::styled("   \u{00b7}   ", Style::new().fg(theme.muted)),
        Span::styled(subtitle, Style::new().fg(theme.muted)),
    ]);

    let paragraph = Paragraph::new(line)
        .alignment(Alignment::Center)
        .style(Style::new().bg(theme.bg));
    frame.render_widget(paragraph, rows[1]);
}

pub fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect, layout_cache: &mut LayoutCache) {
    let theme = &app.theme;
    layout_cache.tab_rects.clear();

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(area);

    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme.bg)),
        rows[0],
    );

    let tabs = [
        ("Welcome", Screen::Welcome),
        ("Browse", Screen::Browse),
        ("Scan", Screen::Scan),
        ("Backups", Screen::Backups),
        ("Profiles", Screen::Profiles),
    ];

    let root_short = shorten_root(&app.root);
    let root_width = (root_short.len() + 2).clamp(0, area.width as usize) as u16;

    let tab_chunks = Layout::horizontal([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(root_width),
    ]).split(rows[1]);

    let tab_area = tab_chunks[1];
    let root_area = tab_chunks[2];

    let mut spans = Vec::new();
    let mut x = tab_area.x;

    for (i, (label, screen)) in tabs.iter().enumerate() {
        let active = app.screen == *screen;
        let padded = format!(" {} ", label);
        let width = padded.len() as u16;

        let style = if active {
            Style::new()
                .fg(theme.bg)
                .bg(theme.focus)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(theme.muted)
        };

        spans.push(Span::styled(padded, style));

        layout_cache
            .tab_rects
            .push((Rect::new(x, tab_area.y, width, 1), *screen));
        x += width;

        if i < tabs.len() - 1 {
            spans.push(Span::styled("  ", Style::new().fg(theme.bg)));
            x += 2;
        }
    }

    let tabs_paragraph =
        Paragraph::new(Line::from(spans)).style(Style::new().bg(theme.bg));
    frame.render_widget(tabs_paragraph, tab_area);

    let root_paragraph = Paragraph::new(Line::from(vec![Span::styled(
        root_short,
        Style::new().fg(theme.muted),
    )]))
    .alignment(Alignment::Right)
    .style(Style::new().bg(theme.bg));
    frame.render_widget(root_paragraph, root_area);
}

pub fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let kbd_bg = Color::Rgb(27, 32, 48);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(area);

    let (line1, line2, tagline): (Vec<(&str, &str)>, Vec<(&str, &str)>, &str) =
        match app.screen {
            Screen::Welcome => (
                vec![
                    ("enter", "scan"),
                    ("b", "browse"),
                    ("tab", "panes"),
                    ("click", "everywhere"),
                    ("t", "tetris focus"),
                    ("q", "quit"),
                ],
                vec![],
                "",
            ),
            Screen::Scan => (
                vec![
                    ("\u{2191}\u{2193}/jk", "move"),
                    ("space", "toggle"),
                    ("a", "APPLY"),
                    ("click", "select / toggle"),
                    ("scroll", "list"),
                ],
                vec![("l", "log"), ("q", "quit")],
                "backup first. then we get messy.",
            ),
            Screen::Backups => (
                vec![
                    ("\u{2191}\u{2193}", "select run"),
                    ("r", "restore"),
                    ("e", "edit profile"),
                    ("tab", "pane focus"),
                    ("click", "works too"),
                ],
                vec![],
                "safety copy is not a bit.",
            ),
            Screen::Browse => (
                vec![
                    ("jk", "move"),
                    ("enter", "open"),
                    ("backspace", "up"),
                    ("s", "select dir"),
                    ("q", "quit"),
                ],
                vec![],
                "",
            ),
            Screen::Profiles => (
                vec![("tab", "focus"), ("q", "quit")],
                vec![],
                "",
            ),
        };

    fn render_key_line(
        legends: &[(&str, &str)],
        theme: &crate::theme::Theme,
        kbd_bg: Color,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        for (i, (key, desc)) in legends.iter().enumerate() {
            spans.push(Span::styled(
                format!(" {} ", key),
                Style::new()
                    .fg(theme.text)
                    .bg(kbd_bg)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                format!(" {} ", desc),
                Style::new().fg(theme.muted),
            ));
            if i < legends.len() - 1 {
                spans.push(Span::styled("  ", Style::new().fg(theme.bg)));
            }
        }
        spans
    }

    let spans1 = render_key_line(&line1, theme, kbd_bg);
    let left1 = Paragraph::new(Line::from(spans1)).style(Style::new().bg(theme.bg));
    frame.render_widget(left1, rows[0]);

    if !line2.is_empty() || !tagline.is_empty() {
        let footer_cols = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(tagline.len() as u16 + 2),
        ]).split(rows[1]);

        if !line2.is_empty() {
            let spans2 = render_key_line(&line2, theme, kbd_bg);
            let left2 =
                Paragraph::new(Line::from(spans2)).style(Style::new().bg(theme.bg));
            frame.render_widget(left2, footer_cols[0]);
        } else {
            frame.render_widget(
                Paragraph::new("").style(Style::new().bg(theme.bg)),
                footer_cols[0],
            );
        }

        if !tagline.is_empty() {
            let right = Paragraph::new(Line::from(vec![Span::styled(
                tagline.to_string(),
                Style::new().fg(theme.clean),
            )]))
            .alignment(Alignment::Right)
            .style(Style::new().bg(theme.bg));
            frame.render_widget(right, footer_cols[1]);
        }
    } else {
        let flash = app.flash.clone();
        let right = Paragraph::new(Line::from(vec![Span::styled(
            flash,
            Style::new().fg(app.flash_color),
        )]))
        .alignment(Alignment::Right)
        .style(Style::new().bg(theme.bg));
        frame.render_widget(right, rows[1]);
    }
}

fn shorten_root(path: &PathBuf) -> String {
    let s = path.display().to_string();
    if let Ok(home) = env::var("HOME") {
        if let Some(stripped) = s.strip_prefix(&home) {
            return format!("~{}", stripped);
        }
    }
    s
}
