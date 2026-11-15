use std::{cell::RefCell, rc::Rc};

use gpui::{
    Context, IntoElement, Modifiers, Render, SharedString, Styled, TestAppContext, Window, px,
};

use super::settle;
use crate::{
    tables::{Cell, Column, DataTable, Row, RowAction},
    theme::Theme,
};

type Seen = Rc<RefCell<Vec<(SharedString, SharedString)>>>;

/// Two selectable rows with an actions column; it keeps the presses and the selection.
struct Acts(Seen, Rc<RefCell<Vec<SharedString>>>);

impl Render for Acts {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (presses, picks) = (self.0.clone(), self.1.clone());
        DataTable::new(
            "acts",
            [
                Column::new("name", "Name"),
                Column::new("do", "").actions([
                    RowAction::new("confirm", "Confirm").primary(),
                    RowAction::new("reject", "Reject"),
                ]),
            ],
        )
        .rows(vec![
            Row::new("a", ["Alpha".into(), Cell::Empty]),
            Row::new("b", ["Beta".into(), Cell::Actions(vec!["reject".into()])]),
        ])
        .selected(Vec::<SharedString>::new())
        .on_select(move |keys, _, _| *picks.borrow_mut() = keys.to_vec())
        .on_action(move |row, action, _, _| {
            presses.borrow_mut().push((row.clone(), action.clone()))
        })
        .w(px(500.0))
    }
}

#[gpui::test]
fn a_press_on_an_action_reports_its_row_and_key_and_does_not_select(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (presses, picks) = (Seen::default(), Rc::new(RefCell::new(Vec::new())));
    let (_, cx) = cx.add_window_view({
        let (presses, picks) = (presses.clone(), picks.clone());
        |_, _| Acts(presses, picks)
    });
    settle(cx);
    assert!(
        cx.debug_bounds("table-action b confirm").is_none(),
        "row b offers only reject"
    );
    for selector in [
        "table-action a confirm",
        "table-action a reject",
        "table-action b reject",
    ] {
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("draws {selector}"));
        cx.simulate_click(bounds.center(), Modifiers::none());
        settle(cx);
    }
    let seen: Vec<(String, String)> = presses
        .borrow()
        .iter()
        .map(|(row, action)| (row.to_string(), action.to_string()))
        .collect();
    assert_eq!(
        seen,
        [("a", "confirm"), ("a", "reject"), ("b", "reject")]
            .map(|(row, action)| (row.to_string(), action.to_string()))
    );
    assert!(
        picks.borrow().is_empty(),
        "an action press does not select its row"
    );
}

struct Subtitles(bool);

impl Render for Subtitles {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let table = DataTable::new(
            "subtitles",
            [
                Column::new("name", "Name"),
                Column::new("actions", "")
                    .width(gpui::rems(7.))
                    .actions([RowAction::new("edit", "Edit")]),
            ],
        )
        .rows(vec![
            Row::new(
                "short",
                [Cell::Lines("Short".into(), "A note".into()), Cell::Empty],
            ),
            Row::new(
                "long",
                [
                    Cell::Lines(
                        "Long".into(),
                        "A long item subtitle that must stay in the name column. "
                            .repeat(8)
                            .into(),
                    ),
                    Cell::Empty,
                ],
            ),
        ])
        .on_action(|_, _, _, _| {})
        .w(px(500.))
        .h(px(200.));
        if self.0 { table.virtualized() } else { table }
    }
}

#[gpui::test]
fn long_subtitles_keep_row_actions_in_their_column(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for virtualized in [false, true] {
        let (_, cx) = cx.add_window_view(|_, _| Subtitles(virtualized));
        settle(cx);
        let short = cx.debug_bounds("table-action short edit").unwrap();
        let long = cx.debug_bounds("table-action long edit").unwrap();
        assert_eq!(short.left(), long.left(), "virtualized={virtualized}");
        assert!(
            long.right() <= px(500.),
            "the action stays inside the table"
        );
    }
}
