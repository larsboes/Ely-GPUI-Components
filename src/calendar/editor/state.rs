use super::*;

/// An event's parts as an editor hands them back.
#[derive(Clone, Debug, PartialEq)]
pub struct EventDraft {
    pub title: SharedString,
    pub when: When,
    pub place: SharedString,
    pub hue: usize,
    pub repeat: Option<Recurrence>,
    pub reminders: Vec<u32>,
    pub notes: SharedString,
}

impl EventDraft {
    /// `event`'s parts, with no repeat, reminders or notes.
    pub fn of(event: &Event) -> Self {
        Self {
            title: event.title.clone(),
            when: event.when,
            place: event.place.clone().unwrap_or_default(),
            hue: event.hue,
            repeat: None,
            reminders: Vec::new(),
            notes: SharedString::default(),
        }
    }

    /// Why it cannot be saved: no title, or an end before its start.
    pub(crate) fn fault(&self) -> Option<&'static str> {
        if self.title.trim().is_empty() {
            return Some("Give it a title.");
        }
        let backward = match self.when {
            When::Timed { start, end } => end <= start,
            When::AllDay { first, last } => last < first,
        };
        backward.then_some("It ends before it starts.")
    }
}

/// `when` as whole days, or as hours from nine to ten on its first day; none when the zone holds no such hours.
pub(in crate::calendar) fn toggled(when: When, zone: &TimeZone) -> Option<When> {
    match when {
        When::Timed { start, end } => {
            let first = start.to_zoned(zone.clone()).date();
            let last = (end - SignedDuration::from_nanos(1))
                .to_zoned(zone.clone())
                .date()
                .max(first);
            Some(When::AllDay { first, last })
        }
        When::AllDay { first, last } => {
            let at = |day: jiff::civil::Date, hour| match zone
                .to_ambiguous_timestamp(day.at(hour, 0, 0, 0))
                .unambiguous()
            {
                Ok(at) => Some(at),
                Err(error) => {
                    log::error!("event editor: {first} at {hour}:00 has no moment: {error}");
                    None
                }
            };
            Some(When::Timed {
                start: at(first, 9)?,
                end: at(last, 10)?,
            })
        }
    }
}

/// `when` with its start moved to `start`, its length kept; none for an all-day event or an end past what jiff holds.
pub(in crate::calendar) fn moved(when: When, start: Timestamp) -> Option<When> {
    let When::Timed { start: was, end } = when else {
        log::error!("event editor: an all-day event does not move by the hour");
        return None;
    };
    let length: SignedDuration = end.duration_since(was);
    match start.checked_add(length) {
        Ok(end) => Some(When::Timed { start, end }),
        Err(error) => {
            log::error!("event editor: no end {length} after {start}: {error}");
            None
        }
    }
}

/// Moves the start while keeping the end clock and clamping its date.
pub(in crate::calendar) fn clamped(when: When, start: Timestamp, zone: &TimeZone) -> Option<When> {
    let When::Timed { end, .. } = when else {
        return None;
    };
    let civil_end = end.to_zoned(zone.clone()).datetime();
    let day = start.to_zoned(zone.clone()).date();
    let end = if civil_end.date() < day {
        zone.to_ambiguous_timestamp(day.to_datetime(civil_end.time()))
            .unambiguous()
            .ok()?
    } else {
        end
    };
    Some(When::Timed { start, end })
}

/// The editor's working copy: what the owner gave, the parts changed so far, and the fields that hold its words.
pub(super) struct Editing {
    pub(super) seed: EventDraft,
    pub(super) draft: EventDraft,
    pub(super) title: Entity<TextInput>,
    pub(super) place: Entity<TextInput>,
    pub(super) notes: Entity<TextInput>,
    pub(super) _title: Subscription,
    pub(super) on_save: Option<OnDraft>,
    pub(super) pending: bool,
    pub(super) invalid_local: Option<SharedString>,
}

impl Editing {
    pub(super) fn fill(&mut self, seed: EventDraft, cx: &mut Context<Self>) {
        for (input, text) in [
            (&self.title, &seed.title),
            (&self.place, &seed.place),
            (&self.notes, &seed.notes),
        ] {
            input.update(cx, |input, cx| input.set_text(text.to_string(), cx));
        }
        self.draft = seed.clone();
        self.seed = seed;
        self.invalid_local = None;
    }

    /// The draft as it stands, its words read from the fields.
    pub(super) fn current(&self, cx: &App) -> EventDraft {
        let words =
            |input: &Entity<TextInput>| SharedString::from(input.read(cx).text().to_string());
        EventDraft {
            title: words(&self.title).trim().to_string().into(),
            place: words(&self.place).trim().to_string().into(),
            notes: words(&self.notes),
            ..self.draft.clone()
        }
    }
}
