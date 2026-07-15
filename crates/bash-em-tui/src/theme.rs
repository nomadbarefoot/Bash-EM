use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType};

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub panel: Color,
    pub border: Color,
    pub focus: Color,
    pub guilty: Color,
    pub clean: Color,
    pub count: Color,
    pub muted: Color,
    pub text: Color,
    pub accent: Color,
    pub en: Color,
    pub bar_track: Color,
    pub highlight_bg: Color,
    pub rule_on_bg: Color,
    pub rule_on_border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Rgb(7, 8, 13),
            panel: Color::Rgb(16, 18, 26),
            border: Color::Rgb(42, 49, 68),
            focus: Color::Rgb(92, 225, 255),
            guilty: Color::Rgb(255, 77, 109),
            clean: Color::Rgb(184, 245, 116),
            count: Color::Rgb(255, 229, 102),
            muted: Color::Rgb(122, 132, 156),
            text: Color::Rgb(232, 236, 247),
            accent: Color::Rgb(255, 140, 66),
            en: Color::Rgb(217, 120, 255),
            bar_track: Color::Rgb(26, 31, 46),
            highlight_bg: Color::Rgb(26, 36, 56),
            rule_on_bg: Color::Rgb(18, 26, 18),
            rule_on_border: Color::Rgb(61, 90, 46),
        }
    }
}

impl Theme {
    pub fn panel_block<'a>(&self, title: &'a str, focused: bool) -> Block<'a> {
        let border_color = if focused { self.focus } else { self.border };
        let border_type = if focused {
            BorderType::Double
        } else {
            BorderType::Plain
        };
        Block::bordered()
            .title(title)
            .border_style(Style::default().fg(border_color))
            .border_type(border_type)
            .style(Style::default().bg(self.panel))
            .title_style(Style::default().fg(self.muted))
    }

    pub fn tab_style(&self, active: bool) -> Style {
        if active {
            Style::default().fg(self.bg).bg(self.focus)
        } else {
            Style::default().fg(self.muted)
        }
    }
}
