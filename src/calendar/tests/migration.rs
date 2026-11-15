use super::{Planning, at, heard, note, planning, settle};
use crate::calendar::{Event, FreeInterval, TimeGrid};
use gpui::{
    Entity, IntoElement, Modifiers, MouseButton, ParentElement, Styled, TestAppContext, div, point,
    px,
};
use jiff::{civil::date, tz::TimeZone};

fn grid(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let (made, moved, resized, picked) = (owner.clone(), owner.clone(), owner.clone(), owner);
    div()
        .h(px(600.0))
        .child(
            TimeGrid::new(
                "migration",
                [date(2026, 9, 22), date(2026, 9, 23)],
                [Event::timed("review", "Review", at(22, 13), at(22, 14), 0)],
            )
            .on_event(|_, _, _| {})
            .zone(TimeZone::UTC)
            .now(at(22, 12))
            .free_intervals([FreeInterval {
                day: date(2026, 9, 23),
                start: 900,
                end: 930,
            }])
            .on_slot(move |start, end, _, cx| note(&picked, format!("slot {start} {end}"), cx))
            .on_create(move |start, end, _, cx| note(&made, format!("create {start} {end}"), cx))
            .on_move(move |key, start, end, _, cx| {
                note(&moved, format!("move {key} {start} {end}"), cx)
            })
            .on_resize(move |key, end, _, cx| note(&resized, format!("resize {key} {end}"), cx)),
        )
        .into_any_element()
}

fn begin(
    from: gpui::Point<gpui::Pixels>,
    to: gpui::Point<gpui::Pixels>,
    cx: &mut gpui::VisualTestContext,
) {
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for step in 1..=4 {
        let share = step as f32 / 4.0;
        cx.simulate_mouse_move(
            from + (to - from) * share,
            MouseButton::Left,
            Modifiers::none(),
        );
    }
    settle(cx);
}

#[gpui::test]
fn moving_across_days_preserves_duration_and_commits_only_on_drop(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let card = cx.debug_bounds("event-card review").unwrap();
    let column = cx.debug_bounds("column 2026-09-23").unwrap();
    let to = point(column.center().x, column.top() + px(48.0 * 14.0));
    begin(card.center(), to, cx);
    assert!(heard(&host, cx).is_empty());
    assert!(cx.debug_bounds("calendar-drop-preview").is_some());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(
        heard(&host, cx),
        [format!("move review {} {}", at(23, 14), at(23, 15))]
    );
}

#[gpui::test]
fn resizing_across_days_commits_one_end(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let edge = cx.debug_bounds("end review 2026-09-22").unwrap();
    let column = cx.debug_bounds("column 2026-09-23").unwrap();
    let to = point(column.center().x, column.top() + px(48.0 * 14.0));
    begin(edge.center(), to, cx);
    assert!(heard(&host, cx).is_empty());
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(heard(&host, cx), [format!("resize review {}", at(23, 14))]);
}

#[gpui::test]
fn dropping_outside_the_grid_does_not_commit(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let from = cx.debug_bounds("event-card review").unwrap().center();
    let to = point(px(950.0), px(650.0));
    begin(from, to, cx);
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert!(heard(&host, cx).is_empty());
    assert!(cx.debug_bounds("calendar-drop-preview").is_none());
}

#[gpui::test]
fn free_interval_click_caps_the_draft_to_the_gap(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let free = cx.debug_bounds("free 2026-09-23 900").unwrap();
    cx.simulate_click(free.center(), Modifiers::none());
    settle(cx);
    assert_eq!(
        heard(&host, cx),
        [format!(
            "slot {} {}",
            at(23, 15),
            at(23, 15) + jiff::SignedDuration::from_mins(30)
        )]
    );
}

#[gpui::test]
fn clicking_a_slot_suggests_one_hour_without_starting_a_drag(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let column = cx.debug_bounds("column 2026-09-23").unwrap();
    cx.simulate_click(
        point(column.center().x, column.top() + px(48.0 * 13.0)),
        Modifiers::none(),
    );
    settle(cx);
    assert_eq!(
        heard(&host, cx),
        [format!("slot {} {}", at(23, 13), at(23, 14))]
    );
}

fn waiting_editor(host: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    use crate::calendar::{EventDraft, EventEditor};
    let draft = EventDraft::of(&Event::timed("edit", "", at(22, 9), at(22, 10), 0));
    EventEditor::new("waiting", draft)
        .zone(TimeZone::UTC)
        .autofocus(true)
        .scheduling(false)
        .pending(host.month.day() == 1)
        .error(Some("Save failed".into()))
        .on_save(move |draft, _, cx| note(&owner, format!("saved {}", draft.title), cx))
        .into_any_element()
}

#[gpui::test]
fn pending_editor_rejects_submit_and_keeps_edited_text_through_owner_updates(
    cx: &mut TestAppContext,
) {
    let (host, cx) = planning(waiting_editor, cx);
    cx.simulate_input("Review");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert!(heard(&host, cx).is_empty());
    host.update(cx, |host, cx| {
        host.month = date(2026, 9, 2);
        cx.notify();
    });
    settle(cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(heard(&host, cx), ["saved Review"]);
}

#[test]
fn all_day_switching_preserves_the_last_day_and_excludes_midnight() {
    use crate::calendar::{When, editor::toggled};
    let when = When::AllDay {
        first: date(2026, 9, 22),
        last: date(2026, 9, 24),
    };
    let timed = toggled(when, &TimeZone::UTC).unwrap();
    assert_eq!(
        timed,
        When::Timed {
            start: at(22, 9),
            end: at(24, 10)
        }
    );
    assert_eq!(
        toggled(
            When::Timed {
                start: at(22, 9),
                end: at(24, 0)
            },
            &TimeZone::UTC
        ),
        Some(When::AllDay {
            first: date(2026, 9, 22),
            last: date(2026, 9, 23)
        })
    );
}
