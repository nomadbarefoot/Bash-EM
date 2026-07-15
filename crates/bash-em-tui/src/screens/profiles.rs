use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Pane, PROFILE_RULES};
use crate::event::LayoutCache;

pub fn draw_profiles(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let columns =
        Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]).split(area);
    draw_rules(frame, app, columns[0], cache);
    draw_details(frame, app, columns[1]);
}

fn draw_rules(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let status = if app.profile_dirty {
        " · UNSAVED"
    } else if app.profile_persisted {
        " · SAVED"
    } else {
        " · BUILT-IN"
    };
    let title = format!(" RULES · {}{} ", app.profile.name.to_uppercase(), status);
    let block = app
        .theme
        .panel_block(&title, app.focused_pane == Pane::ProfileRules);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    cache.profile_area = Some(inner);
    cache.profile_offset = visible_offset(
        app.profile_selected,
        PROFILE_RULES.len(),
        inner.height as usize,
    );

    let items = PROFILE_RULES
        .iter()
        .enumerate()
        .skip(cache.profile_offset)
        .take(inner.height as usize)
        .map(|(index, rule)| {
            let enabled = config::is_rule_enabled(&app.profile, rule.key);
            let selected = index == app.profile_selected;
            ListItem::new(Line::from(vec![
                Span::styled(
                    if enabled { "● " } else { "○ " },
                    Style::new().fg(if enabled {
                        app.theme.clean
                    } else {
                        app.theme.muted
                    }),
                ),
                Span::styled(rule.label, Style::new().fg(app.theme.text)),
                Span::raw("  "),
                Span::styled(
                    if enabled { "ON" } else { "OFF" },
                    Style::new()
                        .fg(if enabled {
                            app.theme.clean
                        } else {
                            app.theme.muted
                        })
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

fn draw_details(frame: &mut Frame, app: &App, area: Rect) {
    let block = app.theme.panel_block(" LIVE PROFILE ", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rule = &PROFILE_RULES[app.profile_selected];
    let enabled = config::is_rule_enabled(&app.profile, rule.key);
    let state_color = if enabled {
        app.theme.clean
    } else {
        app.theme.muted
    };
    let backup_dir = config::resolve_backup_dir(&app.profile.prefs);
    let ignore_path = app.root.join(".bash-emignore");
    let profile_location = if app.profile_explicit {
        format!("profile  {}", compact_path(&app.profile_path))
    } else {
        "profile  .bash-em.yaml · automatic per root".to_string()
    };
    let lines = vec![
        Line::from(vec![
            Span::styled(
                rule.label,
                Style::new()
                    .fg(app.theme.focus)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                if enabled { "ENABLED" } else { "DISABLED" },
                Style::new().fg(state_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(rule.detail, Style::new().fg(app.theme.text))),
        Line::default(),
        Line::from(Span::styled(
            if app.profile_dirty {
                "Unsaved. Press w before the next scan to persist."
            } else if app.profile_persisted {
                "Saved. Press w after changing a rule."
            } else {
                "Built-in defaults. Press w to create the project profile."
            },
            Style::new().fg(app.theme.count),
        )),
        Line::from(Span::styled(
            profile_location,
            Style::new().fg(app.theme.text),
        )),
        Line::from(Span::styled(
            "l reloads from disk",
            Style::new().fg(app.theme.muted),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("fence guard  ", Style::new().fg(app.theme.muted)),
            Span::styled(
                if app.profile.prefs.fence_guard {
                    "ON"
                } else {
                    "OFF"
                },
                Style::new().fg(app.theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("preview      ", Style::new().fg(app.theme.muted)),
            Span::styled(
                format!("{} changed lines per file", app.profile.prefs.preview_lines),
                Style::new().fg(app.theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("max file     ", Style::new().fg(app.theme.muted)),
            Span::styled(
                format!("{} MiB", app.profile.prefs.max_file_bytes / 1_048_576),
                Style::new().fg(app.theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("profile globs ", Style::new().fg(app.theme.muted)),
            Span::styled(
                app.profile.ignore.len().to_string(),
                Style::new().fg(app.theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("root ignore   ", Style::new().fg(app.theme.muted)),
            Span::styled(
                if ignore_path.is_file() {
                    ".bash-emignore active"
                } else {
                    ".bash-emignore not found"
                },
                Style::new().fg(if ignore_path.is_file() {
                    app.theme.clean
                } else {
                    app.theme.muted
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("backup vault  ", Style::new().fg(app.theme.muted)),
            Span::styled(compact_path(&backup_dir), Style::new().fg(app.theme.text)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(ratatui::widgets::Wrap { trim: true })
            .style(Style::new().bg(app.theme.panel)),
        inner,
    );
}

fn compact_path(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(relative) = path.strip_prefix(home) {
            return format!("~/{}", relative.display());
        }
    }
    path.display().to_string()
}

fn visible_offset(selected: usize, total: usize, height: usize) -> usize {
    if total <= height || height == 0 {
        0
    } else {
        selected.saturating_sub(height - 1).min(total - height)
    }
}
