use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::App;

pub fn draw_profiles(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let cols = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .split(area);

    // Rule toggles (left)
    let rules_block = theme.panel_block(" RULES ", true);
    let rules_inner = rules_block.inner(cols[0]);
    frame.render_widget(rules_block, cols[0]);

    let items: Vec<ListItem> = app
        .rule_toggles
        .iter()
        .map(|(name, enabled)| {
            let state_text = if *enabled { "ON" } else { "OFF" };
            let (state_color, _border_color) = if *enabled {
                (theme.clean, theme.rule_on_border)
            } else {
                (theme.muted, theme.border)
            };
            let bg = if *enabled {
                theme.rule_on_bg
            } else {
                theme.panel
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", name),
                    Style::new().fg(theme.text).bg(bg),
                ),
                Span::styled("  ", Style::new().bg(bg)),
                Span::styled(
                    state_text,
                    Style::new()
                        .fg(state_color)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .style(Style::new().bg(bg))
        })
        .collect();

    let list = List::new(items).style(Style::new().bg(theme.panel));
    frame.render_widget(list, rules_inner);

    // YAML editor (right)
    let yaml_title = format!(
        " PROFILE \u{00b7} {} ",
        app.profile.name.to_uppercase()
    );
    let yaml_block = theme.panel_block(&yaml_title, false);
    let yaml_inner = yaml_block.inner(cols[1]);
    frame.render_widget(yaml_block, cols[1]);

    let header = Paragraph::new(Line::from(vec![Span::styled(
        "# editable here or on disk",
        Style::new().fg(theme.muted),
    )]))
    .style(Style::new().bg(theme.panel));
    let header_area = Rect::new(yaml_inner.x, yaml_inner.y, yaml_inner.width, 1);
    frame.render_widget(header, header_area);

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
                let val_color = if val.trim() == "true" {
                    theme.count
                } else if val.trim() == "false" {
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
