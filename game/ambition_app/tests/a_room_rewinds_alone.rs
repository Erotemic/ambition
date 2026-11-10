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
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d::platformer::projectile::ProjectileGameplay;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::rollback::RoomSnapshotRows;
use ambition_platformer2d::time::{ClockState, DayClock, GameplayElapsed, SimTick, WorldTime};
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
pub(crate) fn of_room(world: &mut World, room: LiveRoomInstance) -> Vec<Entity> {
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

/// The shared clock: the tick, the frame's time, the day, the scale, and the
/// session's elapsed seconds (`GameplayElapsed`, on the session root, which a
/// brain's timers read).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Clock(SimTick, WorldTime, DayClock, ClockState, GameplayElapsed);

pub(crate) fn clock(world: &World) -> Clock {
    Clock(
        *world.resource::<SimTick>(),
        *world.resource::<WorldTime>(),
        *world.resource::<DayClock>(),
        *world.resource::<ClockState>(),
        *ambition_platformer2d::platformer::lifecycle::session_world_component::<GameplayElapsed>(world)
            .expect("the session root carries the clock"),
    )
}

pub(crate) fn set_clock(world: &mut World, clock: Clock) {
    world.insert_resource(clock.0);
    world.insert_resource(clock.1);
    world.insert_resource(clock.2);
    world.insert_resource(clock.3);
    *ambition_platformer2d::platformer::lifecycle::session_world_component_mut::<GameplayElapsed>(world)
        .expect("the session root carries the clock") = clock.4;
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

/// Alice fires a shot to her right: at frame 10 one that ends inside the
/// window, at frame 30 one still in flight when it closes. The same shots in
/// the first run and the replay, so the replay must spawn them again.
fn alice_fires(sim: &mut Platformer2dSimHarness, frame: i32) {
    use ambition_platformer2d::projectiles::{ProjectileSpawnRequest, ProjectileStart};
    let lifetime = match frame {
        10 => 0.3,
        30 => 1.0,
        _ => return,
    };
    let world = sim.world_mut();
    let (alice, at) = world
        .query_filtered::<(Entity, &BodyKinematics), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .map(|(entity, body)| (entity, body.pos))
        .expect("Alice has a body");
    world.write_message(ProjectileSpawnRequest::open(
        alice,
        ambition_platformer2d::projectiles::ProjectileSpawn {
            origin: at + bevy::math::Vec2::new(24.0, 0.0),
            dir: bevy::math::Vec2::new(1.0, 0.0),
            speed: 240.0,
            damage: 1,
            max_lifetime: lifetime,
            half_extent: bevy::math::Vec2::new(6.0, 6.0),
            gravity: 0.0,
            visual_id: String::new(),
            bounces: 0,
            bounce_on_world_contact: false,
            boomerang_return_s: None,
            splash_half_extent: 0.0,
        },
        ProjectileStart::StepThisTick,
    ));
}

/// The hub's shots in flight: age and position, in order.
fn shots_in(world: &mut World, room: LiveRoomInstance) -> Vec<[f32; 3]> {
    let mut shots: Vec<[f32; 3]> = world
        .query::<(&ProjectileGameplay, &BodyKinematics, &InRoomInstance)>()
        .iter(world)
        .filter(|(_, _, at)| at.0 == room)
        .map(|(shot, body, _)| [shot.age, body.pos.x, body.pos.y])
        .collect();
    shots.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    shots
}

/// What an entity is, independent of its `Entity` id: its `SimId`, else the
/// rows it carries.
fn describe(world: &World, rows: &RoomSnapshotRows, entity: Entity) -> String {
    match world.get::<SimId>(entity) {
        Some(id) => format!("{id:?}"),
        None => format!("{:?}", rows.carried(world, &[entity]).into_iter().take(6).collect::<Vec<_>>()),
    }
}

/// The room's entities, described, sorted: two runs that built the same
/// entities give the same list whatever ids the entities got.
fn census(world: &World, rows: &RoomSnapshotRows, entities: &[Entity]) -> Vec<String> {
    let mut out: Vec<String> = entities.iter().map(|entity| describe(world, rows, *entity)).collect();
    out.sort();
    out
}

/// Rewind the hub to N and replay it, hiding Bob's room for the replay when
/// `hide_the_lab`, with Alice firing her shots in the window when `shots`.
/// Returns what differs from the first run. Without shots, an entity spawned
/// in or taken from the hub in the window fails the premise; with them, the
/// replay has to build the same entities again.
fn rewind_the_hub(hide_the_lab: bool, shots: bool) -> Vec<String> {
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
    let mut saved = rows.save(sim.world(), &hub_entities);
    let at_n = clock(sim.world());
    let hub_at_n = bodies(&mut sim, Some(hub));
    let lab_at_n = bodies(&mut sim, Some(lab));
    println!("[rewind] hub: {} entities, {} rows saved; lab bodies {}", hub_entities.len(), saved.rows(), lab_at_n.len());

    // Both rooms run on.
    for frame in 1..=WINDOW {
        if shots {
            alice_fires(&mut sim, frame);
        }
        sim.drive_seat(0, alices_walk(frame));
        sim.drive_seat(1, bobs_walk(frame));
        sim.app_mut().update();
    }
    let at_end = clock(sim.world());
    let shots_at_end = shots_in(sim.world_mut(), hub);
    let hub_at_end = bodies(&mut sim, Some(hub));
    let lab_at_end = bodies(&mut sim, Some(lab));
    assert_ne!(hub_at_n, hub_at_end, "premise: the hub changed in the window");
    assert_ne!(lab_at_n, lab_at_end, "premise: Bob's room changed in the window");
    let hub_entities_at_end = of_room(sim.world_mut(), hub);
    let first_census = census(sim.world(), &rows, &hub_entities_at_end);
    let mut spawned_or_gone = Vec::new();
    for entity in hub_entities_at_end.iter().filter(|entity| !hub_entities.contains(entity)) {
        spawned_or_gone.push(format!("spawned in the window: {entity:?} {}", describe(sim.world(), &rows, *entity)));
    }
    for entity in hub_entities.iter().filter(|entity| !hub_entities_at_end.contains(entity)) {
        spawned_or_gone.push(format!("gone in the window: {entity:?}"));
    }
    if !shots {
        assert!(spawned_or_gone.is_empty(), "premise: no entity was spawned in or taken from the hub in the window: {spawned_or_gone:?}");
    }

    // The hub alone goes back to N and replays; Bob's room is hidden.
    let hidden = if hide_the_lab { of_room(sim.world_mut(), lab) } else { Vec::new() };
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let restored = rows.restore(sim.world_mut(), &mut saved, &hub_entities_at_end);
    println!("[rewind] the restore despawned {} and re-created {}", restored.despawned.len(), restored.recreated.len());
    set_clock(sim.world_mut(), at_n);
    assert_eq!(bodies(&mut sim, Some(hub)), hub_at_n, "premise: the restore put the hub back at N");
    for frame in 1..=WINDOW {
        if shots {
            alice_fires(&mut sim, frame);
        }
        sim.drive_seat(0, alices_walk(frame));
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let replayed = clock(sim.world());
    sim.world_mut().insert_resource(rows);

    for line in &spawned_or_gone {
        println!("[rewind] {line}");
    }
    let mut wrong = Vec::new();
    let after_replay = of_room(sim.world_mut(), hub);
    let replayed_census = census(sim.world(), sim.world().resource::<RoomSnapshotRows>(), &after_replay);
    if replayed_census != first_census {
        for line in replayed_census.iter().filter(|line| !first_census.contains(line)) {
            wrong.push(format!("in the hub after the replay only: {line}"));
        }
        for line in first_census.iter().filter(|line| !replayed_census.contains(line)) {
            wrong.push(format!("in the hub after the first run only: {line}"));
        }
        if replayed_census.len() != first_census.len() {
            wrong.push(format!("the hub holds {} entities after the first run, {} after the replay", first_census.len(), replayed_census.len()));
        }
    }
    wrong.extend(differences("hub", &hub_at_end, &bodies(&mut sim, Some(hub))));
    if shots {
        assert_eq!(shots_at_end.len(), 1, "premise: one shot is in flight when the window closes: {shots_at_end:?}");
        assert!(!spawned_or_gone.is_empty(), "premise: the shots were spawned in the hub");
    }
    let shots_after = shots_in(sim.world_mut(), hub);
    if shots_after != shots_at_end {
        wrong.push(format!("shots: {shots_at_end:?} after the first run, {shots_after:?} after the replay"));
    }
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
    let wrong = rewind_the_hub(true, false);
    assert!(wrong.is_empty(), "the replayed hub or the hidden room differs: {} differences", wrong.len());
}

/// The control: the same replay with Bob's room left in the world runs Bob's
/// room on (his seat is idle in the replay), so hiding it is what keeps it.
/// Run it beside the witness when the rewind changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the rewind changes"]
fn without_hiding_the_other_room_the_replay_moves_it_too() {
    let wrong = rewind_the_hub(false, false);
    assert!(wrong.iter().any(|line| line.starts_with("lab ")), "the replay left Bob's room as it was although it ran: {wrong:?}");
    assert!(!wrong.iter().any(|line| line.starts_with("hub ")), "the hub replayed differently: {wrong:?}");
}

/// ISL-2c: a shot that ended inside the window and one still in flight are
/// spawned after the save. The restore despawns them, and the replay fires
/// them again: the hub ends with the same shot where it was.
#[test]
fn a_rewound_room_fires_its_shots_again() {
    let wrong = rewind_the_hub(true, true);
    assert!(wrong.is_empty(), "the replay with shots differs: {wrong:?}");
}

/// ISL-2d probe: a world write made in the window. Alice takes the coin in
/// `basement_treasure` while Bob stays in the hub; the coin room rewinds to
/// before the pickup and replays it. Her wallet is on her body and rewinds
/// with the room; the grant record (`RewardGrantsSinceCheckpoint`) and the
/// save belong to the world and do not. Prints what each says after the first
/// run and after the replay.
#[test]
#[ignore = "probe: prints what a world write in a rewound window does; run with --ignored --nocapture"]
fn probe_a_coin_taken_in_a_rewound_window() {
    use crate::pickup_regrowth_across_rooms::{alices_balance, collect, COIN, COIN_ROOM, HUB};
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        COIN_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    assert_ne!(room, bobs, "premise: Alice and Bob are in two live rooms");
    let grants = |sim: &Platformer2dSimHarness| {
        sim.world().resource::<ambition_platformer2d::item_runtime::pickup::RewardGrantsSinceCheckpoint>().grants().len()
    };
    let saved_flags = |sim: &Platformer2dSimHarness| {
        format!("{:?}", sim.world().resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data()).len()
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let (balance_n, grants_n, save_n) = (alices_balance(&mut sim), grants(&sim), saved_flags(&sim));

    collect(&mut sim, COIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (balance_1, grants_1, save_1) = (alices_balance(&mut sim), grants(&sim), saved_flags(&sim));

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    let balance_restored = alices_balance(&mut sim);
    collect(&mut sim, COIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let (balance_2, grants_2, save_2) = (alices_balance(&mut sim), grants(&sim), saved_flags(&sim));
    println!("[coin] wallet: {balance_n} at N, {balance_1} after the pickup, {balance_restored} restored, {balance_2} after the replay");
    println!("[coin] grants since the checkpoint: {grants_n} at N, {grants_1} after the pickup, {grants_2} after the replay");
    println!("[coin] save (debug length): {save_n} at N, {save_1} after the pickup, {save_2} after the replay");
    sim.world_mut().insert_resource(rows);
}
