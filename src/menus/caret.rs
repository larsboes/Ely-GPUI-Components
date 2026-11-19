use std::{panic::Location, rc::Rc};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, Entity, GlobalElementId,
    InspectorElementId, LayoutId, Pixels, Position, ScrollHandle, Style, Window, size,
};

use crate::forms::{Choice, Pick, Popup, Run, reveal};

type Anchor = Rc<dyn Fn(&mut Window, &mut App) -> Option<Bounds<Pixels>>>;

#[derive(Default)]
struct State {
    scroll: ScrollHandle,
    revealed: Option<usize>,
}

/// A host-controlled, focus-neutral menu; place it after the input to read this frame's caret anchor in prepaint.
pub struct CaretMenu {
    id: ElementId,
    rows: Vec<Choice>,
    selected: Option<usize>,
    anchor: Anchor,
    pick: Pick,
    dismiss: Run,
    state: Option<Entity<State>>,
}

impl CaretMenu {
    pub fn new(
        id: impl Into<ElementId>,
        rows: impl IntoIterator<Item = Choice>,
        anchor: impl Fn(&mut Window, &mut App) -> Option<Bounds<Pixels>> + 'static,
        pick: impl Fn(usize, &mut Window, &mut App) + 'static,
        dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            rows: rows.into_iter().collect(),
            selected: None,
            anchor: Rc::new(anchor),
            pick: Rc::new(pick),
            dismiss: Rc::new(dismiss),
            state: None,
        }
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.selected = selected;
        self
    }
}

#[cfg(not(feature = "gpui-fast"))]
impl gpui::IntoElement for CaretMenu {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for CaretMenu {
    type RequestLayoutState = ();
    type PrepaintState = Option<AnyElement>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        self.state = Some(
            window.use_keyed_state((self.id.clone(), "caret-scroll"), cx, |_, _| {
                State::default()
            }),
        );
        (
            window.request_layout(
                Style {
                    position: Position::Absolute,
                    ..Style::default()
                },
                [],
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let anchor = (self.anchor)(window, cx);
        let state = self
            .state
            .as_ref()
            .expect("a caret menu lays out before prepaint");
        let scroll = state.read(cx).scroll.clone();
        let selected = self.selected.filter(|&at| at < self.rows.len());
        let reveal = reveal(
            state,
            |state| &mut state.revealed,
            anchor.is_some() && selected.is_some(),
            selected.unwrap_or_default(),
            cx,
        );
        let anchor = anchor?;
        if self.rows.is_empty() {
            return None;
        }
        let mut popup = Popup {
            id: self.id.clone(),
            anchor,
            rows: &self.rows,
            highlighted: selected,
            checked: None,
            pick: self.pick.clone(),
            dismiss: Some(self.dismiss.clone()),
            scroll: Some(&scroll),
            reveal,
        }
        .render(window, cx);
        let viewport = window.viewport_size();
        popup.prepaint_as_root(
            gpui::Point::default(),
            size(
                AvailableSpace::Definite(viewport.width),
                AvailableSpace::Definite(viewport.height),
            ),
            window,
            cx,
        );
        Some(popup)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        popup: &mut Option<AnyElement>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(popup) = popup {
            popup.paint(window, cx);
        }
    }
}
