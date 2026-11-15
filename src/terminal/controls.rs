use alacritty_terminal::{
    grid::Dimensions,
    index::{Column, Line, Point, Side},
    selection::{Selection, SelectionType},
    term::TermMode,
};
use gpui::Context;

use super::{keys, view::Terminal};

impl Terminal {
    pub fn dimensions(&self) -> (usize, usize) {
        self.size
    }

    pub fn scroll_offset(&self) -> usize {
        self.term.lock().grid().display_offset()
    }

    pub fn selected_text(&self) -> Option<String> {
        self.term.lock().selection_to_string()
    }

    /// Sends host-provided keyboard input and returns to the live screen.
    pub fn send_input(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.input(bytes, cx);
    }

    /// Applies the program's bracketed-paste mode to text provided by the host.
    pub fn paste_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let bracketed = self.term.lock().mode().contains(TermMode::BRACKETED_PASTE);
        self.input(keys::paste(text, bracketed), cx);
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        let mut term = self.term.lock();
        let top = Point::new(Line(-(term.grid().history_size() as i32)), Column(0));
        let bottom = Point::new(Line(self.size.1 as i32 - 1), Column(self.size.0 - 1));
        let mut selection = Selection::new(SelectionType::Lines, top, Side::Left);
        selection.update(bottom, Side::Right);
        term.selection = Some(selection);
        cx.notify();
    }
}
