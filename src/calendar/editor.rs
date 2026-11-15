mod state;
use state::Editing;
pub use state::EventDraft;
pub(super) use state::{clamped, moved, toggled};

use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, Focusable, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Subscription, Window, div, prelude::*,
};
use jiff::{SignedDuration, Timestamp, civil::DateTime, tz::TimeZone};

use super::{
    event::{Event, When},
    recurrence::RecurrenceEditor,
    reminders::ReminderPicker,
    repeat::Recurrence,
};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{
        ColorPalette, DatePicker, DateTimePicker, FormError, FormField, Input, InputEvent, Run,
        Switch, TextInput,
    },
    theme::{ActiveTheme, HUE_NAMES},
    typography::format,
};

type OnDraft = Rc<dyn Fn(&EventDraft, &mut Window, &mut App)>;

/// An event's parts in a form: its title, whole days or hours from and until, where, its color, how it repeats, its reminders and notes. Save waits for a title and an end after the start; Delete shows with its handler. A new draft from the owner starts it over.
#[derive(IntoElement)]
pub struct EventEditor {
    id: ElementId,
    draft: EventDraft,
    zone: Option<TimeZone>,
    on_save: Option<OnDraft>,
    on_cancel: Option<Run>,
    on_delete: Option<Run>,
    autofocus: bool,
    pending: bool,
    error: Option<SharedString>,
    scheduling: bool,
    clamp_end: bool,
    extra: Option<gpui::AnyElement>,
}

impl EventEditor {
    pub fn new(id: impl Into<ElementId>, draft: EventDraft) -> Self {
        Self {
            id: id.into(),
            draft,
            zone: None,
            on_save: None,
            on_cancel: None,
            on_delete: None,
            autofocus: false,
            pending: false,
            error: None,
            scheduling: true,
            clamp_end: false,
            extra: None,
        }
    }

    /// Focuses the title when a new editor session mounts.
    pub fn autofocus(mut self, focus: bool) -> Self {
        self.autofocus = focus;
        self
    }
    /// Disables save while the owner persists the draft.
    pub fn pending(mut self, pending: bool) -> Self {
        self.pending = pending;
        self
    }
    /// Displays a persistence error without replacing the draft.
    pub fn error(mut self, error: Option<SharedString>) -> Self {
        self.error = error;
        self
    }
    /// Shows color, recurrence and reminder controls.
    pub fn scheduling(mut self, show: bool) -> Self {
        self.scheduling = show;
        self
    }
    /// Clamps an earlier end date to the new start date.
    pub fn clamp_end_date(mut self, clamp: bool) -> Self {
        self.clamp_end = clamp;
        self
    }
    /// Adds application-owned fields before the actions.
    pub fn extra(mut self, extra: impl IntoElement) -> Self {
        self.extra = Some(extra.into_any_element());
        self
    }

