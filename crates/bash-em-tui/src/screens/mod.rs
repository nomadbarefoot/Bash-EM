mod common;
mod welcome;
mod scan;
mod backups;
mod browse;
mod profiles;

pub use common::{draw_titlebar, draw_tab_bar, draw_footer};
pub use welcome::draw_welcome;
pub use scan::draw_scan;
pub use backups::draw_backups;
pub use browse::draw_browse;
pub use profiles::draw_profiles;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use crate::app::{App, Screen};
use crate::event::LayoutCache;

pub fn draw(frame: &mut Frame, app: &App, layout_cache: &mut LayoutCache) {
    layout_cache.reset();
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(2),
    ]).split(frame.area());

    draw_titlebar(frame, app, rows[0]);
    draw_tab_bar(frame, app, rows[1], layout_cache);
    match app.screen {
        Screen::Welcome => draw_welcome(frame, app, rows[2], layout_cache),
        Screen::Scan => draw_scan(frame, app, rows[2], layout_cache),
        Screen::Backups => draw_backups(frame, app, rows[2], layout_cache),
        Screen::Browse => draw_browse(frame, app, rows[2]),
        Screen::Profiles => draw_profiles(frame, app, rows[2]),
    }
    draw_footer(frame, app, rows[3]);
}
