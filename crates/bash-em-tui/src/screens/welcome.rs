use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw_welcome(frame: &mut Frame, app: &App, area: Rect) {
    let block = app.theme.panel_block(" MISSION CONTROL ", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Min(0),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Bash—EM",
                Style::new()
                    .fg(app.theme.guilty)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "find the tells. bash them. keep the receipt.",
                Style::new().fg(app.theme.accent),
            )),
            Line::default(),
            Line::from(vec![
                Span::styled("selected root  ", Style::new().fg(app.theme.muted)),
                Span::styled(
                    app.root.display().to_string(),
                    Style::new().fg(app.theme.text),
                ),
            ]),
        ])
        .alignment(Alignment::Center)
        .style(Style::new().bg(app.theme.panel)),
        rows[0],
    );

    if let Some(health) = &app.health {
        frame.render_widget(
            Gauge::default()
                .ratio(health.score as f64 / 100.0)
                .label(format!(
                    "{}% dirty files · {} artifacts",
                    health.score,
                    health.corruption.total()
                ))
                .gauge_style(Style::new().fg(app.theme.guilty).bg(app.theme.bar_track)),
            rows[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new("not scanned yet — no surprise work happens at startup")
                .alignment(Alignment::Center)
                .style(Style::new().fg(app.theme.muted).bg(app.theme.panel)),
            rows[1],
        );
    }
    frame.render_widget(
        Paragraph::new(format!(
            "enter  scan     b  browse     5  profile rules     active  {}",
            app.profile.name
        ))
        .alignment(Alignment::Center)
        .style(Style::new().fg(app.theme.clean).bg(app.theme.panel)),
        rows[2],
    );
}
