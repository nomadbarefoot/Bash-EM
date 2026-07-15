use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::Frame;

pub trait Addon: Send {
    fn name(&self) -> &str;
    fn tick(&mut self, dt_ms: u64);
    fn handle_key(&mut self, key: KeyEvent);
    fn handle_mouse(&mut self, event: MouseEvent, area: Rect);
    fn draw(&self, frame: &mut Frame, area: Rect);
}
