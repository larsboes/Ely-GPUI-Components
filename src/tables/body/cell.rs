use super::*;
use crate::tables::{Cell, cell::draw, model::tint};
use crate::{
    buttons::Button,
    forms::Input,
    menus::{Menu, MenuItem, menu_under},
    primitives::tab_stop,
};

impl Body {
    pub(super) fn cell(
        &self,
        ix: usize,
        col: usize,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let (row, column) = (&self.rows[ix], &self.columns[col]);
        let cell = &row.cells[col];
        let colors = &cx.theme().colors;
        let editing = self
            .editing
            .as_ref()
            .is_some_and(|(key, at)| *key == row.key && *at == column.key);
        let tinted = self.ranges[col]
            .zip(cell.number())
            .map(|(range, value)| tint(value, range, colors));
        let frame = div()
            .id((self.id.clone(), format!("cell-{}-{col}", row.key)))
            .h_full()
            .flex()
            .items_center()
            .px_3()
            .border_x_1()
            .border_color(transparent_black());
        let frame = match self.widths[col] {
            Some(width) => frame
                .w(width)
                .flex_none()
                .when(column.align == Align::End, |cell| cell.justify_end()),
            None => sized(frame, column, self.narrowest),
        };
        let content = match editing.then(|| self.editor.read(cx).field()).flatten() {
            Some(field) => {
                let editor = self.editor.clone();
                div()
                    .w_full()
                    .on_key_down(move |event, window, cx| {
                        if event.keystroke.key == "escape" {
                            cx.stop_propagation();
                            editor.update(cx, |editing, cx| editing.finish(true, window, cx));
                        }
                    })
                    .child(Input::new(&field).size(ControlSize::Sm))
                    .into_any_element()
            }
            None if !column.actions.is_empty() => self.actions(row, column, cell),
            None if !column.choices.is_empty() && self.on_edit.is_some() => {
                self.choices(row, column, cell, window, cx)
            }
            None => draw(
                cell,
                column,
                (self.id.clone(), format!("draw-{}-{col}", row.key)).into(),
                cx,
            ),
        };
        let check = match cell {
            Cell::Check(on) if column.editable => {
                self.on_edit.clone().map(|on_edit| (*on, on_edit))
            }
            _ => None,
        };
        let texted = column.choices.is_empty() && check.is_none();
        frame
            .when_some(tinted, |cell, bg| cell.bg(bg))
            .when_some(check, |frame, (on, on_edit)| {
                let (key, at) = (row.key.clone(), column.key.clone());
                frame.cursor_pointer().on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    let next = if on { "false" } else { "true" };
                    on_edit(&key, &at, &next.into(), window, cx);
                })
            })
            .when(
                texted && column.editable && !editing && self.on_edit.is_some(),
                |frame| {
                    let (view, editor, on_edit) = (
                        self.view.clone(),
                        self.editor.clone(),
                        self.on_edit.clone().expect("checked"),
                    );
                    let (key, at, words) = (row.key.clone(), column.key.clone(), cell.words());
                    frame.cursor_text().on_click(move |event, window, cx| {
                        if event.click_count() != 2 {
                            return;
                        }
                        cx.stop_propagation();
                        let (key, at, on_edit) = (key.clone(), at.clone(), on_edit.clone());
                        view.update(cx, |view, _| view.editing = Some((key.clone(), at.clone())));
                        editor.update(cx, |editing, _| {
                            editing.on_commit = Some(Rc::new(move |text, window, cx| {
                                on_edit(&key, &at, text, window, cx)
                            }))
                        });
                        Editing::begin(&editor, words.to_string(), window, cx);
                    })
                },
            )
            .child(content)
            .into_any_element()
    }

    /// The cell drawn as a trigger that opens `column`'s choices, the current one marked; a pick goes to `on_edit`.
    fn choices(
        &self,
        row: &Row,
        column: &Column,
        cell: &Cell,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let (now, on_edit) = (cell.words(), self.on_edit.clone().expect("checked"));
        let menu = column.choices.iter().fold(Menu::new(), |menu, choice| {
            let (key, at, choice, on_edit) = (
                row.key.clone(),
                column.key.clone(),
                choice.clone(),
                on_edit.clone(),
            );
            menu.item(
                MenuItem::radio(choice.clone(), choice == now)
                    .on_click(move |window, cx| on_edit(&key, &at, &choice, window, cx)),
            )
        });
        let id: ElementId = (
            self.id.clone(),
            format!("choices-{}-{}", row.key, column.key),
        )
            .into();
        let drawn = draw(cell, column, (id.clone(), "draw").into(), cx);
        let focus = tab_stop((id.clone(), "trigger").into(), true, window, cx);
        let label = format!("{}: {}", column.title, now);
        let ring = cx.theme().colors.focus;
        let named = format!("table-choices {} {}", row.key, column.key);
        menu_under(
            id.clone(),
            &menu,
            |click| {
                div()
                    .id((id, "trigger"))
                    .debug_selector(move || named.clone())
                    .track_focus(&focus)
                    .role(gpui::Role::Button)
                    .aria_label(label)
                    .border_1()
                    .border_color(transparent_black())
                    .focus(move |style| style.border_color(ring))
                    .min_w_0()
                    .cursor_pointer()
                    .on_click(move |event, window, cx| {
                        cx.stop_propagation();
                        click(event, window, cx)
                    })
                    .child(drawn)
                    .into_any_element()
            },
            window,
            cx,
        )
        .min_w_0()
        .into_any_element()
    }

    /// The buttons `cell` offers in actions column `column`; a press goes to `on_action`, not to the row.
    fn actions(&self, row: &Row, column: &Column, cell: &Cell) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap_1()
            .children(column.offered(cell).map(|action| {
                let (key, at, on_action) =
                    (row.key.clone(), action.key.clone(), self.on_action.clone());
                let named = format!("table-action {} {}", row.key, action.key);
                let button = Button::new(
                    (
                        self.id.clone(),
                        format!("action-{}-{}", row.key, action.key),
                    ),
                    action.label.clone(),
                )
                .variant(action.variant)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match &on_action {
                        Some(on_action) => on_action(&key, &at, window, cx),
                        None => log::warn!("data table: action {at} on {key} has no on_action"),
                    }
                });
                div().debug_selector(move || named.clone()).child(button)
            }))
            .into_any_element()
    }
}
