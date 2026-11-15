use super::*;
use crate::calendar::hours::snap;
use gpui::{Div, DragMoveEvent, EmptyView, MouseButton, Stateful};

#[derive(Clone)]
pub(in crate::calendar) struct Held {
    owner: EntityId,
    event: Event,
    resize: bool,
}

#[derive(Clone)]
pub(super) struct Target {
    day: Date,
    minute: i64,
    held: Held,
}

impl Target {
    fn times(&self, zone: &TimeZone) -> Option<(Timestamp, Timestamp)> {
        let When::Timed { start, end } = self.held.event.when else {
            return None;
        };
        let at = try_moment(self.day, self.minute, zone)?;
        let pair = if self.held.resize {
            (start, at)
        } else {
            let before = start.to_zoned(zone.clone()).datetime();
            let after = end.to_zoned(zone.clone()).datetime();
            let duration = after - before;
            let last = at
                .to_zoned(zone.clone())
                .datetime()
                .checked_add(duration)
                .ok()?;
            (at, zone.to_ambiguous_timestamp(last).unambiguous().ok()?)
        };
        (pair.1 > pair.0).then_some(pair)
    }
}

impl Frame<'_> {
    pub(super) fn movable(&self, card: EventCard, event: &Event, grid: &TimeGrid) -> AnyElement {
        let mut card = card;
        if grid.on_move.is_some() {
            card.drag = Some(Held {
                owner: self.owner,
                event: event.clone(),
                resize: false,
            });
        }
        card.into_any_element()
    }

    pub(super) fn resize_handle(
        &self,
        day: Date,
        event: &Event,
        grid: &TimeGrid,
    ) -> Option<AnyElement> {
        let When::Timed { end, .. } = event.when else {
            return None;
        };
        if grid.on_resize.is_none() || end.to_zoned(self.zone.clone()).date() != day {
            return None;
        }
        let key = event.key.clone();
        Some(
            div()
                .id((self.id.clone(), format!("end-{key}-{day}")))
                .debug_selector(move || format!("end {key} {day}"))
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .h_1p5()
                .cursor_row_resize()
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_drag(
                    Held {
                        owner: self.owner,
                        event: event.clone(),
                        resize: true,
                    },
                    |_, _, _, cx| cx.new(|_| EmptyView),
                )
                .into_any_element(),
        )
    }
}

impl TimeGrid {
    pub(super) fn drop_target(
        &self,
        pad: Stateful<Div>,
        frame: &Frame,
        day: Date,
        drawing: &Entity<Drawing>,
    ) -> Stateful<Div> {
        let (owner, hour, target, column) = (
            frame.owner,
            frame.hour,
            frame.target.clone(),
            drawing.clone(),
        );
        let moving = target.clone();
        let zone = frame.zone.clone();
        let (on_move, on_resize) = (self.on_move.clone(), self.on_resize.clone());
        pad.on_drag_move(move |event: &DragMoveEvent<Held>, _, cx| {
            if event.drag(cx).owner != owner
                || !column.read(cx).bounds.contains(&event.event.position)
            {
                return;
            }
            let minute =
                snap((event.event.position.y - column.read(cx).bounds.top()) / hour * 60.0);
            moving.update(cx, |target, cx| {
                *target = Some(Target {
                    day,
                    minute,
                    held: event.drag(cx).clone(),
                });
                cx.notify();
            });
        })
        .on_drop(move |held: &Held, window, cx| {
            if held.owner != owner {
                return;
            }
            let value = target.update(cx, |target, cx| {
                cx.notify();
                target.take()
            });
            let Some(value) = value.filter(|v| v.day == day && v.held.event.key == held.event.key)
            else {
                return;
            };
            let Some((start, end)) = value.times(&zone) else {
                log::warn!("calendar grid: rejected invalid local drop time");
                return;
            };
            if held.resize {
                if let Some(handler) = &on_resize {
                    handler(&held.event.key, end, window, cx);
                }
            } else if let Some(handler) = &on_move {
                handler(&held.event.key, start, end, window, cx);
            }
        })
    }

    pub(super) fn preview(&self, frame: &Frame, day: Date, cx: &App) -> Option<AnyElement> {
        let target = frame.target.read(cx).as_ref()?;
        if target.day != day || target.times(frame.zone).is_none() {
            return None;
        }
        Some(
            div()
                .debug_selector(|| "calendar-drop-preview".into())
                .absolute()
                .top(frame.y(target.minute))
                .left_0()
                .right_0()
                .h(frame.hour / 2.0)
                .border_2()
                .border_color(cx.theme().colors.accent)
                .bg(cx.theme().colors.accent.alpha(0.12))
                .into_any_element(),
        )
    }

    pub(super) fn slot_click(
        &self,
        pad: Stateful<Div>,
        frame: &Frame,
        day: Date,
        drawing: &Entity<Drawing>,
    ) -> Stateful<Div> {
        let Some(handler) = self.on_slot.clone() else {
            return pad;
        };
        let (column, zone, hour) = (drawing.clone(), frame.zone.clone(), frame.hour);
        pad.on_click(move |event, window, cx| {
            let minute =
                snap((event.position().y - column.read(cx).bounds.top()) / hour * 60.0).min(1425);
            if let (Some(start), Some(end)) = (
                try_moment(day, minute, &zone),
                try_moment(day, minute + 60, &zone),
            ) {
                handler(start, end, window, cx);
            }
        })
    }

    pub(super) fn free_slots(&self, frame: &Frame, day: Date, cx: &App) -> Vec<AnyElement> {
        self.free
            .iter()
            .filter(|free| free.day == day && free.start < free.end && free.end <= 1440)
            .map(|free| {
                let (start, end, zone, handler) = (
                    free.start,
                    free.end,
                    frame.zone.clone(),
                    self.on_slot.clone(),
                );
                div()
                    .id((self.id.clone(), format!("free-{day}-{start}")))
                    .debug_selector(move || format!("free {day} {start}"))
                    .absolute()
                    .top(frame.y(i64::from(start)))
                    .left_0()
                    .right_0()
                    .h(frame.y(i64::from(end - start)))
                    .border_1()
                    .border_color(cx.theme().colors.accent.alpha(0.3))
                    .child(format!("Free · {} min", end - start))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(move |_, window, cx| {
                        if let (Some(handler), Some(start), Some(end)) = (
                            &handler,
                            try_moment(day, i64::from(start), &zone),
                            try_moment(day, i64::from(end.min(start + 60)), &zone),
                        ) {
                            handler(start, end, window, cx);
                        }
                    })
                    .into_any_element()
            })
            .collect()
    }
}
