use gpui::{
    Context, IntoElement, Render, ScrollStrategy, Styled, TestAppContext, UniformListScrollHandle,
    Window, px,
};

use super::settle;
use crate::{
    tables::{Column, DataTable, Row},
    theme::Theme,
};

/// A hundred rows in a virtualized table 200 tall, on a scroll the view owns.
struct Long(UniformListScrollHandle);

impl Render for Long {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        DataTable::new("long", [Column::new("n", "N")])
            .rows(
                (0..100)
                    .map(|n| Row::new(format!("r{n}"), [format!("{n}").into()]))
                    .collect::<Vec<_>>(),
            )
            .virtualized()
            .track_scroll(&self.0)
            .h(px(200.0))
    }
}

#[gpui::test]
fn the_app_scrolls_a_long_table_to_a_row(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let handle = UniformListScrollHandle::new();
    let owned = handle.clone();
    let (_, cx) = cx.add_window_view(|_, _| Long(owned));
    settle(cx);
    let down = || -handle.0.borrow().base_handle.offset().y;
    assert_eq!(down(), px(0.0));
    handle.scroll_to_item(60, ScrollStrategy::Top);
    settle(cx);
    assert!(down() > px(1000.0), "row 60 is far down: {:?}", down());
}

#[test]
fn an_empty_table_says_its_own_words() {
    let table = DataTable::new("bare", [Column::new("n", "N")]);
    assert_eq!(table.empty, None, "Ely's own words by default");
    let table = table.empty("No transactions match.");
    assert_eq!(table.empty.as_deref(), Some("No transactions match."));
}
