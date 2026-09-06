//! Two humans, two pads, and a rewind between them.
//!
//! The couch-multiplayer question has two halves that had never been asked in
//! one App. `rollback_match_activation.rs` covers the SIM half — two seats whose
//! authored frames survive resimulation, checked with `rollback_health` on every
//! tick. This file covers the DEVICE half: which pad each seat OWNS, and whether
//! that ownership survives the same rewinds.
//!
//! It printed `[seat-probe] []` and passed. A test whose subject is absent is worse than a
//! missing test, because it reports coverage.
//!
//! So this composes both: `Platformer2dSimHarness::app_mut()` installs the four
//! host systems that own seating and device assignment onto a sync-test harness.
//! Under a sync test EVERY frame is saved, rewound and resimulated, so "seat one
//! still holds pad one" is not an observation about one frame — it is an
//! observation about every frame being replayable.
//!
//! the systems are installed BEFORE the first step. `step` runs a full
//! `app.update()`, and a system added mid-episode runs against a baseline taken
//! without it — which desyncs on the next rewind rather than failing where it
//! was added.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
// `ambition_input` is not a direct dep of the app crate; the facade re-exports
// it as `ambition_platformer2d::input`, which is the path every other test here
// reaches it by.
use ambition_platformer2d::input::{
    assign_local_seat_devices, track_local_device_order, InputParticipant, LocalDeviceOrder,
    LocalSeatDeviceOwnership, Platformer2dInputActionMonolith,
};
use ambition_platformer2d::versus_match::{
    ControllerBinding, MatchParticipant, MatchParticipantRoster,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;

/// Frames the sandbox needs before a roster inserted into it means anything.
const FRAMES_BEFORE_THE_ROSTER: usize = 20;

/// A sync-test harness carrying TWO rollback players and the host's
/// device→seat layer.
///
/// The four systems are the host's own, in the host's own order
/// (`ambition_platformer2d_host`): device order and assignment in `PreUpdate`,
/// then freeze-then-seat in `Update`. Copying the ORDER matters more than
/// copying the set — `freeze_local_seating_for_the_decided_match` exists to run
/// before the seats it describes materialize, and a fixture that seated first
/// would test a frame shape the host never produces.
fn two_pad_rollback_harness() -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10)
            // The session must actually CARRY seat two, or its ownership is
            // decided and never consulted.
            .with_rollback_players(2),
    )
    .expect("a two-player sync-test harness builds");

    let app = sim.app_mut();
    app.init_resource::<LocalDeviceOrder>();
    // Without it `assign_local_seat_devices` panics on a missing resource.
    app.init_resource::<LocalSeatDeviceOwnership>();
    app.add_systems(
        PreUpdate,
        (track_local_device_order, assign_local_seat_devices).chain(),
    );
    app.add_systems(
        Update,
        (
            // Without this the fixture seated exactly one participant, numbered 1, and every
            // assertion about "two seats" failed on a world that only ever had one.
            //
            // `Update`, where the host uses `Startup`: the harness has already
            // updated by the time `app_mut()` is reachable, so a Startup system
            // added here would never run. Safe because the system is idempotent
            // by construction — it returns early if a PRIMARY already exists.
            ambition_platformer2d::runtime::host_input::spawn_primary_input_participant,
            ambition_platformer2d::runtime::host_input::freeze_local_seating_for_the_decided_match,
            ambition_platformer2d::runtime::host_input::seat_input_participants_for_roster,
        )
            .chain(),
    );
    sim
}

/// Plug in a pad. Returns its entity so a test can unplug the SAME one.
///
/// `Name` is not decoration: `SeatDeviceOwnership` remembers a disconnected pad
/// by its `PadIdentity`, and a nameless pad is indistinguishable from every
/// other nameless pad — which is precisely the case where a reconnect can hand
/// the wrong seat the wrong controller.
fn plug_in_a_pad(sim: &mut Platformer2dSimHarness, name: &str) -> Entity {
    sim.world_mut()
        .spawn((Gamepad::default(), Name::new(name.to_owned())))
        .id()
}

fn two_human_roster() -> MatchParticipantRoster {
    let human = |character: &str, slot: u8, team: &str| {
        MatchParticipant::new(character)
            .driven_by(ControllerBinding::Human {
                source: ambition_platformer2d::actor::LocalInputSource::Pad(slot),
            })
            .on_team(team)
    };
    MatchParticipantRoster {
        participants: vec![
            human("player_robot_v3", 0, "blue"),
            human("player_robot_v2", 1, "red"),
        ],
        seating: ambition_platformer2d::actor::RosterSeating::default(),
        published_by: None,
        rules: ambition_platformer2d::versus_match::MatchRules {
            item_spawns: None,
            // Not suspended: an opening hold would keep both fighters still and let
            // every assertion below pass for the wrong reason.
            opens_suspended: false,
            // No ceremony in a rollback fixture: the stage that owns the opening
            // is not part of what these tests exercise.
            opening_countdown_ticks: 0,
            time_limit_ticks: 0,
            abilities: None,
            body: None,
            stocks: None,
            health_pool: None,
            ..Default::default()
        },
    }
}

/// Which pad each seat's input map is restricted to, keyed by participant id.
///
/// read from the INPUT MAP rather than from `SeatDeviceOwnership`. The
/// ownership resource is what the assignment system WROTE; the map is what the
/// input layer will actually READ. Asserting on the former proves the bookkeeping
/// agrees with itself.
fn pad_per_seat(sim: &mut Platformer2dSimHarness) -> Vec<(u8, Option<Entity>)> {
    let world = sim.world_mut();
    let mut query = world.query::<(
        &InputParticipant,
        &InputMap<Platformer2dInputActionMonolith>,
    )>();
    let mut seats: Vec<(u8, Option<Entity>)> = query
        .iter(world)
        .map(|(participant, map)| (participant.id.0, map.gamepad()))
        .collect();
    seats.sort_by_key(|(id, _)| *id);
    seats
}

