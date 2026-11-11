//! SIMULATION-ISLANDS ISL-1b: a freeze is for one room's fight.
//!
//! An impact hold (`ImpactHitstop`, what a connect, a death beat or a boss
//! beat writes) stops the shared clock. With Alice in the hub and Bob in
//! `switch_lab`, a hold from Alice's fight stopped Bob's room too. Now, with
//! several live rooms, the hold does not stop the clock: Bob walks on. The
//! control: with one live room the same hold still stops it. And the struck
//! room still freezes (its root's `RoomPace`, read through `RoomFreeze`):
//! Alice, driven, does not move while her room is held.

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::time::{ClockState, SimTick};

use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_leaves, alice_on_the_hub_door, bodies};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

/// Hold the impact freeze for the next thirty ticks, as a connect would.
fn hold_a_freeze(sim: &mut Platformer2dSimHarness) {
    let now = sim.world().resource::<SimTick>().0;
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::combat::impact_hitstop::ImpactHitstop>()
        .until_tick = Some(now + 30);
}

#[test]
fn an_impact_freeze_in_the_hub_does_not_stop_bobs_room() {
    // Control: one live room, and the hold stops the clock.
    let mut alone = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    for _ in 0..10 {
        alone.app_mut().update();
    }
    hold_a_freeze(&mut alone);
    for _ in 0..5 {
        alone.app_mut().update();
    }
    assert_eq!(alone.world().resource::<ClockState>().time_scale, 0.0, "control: in one live room the impact hold no longer stops the clock");

    // Two live rooms: Alice in the hub, Bob in switch_lab.
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    alice_on_the_hub_door(&mut sim);
    for frame in 1..=120 {
        sim.drive_seat(0, alice_leaves(frame));
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    let (hub, lab) = where_they_are(&mut sim);
    let lab = lab.flatten().expect("Bob has a room");
    assert_ne!(hub, Some(lab), "premise: Alice and Bob are in two live rooms");

    hold_a_freeze(&mut sim);
    let before = bodies(&mut sim, Some(lab));
    let mut stopped = 0;
    for _ in 0..20 {
        sim.drive_seat(0, ControlFrame::default());
        sim.drive_seat(1, ControlFrame { axis_x: -1.0, ..Default::default() });
        sim.app_mut().update();
        if sim.world().resource::<ClockState>().time_scale == 0.0 {
            stopped += 1;
        }
    }
    let after = bodies(&mut sim, Some(lab));
    assert_eq!(stopped, 0, "the hold stopped the shared clock on {stopped} of 20 ticks with two live rooms");
    assert_ne!(before, after, "Bob did not move while a freeze was held in the hub");
}

/// The struck room freezes while the other plays on: a hold in Alice's room
/// keeps every body of that room where it stood (Alice is driven left the
/// whole time), and Bob, driven left in his room, moves.
#[test]
fn an_impact_freeze_in_the_hub_holds_the_hub_and_not_bobs_room() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    alice_on_the_hub_door(&mut sim);
    for frame in 1..=120 {
        sim.drive_seat(0, alice_leaves(frame));
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    let (hub, lab) = where_they_are(&mut sim);
    let hub = hub.expect("Alice has a room");
    let lab = lab.flatten().expect("Bob has a room");
    assert_ne!(hub, lab, "premise: Alice and Bob are in two live rooms");

    let at = |sim: &mut Platformer2dSimHarness, room| -> Vec<[f32; 2]> {
        bodies(sim, Some(room)).values().map(|body| [body[0], body[1]]).collect()
    };
    let now = sim.world().resource::<SimTick>().0;
    let root = {
        let world = sim.world_mut();
        // The room's root is the one entity carrying its `LiveRoomInstance`.
        let mut roots =
            world.query::<(bevy::prelude::Entity, &ambition_platformer2d::platformer::lifecycle::LiveRoomInstance)>();
        roots.iter(world).find(|(_, room)| **room == hub).map(|(root, _)| root).expect("the hub has a root")
    };
    sim.world_mut().entity_mut(root).insert(ambition_platformer2d::platformer::lifecycle::RoomPace {
        held_until: now + 30,
        clock_scale: 1.0,
    });
    let (hub_before, lab_before) = (at(&mut sim, hub), at(&mut sim, lab));
    for _ in 0..20 {
        sim.drive_seat(0, ControlFrame { axis_x: -1.0, ..Default::default() });
        sim.drive_seat(1, ControlFrame { axis_x: -1.0, ..Default::default() });
        sim.app_mut().update();
    }
    assert_eq!(at(&mut sim, hub), hub_before, "a body of the held room moved during its freeze");
    assert_ne!(at(&mut sim, lab), lab_before, "Bob did not move while only the hub was held");
    // The hold runs out and the hub moves again.
    for _ in 0..20 {
        sim.drive_seat(0, ControlFrame { axis_x: -1.0, ..Default::default() });
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    assert_ne!(at(&mut sim, hub), hub_before, "the hub stayed held after its freeze ran out");
}

/// Each live room's hold: the tick its root's `RoomPace` holds it until.
fn holds(sim: &mut Platformer2dSimHarness) -> std::collections::BTreeMap<u32, u64> {
    let world = sim.world_mut();
    world
        .query::<(&ambition_platformer2d::platformer::lifecycle::LiveRoomInstance, &ambition_platformer2d::platformer::lifecycle::RoomPace)>()
        .iter(world)
        .map(|(room, pace)| (room.ordinal(), pace.held_until))
        .collect()
}

/// ISL-1b: a kill holds its own room. Ambition's rules give a kill a beat
/// (a freeze of 0.083 s). With Alice in the hub and Bob in `switch_lab`, an
/// enemy in the hub dies: the hub is held past the tick of the kill, and
/// Bob's room is not. Before, the kill wrote only the shared hold, which
/// stops no clock while two rooms are live, so a kill had no freeze at all.
#[test]
fn a_kill_in_the_hub_holds_the_hub_and_not_bobs_room() {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ROOM)).expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    alice_on_the_hub_door(&mut sim);
    for frame in 1..=120 {
        sim.drive_seat(0, alice_leaves(frame));
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    let (hub, lab) = where_they_are(&mut sim);
    let (hub, lab) = (hub.expect("Alice has a room"), lab.flatten().expect("Bob has a room"));
    assert_ne!(hub, lab, "premise: Alice and Bob are in two live rooms");
    sim.spawn_enemy_character_at(
        "isl1b_kill_probe",
        "Target",
        (200.0, 100.0),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_puppy_slug",
    );
    sim.app_mut().update();
    let enemy = {
        let world = sim.world_mut();
        world
            .query::<(bevy::prelude::Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, id)| id.0 == "isl1b_kill_probe")
            .expect("the enemy was spawned")
            .0
    };
    sim.world_mut()
        .entity_mut(enemy)
        .insert(ambition_platformer2d::platformer::lifecycle::InRoomInstance(hub));
    sim.app_mut().update();
    let before = holds(&mut sim);
    let now = sim.world().resource::<SimTick>().0;
    sim.world_mut()
        .get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(enemy)
        .expect("the enemy has health")
        .health
        .current = 0;
    sim.app_mut().update();
    let after = holds(&mut sim);
    assert!(
        after.get(&hub.ordinal()).is_some_and(|until| *until > now),
        "the kill did not hold the hub ({before:?} -> {after:?}, now {now})"
    );
    assert_eq!(after.get(&lab.ordinal()), before.get(&lab.ordinal()), "the kill held Bob's room");
}
