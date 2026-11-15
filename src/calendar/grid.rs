pub(super) mod drag;

use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, EntityId, InteractiveElement, IntoElement, ParentElement,
    Pixels, Point, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement, Styled,
    Window, canvas, div, prelude::*, relative,
};
use jiff::{Timestamp, civil::Date, tz::TimeZone};

use super::{
    card::EventCard,
    event::{Event, When, unique},
    hours::{Block, blocks, dragged, try_moment},
    now::CurrentTimeIndicator,
};
use crate::{
    forms::{Drawing, OnValue, Stroke, pen},
    layout::on_axis,
    theme::{ActiveTheme, Radius, TextSize},
    typography::{format, fresh},
};

/// Where a grid first opens: half past seven, or an hour and a half before now when that is later.
const MORNING: i64 = 7 * 60 + 30;

/// Gets the start and end of a stretch dragged out on the grid.
pub(crate) type OnSpan = Rc<dyn Fn(Timestamp, Timestamp, &mut Window, &mut App)>;
/// Gets an event's key and its new end.
pub(crate) type OnResize = Rc<dyn Fn(&SharedString, Timestamp, &mut Window, &mut App)>;

/// Another zone's clock beside a time grid's hours, so a day reads in both places.
#[derive(Clone, Debug)]
pub struct TimezoneOverlay {
    pub(crate) zone: TimeZone,
}

impl TimezoneOverlay {
    pub fn new(zone: TimeZone) -> Self {
        Self { zone }
    }
}

/// A free wall-clock interval supplied by the application.
#[derive(Clone, Debug)]
pub struct FreeInterval {
    pub day: Date,
    pub start: u16,
    pub end: u16,
}

pub(crate) type OnMove = Rc<dyn Fn(&SharedString, Timestamp, Timestamp, &mut Window, &mut App)>;

/// Days side by side under a column of hours. Each event is a card by its hours, and events that overlap share the width. A drag on empty time makes an event of a quarter hour or more; a drag on an event's lower edge moves its end. Today carries a line at the minute now. It fills its box and scrolls.
#[derive(IntoElement)]
pub struct TimeGrid {
    id: ElementId,
    days: Vec<Date>,
    events: Vec<Event>,
    zone: Option<TimeZone>,
    pub(crate) now: Option<Timestamp>,
    pub(crate) overlay: Option<TimezoneOverlay>,
    pub(crate) on_create: Option<OnSpan>,
    pub(crate) on_resize: Option<OnResize>,
    pub(crate) on_event: Option<OnValue>,
    pub(crate) on_move: Option<OnMove>,
    pub(crate) on_slot: Option<OnSpan>,
    pub(crate) free: Vec<FreeInterval>,
    pub(crate) scroll: Option<ScrollHandle>,
}

impl TimeGrid {
    pub fn new(
        id: impl Into<ElementId>,
        days: impl IntoIterator<Item = Date>,
        events: impl IntoIterator<Item = Event>,
    ) -> Self {
        let id = id.into();
        let days: Vec<Date> = days.into_iter().collect();
        assert!(!days.is_empty(), "time grid {id:?} has no days");
        Self {
            id,
            days,
            events: unique(events),
            zone: None,
            now: None,
            overlay: None,
            on_create: None,
            on_resize: None,
            on_event: None,
            on_move: None,
            on_slot: None,
            free: Vec::new(),
            scroll: None,
        }
    }

