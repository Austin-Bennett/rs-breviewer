use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Stylize, Widget};
use ratatui::style::Color;
use ratatui::widgets::Block;
use ratatui_textarea::TextArea;
use crate::ui::{set_cursor_visible, HOVER_COLOR};

// size of a single review's cell in the grid, including its border. Chosen so
// a reasonably sized terminal fits at least 24 without scrolling; anything
// smaller just scrolls.
const CELL_WIDTH: u16 = 20;
const CELL_HEIGHT: u16 = 4;

// default border color for a review's cell; overridden by HOVER_COLOR when
// it's the highlighted one
const REVIEW_COLOR: Color = Color::Rgb(255, 165, 0);

pub struct Review<'a> {
    pub status: String,
    pub message: TextArea<'a>,
}

impl<'a> Review<'a> {
    pub fn new(status: String) -> Self {
        let mut area = TextArea::default();

        set_cursor_visible(&mut area, false);

        Self { status, message: area }
    }
}

pub struct Reviews<'a> {
    inputs: HashMap<char, String>,
    pub reviews: Vec<Review<'a>>,
    block: Block<'a>,
    // index of the first grid row currently visible
    scroll: usize,
    // index of the review currently highlighted for Left/Right navigation
    hovered: usize,
    // whether this widget is the one currently receiving input
    active: bool,
    // whether the hovered review's message text area is the one currently
    // receiving input, as opposed to Left/Right/Up/Down navigating the grid
    editing: bool,
}

impl<'a> Reviews<'a> {
    pub fn new(mut reviews: Vec<Review<'a>>, inputs: HashMap<char, String>, block: Block<'a>) -> Self {
        for review in &mut reviews {
            set_cursor_visible(&mut review.message, false);
        }

        Self {
            inputs,
            reviews,
            block,
            scroll: 0,
            hovered: 0,
            active: false,
            editing: false,
        }
    }

    pub fn block(&self) -> &Block<'a> {
        &self.block
    }

    pub fn set_block(&mut self, block: Block<'a>) {
        self.block = block;
    }

    pub fn select(&mut self) {
        self.active = true;
    }

    pub fn deselect(&mut self) {
        self.active = false;
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    // Returns true if Reviews should no longer be selected: only happens on
    // Esc when no inner review is being edited. While editing a review's
    // message, keys go to its text area instead of moving the grid highlight
    // or scrolling, and Esc there just stops editing that one review rather
    // than deselecting Reviews itself.
    pub fn handle_input(&mut self, key: KeyEvent) -> bool {
        if self.editing {
            // Tab has no meaning inside a review's message, so - like Esc -
            // it just stops editing that one review rather than being typed.
            if key.code == KeyCode::Esc || key.code == KeyCode::Tab {
                set_cursor_visible(&mut self.reviews[self.hovered].message, false);
                self.editing = false;
            } else {
                self.reviews[self.hovered].message.input(key);
            }

            return false;
        }

        match key.code {
            KeyCode::Left => self.hovered = self.hovered.saturating_sub(1),
            KeyCode::Right => self.hovered = (self.hovered + 1).min(self.reviews.len().saturating_sub(1)),
            KeyCode::Up => self.scroll_up(),
            KeyCode::Down => self.scroll_down(),
            KeyCode::Tab => self.tab_next(),
            KeyCode::Enter if !self.reviews.is_empty() => {
                self.editing = true;
                set_cursor_visible(&mut self.reviews[self.hovered].message, true);
            }
            KeyCode::Backspace if !self.reviews.is_empty() => {
                self.reviews.remove(self.hovered);
                self.hovered = self.hovered.min(self.reviews.len().saturating_sub(1));
            }
            KeyCode::Esc => return true,
            KeyCode::Char(c) => {
                let c = c.to_ascii_lowercase();

                if let Some(status) = self.inputs.get(&c) {

                    self.reviews.push(Review::new(status.clone()))
                }
            }
            _ => {}
        }

        false
    }

    // moves the hovered review to the "next" one in reading order. Reviews
    // render in row-major order (see Widget::render below), so the "right,
    // else down-and-leftmost, else wrap to up-and-leftmost" rule used
    // elsewhere reduces to just advancing the flat index and wrapping at
    // the end.
    fn tab_next(&mut self) {
        if !self.reviews.is_empty() {
            self.hovered = (self.hovered + 1) % self.reviews.len();
        }
    }

    // how many grid columns/rows fit in an area this wide/tall
    fn grid_dims(inner: Rect) -> (usize, usize) {
        let columns = (inner.width / CELL_WIDTH).max(1) as usize;
        let visible_rows = (inner.height / CELL_HEIGHT).max(1) as usize;
        (columns, visible_rows)
    }
}

impl<'a> Widget for &Reviews<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized
    {
        (&self.block).render(area, buf);

        let inner = self.block.inner(area);
        let (columns, visible_rows) = Reviews::grid_dims(inner);

        let total_rows = self.reviews.len().div_ceil(columns);
        let max_scroll = total_rows.saturating_sub(visible_rows);
        let scroll = self.scroll.min(max_scroll);

        for (i, review) in self.reviews.iter().enumerate() {
            let row = i / columns;
            if row < scroll || row >= scroll + visible_rows {
                continue;
            }
            let col = i % columns;

            let cell = Rect {
                x: inner.x + col as u16 * CELL_WIDTH,
                y: inner.y + (row - scroll) as u16 * CELL_HEIGHT,
                width: CELL_WIDTH,
                height: CELL_HEIGHT,
            }.intersection(inner);

            let title = format!("VM{} {}", i + 1, review.status);
            let cell_block = if i == self.hovered && self.active {
                Block::bordered().fg(HOVER_COLOR).title(title)
            } else {
                Block::bordered().fg(REVIEW_COLOR).title(title)
            };
            let message_area = cell_block.inner(cell);

            cell_block.render(cell, buf);
            (&review.message).render(message_area, buf);
        }
    }
}