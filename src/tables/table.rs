use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, Entity, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    uniform_list,
};

use super::{
    body::{Body, Line, Press, Select, sideways, sized},
    header::header,
    model::{filtered, page, range},
    rules::{groups, kept, sorted_by},
};
use crate::{
    forms::{CheckState, Editing, check_mark},
    layout::{on_axis, seeded::use_seeded},
    lists::{Pick, picked},
    primitives::tab_stop,
    theme::{ActiveTheme, ControlSize, TextSize},
};

pub(crate) use super::state::View;
pub use super::state::{DataTable, Row};

impl RenderOnce for DataTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let count = self.columns.len();
        for row in self.rows.iter() {
            assert_eq!(
                row.cells.len(),
                count,
                "row {} needs a cell per column",
                row.key
            );
        }
        assert!(
            !(self.detail.is_some() && self.virtualized),
            "a virtualized table draws rows of one height, so rows cannot open"
        );
        assert!(
            !(self.group_by.is_some() && self.page_size.is_some()),
            "a grouped table is not paged"
        );
        let pinned_any = self.columns.iter().any(|column| column.pinned);
        assert!(
            !(pinned_any && (self.virtualized || self.detail.is_some())),
            "pinned columns need rows of one height in one list"
        );
        assert!(
            self.columns
                .iter()
                .all(|column| !column.pinned || column.width.is_some()),
            "a pinned column needs a width"
        );
        let view: Entity<View> =
            window.use_keyed_state((id.clone(), "view"), cx, |_, _| View::default());
        let sorting = use_seeded((id.clone(), "sort"), self.sorts.clone(), window, cx);
        let current = use_seeded((id.clone(), "current"), self.current.clone(), window, cx);
        let keyed = self.on_open.is_some() || self.on_current.is_some();
        let rows_focus = keyed.then(|| tab_stop((id.clone(), "rows").into(), true, window, cx));
        let editor = window.use_keyed_state((id.clone(), "editor"), cx, |_, _| Editing::default());
        let focuses: Vec<Option<FocusHandle>> = (self.columns.iter())
            .map(|column| {
                let key = (id.clone(), format!("head-{}", column.key)).into();
                column.sortable.then(|| tab_stop(key, true, window, cx))
            })
            .collect();
        let narrowest = cx.theme().label_width() * 0.5;
        let rem = window.rem_size();
        let keys_now: Vec<SharedString> = self
            .columns
            .iter()
            .map(|column| column.key.clone())
            .collect();
        view.update(cx, |view, _| {
            view.narrowest = narrowest.to_pixels(rem);
            view.order.retain(|key| keys_now.contains(key));
            for key in &keys_now {
                if !view.order.contains(key) {
                    view.order.push(key.clone());
                }
            }
        });
        if view.read(cx).editing.is_some() && editor.read(cx).field().is_none() {
            view.update(cx, |view, _| view.editing = None);
        }
        let rows = self.rows;
        let columns = Rc::new(self.columns);
        let index = |key: &SharedString| columns.iter().position(|column| column.key == *key);
        let shown_columns: Vec<usize> = view
            .read(cx)
            .order
            .iter()
            .filter(|key| !self.hidden.contains(key))
            .filter_map(index)
            .collect();
        let sort: Vec<(usize, bool)> = sorting
            .read(cx)
            .value
            .iter()
            .filter_map(|(key, rising)| index(key).map(|col| (col, *rising)))
            .collect();
        let mut order = kept(
            &rows,
            &columns,
            filtered(&rows, &self.query),
            &self.filters,
            self.any,
        );
        if !sort.is_empty() {
            order = sorted_by(&rows, order, &sort);
        }
        let total = order.len();
        let (page_at, open, folded) = {
            let view = view.read(cx);
            (view.page, view.open.clone(), view.folded.clone())
        };
        let pages = self.page_size.map(|size| total.div_ceil(size).max(1));
        let page_at = pages.map_or(0, |pages| page_at.min(pages - 1));
        let opens = self.detail.is_some();
        let mut lines = Vec::new();
        let mut ordered: Vec<SharedString> = Vec::new();
        let push_row = |ix: usize, position: usize, lines: &mut Vec<Line>| {
            lines.push(Line::Row { ix, position });
            if opens && open.contains(&rows[ix].key) {
                lines.push(Line::Detail { ix });
            }
        };
        match self.group_by.as_ref().and_then(|key| {
            let col = index(key);
            if col.is_none() {
                log::error!("data table {id:?}: no column {key} to group by; ungrouped");
            }
            col
        }) {
            Some(col) => {
                for (name, members) in groups(&rows, &order, col) {
                    let shut = folded.contains(&name);
                    lines.push(Line::Group {
                        name: name.clone(),
                        count: members.len(),
                        folded: shut,
                    });
                    if !shut {
                        for ix in members {
                            push_row(ix, ordered.len(), &mut lines);
                            ordered.push(rows[ix].key.clone());
                        }
                    }
                }
            }
            None => {
                ordered.extend(order.iter().map(|ix| rows[*ix].key.clone()));
                let offset = self.page_size.map_or(0, |size| page_at * size);
                let visible = match self.page_size {
                    Some(size) => page(&order, page_at, size).to_vec(),
                    None => order.clone(),
                };
                for (at, ix) in visible.into_iter().enumerate() {
                    push_row(ix, offset + at, &mut lines);
                }
            }
        }
        let ordered = Rc::new(ordered);
        let selected = self.selected.map(Rc::new);
        let select: Option<Select> = selected.clone().map(|selected| {
            let (view, ordered, on_select, id) = (
                view.clone(),
                ordered.clone(),
                self.on_select.clone(),
                id.clone(),
            );
            Rc::new(
                move |position: usize, how: Pick, window: &mut Window, cx: &mut App| {
                    let next = picked(&ordered, &selected, view.read(cx).anchor, position, how);
                    view.update(cx, |view, _| {
                        if how != Pick::Range {
                            view.anchor = position;
                        }
                    });
                    log::info!("data table {id:?}: {} selected", next.len());
                    if let Some(on_select) = &on_select {
                        on_select(&next, window, cx);
                    }
                },
            ) as Select
        });
        let scroll_handle = (self.scroll.clone()).unwrap_or_else(|| view.read(cx).scroll.clone());
        let line_of = {
            let lines: Vec<Option<SharedString>> = (lines.iter())
                .map(|line| match line {
                    Line::Row { ix, .. } => Some(rows[*ix].key.clone()),
                    _ => None,
                })
                .collect();
            Rc::new(move |key: &SharedString| lines.iter().position(|at| at.as_ref() == Some(key)))
        };
        // Makes `key` current, reports it, and keeps a long table's row in view.
        let make_current = {
            let (current, on_current, scroll, line_of, virtualized) = (
                current.clone(),
                self.on_current.clone(),
                scroll_handle.clone(),
                line_of.clone(),
                self.virtualized,
            );
            Rc::new(
                move |key: &SharedString, window: &mut Window, cx: &mut App| {
                    current.update(cx, |current, cx| {
                        current.value = Some(key.clone());
                        cx.notify();
                    });
                    if virtualized && let Some(line) = line_of(key) {
                        scroll.scroll_to_item(line, gpui::ScrollStrategy::Nearest);
                    }
                    if let Some(on_current) = &on_current {
                        on_current(key, window, cx);
                    }
                },
            )
        };
        let press: Option<Press> = keyed.then(|| {
            let (ordered, make_current, on_open, focus) = (
                ordered.clone(),
                make_current.clone(),
                self.on_open.clone(),
                rows_focus.clone(),
            );
            Rc::new(move |position: usize, window: &mut Window, cx: &mut App| {
                let Some(key) = ordered.get(position) else {
                    return;
                };
                if let Some(focus) = &focus {
                    window.focus(focus, cx);
                }
                make_current(key, window, cx);
                if let Some(on_open) = &on_open {
                    on_open(key, window, cx);
                }
            }) as Press
        });
        let theme = cx.theme();
        let height = theme.table_row(self.density);
        let widths: Rc<Vec<Option<Pixels>>> = Rc::new(
            columns
                .iter()
                .map(|column| view.read(cx).widths.get(&column.key).copied())
                .collect(),
        );
        let body = Rc::new(Body {
            id: id.clone(),
            view: view.clone(),
            columns: columns.clone(),
            widths: widths.clone(),
            rows: rows.clone(),
            lines: Rc::new(lines),
            selected: selected.clone(),
            select,
            detail: self.detail,
            open: Rc::new(open),
            editing: view.read(cx).editing.clone(),
            editor,
            on_edit: self.on_edit,
            on_action: self.on_action,
            current: current.read(cx).value.clone(),
            press,
            ranges: Rc::new(
                columns
                    .iter()
                    .enumerate()
                    .map(|(col, column)| column.scale.then(|| range(&rows, col)).flatten())
                    .collect(),
            ),
            height,
            lead: theme.control_height(ControlSize::Md),
            narrowest,
        });
        let all = selected.as_ref().map(|selected| {
            match ordered.iter().filter(|key| selected.contains(key)).count() {
                0 => CheckState::Off,
                on if on == ordered.len() => CheckState::On,
                _ => CheckState::Mixed,
            }
        });
        let on_select = self.on_select;
        let heads = |cols: &[usize], lead: bool, cx: &App| {
            let theme = cx.theme();
            div()
                .flex()
                .items_center()
                .h(height)
                .border_b_1()
                .border_color(theme.colors.border)
                .when(lead, |row| {
                    row.when_some(all.zip(selected.clone()), |row, (state, selected)| {
                        let (ordered, on_select) = (ordered.clone(), on_select.clone());
                        row.child(
                            div()
                                .id((id.clone(), "all"))
                                .flex_none()
                                .w(body.lead)
                                .flex()
                                .justify_center()
                                .cursor_pointer()
                                .on_click(move |_, window, cx| {
                                    let next: Vec<SharedString> = if state == CheckState::On {
                                        selected
                                            .iter()
                                            .filter(|key| !ordered.contains(key))
                                            .cloned()
                                            .collect()
                                    } else {
                                        let mut next = (*selected).clone();
                                        next.extend(
                                            ordered
                                                .iter()
                                                .filter(|key| !selected.contains(key))
                                                .cloned(),
                                        );
                                        next
                                    };
                                    if let Some(on_select) = &on_select {
                                        on_select(&next, window, cx);
                                    }
                                })
                                .child(check_mark(state, false, false, 0, cx)),
                        )
                    })
                    .when(body.detail.is_some(), |row| {
                        row.child(div().flex_none().w(body.lead))
                    })
                })
                .children(cols.iter().map(|col| {
                    let head = header(
                        &id,
                        &view,
                        &sorting,
                        &columns[*col],
                        focuses[*col].clone(),
                        self.on_sort.clone(),
                        cx,
                    );
                    match widths[*col] {
                        Some(width) => head.w(width).flex_none(),
                        None => sized(head, &columns[*col], narrowest),
                    }
                }))
        };
        let figures = |cols: &[usize], lead: bool, cx: &App| body.figures(&order, cols, lead, cx);
        let slide = view.read(cx).sideways.clone();
        let part = |cols: &[usize], lead: bool, window: &mut Window, cx: &mut App| -> Div {
            let lines: Vec<AnyElement> = (0..body.lines.len())
                .map(|at| body.line(at, cols, lead, window, cx))
                .collect();
            div()
                .flex()
                .flex_col()
                .child(heads(cols, lead, cx))
                .children(lines)
                .children(figures(cols, lead, cx))
        };
        let table: AnyElement = if total == 0 {
            let theme = cx.theme();
            let least = body.least(&shown_columns, true, rem);
            sideways(&id, &slide)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w(least)
                        .child(heads(&shown_columns, true, cx))
                        .child(
                            div()
                                .flex()
                                .justify_center()
                                .py_6()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(theme.colors.fg_subtle)
                                .child(self.empty.unwrap_or_else(|| "Nothing matches.".into())),
                        ),
                )
                .into_any_element()
        } else if self.virtualized {
            let (scroll, count, cols, drawn) = (
                self.scroll
                    .clone()
                    .unwrap_or_else(|| view.read(cx).scroll.clone()),
                body.lines.len(),
                shown_columns.clone(),
                body.clone(),
            );
            let least = body.least(&shown_columns, true, rem);
            sideways(&id, &slide)
                .size_full()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .size_full()
                        .min_w(least)
                        .child(heads(&shown_columns, true, cx))
                        .child(on_axis(
                            uniform_list((id.clone(), "rows"), count, move |range, window, cx| {
                                range
                                    .map(|at| drawn.line(at, &cols, true, window, cx))
                                    .collect()
                            })
                            .track_scroll(&scroll)
                            .flex_1()
                            .min_h_0(),
                        ))
                        .children(figures(&shown_columns, true, cx)),
                )
                .into_any_element()
        } else if pinned_any {
            let (held, moving): (Vec<usize>, Vec<usize>) =
                shown_columns.iter().partition(|col| columns[**col].pinned);
            div()
                .flex()
                .child(part(&held, true, window, cx).flex_none())
                .child(
                    sideways(&id, &slide).flex_1().min_w_0().child(
                        part(&moving, false, window, cx).min_w(body.least(&moving, false, rem)),
                    ),
                )
                .into_any_element()
        } else {
            let least = body.least(&shown_columns, true, rem);
            sideways(&id, &slide)
                .child(part(&shown_columns, true, window, cx).min_w(least))
                .into_any_element()
        };
        let pager = pages.filter(|pages| *pages > 1).map(|pages| {
            let size = self.page_size.expect("pages come from a page size");
            super::foot::pager(&id, &view, page_at, size, total, pages)
        });
        let base = self
            .base
            .id((id.clone(), "table"))
            .role(gpui::Role::Table)
            .when_some(self.label, |base, label| base.aria_label(label))
            .flex()
            .flex_col();
        let base = match rows_focus {
            Some(focus) => {
                let (ordered, current, on_open) = (ordered.clone(), current.clone(), self.on_open);
                let watch = focus.clone();
                base.track_focus(&focus)
                    .on_key_down(move |event, window, cx| {
                        if !watch.is_focused(window) || event.keystroke.modifiers.modified() {
                            return;
                        }
                        let now = current.read(cx).value.clone();
                        let at = now
                            .as_ref()
                            .and_then(|key| ordered.iter().position(|k| k == key));
                        let last = ordered.len().saturating_sub(1);
                        let next = match event.keystroke.key.as_str() {
                            "down" | "j" => at.map_or(0, |at| (at + 1).min(last)),
                            "up" | "k" => at.map_or(0, |at| at.saturating_sub(1)),
                            "enter" => {
                                if let (Some(key), Some(on_open)) = (now, &on_open) {
                                    cx.stop_propagation();
                                    on_open(&key, window, cx);
                                }
                                return;
                            }
                            _ => return,
                        };
                        cx.stop_propagation();
                        if let Some(key) = ordered.get(next) {
                            make_current(key, window, cx);
                        }
                    })
            }
            None => base,
        };
        base.child(table).children(pager)
    }
}
