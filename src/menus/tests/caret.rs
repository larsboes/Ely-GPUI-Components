use std::{cell::Cell, rc::Rc};

use crate::{
    forms::{Choice, Input, TextInput},
    menus::CaretMenu,
    theme::Theme,
};
use gpui::{
    AppContext, Bounds, Context, Entity, Focusable, IntoElement, MouseButton, Pixels, Render,
    TestAppContext, VisualTestContext, Window, canvas, div, point, prelude::*, px, size,
};

struct Host {
    input: Entity<TextInput>,
    fresh: Rc<Cell<Bounds<Pixels>>>,
    anchor: Bounds<Pixels>,
    open: bool,
    selected: Option<usize>,
    picked: Option<usize>,
    rows: usize,
    raised: bool,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let anchor = self.anchor;
        let fresh = self.fresh.clone();
        let resolve = fresh.clone();
        let owner = cx.entity();
        let dismiss = owner.clone();
        let rows = (0..self.rows).map(|ix| {
            let choice = Choice::new(format!("{ix}"), format!("Choice {ix}"));
            if ix == 1 { choice.disabled() } else { choice }
        });
        let root = div()
            .size_full()
            .child(Input::new(&self.input))
            .child(
                canvas(move |_, _, _| fresh.set(anchor), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .when(self.open, |root| {
                root.child(
                    CaretMenu::new(
                        "caret-test",
                        rows,
                        move |_, _| Some(resolve.get()),
                        move |at, _, cx| {
                            owner.update(cx, |host, cx| {
                                host.picked = Some(at);
                                host.open = false;
                                cx.notify();
                            })
                        },
                        move |_, cx| {
                            dismiss.update(cx, |host, cx| {
                                host.open = false;
                                cx.notify();
                            })
                        },
                    )
                    .selected(self.selected),
                )
            });
        if self.raised {
            crate::primitives::raise("parent-layer", root).into_any_element()
        } else {
            root.into_any_element()
        }
    }
}

fn host(cx: &mut TestAppContext) -> (Entity<Host>, &mut VisualTestContext) {
    cx.update(Theme::init);
    let (host, cx) = cx.add_window_view(|window, cx| Host {
        input: cx.new(|cx| TextInput::new(window, cx)),
        fresh: Rc::new(Cell::new(Bounds::default())),
        anchor: Bounds::new(point(px(200.), px(200.)), size(px(1.), px(20.))),
        open: true,
        selected: Some(0),
        picked: None,
        rows: 3,
        raised: false,
    });
    cx.run_until_parked();
    (host, cx)
}

#[gpui::test]
fn pointer_choices_leave_typing_in_the_host_input(cx: &mut TestAppContext) {
    let (host, cx) = host(cx);
    cx.update(|window, cx| {
        host.read(cx)
            .input
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx)
    });
    cx.simulate_input("before");
    cx.run_until_parked();
    let row = cx.debug_bounds("option-row 0").unwrap();
    cx.simulate_mouse_down(row.center(), MouseButton::Left, gpui::Modifiers::default());
    cx.simulate_mouse_up(row.center(), MouseButton::Left, gpui::Modifiers::default());
    cx.run_until_parked();
    assert_eq!(host.read_with(cx, |host, _| host.picked), Some(0));
    cx.update(|window, cx| {
        assert!(
            host.read(cx)
                .input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        )
    });
    cx.simulate_input("after");
    cx.run_until_parked();
    host.read_with(cx, |host, cx| {
        assert_eq!(host.input.read(cx).text(), "beforeafter")
    });
}

#[gpui::test]
fn fresh_anchor_flips_above_the_bottom_edge(cx: &mut TestAppContext) {
    let (host, cx) = host(cx);
    let anchor = host.read_with(cx, |host, _| host.anchor);
    let list = cx.debug_bounds("option-list").unwrap();
    assert!(list.top() >= anchor.bottom(), "{list:?} vs {anchor:?}");
    let viewport = cx.update(|window, _| window.viewport_size());
    let low = Bounds::new(
        point(viewport.width - px(2.), viewport.height - px(20.)),
        size(px(1.), px(18.)),
    );
    host.update(cx, |host, cx| {
        host.anchor = low;
        cx.notify();
    });
    super::settle(cx);
    let list = cx.debug_bounds("option-list").unwrap();
    assert!(list.bottom() <= low.top() + px(1.), "{list:?} vs {low:?}");
    assert!(list.right() <= viewport.width);
}

#[gpui::test]
fn disabled_rows_do_not_choose_and_outside_press_dismisses(cx: &mut TestAppContext) {
    let (host, cx) = host(cx);
    let row = cx.debug_bounds("option-row 1").unwrap();
    cx.simulate_mouse_down(row.center(), MouseButton::Left, gpui::Modifiers::default());
    cx.simulate_mouse_up(row.center(), MouseButton::Left, gpui::Modifiers::default());
    cx.run_until_parked();
    host.read_with(cx, |host, _| {
        assert!(host.open);
        assert_eq!(host.picked, None);
    });
    cx.simulate_mouse_down(
        point(px(5.), px(100.)),
        MouseButton::Left,
        gpui::Modifiers::default(),
    );
    cx.run_until_parked();
    assert!(!host.read_with(cx, |host, _| host.open));
}

#[gpui::test]
fn clearing_selection_then_restoring_it_reveals_again(cx: &mut TestAppContext) {
    let (host, cx) = host(cx);
    host.update(cx, |host, cx| {
        host.rows = 24;
        host.selected = Some(23);
        cx.notify();
    });
    super::settle(cx);
    host.update(cx, |host, cx| {
        host.rows = 3;
        host.selected = None;
        cx.notify();
    });
    super::settle(cx);
    host.update(cx, |host, cx| {
        host.rows = 24;
        host.selected = Some(23);
        cx.notify();
    });
    super::settle(cx);
    let list = cx.debug_bounds("option-list").unwrap();
    let row = cx.debug_bounds("option-row 23").unwrap();
    assert!(
        row.top() >= list.top() && row.bottom() <= list.bottom(),
        "{row:?} outside {list:?}"
    );
}

#[gpui::test]
fn pointer_choices_work_inside_a_raised_parent(cx: &mut TestAppContext) {
    let (host, cx) = host(cx);
    host.update(cx, |host, cx| {
        host.raised = true;
        cx.notify();
    });
    super::settle(cx);
    cx.update(|window, cx| {
        host.read(cx)
            .input
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx)
    });
    let row = cx.debug_bounds("option-row 0").unwrap();
    cx.simulate_mouse_down(row.center(), MouseButton::Left, gpui::Modifiers::default());
    cx.simulate_mouse_up(row.center(), MouseButton::Left, gpui::Modifiers::default());
    super::settle(cx);
    assert_eq!(host.read_with(cx, |host, _| host.picked), Some(0));
    cx.update(|window, cx| {
        assert!(
            host.read(cx)
                .input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        )
    });
}

#[gpui::test]
fn external_selection_scrolls_into_view(cx: &mut TestAppContext) {
    let (host, cx) = host(cx);
    host.update(cx, |host, cx| {
        host.rows = 24;
        host.selected = Some(23);
        cx.notify();
    });
    super::settle(cx);
    let list = cx.debug_bounds("option-list").unwrap();
    let row = cx.debug_bounds("option-row 23").unwrap();
    assert!(
        row.top() >= list.top() && row.bottom() <= list.bottom(),
        "{row:?} outside {list:?}"
    );
}