    /// The zone its hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the draft to keep.
    pub fn on_save(
        mut self,
        handler: impl Fn(&EventDraft, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_save = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    /// Offers Delete, for an event that already exists.
    pub fn on_delete(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EventEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("event editor"));
        let seed = self.draft.clone();
        let editing = window.use_keyed_state(
            (self.id.clone(), "editing"),
            cx,
            |window, cx: &mut Context<Editing>| {
                let mut field = |placeholder: &'static str, text: &SharedString, lines: bool| {
                    cx.new(|cx| {
                        let input = TextInput::new(window, cx).placeholder(placeholder);
                        let mut input = match lines {
                            true => input.multi_line(3, 6),
                            false => input,
                        };
                        input.set_text(text.to_string(), cx);
                        input
                    })
                };
                let title = field("Title", &seed.title, false);
                let place = field("Where", &seed.place, false);
                let notes = field("Notes", &seed.notes, true);
                if self.autofocus {
                    window.focus(&title.read(cx).focus_handle(cx), cx);
                }

                let watch = cx.subscribe_in(
                    &title,
                    window,
                    |editing: &mut Editing, _, event: &InputEvent, window, cx| {
                        if *event == InputEvent::Submit
                            && !editing.pending
                            && editing.invalid_local.is_none()
                        {
                            let draft = editing.current(cx);
                            if draft.fault().is_none()
                                && let Some(handler) = &editing.on_save
                            {
                                handler(&draft, window, cx);
                            }
                        }
                        cx.notify();
                    },
                );
                Editing {
                    seed: seed.clone(),
                    draft: seed.clone(),
                    title,
                    place,
                    notes,
                    _title: watch,
                    on_save: self.on_save.clone(),
                    pending: self.pending,
                    invalid_local: None,
                }
            },
        );
        if editing.read(cx).seed != self.draft {
            log::info!("event editor {:?}: started over", self.id);
            editing.update(cx, |editing, cx| editing.fill(self.draft.clone(), cx));
        }
        editing.update(cx, |editing, _| {
            editing.on_save = self.on_save.clone();
            editing.pending = self.pending;
        });
        let draft = editing.read(cx).current(cx);
        let edit = {
            let editing = editing.clone();
            move |change: Rc<dyn Fn(&mut EventDraft)>| {
                let editing = editing.clone();
                move |cx: &mut App| {
                    editing.update(cx, |editing, cx| {
                        change(&mut editing.draft);
                        editing.invalid_local = None;
                        cx.notify();
                    })
                }
            }
        };
        let clamp_end = self.clamp_end;
        let civil = |at: Timestamp| at.to_zoned(zone.clone()).datetime();
        let exact = {
            let zone = zone.clone();
            let editing = editing.clone();
            move |at: DateTime, cx: &mut App| match zone.to_ambiguous_timestamp(at).unambiguous() {
                Ok(at) => Some(at),
                Err(error) => {
                    log::warn!("event editor: {error}");
                    editing.update(cx, |editing, cx| {
                        editing.invalid_local = Some(
                            "Choose a local time outside the daylight-saving gap or repeated hour."
                                .into(),
                        );
                        cx.notify();
                    });
                    None
                }
            }
        };
        let whole = matches!(draft.when, When::AllDay { .. });
        let toggle = {
            let zone = zone.clone();
            let when = draft.when;
            edit(Rc::new(move |draft| {
                if let Some(when) = toggled(when, &zone) {
                    draft.when = when;
                }
            }))
        };
        let span: Vec<gpui::AnyElement> = match draft.when {
            When::Timed { start, end } => {
                let (at, until) = (exact.clone(), exact.clone());
                vec![
                    DateTimePicker::new((self.id.clone(), "starts"), Some(civil(start)))
                        .on_change({
                            let edit = edit.clone();
                            let at_zone = zone.clone();
                            move |picked, _, cx| {
                                let Some(start) = at(picked, cx) else {
                                    return;
                                };
                                let at_zone = at_zone.clone();
                                edit(Rc::new(move |draft| {
                                    let when = if clamp_end {
                                        clamped(draft.when, start, &at_zone)
                                    } else {
                                        moved(draft.when, start)
                                    };
                                    if let Some(when) = when {
                                        draft.when = when;
                                    }
                                }))(cx)
                            }
                        })
                        .into_any_element(),
                    DateTimePicker::new((self.id.clone(), "ends"), Some(civil(end)))
                        .on_change({
                            let edit = edit.clone();
                            move |picked, _, cx| {
                                let Some(end) = until(picked, cx) else {
                                    return;
                                };
                                edit(Rc::new(move |draft| {
                                    if let When::Timed { start, .. } = draft.when {
                                        draft.when = When::Timed { start, end };
                                    }
                                }))(cx)
                            }
                        })
                        .into_any_element(),
                ]
            }
            When::AllDay { first, last } => vec![
                DatePicker::new((self.id.clone(), "first"), Some(first))
                    .on_change({
                        let edit = edit.clone();
                        move |day, _, cx| {
                            let length = last - first;
                            let Ok(last) = (if clamp_end {
                                Ok(last.max(day))
                            } else {
                                day.checked_add(length)
                            }) else {
                                log::error!("event editor: no day {length} after {day}");
                                return;
                            };
                            edit(Rc::new(move |draft| {
                                draft.when = When::AllDay { first: day, last };
                            }))(cx)
                        }
                    })
                    .into_any_element(),
                DatePicker::new((self.id.clone(), "last"), Some(last))
                    .on_change({
                        let edit = edit.clone();
                        move |day, _, cx| {
                            edit(Rc::new(move |draft| {
                                if let When::AllDay { first, .. } = draft.when {
                                    draft.when = When::AllDay { first, last: day };
                                }
                            }))(cx)
                        }
                    })
                    .into_any_element(),
            ],
        };
        let theme = cx.theme();
        let chart = theme.colors.chart;
        let (first, _) = Event {
            key: self.id.to_string().into(),
            title: draft.title.clone(),
            when: draft.when,
            hue: draft.hue,
            place: None,
            color: None,
            tentative: false,
            selected: false,
        }
        .days(&zone);
        let fault = draft.fault();
        let invalid_local = editing.read(cx).invalid_local.clone();
        let save = {
            let (editing, on_save, id) = (editing.clone(), self.on_save.clone(), self.id.clone());
            Button::new((self.id.clone(), "save"), "Save")
                .variant(ButtonVariant::Primary)
                .disabled(fault.is_some() || invalid_local.is_some())
                .loading(self.pending)
                .on_click(move |_, window, cx| {
                    let draft = editing.read(cx).current(cx);
                    log::info!("event editor {id:?}: save {}", draft.title);
                    if let Some(on_save) = &on_save {
                        on_save(&draft, window, cx);
                    }
                })
        };
        let cancel = self.on_cancel.clone().map(|on_cancel| {
            Button::new((self.id.clone(), "cancel"), "Cancel")
                .variant(ButtonVariant::Ghost)
                .disabled(self.pending)
                .on_click(move |_, window, cx| on_cancel(window, cx))
        });
        let delete = self.on_delete.clone().map(|on_delete| {
            Button::new((self.id.clone(), "delete"), "Delete")
                .variant(ButtonVariant::Ghost)
                .disabled(self.pending)
                .on_click(move |_, window, cx| on_delete(window, cx))
        });
        let (title, place, notes) = {
            let editing = editing.read(cx);
            (
                editing.title.clone(),
                editing.place.clone(),
                editing.notes.clone(),
            )
        };
        div()
            .debug_selector(|| "event-editor".into())
            .flex()
            .flex_col()
            .gap_4()
            .child(
                FormField::new((self.id.clone(), "title-field"), "Title")
                    .required()
                    .child(Input::new(&title)),
            )
            .child(
                Switch::new((self.id.clone(), "all-day"), whole)
                    .label("All day")
                    .on_change(move |_, _, cx| toggle(cx)),
            )
            .child(
                div().flex().flex_wrap().gap_3().children(
                    span.into_iter()
                        .zip(["Starts", "Ends"])
                        .map(|(picker, label)| {
                            div().flex_1().min_w(theme.label_width()).child(
                                FormField::new((self.id.clone(), label), label).child(picker),
                            )
                        }),
                ),
            )
            .children((fault == Some("It ends before it starts.")).then(|| {
                div()
                    .debug_selector(|| "editor-fault".into())
                    .child(FormError::new(
                        (self.id.clone(), "fault"),
                        "It ends before it starts.",
                    ))
            }))
            .child(
                FormField::new((self.id.clone(), "place-field"), "Where").child(Input::new(&place)),
            )
            .when(self.scheduling, |editor| {
                editor
                    .child(
                        FormField::new((self.id.clone(), "color-field"), "Color").child(
                            ColorPalette::new(
                                (self.id.clone(), "color"),
                                HUE_NAMES
                                    .iter()
                                    .zip(chart)
                                    .map(|(name, color)| (*name, color)),
                            )
                            .selected(chart[draft.hue])
                            .on_change({
                                let edit = edit.clone();
                                move |color, _, cx| {
                                    let hue = chart
                                        .iter()
                                        .position(|each| *each == color)
                                        .expect("a hue of the chart");
                                    edit(Rc::new(move |draft| draft.hue = hue))(cx)
                                }
                            }),
                        ),
                    )
                    .child(
                        FormField::new((self.id.clone(), "repeat-field"), "Repeat").child(
                            RecurrenceEditor::new(
                                (self.id.clone(), "repeat"),
                                first,
                                draft.repeat.clone(),
                            )
                            .on_change({
                                let edit = edit.clone();
                                move |rule, _, cx| {
                                    edit(Rc::new(move |draft| draft.repeat = rule.clone()))(cx)
                                }
                            }),
                        ),
                    )
                    .child(
                        FormField::new((self.id.clone(), "reminders-field"), "Reminders").child(
                            ReminderPicker::new(
                                (self.id.clone(), "reminders"),
                                draft.reminders.clone(),
                            )
                            .on_change({
                                let edit = edit.clone();
                                move |minutes, _, cx| {
                                    let minutes = minutes.to_vec();
                                    edit(Rc::new(move |draft| draft.reminders = minutes.clone()))(
                                        cx,
                                    )
                                }
                            }),
                        ),
                    )
            })
            .child(
                FormField::new((self.id.clone(), "notes-field"), "Notes").child(Input::new(&notes)),
            )
            .children(
                invalid_local.map(|error| FormError::new((self.id.clone(), "local-error"), error)),
            )
            .children(self.extra)
            .children(
                self.error
                    .map(|error| FormError::new((self.id.clone(), "save-error"), error)),
            )
            .child(
                div()
                    .debug_selector(|| "editor-actions".into())
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().children(delete))
                    .child(div().flex().gap_2().children(cancel).child(save)),
            )
    }
}
