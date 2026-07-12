use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::App;

pub fn draw_browse(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(5),
    ])
    .split(area);

    // Stats bar
    let total_files = app.browse_entries.iter().filter(|e| !e.is_dir).count();
    let total_dirs = app.browse_entries.iter().filter(|e| e.is_dir).count();
    let total_artifacts: usize = app.browse_entries.iter().map(|e| e.artifact_count).sum();

    let stats_rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(rows[0]);

    let stats = Paragraph::new(Line::from(vec![
        Span::styled(" root ", Style::new().fg(theme.muted)),
        Span::styled(
            app.root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            Style::new()
                .fg(theme.text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} dirs", total_dirs),
            Style::new().fg(theme.muted),
        ),
        Span::styled(
            format!("  {} files", total_files),
            Style::new().fg(theme.muted),
        ),
        if total_artifacts > 0 {
            Span::styled(
                format!("  {} artifacts", total_artifacts),
                Style::new().fg(theme.guilty),
            )
        } else {
            Span::styled("  clean", Style::new().fg(theme.clean))
        },
    ]))
    .style(Style::new().bg(theme.bg));
    frame.render_widget(stats, stats_rows[0]);

    frame.render_widget(
        Paragraph::new("").style(Style::new().bg(theme.bg)),
        stats_rows[1],
    );

    // File tree
    let block = theme.panel_block(" DIRECTORY ", true);
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);

    if app.browse_entries.is_empty() {
        let p = Paragraph::new(Line::from(vec![Span::styled(
            "empty directory",
            Style::new().fg(theme.muted),
        )]))
        .style(Style::new().bg(theme.panel));
        frame.render_widget(p, inner);
        return;
    }

    let viewport_h = inner.height as usize;
    let offset = if app.browse_selected >= app.browse_offset + viewport_h {
        app.browse_selected + 1 - viewport_h
    } else if app.browse_selected < app.browse_offset {
        app.browse_selected
    } else {
        app.browse_offset
    };

    let items: Vec<ListItem> = app
        .browse_entries
        .iter()
        .enumerate()
        .skip(offset)
        .take(viewport_h)
        .map(|(i, entry)| {
            let selected = i == app.browse_selected;
            let bg = if selected {
                theme.highlight_bg
            } else {
                theme.panel
            };
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_dir { "\u{25b8} " } else { "  " };
            let name = entry
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let name_color = if entry.is_dir {
                theme.focus
            } else if entry.artifact_count > 0 {
                theme.guilty
            } else {
                theme.text
            };

            let name_style = if selected {
                Style::new()
                    .fg(name_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(name_color)
            };

            let mut spans = vec![
                Span::styled(indent, Style::new().fg(theme.muted)),
                Span::styled(
                    icon,
                    Style::new().fg(if entry.is_dir {
                        theme.focus
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled(name, name_style),
            ];

            if !entry.is_dir && !entry.category.is_empty() {
                spans.push(Span::styled(
                    format!(" [{}]", entry.category),
                    Style::new().fg(theme.muted),
                ));
            }

            if entry.artifact_count > 0 {
                spans.push(Span::styled(
                    format!(" {}", entry.artifact_count),
                    Style::new()
                        .fg(theme.count)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            ListItem::new(Line::from(spans)).style(Style::new().bg(bg))
        })
        .collect();

    let list = List::new(items).style(Style::new().bg(theme.panel));
    frame.render_widget(list, inner);
}
