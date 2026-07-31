//! **The select screen has to be drivable by a controller, not only by a test.**
//!
//! `SmashSelect` shipped fully unit-tested and completely inert: every state
//! transition was covered, and nothing in the app ever WROTE to it, so the
//! battle could not start from the screen at all. Every one of those unit tests
//! drove the resource directly, which is exactly why they were all green over a
//! screen nobody could use.
//!
//! So these press buttons.

use ambition::input::{MenuControlFrame, SeatMenuFrames};
use ambition_demo_smash::select::{SELECTABLE, SeatSelection, SmashSelect};
use ambition_demo_smash_app::build_demo_app;
use bevy::prelude::*;

/// Plug in `count` controllers. The screen offers exactly as many seats as there
/// are pads — one pad is one seat, which is the right answer on a couch and the
/// reason a test has to say how many people are in the room.
fn plug_in(app: &mut App, count: usize) {
    // ⚠ SPAWN PADS, do not insert the order. `track_local_device_order` rebuilds
    // `LocalDeviceOrder` from live `Gamepad` entities every frame, so a
    // hand-inserted order is clobbered on the next update — and only when the
    // `input` feature is on, which is how this passed by default and failed
    // under `--features input,visible`. The resource is derived; the pads are
    // the fact.
    let pads: Vec<Entity> = (0..count)
        .map(|_| {
            app.world_mut()
                .spawn(bevy::input::gamepad::Gamepad::default())
                .id()
        })
        .collect();
    app.update();
    // …and the tracker itself is behind the `input` feature, so under default
    // features nothing derives the order and the pads sit there unread. Seed it
    // only when that happened: seeding unconditionally would put the test back
    // to fighting the tracker in the configuration where the tracker runs.
    let derived = app
        .world()
        .get_resource::<ambition::input::LocalDeviceOrder>()
        .map(|order| order.devices().len())
        .unwrap_or(0);
    if derived < count {
        app.world_mut()
            .insert_resource(ambition::input::LocalDeviceOrder::from_devices(pads));
    }
}

fn press(app: &mut App, seat: u8, frame: MenuControlFrame) {
    let mut frames = app.world_mut().resource_mut::<SeatMenuFrames>();
    frames.clear();
    frames.set(seat, frame);
    app.update();
    // Release, so a held button is not a new press next frame — the screen
    // reads edges and the writer produces them.
    app.world_mut().resource_mut::<SeatMenuFrames>().clear();
    app.update();
}

fn confirm() -> MenuControlFrame {
    MenuControlFrame {
        select: true,
        ..Default::default()
    }
}

fn right() -> MenuControlFrame {
    MenuControlFrame {
        right: true,
        ..Default::default()
    }
}

fn back() -> MenuControlFrame {
    MenuControlFrame {
        back: true,
        ..Default::default()
    }
}

fn seat(app: &mut App, index: usize) -> SeatSelection {
    app.world().resource::<SmashSelect>().seat(index)
}

#[test]
fn two_players_join_choose_and_lock_in_and_the_battle_starts() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    plug_in(&mut app, 2);
    assert!(
        app.world().get_resource::<SeatMenuFrames>().is_some(),
        "the per-seat menu frames are the screen's input port; without them \
         installed this test would pass by pressing nothing"
    );

    // Seat 0 joins, browses one to the right, locks in.
    press(&mut app, 0, confirm());
    assert!(
        matches!(seat(&mut app, 0), SeatSelection::Browsing { cursor: 0 }),
        "confirm at an empty seat is the join: {:?}",
        seat(&mut app, 0)
    );
    press(&mut app, 0, right());
    assert!(
        matches!(seat(&mut app, 0), SeatSelection::Browsing { cursor: 1 }),
        "the cursor has to move: {:?}",
        seat(&mut app, 0)
    );
    press(&mut app, 0, confirm());
    assert_eq!(seat(&mut app, 0), SeatSelection::LockedIn { character: 1 });

    // One locked seat is not a match.
    assert!(
        app.world()
            .get_resource::<ambition::actor::MatchParticipantRoster>()
            .is_none(),
        "a match started with one fighter in it"
    );

    // Seat 1 joins and locks in on the default character.
    press(&mut app, 1, confirm());
    press(&mut app, 1, confirm());
    assert_eq!(seat(&mut app, 1), SeatSelection::LockedIn { character: 0 });

    let roster = app
        .world()
        .get_resource::<ambition::actor::MatchParticipantRoster>()
        .expect("two locked seats is a decided match, and the screen has to publish it");
    assert_eq!(roster.participants.len(), 2);
    assert_eq!(roster.participants[0].character, SELECTABLE[1]);
    assert_eq!(roster.participants[1].character, SELECTABLE[0]);
}

