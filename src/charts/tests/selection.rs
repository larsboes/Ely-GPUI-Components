use super::*;

struct Selection {
    kind: usize,
    heard: Rc<RefCell<Vec<usize>>>,
}

impl Render for Selection {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let heard = self.heard.clone();
        let select = move |ix, _: &mut Window, _: &mut gpui::App| heard.borrow_mut().push(ix);
        let chart = match self.kind {
            0 => BarChart::new("selected-bars", ["small", "large"])
                .series(Series::new("values", [1., 3.]))
                .horizontal()
                .on_select(select)
                .into_any_element(),
            1 => PieChart::new("selected-pie")
                .slice("small", 1.)
                .slice("large", 3.)
                .donut("4", "total")
                .on_select(select)
                .into_any_element(),
            2 => Treemap::new("selected-tiles")
                .tile("small", 1.)
                .tile("large", 3.)
                .on_select(select)
                .into_any_element(),
            _ => BarChart::new("selected-zoom", ["a", "b", "c"])
                .series(Series::new("values", [1., 2., 3.]))
                .zoom()
                .on_select(select)
                .into_any_element(),
        };
        div().w(px(400.)).h(px(240.)).child(chart)
    }
}

#[gpui::test]
fn brushing_does_not_select_a_category(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Selection {
        kind: 3,
        heard: seen,
    });
    settle(cx);
    let bounds = cx.debug_bounds("chart-root").unwrap();
    let start = bounds.origin + point(px(80.), px(60.));
    let end = bounds.origin + point(px(350.), px(60.));
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(end, gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(heard.borrow().is_empty(), "a zoom brush does not filter");
    cx.simulate_click(start, Modifiers::none());
    settle(cx);
    assert_eq!(*heard.borrow(), vec![0], "a later click still selects");
}

#[gpui::test]
fn clicks_select_original_indices_without_hover_and_ignore_outside(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
    });
    for kind in 0..3 {
        let heard = Rc::new(RefCell::new(Vec::new()));
        let seen = heard.clone();
        let (_, cx) = cx.add_window_view(|_, _| Selection { kind, heard: seen });
        settle(cx);
        let bounds = cx.debug_bounds("chart-root").expect("the chart draws");
        for y in (10..200).step_by(20) {
            for x in (60..380).step_by(20) {
                cx.simulate_click(
                    bounds.origin + point(px(x as f32), px(y as f32)),
                    Modifiers::none(),
                );
                settle(cx);
            }
        }
        assert!(
            heard.borrow().contains(&0),
            "chart {kind}: first part selects"
        );
        assert!(
            heard.borrow().contains(&1),
            "chart {kind}: second part selects"
        );
        assert!(heard.borrow().iter().all(|ix| *ix < 2));
        let before = heard.borrow().len();
        cx.simulate_click(bounds.origin - point(px(10.), px(10.)), Modifiers::none());
        settle(cx);
        assert_eq!(heard.borrow().len(), before, "outside selects nothing");
        if kind == 2 {
            heard.borrow_mut().clear();
            cx.simulate_click(bounds.origin + point(px(50.), px(100.)), Modifiers::none());
            assert_eq!(
                *heard.borrow(),
                vec![1],
                "largest tile keeps its input index after sorting"
            );
        }
    }
}
