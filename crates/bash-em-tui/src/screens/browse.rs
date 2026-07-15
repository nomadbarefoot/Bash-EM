use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::event::LayoutCache;

pub fn draw_browse(frame: &mut Frame, app: &App, area: Rect, cache: &mut LayoutCache) {
    let rows = Layout::vertical([Constraint::Length(2), Constraint::Min(4)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" location  ", Style::new().fg(app.theme.muted)),
            Span::styled(
                app.browse_cwd.display().to_string(),
                Style::new().fg(app.theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if app.show_hidden {
                    "   hidden shown"
                } else {
                    "   hidden off"
                },
                Style::new().fg(app.theme.muted),
            ),
        ]))
        .style(Style::new().bg(app.theme.bg)),
        rows[0],
    );
    let block = app
        .theme
        .panel_block(" DIRECTORY — s scans this folder ", true);
    let inner = block.inner(rows[1]);
    frame.render_widget(block, rows[1]);
    cache.browse_area = Some(inner);
    let offset = visible_offset(
        app.browse_selected,
        app.browse_entries.len(),
        inner.height as usize,
    );
    cache.browse_offset = offset;
    if app.browse_entries.is_empty() {
        frame.render_widget(
            Paragraph::new("empty or unreadable directory")
                .style(Style::new().fg(app.theme.muted).bg(app.theme.panel)),
            inner,
        );
        return;
    }
    let items = app
        .browse_entries
        .iter()
        .enumerate()
        .skip(offset)
        .take(inner.height as usize)
        .map(|(index, entry)| {
            let selected = index == app.browse_selected;
            let name = entry
                .path
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default();
            let (icon, color) = if entry.is_symlink {
                ("↪ ", app.theme.muted)
            } else if entry.is_dir {
                ("▸ ", app.theme.focus)
            } else {
                ("  ", app.theme.text)
            };
            let mut spans = vec![
                Span::styled(icon, Style::new().fg(color)),
                Span::styled(
                    name.to_string(),
                    Style::new().fg(color).add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ),
            ];
            if !entry.category.is_empty() {
                spans.push(Span::styled(
                    format!("  [{}]", entry.category),
                    Style::new().fg(app.theme.muted),
                ));
            }
            ListItem::new(Line::from(spans)).style(Style::new().bg(if selected {
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

fn visible_offset(selected: usize, total: usize, height: usize) -> usize {
    if total <= height || height == 0 {
        0
    } else {
        selected.saturating_sub(height - 1).min(total - height)
    }
}
