//! SIMULATION-ISLANDS ISL-1 and ISL-2a: which rollback state belongs to one
//! room, and what saving one room costs.
//!
//! The question for keeping rollback inside a room while the world around it
//! is hosted: how much of what the snapshot holds today has a room, and how
//! much has none. Two rooms are live (Alice crossed to the hub, Bob stayed in
//! `switch_lab`). For every registered rollback component type this counts the
//! entities carrying it in each live room and with no room
//! ([`InRoomInstance`] absent), and groups the roomless entities by what they
//! are. The resources and message buffers have no entity; their owners are
//! classified in the owner document, not here.
//!
//! Under the rollback session it also times one App-wide `SaveWorld` and, for
//! each room, a save and restore of that room's entities alone through
//! [`RoomSnapshotRows`] (ISL-2a).
//!
//! A measurement, not a contract: it prints and asserts only its premises.
//! The owner is `docs/planning/engine/island-rollback-and-compact-peer-input.md`.

use std::collections::{BTreeMap, BTreeSet};

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_platformer2d::rollback::{RollbackEntryKind, RollbackRegistry, RoomSnapshotRows, SaveWorld};
use ambition_platformer2d::engine_core::BodyKinematics;
use std::time::Instant;
use bevy::ecs::component::ComponentId;
use bevy::prelude::Entity;

use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_leaves, alice_on_the_hub_door};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

fn is_component_row(kind: RollbackEntryKind) -> bool {
    matches!(
        kind,
        RollbackEntryKind::ComponentCanonical
            | RollbackEntryKind::ComponentCloneCursor
            | RollbackEntryKind::ComponentCloneResolved
            | RollbackEntryKind::ComponentClone
            | RollbackEntryKind::ComponentCloneCanonicalChecksum
            | RollbackEntryKind::ComponentCloneCustomChecksum
            | RollbackEntryKind::ComponentCanonicalCustomChecksum
    )
}

/// Where an entity is: a live room, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Place {
    Room(LiveRoomInstance),
    Roomless,
}

#[test]
#[ignore = "measurement: prints which rollback state has a room; run with --ignored --nocapture --test-threads=1"]
fn which_rollback_state_belongs_to_one_room() {
    println!("\n[census] === plain host (no rollback session)");
    census(fixed_60hz_room_options(ROOM));
    // The control: under a rollback session `Rollback` marks what the snapshot
    // takes, so an entity marked with no registered row is visible.
    println!("[census] === sync-test rollback session");
    census(fixed_60hz_room_options(ROOM).with_sync_test_rollback_settings(2, 10).with_rollback_players(2));
}