/// the extra frames are the point, not padding. Seats are created in
/// `Update` (`seat_input_participants_for_roster`) and devices are assigned in
/// `assign_local_seat_devices` runs in `PreUpdate`, so a newly created seat is
/// not expected to have a device assignment until the next assignment pass.
fn step_until_seated(sim: &mut Platformer2dSimHarness, want: usize, what: &str) {
    for tick in 0..180 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("{what}, tick {tick}: {error}"));
        if pad_per_seat(sim).len() >= want {
            settle(sim, what);
            return;
        }
    }
    let got = pad_per_seat(sim);
    panic!(
        "{what}: wanted {want} seated participants, got {} ({got:?})",
        got.len()
    );
}

fn settle(sim: &mut Platformer2dSimHarness, what: &str) {
    for tick in 0..3 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("{what} (settling), tick {tick}: {error}"));
    }
}

#[test]
fn two_pads_own_two_seats_and_keep_them_across_every_rewind() {
    let mut sim = two_pad_rollback_harness();
    let pad_one = plug_in_a_pad(&mut sim, "pad one");
    let pad_two = plug_in_a_pad(&mut sim, "pad two");

    for _ in 0..FRAMES_BEFORE_THE_ROSTER {
        sim.step(AgentAction::default());
    }
    sim.world_mut().insert_resource(two_human_roster());
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");

    step_until_seated(&mut sim, 2, "seating two humans on two pads");
    let assigned = pad_per_seat(&mut sim);
    assert_eq!(assigned.len(), 2, "two humans seated {assigned:?}");

    // Distinct pads. Two seats sharing one pad is the couch-multiplayer bug
    // in its purest form: both fighters answer to one controller, which looks
    // exactly like "player two does not work".
    let owned: Vec<Entity> = assigned.iter().filter_map(|(_, pad)| *pad).collect();
    assert_eq!(
        owned.len(),
        2,
        "a seat with no pad restriction answers to EVERY pad: {assigned:?}"
    );
    assert_ne!(
        owned[0], owned[1],
        "both seats own the same pad: {assigned:?}"
    );
    let mut expected = vec![pad_one, pad_two];
    expected.sort();
    let mut got = owned.clone();
    got.sort();
    assert_eq!(got, expected, "the seats own pads nobody plugged in");

    // And the assignment survives resimulation. Under a sync test the frames
    // below are each saved, rewound and replayed; ownership that were rebuilt
    // per-frame from a changing device order would drift here and nowhere else.
    for tick in 0..40 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("holding ownership, tick {tick}: {error}"));
        assert_eq!(
            pad_per_seat(&mut sim),
            assigned,
            "tick {tick}: pad ownership changed under resimulation"
        );
    }
}

#[test]
fn unplugging_one_pad_leaves_the_other_seat_alone_and_the_reconnect_goes_back() {
    let mut sim = two_pad_rollback_harness();
    let pad_one = plug_in_a_pad(&mut sim, "pad one");
    let pad_two = plug_in_a_pad(&mut sim, "pad two");

    for _ in 0..FRAMES_BEFORE_THE_ROSTER {
        sim.step(AgentAction::default());
    }
    sim.world_mut().insert_resource(two_human_roster());
    sim.rebase_rollback_history()
        .expect("the roster insert becomes the rollback baseline");
    step_until_seated(&mut sim, 2, "seating two humans before the unplug");

    let before = pad_per_seat(&mut sim);
    let seat_of = |seats: &[(u8, Option<Entity>)], pad: Entity| {
        seats
            .iter()
            .find(|(_, owned)| *owned == Some(pad))
            .map(|(id, _)| *id)
    };
    let seat_on_one = seat_of(&before, pad_one).expect("pad one owns a seat");
    let seat_on_two = seat_of(&before, pad_two).expect("pad two owns a seat");

    // Unplug pad two. Somebody's batteries died mid-match.
    sim.world_mut().entity_mut(pad_two).despawn();
    sim.rebase_rollback_history()
        .expect("the unplug becomes the rollback baseline");
    for tick in 0..20 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("after the unplug, tick {tick}: {error}"));
    }

    // A surviving controller keeps its seat; reconnect must not rederive seats from device order.
    let after_unplug = pad_per_seat(&mut sim);
    assert_eq!(
        seat_of(&after_unplug, pad_one),
        Some(seat_on_one),
        "unplugging seat {seat_on_two}'s pad moved seat {seat_on_one}'s: {after_unplug:?}"
    );

    // Plug it back in. A new entity with the same identity — which is what a
    // reconnect IS; the old `Entity` is gone and cannot come back.
    let pad_two_again = plug_in_a_pad(&mut sim, "pad two");
    sim.rebase_rollback_history()
        .expect("the reconnect becomes the rollback baseline");
    for tick in 0..20 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("after the reconnect, tick {tick}: {error}"));
    }

    let after_reconnect = pad_per_seat(&mut sim);
    assert_eq!(
        seat_of(&after_reconnect, pad_one),
        Some(seat_on_one),
        "the reconnect moved the seat that never lost its pad: {after_reconnect:?}"
    );
    assert_eq!(
        seat_of(&after_reconnect, pad_two_again),
        Some(seat_on_two),
        "the reconnected pad went to the wrong seat — it must return to \
         {seat_on_two}, the seat that was holding it: {after_reconnect:?}"
    );
}