    /// Gets one committed move after the event is dropped.
    pub fn on_move(
        mut self,
        handler: impl Fn(&SharedString, Timestamp, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }

    /// Gets the suggested interval from a slot or free-time press.
    pub fn on_slot(
        mut self,
        handler: impl Fn(Timestamp, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_slot = Some(Rc::new(handler));
        self
    }

    /// Free intervals supplied by the owner, in local minutes.
    pub fn free_intervals(mut self, free: impl IntoIterator<Item = FreeInterval>) -> Self {
        self.free = free.into_iter().collect();
        self
    }

    /// Uses the owner's scroll handle.
    pub fn track_scroll(mut self, scroll: ScrollHandle) -> Self {
        self.scroll = Some(scroll);
        self
    }

    /// The zone its hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// The moment its line marks, on the day it falls; the clock's otherwise.
    pub fn now(mut self, now: Timestamp) -> Self {
        self.now = Some(now);
        self
    }

    /// Another zone's hours beside its own.
    pub fn overlay(mut self, overlay: TimezoneOverlay) -> Self {
        self.overlay = Some(overlay);
        self
    }

    /// Gets the start and end of a stretch dragged out on empty time.
    pub fn on_create(
        mut self,
        handler: impl Fn(Timestamp, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }

    /// Gets one committed end change after the lower edge is dropped.
    pub fn on_resize(
        mut self,
        handler: impl Fn(&SharedString, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Rc::new(handler));
        self
    }

    /// Gets the key of an event pressed.
    pub fn on_event(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_event = Some(Rc::new(handler));
        self
    }
}

/// What a grid's parts draw with: its id and drag owner, an hour's and a chip's height, its zone and events.
struct Frame<'a> {
    id: &'a ElementId,
    owner: EntityId,
    hour: Pixels,
    chip: Pixels,
    zone: &'a TimeZone,
    events: &'a [Event],
    target: Entity<Option<drag::Target>>,
}

impl Frame<'_> {
    fn y(&self, minutes: i64) -> Pixels {
        self.hour * (minutes as f32 / 60.0)
    }

    /// Hour labels down a gutter, each centered on its line, read in `zone`.
    fn labels(
        &self,
        day: Date,
        zone: &TimeZone,
        left: Pixels,
        width: Pixels,
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        div()
            .absolute()
            .top_0()
            .left(left)
            .w(width)
            .h_full()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(theme.colors.fg_subtle)
            .children((1..24).map(|hour| {
                let label = if zone == self.zone {
                    format!("{hour:02}:00")
                } else {
                    try_moment(day, hour * 60, self.zone)
                        .and_then(|at| format::datetime(at, zone, "%H:%M").ok())
                        .unwrap_or_else(|| "—".into())
                };
                div()
                    .absolute()
                    .top(self.y(hour * 60))
                    .left_0()
                    .w_full()
                    .h_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .pr_2()
                    .child(label)
            }))
            .into_any_element()
    }

    /// A card for `block`, and a handle on its lower edge when the event ends there.
    fn card(&self, day: Date, block: &Block, grid: &TimeGrid) -> AnyElement {
        let event = &self.events[block.event];
        let key = event.key.clone();
        let card = EventCard::new(
            (self.id.clone(), format!("card-{key}-{day}")),
            event.clone(),
        )
        .zone(self.zone.clone());
        let card = match grid.on_event.clone() {
            Some(on_event) => {
                let key = key.clone();
                card.on_click(move |window, cx| on_event(&key, window, cx))
            }
            None => card,
        };
        let handle = self.resize_handle(day, event, grid);

        div()
            .absolute()
            .top(self.y(block.from))
            .h(self.y(block.to - block.from).max(self.chip))
            .left(relative(block.lane as f32 / block.lanes as f32))
            .w(relative(1.0 / block.lanes as f32))
            .pr_0p5()
            .pb_0p5()
            .child(self.movable(card, event, grid))
            .children(handle)
            .into_any_element()
    }
}

/// The quarter hours a stroke down a column covers, where an hour is `hour` tall.
fn stretch(stroke: &Stroke, hour: Pixels) -> Option<(i64, i64)> {
    let [first, .., last] = stroke.as_slice() else {
        return None;
    };
    dragged(f32::from(first.y), f32::from(last.y), f32::from(hour))
}

impl TimeGrid {
    /// Day `day`'s column: its cards, the stretch being dragged out, and the line at now when it is today.
    fn column(
        &self,
        frame: &Frame,
        day: Date,
        ix: usize,
        today: Date,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let drawing =
            window.use_keyed_state((self.id.clone(), format!("draw-{day}")), cx, |_, _| {
                Drawing::default()
            });
        let draft = stretch(&drawing.read(cx).stroke, frame.hour);
        let cards: Vec<AnyElement> = blocks(&self.events, day, frame.zone)
            .iter()
            .map(|block| frame.card(day, block, self))
            .collect();
        let colors = &cx.theme().colors;
        let (radius, count) = (cx.theme().radius(Radius::Sm), self.days.len() as f32);
        let measure = drawing.clone();
        let pad = div()
            .id((self.id.clone(), format!("day-{day}")))
            .debug_selector(move || format!("column {day}"))
            .absolute()
            .top_0()
            .h_full()
            .left(relative(ix as f32 / count))
            .w(relative(1.0 / count))
            .border_l_1()
            .border_color(colors.border)
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if measure.read(cx).bounds != bounds {
                            measure.update(cx, |drawing, _| drawing.bounds = bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(self.free_slots(frame, day, cx))
            .children(cards)
            .children(self.preview(frame, day, cx))
            .children(draft.map(|(from, to)| {
                div()
                    .absolute()
                    .top(frame.y(from))
                    .h(frame.y(to - from))
                    .left_0()
                    .right_0()
                    .pr_0p5()
                    .child(
                        div()
                            .size_full()
                            .rounded(radius)
                            .border_1()
                            .border_color(colors.accent.opacity(0.5))
                            .bg(colors.accent.alpha(0.08)),
                    )
            }))
            .children((day == today).then(|| {
                let line =
                    CurrentTimeIndicator::new((self.id.clone(), "now")).zone(frame.zone.clone());
                match self.now {
                    Some(now) => line.now(now),
                    None => line,
                }
            }));
        let pad = self.drop_target(pad, frame, day, &drawing);
        let pad = self.slot_click(pad, frame, day, &drawing);
        let Some(on_create) = self.on_create.clone() else {
            return pad.into_any_element();
        };
        let (zone, hour) = (frame.zone.clone(), frame.hour);
        pen(pad, &drawing, move |stroke, window, cx| {
            if let Some((from, to)) = stretch(&stroke, hour) {
                log::info!("time grid: made minutes {from} to {to} of {day}");
                if let (Some(start), Some(end)) =
                    (try_moment(day, from, &zone), try_moment(day, to, &zone))
                {
                    on_create(start, end, window, cx);
                }
            }
        })
        .into_any_element()
    }
}

impl RenderOnce for TimeGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.now.is_none() {
            fresh((self.id.clone(), "clock"), window, cx);
        }
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| format::system_zone("time grid"));
        let now = self
            .now
            .unwrap_or_else(Timestamp::now)
            .to_zoned(zone.clone());
        let today = now.date();
        let rem = window.rem_size();
        let sizes = cx.theme().calendar();
        let (hour, chip, gutter) = (
            sizes.hour.to_pixels(rem),
            sizes.chip.to_pixels(rem),
            sizes.gutter.to_pixels(rem),
        );
        let gutters = gutter * (1 + usize::from(self.overlay.is_some())) as f32;
        let scroll =
            window.use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new());
        let owner = scroll.entity_id();
        let scroll = self
            .scroll
            .clone()
            .unwrap_or_else(|| scroll.read(cx).clone());
        let target =
            window.use_keyed_state((self.id.clone(), "target"), cx, |_, _| None::<drag::Target>);
        if !cx.has_active_drag() && target.read(cx).is_some() {
            target.update(cx, |target, _| *target = None);
        }
        let placed = window.use_keyed_state((self.id.clone(), "placed"), cx, |_, _| false);
        if !*placed.read(cx) {
            let minute = i64::from(now.hour()) * 60 + i64::from(now.minute());
            let shown = self.days.contains(&today).then_some(minute - 90);
            let top = shown.unwrap_or(0).max(MORNING);
            scroll.set_offset(Point::new(Pixels::ZERO, hour * (top as f32 / -60.0)));
            placed.update(cx, |placed, _| *placed = true);
        }
        let frame = Frame {
            id: &self.id,
            owner,
            hour,
            chip,
            zone: &zone,
            events: &self.events,
            target,
        };
        let first = self.days[0];
        let mut labels = vec![frame.labels(first, &zone, gutters - gutter, gutter, cx)];
        if let Some(overlay) = &self.overlay {
            labels.push(frame.labels(first, &overlay.zone, Pixels::ZERO, gutter, cx));
        }
        let columns: Vec<AnyElement> = self
            .days
            .iter()
            .enumerate()
            .map(|(ix, day)| self.column(&frame, *day, ix, today, window, cx))
            .collect();
        let border = cx.theme().colors.border;
        let lines = (1..24).map(|line| {
            div()
                .absolute()
                .top(frame.y(line * 60))
                .left(gutters)
                .right_0()
                .border_t_1()
                .border_color(border)
        });
        on_axis(
            div()
                .id((self.id.clone(), "scroll"))
                .debug_selector(|| "time-grid".into())
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&scroll),
        )
        .child(
            div()
                .relative()
                .w_full()
                .h(hour * 24.0)
                .children(labels)
                .children(lines)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(gutters)
                        .right_0()
                        .children(columns),
                ),
        )
    }
}
