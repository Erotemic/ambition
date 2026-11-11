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
pub(crate) struct Clock(pub(crate) SimTick, WorldTime, DayClock, ClockState, GameplayElapsed);

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

/// What Alice does in the window.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Window {
    /// She walks: no entity is spawned or taken.
    Walks,
    /// She fires her shots ([`alice_fires`]).
    Fires,
    /// She fires them at an enemy standing in her way, which they hurt.
    FiresAtAnEnemy,
}

/// An enemy that stands still, to Alice's right in the hub.
const TARGET: &str = "isl2c_target";

fn an_enemy_in_alices_way(sim: &mut Platformer2dSimHarness, hub: LiveRoomInstance) {
    let at = {
        let world = sim.world_mut();
        world
            .query_filtered::<&BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
            .single(world)
            .expect("Alice has a body")
            .pos
    };
    sim.spawn_enemy_character_at(
        TARGET,
        "Target",
        (at.x + 72.0, at.y),
        (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_puppy_slug",
    );
    let world = sim.world_mut();
    let target = world
        .query::<(Entity, &ambition_platformer2d::combat::components::FeatureId)>()
        .iter(world)
        .find(|(_, id)| id.0 == TARGET)
        .expect("the enemy was spawned")
        .0;
    world.entity_mut(target).insert(InRoomInstance(hub));
}

/// Each body's health in `room`, by peer-stable id.
fn healths_in(world: &mut World, room: LiveRoomInstance) -> BTreeMap<SimId, String> {
    world
        .query::<(&SimId, &ambition_platformer2d::characters::actor::BodyHealth, &InRoomInstance)>()
        .iter(world)
        .filter(|(_, _, at)| at.0 == room)
        .map(|(id, health, _)| (id.clone(), format!("{health:?}")))
        .collect()
}

/// Rewind the hub to N and replay it, hiding Bob's room for the replay when
/// `hide_the_lab`, with Alice doing `window` in it. Returns what differs from
/// the first run. When she only walks, an entity spawned in or taken from the
/// hub in the window fails the premise; when she fires, the replay has to
/// build the same entities again.
fn rewind_the_hub(hide_the_lab: bool, window: Window) -> Vec<String> {
    let shots = window != Window::Walks;
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
    if window == Window::FiresAtAnEnemy {
        an_enemy_in_alices_way(&mut sim, hub);
    }
    let health_at_n = healths_in(sim.world_mut(), hub);

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
    let health_at_end = healths_in(sim.world_mut(), hub);
    if window == Window::FiresAtAnEnemy {
        assert_ne!(health_at_n, health_at_end, "premise: a shot hurt the enemy in the window");
    }
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
    assert_eq!(healths_in(sim.world_mut(), hub), health_at_n, "premise: the restore put the hub's health back at N");
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
    let health_after = healths_in(sim.world_mut(), hub);
    for (id, want) in &health_at_end {
        if health_after.get(id) != Some(want) {
            wrong.push(format!("health {id:?}: {want} after the first run, {:?} after the replay", health_after.get(id)));
        }
    }
    if window == Window::Fires {
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
    let wrong = rewind_the_hub(true, Window::Walks);
    assert!(wrong.is_empty(), "the replayed hub or the hidden room differs: {} differences", wrong.len());
}

/// The control: the same replay with Bob's room left in the world runs Bob's
/// room on (his seat is idle in the replay), so hiding it is what keeps it.
/// Run it beside the witness when the rewind changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the rewind changes"]
fn without_hiding_the_other_room_the_replay_moves_it_too() {
    let wrong = rewind_the_hub(false, Window::Walks);
    assert!(wrong.iter().any(|line| line.starts_with("lab ")), "the replay left Bob's room as it was although it ran: {wrong:?}");
    assert!(!wrong.iter().any(|line| line.starts_with("hub ")), "the hub replayed differently: {wrong:?}");
}

/// ISL-2c: a shot that ended inside the window and one still in flight are
/// spawned after the save. The restore despawns them, and the replay fires
/// them again: the hub ends with the same shot where it was.
#[test]
fn a_rewound_room_fires_its_shots_again() {
    let wrong = rewind_the_hub(true, Window::Fires);
    assert!(wrong.is_empty(), "the replay with shots differs: {wrong:?}");
}

/// ISL-2c: the shots hurt an enemy in the window. The restore gives it back
/// its health, and the replay hurts it again by as much.
#[test]
fn a_rewound_room_hurts_its_enemy_again() {
    let wrong = rewind_the_hub(true, Window::FiresAtAnEnemy);
    assert!(wrong.is_empty(), "the replay with a hurt enemy differs: {wrong:?}");
}

/// What one coin run reads: (wallet, grants since the checkpoint, the save's
/// debug length) at frame N, after the pickup, and after the replay; and the
/// wallet just after the restore.
struct CoinRun {
    at_n: (i32, usize, usize),
    after_pickup: (i32, usize, usize),
    restored_wallet: i32,
    after_replay: (i32, usize, usize),
}

/// ISL-2d: a world write made in the window. Alice takes the coin in
/// `basement_treasure` while Bob stays in the hub; the coin room rewinds to
/// before the pickup and replays it. Her wallet is on her body and rewinds
/// with the room; the grant record (`RewardGrantsSinceCheckpoint`) and the
/// save belong to the world. With `retract`, the rewind announces itself
/// (`retract_room_writes`), as the room rollback host will.
fn a_coin_taken_in_a_rewound_window(retract: bool) -> CoinRun {
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
    let at_n_reading = (alices_balance(&mut sim), grants(&sim), saved_flags(&sim));

    collect(&mut sim, COIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let after_pickup = (alices_balance(&mut sim), grants(&sim), saved_flags(&sim));

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    let restored_wallet = alices_balance(&mut sim);
    collect(&mut sim, COIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = (alices_balance(&mut sim), grants(&sim), saved_flags(&sim));
    sim.world_mut().insert_resource(rows);
    CoinRun { at_n: at_n_reading, after_pickup, restored_wallet, after_replay }
}

/// ISL-2d probe, the control: the same rewind with no retraction. Prints what
/// each store says (measured 2026-11-10: the replay records a second grant).
#[test]
#[ignore = "probe: prints what a world write in a rewound window does; run with --ignored --nocapture"]
fn probe_a_coin_taken_in_a_rewound_window() {
    let run = a_coin_taken_in_a_rewound_window(false);
    println!(
        "[coin] wallet: {} at N, {} after the pickup, {} restored, {} after the replay",
        run.at_n.0, run.after_pickup.0, run.restored_wallet, run.after_replay.0
    );
    println!("[coin] grants since the checkpoint: {} at N, {} after the pickup, {} after the replay", run.at_n.1, run.after_pickup.1, run.after_replay.1);
    println!("[coin] save (debug length): {} at N, {} after the pickup, {} after the replay", run.at_n.2, run.after_pickup.2, run.after_replay.2);
}

/// ISL-2d (review 2026-11-10, finding 7): a room that rewinds takes back the
/// grants it wrote after the tick it was saved at, so the replay's pickup is
/// recorded once. Without the retraction (the probe above) it is recorded
/// twice, and a death restore would take back two grants for one coin.
#[test]
fn a_rewound_room_takes_back_the_grant_its_replay_writes_again() {
    let run = a_coin_taken_in_a_rewound_window(true);
    assert_eq!(run.after_pickup.1, run.at_n.1 + 1, "premise: the pickup records one grant");
    assert_eq!(run.restored_wallet, run.at_n.0, "premise: the restore gives back the wallet of frame N");
    assert_eq!(run.after_replay.0, run.after_pickup.0, "the replay's wallet is not the first run's");
    assert_eq!(run.after_replay.1, run.after_pickup.1, "the replayed pickup's grant is recorded twice");
}

/// What the ledgers say of the one-time heart: (its occurrence row, whether
/// the record of consumptions since the checkpoint names it).
type HeartRows = (Option<String>, bool);

/// ISL-2d, a keyed store: Alice takes the one-time heart (authored `Never`)
/// in `basement_breakables` while Bob stays in the hub, so the occurrence
/// ledger writes it `Consumed` and the record of consumptions names it. The
/// room rewinds to before the pickup and replays WITHOUT taking it: the
/// corrected input differs, as a late input's does. Returns the rows at N,
/// after the pickup, and after the replay.
fn a_heart_taken_only_before_the_rewind(retract: bool) -> [HeartRows; 3] {
    use crate::pickup_regrowth_across_rooms::{collect, HUB, PLAIN, ROOM as HEART_ROOM};
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        HEART_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    assert_ne!(room, bobs, "premise: Alice and Bob are in two live rooms");
    let heart = {
        let world = sim.world_mut();
        world
            .query::<(&ambition_platformer2d::combat::components::FeatureName, &SimId)>()
            .iter(world)
            .find(|(name, _)| name.0.as_str() == PLAIN)
            .map(|(_, sim_id)| sim_id.clone())
            .expect("the room authors the one-time heart")
    };
    let read = |sim: &Platformer2dSimHarness| -> HeartRows {
        let world = sim.world();
        (
            world
                .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
                .whereabouts(&heart)
                .map(|row| format!("{row:?}")),
            world
                .resource::<ambition_platformer2d::item_runtime::pickups::ConsumedSinceCheckpoint>()
                .owners(&heart)
                .is_some(),
        )
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_rows = read(&sim);

    collect(&mut sim, PLAIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let after_pickup = read(&sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    // The replay: Alice stands still, away from the heart.
    for _ in 0..20 {
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = read(&sim);
    sim.world_mut().insert_resource(rows);
    [at_n_rows, after_pickup, after_replay]
}

/// ISL-2d: a room that rewinds puts back the terminal rows it wrote after the
/// tick it was saved at, so a pickup taken only in the mispredicted future is
/// not remembered taken. Its control is the same run without the retraction.
#[test]
fn a_rewound_room_untakes_the_heart_its_replay_does_not_take() {
    let [at_n, after_pickup, after_replay] = a_heart_taken_only_before_the_rewind(true);
    assert_eq!(at_n, (None, false), "premise: the heart is untaken at N");
    assert_eq!(after_pickup, (Some("Consumed".into()), true), "premise: the pickup writes both ledgers");
    assert_eq!(after_replay, (None, false), "the replay did not take the heart, and the ledgers still say it did");
}

/// The control: without the retraction the rows written in the window stay,
/// although the replay did not take the heart. Run it beside the witness when
/// the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_untaken_heart_stays_taken() {
    let [_, _, after_replay] = a_heart_taken_only_before_the_rewind(false);
    assert_eq!(after_replay, (Some("Consumed".into()), true), "the rows went without the retraction");
}

/// What the bag says of the thrown item: (its count in the bag, the spends
/// since the checkpoint, the occurrence ledger's rows).
type BagReading = (u32, usize, Vec<String>);

/// ISL-2d, the shared bag: Alice holds a gun sword out of the bag in
/// `basement_breakables` while Bob stays in the hub, and throws it in the
/// window, which spends one from the bag and mints an object. The room
/// rewinds to before the throw and replays WITHOUT it. Returns the readings
/// at N, after the throw, and after the replay.
fn a_throw_made_only_before_the_rewind(retract: bool) -> [BagReading; 3] {
    use crate::pickup_regrowth_across_rooms::{HUB, ROOM as THROW_ROOM};
    use ambition_platformer2d::item::{Item, OwnedItems};
    use bevy::ecs::system::RunSystemOnce;
    const ITEM: Item = Item::GunSword;
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        THROW_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    assert_ne!(room, bobs, "premise: Alice and Bob are in two live rooms");
    sim.world_mut()
        .run_system_once(
            move |mut commands: bevy::prelude::Commands,
                  items: ambition_platformer2d::items::ItemCatalogRead,
                  mut bodies: bevy::prelude::Query<
                (Entity, ambition_platformer2d::combat::hand::RepertoireQuery),
                ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
            >| {
                let (player, mut repertoire) = bodies.single_mut().expect("one primary body");
                let spec = ambition_platformer2d::held_items::held_spec_for_item(items.get(), ITEM)
                    .expect("the item is a wired weapon with a held spec");
                ambition_platformer2d::held_items::equip_held_spec(&mut commands, player, &mut repertoire, spec);
            },
        )
        .expect("the equip verb runs");
    for _ in 0..4 {
        sim.step(crate::common::base());
    }
    let read = |sim: &Platformer2dSimHarness| -> BagReading {
        let world = sim.world();
        (
            world.resource::<OwnedItems>().count(ITEM),
            world.resource::<ambition_platformer2d::held_items::BagSpendsSinceCheckpoint>().spends().len(),
            world
                .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
                .rows()
                .map(|(id, row)| format!("{id} {row:?}"))
                .collect(),
        )
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_reading = read(&sim);

    sim.step_frame(ControlFrame { attack_pressed: true, shield_held: true, ..ControlFrame::default() });
    for _ in 0..30 {
        sim.step(crate::common::base());
    }
    let after_throw = read(&sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    // The replay: Alice holds the sword and does not throw it.
    for _ in 0..30 {
        sim.step(crate::common::base());
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = read(&sim);
    sim.world_mut().insert_resource(rows);
    [at_n_reading, after_throw, after_replay]
}

/// ISL-2d, the shared bag: a room that rewinds puts back in the bag what it
/// threw after the tick it was saved at, and takes the thrown object out of
/// the occurrence ledger. Without the ledger half, the replay found the
/// object's `Placed` row with no object and wrote it `Consumed`: a mint that
/// never existed, remembered ended.
#[test]
fn a_rewound_room_puts_back_the_throw_its_replay_does_not_make() {
    let [at_n, after_throw, after_replay] = a_throw_made_only_before_the_rewind(true);
    assert_eq!((at_n.0, at_n.1, at_n.2.len()), (1, 0, 0), "premise: one in the bag, nothing spent, no rows at N");
    assert_eq!((after_throw.0, after_throw.1, after_throw.2.len()), (0, 1, 1), "premise: the throw spends one and places a mint");
    assert_eq!(after_replay, at_n, "the replay did not throw, and the bag, the spends or the ledger say it did");
}

/// The control: without the retraction the bag stays spent. Run it beside the
/// witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_throw_stays_spent() {
    let [_, _, after_replay] = a_throw_made_only_before_the_rewind(false);
    assert_eq!((after_replay.0, after_replay.1), (0, 1), "the bag came back without the retraction");
    assert_eq!(after_replay.2.len(), 1, "the mint's row went without the retraction: {:?}", after_replay.2);
}

/// What the world says of the boss: (its save row cleared, alive, defeats
/// since the checkpoint, the occurrence ledger's rows).
type BossReading = (bool, Option<bool>, usize, usize);

/// ISL-2d, a keyed store one room owns: a mockingbird in Alice's room
/// (`basement_breakables`, Bob in the hub) is killed in the window, which
/// clears its save row and records the defeat. The room rewinds to before the
/// kill and replays WITHOUT it. Returns the readings at N, after the kill, and
/// after the replay.
fn a_boss_killed_only_before_the_rewind(retract: bool) -> [BossReading; 3] {
    use crate::boss_lifecycle::{boss_alive, boss_cleared, force_kill_boss, spawn_mockingbird};
    use crate::pickup_regrowth_across_rooms::{HUB, ROOM as BOSS_ROOM};
    const BOSS: &str = "alices_rewound_boss";
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        BOSS_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    assert_ne!(room, bobs, "premise: Alice and Bob are in two live rooms");
    spawn_mockingbird(&mut sim, BOSS);
    sim.step(crate::common::base());
    {
        let world = sim.world_mut();
        let boss = world
            .query::<(Entity, &ambition_platformer2d::boss_encounter::BossConfig)>()
            .iter(world)
            .find(|(_, config)| config.id == BOSS)
            .map(|(entity, _)| entity)
            .expect("the boss reached the world");
        world.entity_mut(boss).insert(InRoomInstance(room));
    }
    for _ in 0..15 {
        sim.step(crate::common::base());
    }
    let read = |sim: &mut Platformer2dSimHarness| -> BossReading {
        (
            boss_cleared(sim, BOSS),
            boss_alive(sim.world_mut(), BOSS),
            sim.world().resource::<ambition_platformer2d::boss_encounter::BossDefeatsSinceCheckpoint>().defeats().count(),
            sim.world().resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>().rows().count(),
        )
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_reading = read(&mut sim);

    force_kill_boss(&mut sim, BOSS);
    for _ in 0..120 {
        sim.step(crate::common::base());
    }
    let after_kill = read(&mut sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    // The replay: nobody kills the boss.
    for _ in 0..30 {
        sim.step(crate::common::base());
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = read(&mut sim);
    sim.world_mut().insert_resource(rows);
    [at_n_reading, after_kill, after_replay]
}

/// ISL-2d: a room that rewinds takes back the boss defeat it recorded after
/// the tick it was saved at, with its save row. Without it (the control) the
/// restored boss stays dead: its row still says `Cleared`.
#[test]
fn a_rewound_room_takes_back_the_boss_defeat_its_replay_does_not_make() {
    let [at_n, after_kill, after_replay] = a_boss_killed_only_before_the_rewind(true);
    assert_eq!(at_n, (false, Some(true), 0, 0), "premise: the boss is alive and uncleared at N");
    assert_eq!((after_kill.0, after_kill.1, after_kill.2), (true, Some(false), 1), "premise: the kill clears it and records the defeat");
    assert_eq!(after_replay, at_n, "the replay did not kill the boss, and the world says it did");
}

/// The control: without the retraction the defeat and its row stay. Run it
/// beside the witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_boss_stays_defeated() {
    let [_, _, after_replay] = a_boss_killed_only_before_the_rewind(false);
    assert_eq!((after_replay.0, after_replay.2), (true, 1), "the defeat went without the retraction");
}

/// ISL-2d, the world-time schedule: (the regrowing heart's due time, the
/// clock at its collection in the window, the clock at its collection in the
/// replay). The window takes it late; the replay takes it on its first steps.
fn a_regrowth_taken_at_another_time_in_the_replay(retract: bool) -> (Option<f32>, f32, f32) {
    use crate::pickup_regrowth_across_rooms::{collect, HEART, HUB, ROOM as HEART_ROOM};
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        HEART_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());

    for _ in 0..40 {
        sim.step(crate::common::base());
    }
    let taken_in_window = collect(&mut sim, HEART);
    for _ in 0..5 {
        sim.step(crate::common::base());
    }

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    let taken_in_replay = collect(&mut sim, HEART);
    for _ in 0..5 {
        sim.step(crate::common::base());
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let feature = heart_feature(&mut sim);
    let due = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world_returns::WorldTimeSchedule,
    >(sim.world())
    .expect("the session root carries the schedule")
    .due(HEART_ROOM, &feature);
    sim.world_mut().insert_resource(rows);
    (due, taken_in_window, taken_in_replay)
}

/// The authored id of the regrowing heart, the schedule's key.
fn heart_feature(sim: &mut Platformer2dSimHarness) -> String {
    let world = sim.world_mut();
    world
        .query::<(&ambition_platformer2d::combat::components::FeatureName, &ambition_platformer2d::combat::components::FeatureId)>()
        .iter(world)
        .find(|(name, _)| name.0.as_str() == crate::pickup_regrowth_across_rooms::HEART)
        .map(|(_, id)| id.as_str().to_string())
        .expect("the room authors the regrowing heart")
}

/// ISL-2d: a room that rewinds puts back the due times it wrote after the
/// tick it was saved at. The replay takes the regrowing heart (4 s) at
/// another time than the window did, on its first steps, before the mirror
/// could forget the window's record; without the retraction (the control)
/// the record keeps the window's due time, measured 2026-11-10: 6.22 s for a
/// heart the replay took at 1.57 s.
#[test]
fn a_rewound_room_keeps_the_regrowth_time_of_its_replay() {
    let (due, window, replay) = a_regrowth_taken_at_another_time_in_the_replay(true);
    assert!(window - replay > 0.3, "premise: the replay takes the heart earlier than the window: {replay} against {window}");
    let due = due.expect("the replay's take schedules a regrowth");
    assert!((due - replay - 4.0).abs() < 0.1, "the regrowth is due at {due}, not 4 s after the replay's take at {replay}");
}

/// The control: without the retraction the window's due time stays. Run it
/// beside the witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_window_s_regrowth_time_stays() {
    let (due, window, _) = a_regrowth_taken_at_another_time_in_the_replay(false);
    let due = due.expect("a regrowth is scheduled");
    assert!((due - window - 4.0).abs() < 0.1, "the window's due time went without the retraction: {due} for a take at {window}");
}

/// ISL-2d, a cell used from the shared bag: Alice (in `basement_breakables`,
/// Bob in the hub) uses a health cell in the window. The room rewinds to
/// before it and replays without the use. Returns the bag's health cells at
/// N, after the use, and after the replay.
fn a_cell_used_only_before_the_rewind(retract: bool) -> [u32; 3] {
    use crate::pickup_regrowth_across_rooms::{HUB, ROOM as CELL_ROOM};
    use ambition_platformer2d::item::{Item, OwnedItems};
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        CELL_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    let cells = |sim: &Platformer2dSimHarness| sim.world().resource::<OwnedItems>().count(Item::HealthCell);

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_cells = cells(&sim);

    sim.world_mut().write_message(ambition_platformer2d::items::ItemUseRequested {
        participant: ambition_platformer2d::characters::control::PlayerSlot(0),
        item: Item::HealthCell,
    });
    for _ in 0..10 {
        sim.step(crate::common::base());
    }
    let after_use = cells(&sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    for _ in 0..10 {
        sim.step(crate::common::base());
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = cells(&sim);
    sim.world_mut().insert_resource(rows);
    [at_n_cells, after_use, after_replay]
}

/// ISL-2d: a room that rewinds puts back in the bag the cells its bodies
/// used after the tick it was saved at.
#[test]
fn a_rewound_room_puts_back_the_cell_its_replay_does_not_use() {
    let [at_n, after_use, after_replay] = a_cell_used_only_before_the_rewind(true);
    assert_eq!(after_use + 1, at_n, "premise: the use spends one cell (at N {at_n}, after {after_use})");
    assert_eq!(after_replay, at_n, "the replay did not use the cell, and the bag says it did");
}

/// The control: without the retraction the cell stays spent. Run it beside
/// the witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_cell_stays_spent() {
    let [_, after_use, after_replay] = a_cell_used_only_before_the_rewind(false);
    assert_eq!(after_replay, after_use, "the cell came back without the retraction");
}

/// Not a shipped flag: the world sets no flag of this name.
const WITNESS_FLAG: &str = "isl2d_a_room_rewinds_alone_flag";
/// A quest whose first step waits on [`WITNESS_FLAG`].
const WITNESS_QUEST: &str = "isl2d_a_room_rewinds_alone_quest";

/// What the save and the quests say of the witness flag: (the flag, the
/// quest's step, the save's record of it).
type FlagReading = (bool, u8, u8);

/// ISL-2d, the save's flags and the quests they move: Alice (in
/// `basement_breakables`, Bob in the hub) takes a story-flag pickup in the
/// window, which sets a flag and moves a quest's step on. The room rewinds to
/// before the take and replays WITHOUT it. Returns the readings at N, after
/// the take, and after the replay.
fn a_flag_set_only_before_the_rewind(retract: bool) -> [FlagReading; 3] {
    use crate::pickup_regrowth_across_rooms::{collect, HUB, PLAIN, ROOM as FLAG_ROOM};
    use ambition_platformer2d::persistence::quest::{QuestRegistry, QuestSpec, QuestStepCondition, QuestStepSpec};
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        FLAG_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    // The one-time heart becomes a story flag, and a quest waits on it.
    {
        let world = sim.world_mut();
        let mut pickups = world.query::<(
            &ambition_platformer2d::combat::components::FeatureName,
            &mut ambition_platformer2d::item_runtime::pickups::PickupFeature,
        )>();
        let (_, mut pickup) = pickups
            .iter_mut(world)
            .find(|(name, _)| name.0.as_str() == PLAIN)
            .expect("the room authors the one-time heart");
        pickup.pickup.kind = ambition_platformer2d::interaction::PickupKind::StoryFlag { flag: WITNESS_FLAG.into() };
        let mut quests = world.resource_mut::<QuestRegistry>();
        quests.ensure(QuestSpec::new(
            WITNESS_QUEST,
            "Witness",
            "a quest a room's flag moves",
            vec![
                QuestStepSpec::new("Take it.", QuestStepCondition::FlagSet(WITNESS_FLAG.into())),
                QuestStepSpec::new("Then nothing.", QuestStepCondition::FlagSet("isl2d_never_set".into())),
            ],
        ));
        quests.start(WITNESS_QUEST);
    }
    let read = |sim: &Platformer2dSimHarness| -> FlagReading {
        let world = sim.world();
        let save = world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
        (
            save.data().flag(WITNESS_FLAG),
            world.resource::<QuestRegistry>().get(WITNESS_QUEST).expect("ensured above").step,
            save.data().quest(WITNESS_QUEST).1,
        )
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_reading = read(&sim);

    collect(&mut sim, PLAIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let after_take = read(&sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    // The replay: Alice stands still, away from the pickup.
    for _ in 0..20 {
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = read(&sim);
    sim.world_mut().insert_resource(rows);
    [at_n_reading, after_take, after_replay]
}

/// ISL-2d: a room that rewinds puts back the save flags it wrote after the
/// tick it was saved at, and the quest steps those flags moved.
#[test]
fn a_rewound_room_puts_back_the_flag_and_quest_step_its_replay_does_not_set() {
    let [at_n, after_take, after_replay] = a_flag_set_only_before_the_rewind(true);
    assert!(!at_n.0 && at_n.1 == 0, "premise: the flag is clear and the quest at its first step at N ({at_n:?})");
    assert!(after_take.0 && after_take.1 == 1, "premise: the take sets the flag and moves the quest ({after_take:?})");
    assert_eq!(after_take.2, 1, "premise: the save records the quest's step");
    assert_eq!(after_replay, (false, 0, 0), "the replay did not take the pickup, and the save says it did");
}

/// The control: without the retraction the flag and the step stay. Run it
/// beside the witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_flag_stays_set() {
    let [_, after_take, after_replay] = a_flag_set_only_before_the_rewind(false);
    assert_eq!(after_replay, after_take, "the flag went without the retraction");
}

/// A quest whose first step waits on the goblin wave's clear.
const CLEAR_QUEST: &str = "isl2d_a_room_rewinds_alone_clear_quest";

/// What the save and the quests say of the goblin wave: (its linked switches
/// that are on, of how many; the quest's step).
type ClearReading = (usize, usize, u8);

/// ISL-2d, a wave encounter's clear: Alice (in `goblin_encounter`, Bob in the
/// hub) clears the goblin wave in the window, which greens its linked
/// switches in the save and moves a quest's step on. The clear is the
/// encounter's `Completed` event, written as the reducer would: the effects
/// it has on the shared stores are what is under test. The room rewinds to
/// before it and replays WITHOUT it. Returns the readings at N, after the
/// clear, and after the replay.
fn a_wave_cleared_only_before_the_rewind(retract: bool) -> [ClearReading; 3] {
    use crate::pickup_regrowth_across_rooms::HUB;
    use ambition_platformer2d::persistence::quest::{QuestRegistry, QuestSpec, QuestStepCondition, QuestStepSpec};
    const ARENA: &str = "goblin_encounter";
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        ARENA,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::common::walk_through_the_door_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    assert_ne!(room, bobs, "premise: Alice and Bob are in two live rooms");
    {
        let mut quests = sim.world_mut().resource_mut::<QuestRegistry>();
        quests.ensure(QuestSpec::new(
            CLEAR_QUEST,
            "Witness",
            "a quest a room's clear moves",
            vec![
                QuestStepSpec::new("Clear it.", QuestStepCondition::EncounterCleared(ARENA.into())),
                QuestStepSpec::new("Then nothing.", QuestStepCondition::FlagSet("isl2d_never_set".into())),
            ],
        ));
        quests.start(CLEAR_QUEST);
    }
    let read = |sim: &Platformer2dSimHarness| -> ClearReading {
        let world = sim.world();
        let switches = world
            .resource::<ambition_platformer2d::encounter::switches::EncounterSwitchIndex>()
            .switch_ids_for_encounter(ARENA);
        let save = world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
        (
            switches.iter().filter(|switch| save.data().switch(switch)).count(),
            switches.len(),
            world.resource::<QuestRegistry>().get(CLEAR_QUEST).expect("ensured above").step,
        )
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_reading = read(&sim);

    sim.world_mut().write_message(ambition_platformer2d::encounter::EncounterEventMsg {
        encounter: ARENA.into(),
        room: Some(room),
        event: ambition_platformer2d::encounter::EncounterEvent::Completed,
    });
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let after_clear = read(&sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    for _ in 0..20 {
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = read(&sim);
    sim.world_mut().insert_resource(rows);
    [at_n_reading, after_clear, after_replay]
}

/// ISL-2d: a room that rewinds puts back the switches its clear greened after
/// the tick it was saved at, and the quest step the clear moved.
#[test]
fn a_rewound_room_puts_back_the_wave_clear_its_replay_does_not_make() {
    let [at_n, after_clear, after_replay] = a_wave_cleared_only_before_the_rewind(true);
    assert!(at_n.1 > 0, "premise: the goblin wave has linked switches ({at_n:?})");
    assert_eq!((at_n.0, at_n.2), (0, 0), "premise: no switch on and the quest at its first step at N ({at_n:?})");
    assert_eq!((after_clear.0, after_clear.2), (after_clear.1, 1), "premise: the clear greens every switch and moves the quest");
    assert_eq!((after_replay.0, after_replay.2), (0, 0), "the replay did not clear the wave, and the save says it did");
}

/// The control: without the retraction the switches and the step stay. Run
/// it beside the witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_wave_stays_cleared() {
    let [_, after_clear, after_replay] = a_wave_cleared_only_before_the_rewind(false);
    assert_eq!(after_replay, after_clear, "the clear went without the retraction");
}

/// A quest whose first step waits on the one-time heart's collection.
const COLLECT_QUEST: &str = "isl2d_a_room_rewinds_alone_collect_quest";

/// ISL-2d, a quest event that is not a flag: Alice (in `basement_breakables`,
/// Bob in the hub) takes the one-time heart in the window, which queues its
/// `ItemCollected` and moves a quest's step on. The room rewinds to before
/// the take and replays WITHOUT it. Returns the quest's step, and the save's
/// record of it, at N, after the take, and after the replay.
fn an_item_collected_only_before_the_rewind(retract: bool) -> [(u8, u8); 3] {
    use crate::pickup_regrowth_across_rooms::{collect, HUB, PLAIN, ROOM as HEART_ROOM};
    use ambition_platformer2d::persistence::quest::{QuestRegistry, QuestSpec, QuestStepCondition, QuestStepSpec};
    let (mut sim, _) = crate::two_players_two_live_rooms::alice_leaves_bob_in(
        HUB,
        HEART_ROOM,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
        crate::breakable_respawn_across_rooms::cross_to,
    );
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let (room, bobs) = where_they_are(&mut sim);
    let (room, bobs) = (room.expect("Alice has a room"), bobs.flatten().expect("Bob has a room"));
    assert_ne!(room, bobs, "premise: Alice and Bob are in two live rooms");
    {
        let world = sim.world_mut();
        let id = world
            .query::<(
                &ambition_platformer2d::combat::components::FeatureName,
                &ambition_platformer2d::item_runtime::pickups::PickupFeature,
            )>()
            .iter(world)
            .find(|(name, _)| name.0.as_str() == PLAIN)
            .map(|(_, pickup)| pickup.pickup.id.clone())
            .expect("the room authors the one-time heart");
        let mut quests = world.resource_mut::<QuestRegistry>();
        quests.ensure(QuestSpec::new(
            COLLECT_QUEST,
            "Witness",
            "a quest a room's pickup moves",
            vec![
                QuestStepSpec::new("Take it.", QuestStepCondition::ItemCollected(id)),
                QuestStepSpec::new("Then nothing.", QuestStepCondition::FlagSet("isl2d_never_set".into())),
            ],
        ));
        quests.start(COLLECT_QUEST);
    }
    let read = |sim: &Platformer2dSimHarness| -> (u8, u8) {
        let world = sim.world();
        (
            world.resource::<QuestRegistry>().get(COLLECT_QUEST).expect("ensured above").step,
            world.resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>().data().quest(COLLECT_QUEST).1,
        )
    };

    let rows = sim.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let entities = of_room(sim.world_mut(), room);
    let mut saved = rows.save(sim.world(), &entities);
    let at_n = clock(sim.world());
    let at_n_reading = read(&sim);

    collect(&mut sim, PLAIN);
    for _ in 0..10 {
        sim.app_mut().update();
    }
    let after_take = read(&sim);

    let hidden = of_room(sim.world_mut(), bobs);
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).insert(Disabled);
    }
    let now = of_room(sim.world_mut(), room);
    rows.restore(sim.world_mut(), &mut saved, &now);
    set_clock(sim.world_mut(), at_n);
    if retract {
        ambition_platformer2d::platformer::lifecycle::retract_room_writes(sim.world_mut(), room, at_n.0 .0);
    }
    // The replay: Alice stands still, away from the heart.
    for _ in 0..20 {
        sim.app_mut().update();
    }
    for entity in &hidden {
        sim.world_mut().entity_mut(*entity).remove::<Disabled>();
    }
    let after_replay = read(&sim);
    sim.world_mut().insert_resource(rows);
    [at_n_reading, after_take, after_replay]
}

/// ISL-2d: a room that rewinds retracts the quest steps its pickups' events
/// moved after the tick it was saved at.
#[test]
fn a_rewound_room_puts_back_the_quest_step_of_the_pickup_its_replay_does_not_take() {
    let [at_n, after_take, after_replay] = an_item_collected_only_before_the_rewind(true);
    assert_eq!(at_n.0, 0, "premise: the quest is at its first step at N");
    assert_eq!(after_take, (1, 1), "premise: the take moves the quest, and the save records it");
    assert_eq!(after_replay, (0, 0), "the replay did not take the heart, and the quest says it did");
}

/// The control: without the retraction the step stays. Run it beside the
/// witness when the retraction changes; it is not a suite member.
#[test]
#[ignore = "control for the witness above; run with --ignored when the retraction changes"]
fn without_the_retraction_the_collected_step_stays() {
    let [_, after_take, after_replay] = an_item_collected_only_before_the_rewind(false);
    assert_eq!(after_replay, after_take, "the step went without the retraction");
}
