mod backups;
mod browse;
mod common;
mod profiles;
mod scan;
mod welcome;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::{App, Screen};
use crate::event::LayoutCache;

pub fn draw(frame: &mut Frame, app: &App, cache: &mut LayoutCache) {
    cache.reset();
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(10),
        Constraint::Length(2),
    ])
    .split(frame.area());
    common::draw_titlebar(frame, app, rows[0]);
    common::draw_tab_bar(frame, app, rows[1], cache);
    match app.screen {
        Screen::Welcome => welcome::draw_welcome(frame, app, rows[2]),
        Screen::Browse => browse::draw_browse(frame, app, rows[2], cache),
        Screen::Scan => scan::draw_scan(frame, app, rows[2], cache),
        Screen::Backups => backups::draw_backups(frame, app, rows[2], cache),
        Screen::Profiles => profiles::draw_profiles(frame, app, rows[2], cache),
    }
    common::draw_footer(frame, app, rows[3]);
}
