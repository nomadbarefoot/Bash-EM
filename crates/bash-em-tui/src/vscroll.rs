use std::ops::Range;

pub struct VScrollState {
    pub selected: usize,
    pub offset: usize,
    pub viewport_height: usize,
    pub total_items: usize,
}

impl VScrollState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            offset: 0,
            viewport_height: 0,
            total_items: 0,
        }
    }

    pub fn move_selection(&mut self, delta: i64) {
        if self.total_items == 0 {
            return;
        }
        let cur = self.selected as i64;
        self.selected = (cur + delta).rem_euclid(self.total_items as i64) as usize;
        self.ensure_visible();
    }

    pub fn ensure_visible(&mut self) {
        if self.viewport_height == 0 {
            return;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset + self.viewport_height {
            self.offset = self.selected + 1 - self.viewport_height;
        }
    }

    pub fn page_up(&mut self) {
        let jump = self.viewport_height.max(1) as i64;
        self.move_selection(-jump);
    }

    pub fn page_down(&mut self) {
        let jump = self.viewport_height.max(1) as i64;
        self.move_selection(jump);
    }

    pub fn scroll_to(&mut self, index: usize) {
        if index < self.total_items {
            self.selected = index;
            self.ensure_visible();
        }
    }

    pub fn visible_range(&self) -> Range<usize> {
        let end = (self.offset + self.viewport_height).min(self.total_items);
        self.offset..end
    }
}
