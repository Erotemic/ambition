//! SIMULATION-ISLANDS ISL-2b, first witness: one room rewinds and replays
//! while the other room keeps its state.
//!
//! A hosted world (one App, no rollback session) has two live rooms: Alice in
//! the hub, Bob in `switch_lab`. At frame N the hub's entities are saved
//! through the registered rows ([`RoomSnapshotRows`]) with the clock. Both
//! rooms run on. Then the hub alone goes back to N and replays Alice's inputs:
//! Bob's room is hidden for the replay (`Disabled`, which every default query
//! skips), so nothing in it advances. Afterwards the hub must stand as it
//! stood after the first run, and Bob's room as it stood before the replay.
//!
//! A divergence names the state the replay could not keep to one room. The
//! owner is `docs/planning/engine/island-rollback-and-compact-peer-input.md`.

use std::collections::BTreeMap;

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::rollback::RoomSnapshotRows;
use ambition_platformer2d::time::{ClockState, DayClock, SimTick, WorldTime};
use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::{Entity, World};

use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_leaves, alice_on_the_hub_door, bodies};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

/// Frames between the save and the rewind.
const WINDOW: i32 = 60;

/// Alice walks right, then left, and jumps once: motion, no attack (an attack
/// would spawn what the snapshot cannot yet despawn).
fn alices_walk(frame: i32) -> ControlFrame {
    ControlFrame {
        axis_x: if frame < 30 { 1.0 } else { -1.0 },
        jump_pressed: frame == 20,
        jump_held: (20..32).contains(&frame),
        ..Default::default()
    }
}

fn bobs_walk(_frame: i32) -> ControlFrame {
    ControlFrame { axis_x: 1.0, ..Default::default() }
}

/// Every entity in `room`, and the room's own root.
fn of_room(world: &mut World, room: LiveRoomInstance) -> Vec<Entity> {
    let mut entities: Vec<Entity> = world
        .query::<(Entity, &InRoomInstance)>()
        .iter(world)
        .filter(|(_, at)| at.0 == room)
        .map(|(entity, _)| entity)
        .collect();
    entities.extend(world.query::<(Entity, &LiveRoomInstance)>().iter(world).filter(|(_, root)| **root == room).map(|(entity, _)| entity));
    entities.sort();
    entities
}

#[derive(Clone, Copy, Debug)]
struct Clock(SimTick, WorldTime, DayClock, ClockState);

fn clock(world: &World) -> Clock {
    Clock(
        *world.resource::<SimTick>(),
        *world.resource::<WorldTime>(),
        *world.resource::<DayClock>(),
        *world.resource::<ClockState>(),
    )
}

fn set_clock(world: &mut World, clock: Clock) {
    world.insert_resource(clock.0);
    world.insert_resource(clock.1);
    world.insert_resource(clock.2);
    world.insert_resource(clock.3);
}

fn differences(label: &str, expected: &BTreeMap<SimId, [f32; 5]>, got: &BTreeMap<SimId, [f32; 5]>) -> Vec<String> {
    let mut out = Vec::new();
    for (id, want) in expected {
        match got.get(id) {
            Some(have) if have == want => {}
            Some(have) => out.push(format!("{label} {id:?}: {want:?} then, {have:?} now")),
            None => out.push(format!("{label} {id:?}: gone")),
        }
    }
    for id in got.keys().filter(|id| !expected.contains_key(*id)) {
        out.push(format!("{label} {id:?}: new"));
    }
    out
}

/// Rewind the hub to N and replay it, hiding Bob's room for the replay when
/// `hide_the_lab`. Returns what differs from the first run.
fn rewind_the_hub(hide_the_lab: bool) -> Vec<String> {
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

    // Frame N: save the hub and the clock.
    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let hub_entities = of_room(sim.world_mut(), hub);
    let saved = rows.save(sim.world(), &hub_entities);
    let at_n = clock(sim.world());
    let hub_at_n = bodies(&mut sim, Some(hub));
    let lab_at_n = bodies(&mut sim, Some(lab));
    println!("[rewind] hub: {} entities, {} rows saved; lab bodies {}", hub_entities.len(), saved.rows(), lab_at_n.len());

    // Both rooms run on.
    for frame in 1..=WINDOW {
        sim.drive_seat(0, alices_walk(frame));
        sim.drive_seat(1, bobs_walk(frame));
        sim.app_mut().update();
    }
    let at_end = clock(sim.world());
    let hub_at_end = bodies(&mut sim, Some(hub));
    let lab_at_end = bodies(&mut sim, Some(lab));
    assert_ne!(hub_at_n, hub_at_end, "premise: the hub changed in the window");
    assert_ne!(lab_at_n, lab_at_end, "premise: Bob's room changed in the window");
    let hub_entities_at_end = of_room(sim.world_mut(), hub);
    assert_eq!(hub_entities_at_end, hub_entities, "premise: no entity was spawned in or taken from the hub in the window");

    // The hub alone goes back to N and replays; Bob's room is hidden.
    let hidden = if hide_the_lab { of_room(sim.world_mut(), lab) } else { Vec::new() };
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    rows.restore(sim.world_mut(), &saved);
    set_clock(sim.world_mut(), at_n);
    assert_eq!(bodies(&mut sim, Some(hub)), hub_at_n, "premise: the restore put the hub back at N");
    for frame in 1..=WINDOW {
        sim.drive_seat(0, alices_walk(frame));
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let replayed = clock(sim.world());
    sim.world_mut().insert_resource(rows);

    let mut wrong = differences("hub", &hub_at_end, &bodies(&mut sim, Some(hub)));
    wrong.extend(differences("lab", &lab_at_end, &bodies(&mut sim, Some(lab))));
    if replayed.0 != at_end.0 {
        wrong.push(format!("clock: tick {:?} after the first run, {:?} after the replay", at_end.0, replayed.0));
    }
    for line in &wrong {
        println!("[rewind] {line}");
    }
    wrong
}

#[test]
fn a_room_rewinds_and_replays_while_the_other_room_keeps_its_state() {
    let wrong = rewind_the_hub(true);
    assert!(wrong.is_empty(), "the replayed hub or the hidden room differs: {} differences", wrong.len());
}

/// The control: the same replay with Bob's room left in the world runs Bob's
/// room on (his seat is idle in the replay), so hiding it is what keeps it.
#[test]
fn without_hiding_the_other_room_the_replay_moves_it_too() {
    let wrong = rewind_the_hub(false);
    assert!(wrong.iter().any(|line| line.starts_with("lab ")), "the replay left Bob's room as it was although it ran: {wrong:?}");
    assert!(!wrong.iter().any(|line| line.starts_with("hub ")), "the hub replayed differently: {wrong:?}");
}
