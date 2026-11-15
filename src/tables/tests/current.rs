use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext, Context, IntoElement, Modifiers, Render, SharedString, Styled, TestAppContext,
    VisualTestContext, Window, point, px,
};

use super::settle;
use crate::{
    tables::{Cell, Column, DataTable, Row, SortKey},
    theme::Theme,
};

/// What the table reported: rows made current, rows opened, sorts, and edits.
#[derive(Default)]
struct Heard {
    current: Vec<SharedString>,
    opened: Vec<SharedString>,
    sorts: Vec<Vec<(SharedString, bool)>>,
    edits: Vec<(SharedString, SharedString, SharedString)>,
}

type Ears = Rc<RefCell<Heard>>;

/// Three tasks, 36 tall under a 36 tall header: a box, a title, and a status with choices.
struct Tasks(Ears);

impl Render for Tasks {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let ears = [(); 4].map(|_| self.0.clone());
        let [current, opened, sorts, edits] = ears;
        DataTable::new(
            "tasks",
            [
                Column::new("done", "").width(gpui::rems(2.5)).editable(),
                Column::new("title", "Title"),
                Column::new("status", "Status")
                    .width(gpui::rems(8.))
                    .choices(["open", "doing", "done"]),
            ],
        )
        .rows(
            [("a", "Alpha"), ("b", "Beta"), ("c", "Gamma")]
                .map(|(key, title)| {
                    Row::new(key, [Cell::Check(key == "b"), title.into(), "open".into()])
                })
                .to_vec(),
        )
        .current(Some("a"))
        .on_current(move |key, _, _| current.borrow_mut().current.push(key.clone()))
        .on_open(move |key, _, _| opened.borrow_mut().opened.push(key.clone()))
        .on_sort(move |keys, _, _| {
            let keys = keys
                .iter()
                .map(|SortKey { column, rising }| (column.clone(), *rising));
            sorts.borrow_mut().sorts.push(keys.collect())
        })
        .on_edit(move |row, column, text, _, _| {
            edits
                .borrow_mut()
                .edits
                .push((row.clone(), column.clone(), text.clone()))
        })
        .w(px(500.0))
    }
}

fn tasks(cx: &mut TestAppContext) -> (Ears, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let ears = Ears::default();
    let heard = ears.clone();
    let (_, cx) = cx.add_window_view(|_, _| Tasks(heard));
    settle(cx);
    (ears, cx)
}

fn keys(keys: &str, cx: &mut VisualTestContext) {
    for key in keys.split(' ') {
        cx.simulate_keystrokes(key);
        settle(cx);
    }
}

#[gpui::test]
fn keys_move_the_current_row_and_enter_opens_it(cx: &mut TestAppContext) {
    let (ears, cx) = tasks(cx);
    keys("down", cx);
    assert!(
        ears.borrow().current.is_empty(),
        "keys wait for the table's focus"
    );
    cx.simulate_click(point(px(200.0), px(54.0)), Modifiers::none());
    settle(cx);
    assert_eq!(ears.borrow().opened, ["a"], "a press opens its row");
    keys("down j down up k k enter", cx);
    let heard = ears.borrow();
    assert_eq!(heard.current, ["a", "b", "c", "c", "b", "a", "a"]);
    assert_eq!(heard.opened, ["a", "a"]);
}

#[gpui::test]
fn a_header_press_reports_the_sort(cx: &mut TestAppContext) {
    let (ears, cx) = tasks(cx);
    let title = cx
        .debug_bounds("table-row a")
        .expect("row a draws")
        .origin
        .y
        - px(18.0);
    cx.simulate_click(point(px(200.0), title), Modifiers::none());
    settle(cx);
    cx.simulate_click(point(px(200.0), title), Modifiers::none());
    settle(cx);
    assert_eq!(
        ears.borrow().sorts,
        [vec![("title".into(), true)], vec![("title".into(), false)]]
    );
}

#[gpui::test]
fn a_press_flips_a_box_through_on_edit_and_does_not_open(cx: &mut TestAppContext) {
    let (ears, cx) = tasks(cx);
    cx.simulate_click(point(px(20.0), px(54.0 + 36.0)), Modifiers::none());
    settle(cx);
    let heard = ears.borrow();
    assert_eq!(heard.edits, [("b".into(), "done".into(), "false".into())]);
    assert!(heard.opened.is_empty(), "the box takes the press");
}

struct KeyboardTasks {
    tasks: gpui::Entity<Tasks>,
    focus: gpui::FocusHandle,
}

impl Render for KeyboardTasks {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui::ParentElement;
        crate::primitives::FocusScope::new(&self.focus)
            .root()
            .child(self.tasks.clone())
    }
}

#[gpui::test]
fn a_choice_cell_opens_by_keyboard_and_escape_returns_focus(cx: &mut TestAppContext) {
    let press = |cx: &mut VisualTestContext| {
        keys("enter", cx);
        cx.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").unwrap(),
        });
        settle(cx);
    };
    cx.update(crate::init_for_tests);
    let ears = Ears::default();
    let heard = ears.clone();
    let (_, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        KeyboardTasks {
            tasks: cx.new(|_| Tasks(heard)),
            focus,
        }
    });
    settle(cx);
    keys("tab tab tab tab tab", cx);
    assert!(ears.borrow().opened.is_empty());
    press(cx);
    assert!(
        cx.debug_bounds("menu-row doing").is_some(),
        "Tab reaches the choice trigger"
    );
    assert!(
        ears.borrow().opened.is_empty(),
        "opening choices does not open the row"
    );
    keys("escape", cx);
    assert!(cx.debug_bounds("menu-row doing").is_none());
    let opened = ears.borrow().opened.len();
    press(cx);
    assert!(
        cx.debug_bounds("menu-row doing").is_some(),
        "Escape returns focus to the trigger"
    );
    keys("down", cx);
    press(cx);
    assert_eq!(
        ears.borrow().opened.len(),
        opened,
        "editing never opens the row"
    );
    assert_eq!(ears.borrow().edits.len(), 1);
}

#[gpui::test]
fn a_choice_cell_opens_its_choices_and_edits_with_the_pick(cx: &mut TestAppContext) {
    let (ears, cx) = tasks(cx);
    let cell = cx
        .debug_bounds("table-choices c status")
        .expect("row c draws its status as choices");
    cx.simulate_click(cell.center(), Modifiers::none());
    settle(cx);
    assert!(ears.borrow().opened.is_empty(), "the menu takes the press");
    let doing = cx
        .debug_bounds("menu-row doing")
        .expect("the menu lists doing");
    cx.simulate_mouse_move(doing.center(), None, Modifiers::none());
    cx.simulate_click(doing.center(), Modifiers::none());
    settle(cx);
    assert_eq!(
        ears.borrow().edits,
        [("c".into(), "status".into(), "doing".into())]
    );
}
