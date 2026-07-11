//! ui.rs — all drawing. Reads App, never mutates it.
//!
//! ratatui is immediate-mode: every frame we rebuild the whole widget tree
//! from state. "Animation" is just state changing between frames.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Phase, TOP_N};

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn draw(f: &mut Frame, app: &App) {
    // Vertical split: header / body / stats / keybar
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(f.size());

    draw_header(f, app, rows[0]);
    draw_body(f, app, rows[1]);
    draw_stats(f, app, rows[2]);
    draw_keybar(f, app, rows[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let spinner = SPINNER[(app.tick as usize) % SPINNER.len()];
    let (status, color) = match app.phase {
        Phase::Scanning => (format!("{spinner} scanning {}", app.root.display()), Color::Yellow),
        Phase::Review => ("review — pick your targets".to_string(), Color::Cyan),
        Phase::Applying => (format!("{spinner} bashing them..."), Color::Red),
        Phase::Done => ("done. the dashes are gone.".to_string(), Color::Green),
    };
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" bash", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("—", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::CROSSED_OUT)),
        Span::styled("m ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(status, Style::default().fg(color)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    // ---- left: top-N offender list ----
    let items: Vec<ListItem> = app
        .files
        .iter()
        .take(TOP_N)
        .enumerate()
        .map(|(i, file)| {
            let excluded = app.excluded.contains(&i);
            let mark = if excluded { "✗" } else { "●" };
            let mark_color = if excluded { Color::DarkGray } else { Color::Red };
            // Show path relative to root when possible — shorter, friendlier.
            let display = file
                .path
                .strip_prefix(&app.root)
                .unwrap_or(&file.path)
                .display()
                .to_string();
            let style = if excluded {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{mark} "), Style::default().fg(mark_color)),
                Span::styled(format!("{:>4} ", file.counts.total()), Style::default().fg(Color::Yellow)),
                Span::styled(display, style),
            ]))
        })
        .collect();

    let hidden = app.files.len().saturating_sub(TOP_N);
    let list_title = if hidden > 0 {
        format!(" top {} offenders (+{} more, all included) ", TOP_N, hidden)
    } else {
        format!(" offenders ({}) ", app.files.len())
    };

    let mut state = ListState::default();
    if !app.files.is_empty() {
        state.select(Some(app.selected));
    }
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, cols[0], &mut state);

    // ---- right: diff preview of selected file ----
    let mut lines: Vec<Line> = Vec::new();
    if let Some(file) = app.files.get(app.selected) {
        lines.push(Line::from(Span::styled(
            format!(
                "em:{} en:{} bar:{} entities:{}  ({} lines affected)",
                file.counts.em, file.counts.en, file.counts.bar,
                file.counts.entities, file.lines_changed
            ),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
        for ch in &file.previews {
            lines.push(Line::from(Span::styled(
                format!("L{}", ch.line_no),
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(Span::styled(
                format!("- {}", truncate(&ch.before, 200)),
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                format!("+ {}", truncate(&ch.after, 200)),
                Style::default().fg(Color::Green),
            )));
        }
        if file.lines_changed > file.previews.len() {
            lines.push(Line::from(Span::styled(
                format!("… and {} more lines", file.lines_changed - file.previews.len()),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else {
        lines.push(Line::from("no dirty files found (yet)"));
    }
    let preview = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" preview "))
        .wrap(Wrap { trim: false });
    f.render_widget(preview, cols[1]);
}

fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let t = app.total_counts();
    match app.phase {
        Phase::Applying | Phase::Done => {
            // Progress gauge across the apply set.
            let total = app.apply_set().len().max(1);
            let ratio = (app.applied_files as f64 / total as f64).min(1.0);
            let label = format!(
                "{} / {} files · {} dashes purged",
                app.applied_files, total, app.applied_counts.total()
            );
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" extermination progress "))
                .gauge_style(Style::default().fg(Color::Green))
                .ratio(ratio)
                .label(label);
            f.render_widget(gauge, area);
        }
        _ => {
            let throughput = if app.scan_ms > 0 {
                format!("{:.0} files/s", app.scanned as f64 / (app.scan_ms as f64 / 1000.0))
            } else {
                "…".to_string()
            };
            let text = vec![
                Line::from(vec![
                    Span::raw(format!("scanned {}  ", app.scanned)),
                    Span::styled(format!("skipped {}  ", app.skipped), Style::default().fg(Color::DarkGray)),
                    Span::raw(format!("dirty {}  ", app.files.len())),
                    Span::styled(throughput, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled(format!("— em {}  ", t.em), Style::default().fg(Color::Red)),
                    Span::styled(format!("– en {}  ", t.en), Style::default().fg(Color::Magenta)),
                    Span::styled(format!("― bar {}  ", t.bar), Style::default().fg(Color::Blue)),
                    Span::styled(format!("&entities; {}  ", t.entities), Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("Σ {}", t.total()),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]),
            ];
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title(" stats "));
            f.render_widget(p, area);
        }
    }
}

fn draw_keybar(f: &mut Frame, app: &App, area: Rect) {
    let keys = match app.phase {
        Phase::Scanning => "q quit",
        Phase::Review => "↑↓/jk move · space toggle · a APPLY · l save log · q quit",
        Phase::Applying => "hold on…",
        Phase::Done => "l save log · q quit",
    };
    let mut spans = vec![Span::styled(
        format!(" {keys} "),
        Style::default().fg(Color::DarkGray),
    )];
    if !app.flash.is_empty() {
        spans.push(Span::styled(
            format!(" {} ", app.flash),
            Style::default().fg(Color::Green),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Clamp preview lines so a 5000-char minified JS line doesn't eat the panel.
/// char_indices respects UTF-8 boundaries — byte slicing could panic mid-char.
fn truncate(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((idx, _)) => format!("{}…", &s[..idx]),
        None => s.to_string(),
    }
}