fn census(options: ambition_app::Platformer2dSimHarnessOptions) {
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    alice_on_the_hub_door(&mut sim);
    for frame in 1..=120 {
        sim.drive_seat(0, alice_leaves(frame));
        sim.drive_seat(1, ControlFrame::default());
        sim.app_mut().update();
    }
    let (alice, bob) = where_they_are(&mut sim);
    let (alice, bob) = (alice.expect("Alice has a room"), bob.flatten().expect("Bob has a room"));
    assert_ne!(alice, bob, "premise: Alice and Bob are in two live rooms");

    let world = sim.world_mut();
    let rows: Vec<(String, String)> = world
        .resource::<RollbackRegistry>()
        .descriptors()
        .filter(|row| is_component_row(row.kind))
        .map(|row| (row.name.clone(), row.type_name.clone()))
        .collect();
    let by_name: BTreeMap<String, ComponentId> =
        world.components().iter_registered().map(|info| (info.name().to_string(), info.id())).collect();
    let in_room_id = world.component_id::<InRoomInstance>().expect("InRoomInstance is registered");
    let rollback_id = by_name.iter().find(|(name, _)| name.starts_with("bevy_ggrs::") && name.ends_with("::Rollback")).map(|(_, id)| *id);

    let mut unmatched = Vec::new();
    let ids: Vec<(String, ComponentId)> = rows
        .iter()
        .filter_map(|(name, type_name)| match by_name.get(type_name) {
            Some(id) => Some((name.clone(), *id)),
            None => {
                unmatched.push(format!("{name} ({type_name})"));
                None
            }
        })
        .collect();

    // Every entity carrying a registered row, with where it is and what it carries.
    let mut entities: BTreeMap<Entity, (Place, Vec<&str>)> = BTreeMap::new();
    let mut rollback_marked = 0usize;
    let mut marked_without_a_row = 0usize;
    for archetype in world.archetypes().iter() {
        let carried: Vec<&str> =
            ids.iter().filter(|(_, id)| archetype.contains(*id)).map(|(name, _)| name.as_str()).collect();
        if rollback_id.is_some_and(|id| archetype.contains(id)) {
            rollback_marked += archetype.len() as usize;
            if carried.is_empty() {
                marked_without_a_row += archetype.len() as usize;
            }
        }
        if carried.is_empty() {
            continue;
        }
        for entity in archetype.entities() {
            let entity = entity.id();
            let place = if archetype.contains(in_room_id) {
                world.get::<InRoomInstance>(entity).map_or(Place::Roomless, |at| Place::Room(at.0))
            } else {
                Place::Roomless
            };
            entities.insert(entity, (place, carried.clone()));
        }
    }

    let label = |place: Place| match place {
        Place::Room(room) if room == alice => "hub (Alice)".to_string(),
        Place::Room(room) if room == bob => "switch_lab (Bob)".to_string(),
        Place::Room(room) => format!("room {room:?}"),
        Place::Roomless => "no room".to_string(),
    };
    let mut per_place: BTreeMap<Place, usize> = BTreeMap::new();
    for (place, _) in entities.values() {
        *per_place.entry(*place).or_default() += 1;
    }
    println!("[census] {} component rows, {} matched to a live type; {} entities carry one ({} carry Rollback)", rows.len(), ids.len(), entities.len(), rollback_marked);
    println!("[census] entities marked Rollback with no registered component row: {marked_without_a_row}");
    for (place, count) in &per_place {
        println!("[census]   {:<18} {count} entities", label(*place));
    }

    // Per row: entities in each place.
    let mut per_row: BTreeMap<&str, BTreeMap<Place, usize>> = ids.iter().map(|(name, _)| (name.as_str(), BTreeMap::new())).collect();
    for (place, carried) in entities.values() {
        for name in carried {
            *per_row.get_mut(name).expect("a matched row").entry(*place).or_default() += 1;
        }
    }
    let empty: Vec<&str> = per_row.iter().filter(|(_, places)| places.is_empty()).map(|(name, _)| *name).collect();
    let roomed_only: Vec<&str> = per_row
        .iter()
        .filter(|(_, places)| !places.is_empty() && !places.contains_key(&Place::Roomless))
        .map(|(name, _)| *name)
        .collect();
    let roomless_only: Vec<&str> = per_row
        .iter()
        .filter(|(_, places)| places.len() == 1 && places.contains_key(&Place::Roomless))
        .map(|(name, _)| *name)
        .collect();
    let mixed: Vec<&str> = per_row
        .iter()
        .filter(|(_, places)| places.len() > 1 && places.contains_key(&Place::Roomless))
        .map(|(name, _)| *name)
        .collect();
    println!("[census] rows on no entity in this scene: {}", empty.len());
    println!("[census] rows only on entities in a room: {}", roomed_only.len());
    println!("[census] rows only on roomless entities: {} {roomless_only:?}", roomless_only.len());
    println!("[census] rows on both: {} {mixed:?}", mixed.len());
    if !unmatched.is_empty() {
        println!("[census] rows with no live type of that name: {} {unmatched:?}", unmatched.len());
    }

    // The roomless entities, grouped by the rows they carry.
    let mut kinds: BTreeMap<BTreeSet<&str>, usize> = BTreeMap::new();
    for (place, carried) in entities.values() {
        if *place == Place::Roomless {
            *kinds.entry(carried.iter().copied().collect()).or_default() += 1;
        }
    }
    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    println!("[census] roomless entities by the rows they carry ({} kinds):", kinds.len());
    for (carried, count) in kinds {
        let shown: Vec<&str> = carried.iter().copied().take(8).collect();
        let more = carried.len().saturating_sub(shown.len());
        println!("[census]   {count:>4} x {} rows: {shown:?}{}", carried.len(), if more > 0 { format!(" +{more}") } else { String::new() });
    }
    assert!(!ids.is_empty(), "premise: the registry's component rows resolve to live types");

    // What one snapshot of this world costs, beside what it holds.
    if rollback_marked > 0 {
        let systems = sim.app_mut().get_schedule_mut(SaveWorld).map_or(0, |schedule| schedule.systems_len());
        let runs = 60;
        let start = std::time::Instant::now();
        for _ in 0..runs {
            sim.world_mut().run_schedule(SaveWorld);
        }
        let ms = start.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
        println!("[census] one SaveWorld run: {ms:.2} ms over {systems} systems, for {rollback_marked} entities");
        room_snapshots(&mut sim, [("hub (Alice)", alice), ("switch_lab (Bob)", bob)]);
    }
}

/// ISL-2a: what one room's save and restore cost through the registered rows,
/// beside the App-wide `SaveWorld` above.
fn room_snapshots(sim: &mut Platformer2dSimHarness, rooms: [(&str, LiveRoomInstance); 2]) {
    let world = sim.world_mut();
    let rows = world.remove_resource::<RoomSnapshotRows>().expect("a rollback host records room rows");
    let rollback = world
        .components()
        .iter_registered()
        .find(|info| {
            let name = info.name().to_string();
            name.starts_with("bevy_ggrs::") && name.ends_with("::Rollback")
        })
        .map(|info| info.id())
        .expect("bevy_ggrs::Rollback is registered");
    println!("[census] room rows recorded: {}", rows.len());
    for (label, room) in rooms {
        let entities: Vec<Entity> = world
            .query::<(Entity, &InRoomInstance)>()
            .iter(world)
            .filter(|(entity, at)| at.0 == room && world.entity(*entity).contains_id(rollback))
            .map(|(entity, _)| entity)
            .collect();

        // Premise: a restore puts a changed value back.
        let snapshot = rows.save(world, &entities);
        let body = entities
            .iter()
            .copied()
            .find(|entity| world.get::<BodyKinematics>(*entity).is_some())
            .expect("the room has a body");
        let before = world.get::<BodyKinematics>(body).expect("a body").pos;
        world.get_mut::<BodyKinematics>(body).expect("a body").pos.x += 100.0;
        rows.restore(world, &snapshot);
        assert_eq!(world.get::<BodyKinematics>(body).expect("a body").pos, before, "{label}: the restore did not put the body back");

        let runs = 60;
        let start = Instant::now();
        let mut held = 0;
        for _ in 0..runs {
            held = rows.save(world, &entities).rows();
        }
        let save_ms = start.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
        let start = Instant::now();
        for _ in 0..runs {
            rows.restore(world, &snapshot);
        }
        let restore_ms = start.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
        println!(
            "[census] room snapshot {label}: {} entities, {held} rows; save {save_ms:.3} ms, restore {restore_ms:.3} ms",
            entities.len()
        );
    }
    world.insert_resource(rows);
}