#[test]
fn backing_out_of_a_lock_returns_to_browsing_rather_than_leaving() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    plug_in(&mut app, 1);
    press(&mut app, 0, confirm());
    press(&mut app, 0, confirm());
    assert_eq!(seat(&mut app, 0), SeatSelection::LockedIn { character: 0 });

    press(&mut app, 0, back());
    assert!(
        matches!(seat(&mut app, 0), SeatSelection::Browsing { cursor: 0 }),
        "an accidental lock-in must not cost somebody their place in the match: {:?}",
        seat(&mut app, 0)
    );
    press(&mut app, 0, back());
    assert_eq!(seat(&mut app, 0), SeatSelection::Empty);
}

#[test]
fn a_joined_but_still_browsing_seat_holds_the_match() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    plug_in(&mut app, 3);
    // ⚠ ORDER IS THE TEST. `ready()` is instantaneous — the match starts the
    // frame its condition holds — so a third player who joins AFTER the second
    // lock-in has already missed it, and pressing in that order tests nothing.
    // They join while the first two are still choosing, which is the situation
    // the rule exists for.
    press(&mut app, 0, confirm());
    press(&mut app, 1, confirm());
    press(&mut app, 2, confirm()); // joined, still browsing
    press(&mut app, 0, confirm());
    press(&mut app, 1, confirm());

    assert!(
        app.world()
            .get_resource::<ambition::actor::MatchParticipantRoster>()
            .is_none(),
        "a third player joined and is still choosing; starting without them is \
         the screen deciding on their behalf"
    );
}

/// **A screen that works and cannot be seen is the same bug one layer up.**
///
/// Asserting the panels EXIST would pass over four empty boxes, so this asserts
/// what they SAY — and says it by reading the same text the player reads.
#[test]
fn the_panels_say_what_each_seat_has_decided() {
    use ambition_demo_smash::select_ui::{SmashSeatPanel, SmashSelectPrompt};
    use bevy::prelude::{Text, With, Without};

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    plug_in(&mut app, 2);
    app.update();

    let read = |app: &mut App| -> Vec<String> {
        let world = app.world_mut();
        let mut q = world.query_filtered::<(&SmashSeatPanel, &Text), Without<SmashSelectPrompt>>();
        let mut rows: Vec<(usize, String)> = q
            .iter(world)
            .map(|(panel, text)| (panel.0, text.0.clone()))
            .collect();
        rows.sort_by_key(|(seat, _)| *seat);
        rows.into_iter().map(|(_, text)| text).collect()
    };
    let prompt = |app: &mut App| -> String {
        let world = app.world_mut();
        let mut q = world.query_filtered::<&Text, With<SmashSelectPrompt>>();
        q.iter(world)
            .next()
            .map(|t| t.0.clone())
            .unwrap_or_default()
    };

    let before = read(&mut app);
    assert_eq!(before.len(), 4, "one panel per seat: {before:?}");

    // Two pads are plugged in, so two seats are on offer and the other two are
    // hidden — a panel reading "press confirm to join" at a chair nobody can sit
    // in is an invitation the screen cannot honour.
    let visible = {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<(&SmashSeatPanel, &bevy::prelude::Node), Without<SmashSelectPrompt>>(
            );
        let mut seats: Vec<usize> = q
            .iter(world)
            .filter(|(_, node)| node.display != bevy::prelude::Display::None)
            .map(|(panel, _)| panel.0)
            .collect();
        seats.sort();
        seats
    };
    assert_eq!(
        visible,
        vec![0, 1],
        "two controllers is two seats; the rest are chairs nobody can sit in"
    );
    assert!(
        before[0].contains("press confirm to join"),
        "an empty seat has to invite somebody into it: {before:?}"
    );
    assert_eq!(prompt(&mut app), "Two players needed");

    press(&mut app, 0, confirm());
    let browsing = read(&mut app);
    assert!(
        browsing[0].contains(SELECTABLE[0]) && browsing[0].contains('<'),
        "a browsing seat shows the character under its cursor: {browsing:?}"
    );

    press(&mut app, 0, confirm());
    let locked = read(&mut app);
    assert!(
        locked[0].contains("READY"),
        "a locked seat has to look different from a browsing one: {locked:?}"
    );
    assert_eq!(
        prompt(&mut app),
        "Two players needed",
        "one locked seat is not a match, and the screen has to say what it wants"
    );

    // And the screen goes away when the match does.
    press(&mut app, 1, confirm());
    press(&mut app, 1, confirm());
    for _ in 0..10 {
        app.update();
    }
    assert!(
        read(&mut app).is_empty(),
        "the select panels outlived the select route and are drawn over the match"
    );
}
