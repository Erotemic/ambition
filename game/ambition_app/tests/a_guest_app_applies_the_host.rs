//! NET-AUTHORITY-EXPERIMENT NA-2b: what a guest App costs that simulates
//! nothing and draws the host's bodies.
//!
//! The host is NA-2's: Alice and Bob in one App, Bob's input applied
//! [`LATENCY`] frames late. The guest is a second App booted to the same room
//! with the same two seats, so its bodies carry the host's peer-stable ids.
//! Every simulation phase in the guest is gated off. Each frame, it receives
//! the host's body rows from [`LATENCY`] frames earlier, writes them onto its
//! bodies by `SimId`, and updates.
//!
//! The probe prints:
//! - the guest's update cost and the cost of applying a snapshot;
//! - the host ids the guest does not hold (what a spawn by `SimId` would have
//!   to build) and the guest ids the host does not hold;
//! - whether a gated guest moved a body on its own;
//! - where a gated guest spends its update, and what it costs without the
//!   gated systems.
//!
//! A measurement, not a contract. The owner is
//! `docs/planning/engine/hosted-world-networking-experiment.md`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::sim::{Platformer2dSimulationPhase as Phase, SimScheduleExt as _};
use bevy::prelude::IntoScheduleConfigs as _;
use bevy::prelude::World;

use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_fights, bobs, bodies, UpdateTimes, LATENCY};
use crate::two_players_two_live_rooms::{bob_beside_alice, ROOM};

const FRAMES: i32 = 240;

/// The rooms: NA-2's two-body room, and the hub with its NPCs.
const ROOMS: [&str; 2] = [ROOM, "central_hub_complex"];

fn booted(room: &str) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(room)).expect("the room boots");
    bob_beside_alice(&mut sim, room, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    sim
}

/// The simulation phases a guest puts to sleep.
const GATED: [Phase; 12] = [
    Phase::CoreSimulation,
    Phase::FeatureCollection,
    Phase::FeatureInteraction,
    Phase::LdtkRuntimeSpine,
    Phase::EncounterSimulation,
    Phase::Cutscene,
    Phase::GameplayEffects,
    Phase::Progression,
    Phase::ResetProcessing,
    Phase::FeatureViewSync,
    Phase::PresentationVisualSync,
    Phase::Trace,
];

/// Put every simulation phase of `guest` to sleep.
fn simulate_nothing(guest: &mut Platformer2dSimHarness) {
    let app = guest.app_mut();
    let schedule = app.sim_schedule();
    for phase in GATED {
        app.configure_sets(schedule, phase.run_if(|| false));
    }
}

/// Write `rows` onto the guest's bodies by id. Returns the ids it does not hold.
fn apply(guest: &mut Platformer2dSimHarness, rows: &BTreeMap<SimId, [f32; 5]>) -> BTreeSet<SimId> {
    let world = guest.world_mut();
    let mut missing: BTreeSet<SimId> = rows.keys().cloned().collect();
    for (id, mut body) in world.query::<(&SimId, &mut BodyKinematics)>().iter_mut(world) {
        if let Some(row) = rows.get(id) {
            body.pos = bevy::math::Vec2::new(row[0], row[1]);
            body.vel = bevy::math::Vec2::new(row[2], row[3]);
            body.facing = row[4];
            missing.remove(id);
        }
    }
    missing
}

#[test]
#[ignore = "measurement: prints what a guest App that simulates nothing costs; run with --ignored --nocapture --test-threads=1"]
fn probe_what_a_guest_app_costs() {
    for room in ROOMS {
        guest_in(room);
    }
}

fn guest_in(room: &str) {
    let mut host = booted(room);
    let mut guest = booted(room);
    simulate_nothing(&mut guest);

    let mut host_times = UpdateTimes::default();
    let mut guest_times = UpdateTimes::default();
    let mut apply_time = std::time::Duration::ZERO;
    let mut in_flight: VecDeque<(i32, BTreeMap<SimId, [f32; 5]>)> = VecDeque::new();
    let mut not_on_the_guest: BTreeSet<SimId> = BTreeSet::new();
    let mut not_on_the_host: BTreeSet<SimId> = BTreeSet::new();
    let mut moved_on_its_own = 0;
    let mut applied = 0;

    for frame in 1..=FRAMES {
        host.drive_seat(0, alice_fights(frame));
        host.drive_seat(1, if frame > LATENCY { bobs(frame - LATENCY) } else { Default::default() });
        host_times.time(&mut host);
        in_flight.push_back((frame, bodies(&mut host, None)));

        if in_flight.front().is_some_and(|(sent, _)| *sent <= frame - LATENCY) {
            let (_, rows) = in_flight.pop_front().expect("a snapshot arrived");
            let started = std::time::Instant::now();
            not_on_the_guest.extend(apply(&mut guest, &rows));
            apply_time += started.elapsed();
            applied += 1;
            guest_times.time(&mut guest);
            let after = bodies(&mut guest, None);
            not_on_the_host.extend(after.keys().filter(|id| !rows.contains_key(*id)).cloned());
            moved_on_its_own += rows.iter().filter(|(id, row)| after.get(*id).is_some_and(|now| now != *row)).count();
        } else {
            guest_times.time(&mut guest);
        }
    }

    println!("[guest] {room}: {FRAMES} frames, a snapshot {LATENCY} frames late, {applied} applied");
    println!(
        "[guest]   host update {:.2} ms a frame; guest update {:.2} ms a frame (worst {:.1}); applying a snapshot {:.3} ms",
        host_times.mean_ms(),
        guest_times.mean_ms(),
        guest_times.worst.as_secs_f64() * 1e3,
        apply_time.as_secs_f64() * 1e3 / f64::from(applied.max(1))
    );
    println!("[guest]   host ids the guest does not hold: {} {not_on_the_guest:?}", not_on_the_guest.len());
    println!("[guest]   guest ids the host does not hold: {} {not_on_the_host:?}", not_on_the_host.len());
    println!("[guest]   bodies the gated guest moved after a snapshot: {moved_on_its_own}");
    println!(
        "[guest]   Bob's own input reaches his screen {} frames late: {LATENCY} to the host, {LATENCY} back, no interpolation",
        2 * LATENCY
    );
}

/// Where a gated guest spends its update: each schedule of `Main`, run in
/// `MainScheduleOrder`'s order and timed, with its system count (and the
/// fixed loop's own schedules). Close to `App::update`: sub-apps are not run.
#[test]
#[ignore = "measurement: prints where a gated guest's update goes; run with --ignored --nocapture --test-threads=1"]
fn probe_where_a_gated_guest_spends_its_update() {
    use bevy::app::{FixedMainScheduleOrder, MainScheduleOrder};
    use bevy::ecs::schedule::{InternedScheduleLabel, Schedules};
    const WARM: i32 = 30;
    const MEASURED: u32 = 120;
    for room in ROOMS {
        let mut guest = booted(room);
        simulate_nothing(&mut guest);
        for _ in 0..WARM {
            guest.app_mut().update();
        }
        let world = guest.world_mut();
        let main: Vec<InternedScheduleLabel> = world.resource::<MainScheduleOrder>().labels.clone();
        let fixed: Vec<InternedScheduleLabel> =
            world.get_resource::<FixedMainScheduleOrder>().map(|order| order.labels.clone()).unwrap_or_default();
        let count = |world: &World, label: InternedScheduleLabel| {
            world.resource::<Schedules>().get(label).map_or(0, |schedule| schedule.systems_len())
        };
        let mut spent: BTreeMap<String, std::time::Duration> = BTreeMap::new();
        let mut total = std::time::Duration::ZERO;
        for _ in 0..MEASURED {
            for label in &main {
                let started = std::time::Instant::now();
                let _ = world.try_run_schedule(*label);
                let took = started.elapsed();
                total += took;
                *spent.entry(format!("{label:?}")).or_default() += took;
            }
            world.clear_trackers();
        }
        println!("[guest-split] {room}: {MEASURED} updates, {:.2} ms an update", total.as_secs_f64() * 1e3 / f64::from(MEASURED));
        let mut rows: Vec<_> = main.iter().map(|label| (format!("{label:?}"), count(world, *label))).collect();
        rows.sort_by(|a, b| spent[&b.0].cmp(&spent[&a.0]));
        for (name, systems) in rows {
            println!(
                "[guest-split]   {name:<24} {:>6.3} ms  {systems:>5} systems",
                spent[&name].as_secs_f64() * 1e3 / f64::from(MEASURED)
            );
        }
        for label in &fixed {
            println!("[guest-split]   (fixed) {:<16} {:>5} systems", format!("{label:?}"), count(world, *label));
        }
        // How many of the sim schedule's systems the gate puts to sleep, and
        // how many still run.
        let sim = guest.app_mut().sim_schedule();
        let (gated, all, running) = guest.world_mut().resource_scope(|world, mut schedules: bevy::prelude::Mut<Schedules>| {
            let schedule = schedules.get_mut(sim).expect("the sim schedule exists");
            let _ = schedule.initialize(world);
            let mut gated = BTreeSet::new();
            for phase in GATED {
                if let Ok(systems) = schedule.graph().systems_in_set(bevy::ecs::schedule::SystemSet::intern(&phase)) {
                    gated.extend(systems.into_iter().map(|key| format!("{key:?}")));
                }
            }
            let mut running: BTreeMap<String, usize> = BTreeMap::new();
            if let Ok(systems) = schedule.systems() {
                for (key, system) in systems {
                    if gated.contains(&format!("{key:?}")) {
                        continue;
                    }
                    let name = system.name().to_string();
                    let owner = name.trim_start_matches('<').split("::").next().unwrap_or("?").to_string();
                    *running.entry(owner).or_default() += 1;
                }
            }
            (gated.len(), schedule.systems_len(), running)
        });
        println!("[guest-split]   the sim schedule: {all} systems, {gated} in a gated phase, {} still run", all - gated);
        let mut owners: Vec<_> = running.into_iter().collect();
        owners.sort_by(|a, b| b.1.cmp(&a.1));
        println!("[guest-split]   still running, by crate: {owners:?}");
        // The same guest through `App::update`, which also runs sub-apps.
        let mut times = UpdateTimes::default();
        for _ in 0..MEASURED {
            times.time(&mut guest);
        }
        let sub_apps = guest.app_mut().sub_apps().iter().count();
        println!("[guest-split]   App::update {:.2} ms an update ({sub_apps} sub-app(s))", times.mean_ms());
        // The same guest with the gated phases' systems taken out of the sim
        // schedule: the nearest a booted App comes to having been composed
        // without them. The phase sets stay, so their order is kept.
        let removed: usize = guest.world_mut().resource_scope(|world, mut schedules: bevy::prelude::Mut<Schedules>| {
            let schedule = schedules.get_mut(sim).expect("the sim schedule exists");
            GATED
                .iter()
                .map(|phase| {
                    schedule
                        .remove_systems_in_set(*phase, world, bevy::ecs::schedule::ScheduleCleanupPolicy::RemoveSystemsOnly)
                        .unwrap_or(0)
                })
                .sum()
        });
        for _ in 0..WARM {
            guest.app_mut().update();
        }
        let left: BTreeMap<String, usize> = guest.world_mut().resource_scope(|_, schedules: bevy::prelude::Mut<Schedules>| {
            let mut left = BTreeMap::new();
            if let Ok(systems) = schedules.get(sim).expect("the sim schedule exists").systems() {
                for (_, system) in systems {
                    let name = system.name().to_string();
                    *left.entry(name.trim_start_matches('<').split("::").next().unwrap_or("?").to_string()).or_default() += 1;
                }
            }
            left
        });
        let mut left: Vec<_> = left.into_iter().collect();
        left.sort_by(|a, b| b.1.cmp(&a.1));
        println!("[guest-split]   left after removing, by crate: {:?}", &left[..left.len().min(12)]);
        let mut without = UpdateTimes::default();
        for _ in 0..MEASURED {
            without.time(&mut guest);
        }
        println!(
            "[guest-split]   without the gated phases' {removed} systems ({} left in the sim schedule): App::update {:.2} ms an update",
            count(guest.world_mut(), sim),
            without.mean_ms()
        );
    }
}

/// A shot the guest built from a spawn row, which it removes when the host's
/// snapshot no longer names it.
#[derive(bevy::prelude::Component)]
struct ReplicatedShot;

/// One spawn row: the shot's network id, its kind, the half extent it is
/// drawn at and a visual index. The body row (position, velocity, facing)
/// rides beside it as for any body.
const SPAWN_RECORD: u64 = 4 + 1 + 8 + 2;
/// One despawn row: the network id.
const DESPAWN_RECORD: u64 = 4;
/// One body row, as `hosted_world`'s.
const BODY_ROW: u64 = 4 + 8 + 8 + 4;

/// Alice fires a short shot to her right every twelve frames from frame 10
/// to 200: shots spawn and end all through the run.
fn alice_fires_often(sim: &mut Platformer2dSimHarness, frame: i32) {
    use ambition_platformer2d::projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
    if !(10..=200).contains(&frame) || frame % 12 != 10 % 12 {
        return;
    }
    let world = sim.world_mut();
    let (alice, at) = world
        .query_filtered::<(bevy::prelude::Entity, &BodyKinematics), bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world)
        .map(|(entity, body)| (entity, body.pos))
        .expect("Alice has a body");
    world.write_message(ProjectileSpawnRequest::open(
        alice,
        ProjectileSpawn {
            // Low, at a crawler's height: a body's collision is fitted to its
            // art, and a puppy slug is about 14 units tall.
            origin: at + bevy::math::Vec2::new(24.0, 16.0),
            dir: bevy::math::Vec2::new(1.0, 0.0),
            speed: 240.0,
            damage: 1,
            max_lifetime: 0.4,
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

/// The host's shots in flight, by id.
fn shot_ids(sim: &mut Platformer2dSimHarness) -> BTreeSet<SimId> {
    let world = sim.world_mut();
    world
        .query_filtered::<&SimId, bevy::prelude::With<ambition_platformer2d::platformer::projectile::ProjectileGameplay>>()
        .iter(world)
        .cloned()
        .collect()
}

/// What one snapshot did to the guest.
#[derive(Default)]
struct Applied {
    spawned: u64,
    despawned: u64,
    /// Ids on one side only after the snapshot was applied.
    unmatched: usize,
}

/// Apply `rows` to the guest, building the shots it does not hold (`shots`
/// names them) and removing the replicated shots the rows no longer name.
fn apply_with_spawns(
    guest: &mut Platformer2dSimHarness,
    rows: &BTreeMap<SimId, [f32; 5]>,
    shots: &BTreeSet<SimId>,
) -> Applied {
    let mut applied = Applied::default();
    let missing = apply(guest, rows);
    let world = guest.world_mut();
    for id in missing.iter().filter(|id| shots.contains(*id)) {
        let row = rows[id];
        let mut body = BodyKinematics::default();
        body.pos = bevy::math::Vec2::new(row[0], row[1]);
        body.vel = bevy::math::Vec2::new(row[2], row[3]);
        body.facing = row[4];
        world.spawn((id.clone(), body, ReplicatedShot));
        applied.spawned += 1;
    }
    let ended: Vec<bevy::prelude::Entity> = world
        .query_filtered::<(bevy::prelude::Entity, &SimId), bevy::prelude::With<ReplicatedShot>>()
        .iter(world)
        .filter(|(_, id)| !rows.contains_key(*id))
        .map(|(entity, _)| entity)
        .collect();
    for entity in ended {
        world.despawn(entity);
        applied.despawned += 1;
    }
    let held: BTreeSet<SimId> =
        world.query_filtered::<&SimId, bevy::prelude::With<BodyKinematics>>().iter(world).cloned().collect();
    let sent: BTreeSet<SimId> = rows.keys().cloned().collect();
    applied.unmatched = held.symmetric_difference(&sent).count();
    applied
}

/// NA-2b with spawns: Alice fires a short shot every twelve frames, so shots
/// spawn and end all through the run. The guest builds each shot the host's
/// snapshot names and it does not hold, from a spawn row, and removes each
/// replicated shot the snapshot no longer names. Prints the spawns and
/// despawns, the most shots in flight, what applying costs, the frames the
/// guest's ids differ from the snapshot's after applying it, and the link
/// bytes a frame (body rows, spawn rows, despawn rows).
#[test]
#[ignore = "measurement: prints what replicating spawned shots to a guest costs; run with --ignored --nocapture --test-threads=1"]
fn probe_what_a_guest_needs_to_draw_shots() {
    let mut host = booted(ROOM);
    let mut guest = booted(ROOM);
    simulate_nothing(&mut guest);
    let mut in_flight: VecDeque<(i32, BTreeMap<SimId, [f32; 5]>, BTreeSet<SimId>)> = VecDeque::new();
    let (mut spawned, mut despawned, mut unmatched_frames, mut most_shots) = (0u64, 0u64, 0, 0usize);
    let (mut body_bytes, mut applied) = (0u64, 0u32);
    let mut apply_time = std::time::Duration::ZERO;
    for frame in 1..=FRAMES {
        alice_fires_often(&mut host, frame);
        host.drive_seat(0, Default::default());
        host.drive_seat(1, Default::default());
        host.app_mut().update();
        let shots = shot_ids(&mut host);
        most_shots = most_shots.max(shots.len());
        in_flight.push_back((frame, bodies(&mut host, None), shots));
        if in_flight.front().is_some_and(|(sent, ..)| *sent <= frame - LATENCY) {
            let (_, rows, shots) = in_flight.pop_front().expect("a snapshot arrived");
            let started = std::time::Instant::now();
            let done = apply_with_spawns(&mut guest, &rows, &shots);
            apply_time += started.elapsed();
            applied += 1;
            spawned += done.spawned;
            despawned += done.despawned;
            body_bytes += rows.len() as u64 * BODY_ROW;
            if done.unmatched > 0 {
                unmatched_frames += 1;
            }
        }
        guest.app_mut().update();
    }
    let link = body_bytes + spawned * SPAWN_RECORD + despawned * DESPAWN_RECORD;
    println!("[shots] {ROOM}: {FRAMES} frames, a snapshot {LATENCY} frames late, {applied} applied");
    println!("[shots]   shots spawned on the guest {spawned}, despawned {despawned}; most in flight at once {most_shots}");
    println!(
        "[shots]   applying a snapshot {:.3} ms; frames whose ids differ after applying {unmatched_frames}",
        apply_time.as_secs_f64() * 1e3 / f64::from(applied.max(1))
    );
    println!(
        "[shots]   link {:.1} bytes a frame: body rows {:.1}, spawn rows {:.2}, despawn rows {:.2}",
        link as f64 / f64::from(applied.max(1)),
        body_bytes as f64 / f64::from(applied.max(1)),
        (spawned * SPAWN_RECORD) as f64 / f64::from(applied.max(1)),
        (despawned * DESPAWN_RECORD) as f64 / f64::from(applied.max(1)),
    );
    assert!(spawned > 0, "premise: the host fired shots the guest had to build");
}

/// A body the guest built from a spawn row (any kind).
#[derive(bevy::prelude::Component)]
struct ReplicatedBody;

/// What one snapshot did to the guest, by the general rule: every body the
/// snapshot names and the guest does not hold is built, and every body the
/// guest holds and the snapshot no longer names is removed, whoever built it.
#[derive(Default)]
struct AppliedAll {
    built: Vec<SimId>,
    removed_built: Vec<SimId>,
    removed_booted: Vec<SimId>,
    unmatched: usize,
}

fn apply_all(guest: &mut Platformer2dSimHarness, rows: &BTreeMap<SimId, [f32; 5]>) -> AppliedAll {
    let mut applied = AppliedAll::default();
    let missing = apply(guest, rows);
    let world = guest.world_mut();
    for id in missing {
        let row = rows[&id];
        let mut body = BodyKinematics::default();
        body.pos = bevy::math::Vec2::new(row[0], row[1]);
        body.vel = bevy::math::Vec2::new(row[2], row[3]);
        body.facing = row[4];
        world.spawn((id.clone(), body, ReplicatedBody));
        applied.built.push(id);
    }
    let gone: Vec<(bevy::prelude::Entity, SimId, bool)> = world
        .query_filtered::<(bevy::prelude::Entity, &SimId, bevy::prelude::Has<ReplicatedBody>), bevy::prelude::With<BodyKinematics>>()
        .iter(world)
        .filter(|(_, id, _)| !rows.contains_key(*id))
        .map(|(entity, id, built)| (entity, id.clone(), built))
        .collect();
    for (entity, id, built) in gone {
        world.despawn(entity);
        if built {
            applied.removed_built.push(id);
        } else {
            applied.removed_booted.push(id);
        }
    }
    let held: BTreeSet<SimId> =
        world.query_filtered::<&SimId, bevy::prelude::With<BodyKinematics>>().iter(world).cloned().collect();
    let sent: BTreeSet<SimId> = rows.keys().cloned().collect();
    applied.unmatched = held.symmetric_difference(&sent).count();
    applied
}

/// The defeat in the body row: each body's current health on the host, by
/// `SimId` (two bytes on the link: an `i16`).
const HEALTH_FIELD: u64 = 2;

type Health = ambition_platformer2d::characters::actor::BodyHealth;

fn healths(sim: &mut Platformer2dSimHarness) -> BTreeMap<SimId, i32> {
    let world = sim.world_mut();
    world.query::<(&SimId, &Health)>().iter(world).map(|(id, health)| (id.clone(), health.health.current)).collect()
}

/// Write the host's healths onto the bodies the guest holds (a body it built
/// from a spawn row is given one), and return how many bodies the guest then
/// holds whose life (alive or defeated) differs from the snapshot's.
fn apply_healths(guest: &mut Platformer2dSimHarness, sent: &BTreeMap<SimId, i32>) -> usize {
    let world = guest.world_mut();
    let held: Vec<(bevy::prelude::Entity, SimId)> =
        world.query::<(bevy::prelude::Entity, &SimId)>().iter(world).map(|(entity, id)| (entity, id.clone())).collect();
    for (entity, id) in &held {
        let Some(&current) = sent.get(id) else {
            continue;
        };
        match world.get_mut::<Health>(*entity) {
            Some(mut health) => health.health.current = current,
            None => {
                let mut pool = ambition_platformer2d::characters::actor::Health::new(current.max(1));
                pool.current = current;
                world.entity_mut(*entity).insert(Health::new(pool));
            }
        }
    }
    world
        .query::<(&SimId, &Health)>()
        .iter(world)
        .filter(|(id, health)| sent.get(*id).is_some_and(|current| (*current > 0) != health.alive()))
        .count()
}

/// What draws a death: each body's `DeathBeat` on the host, by `SimId`. On
/// the link: the frames since the death (a `u16`, none while it lives) and a
/// byte for its `lived` and corpse bits. Its row length is the body's sheet,
/// a fact of its spawn row, not of every frame.
const BEAT_FIELD: u64 = 2 + 1;

type Beat = ambition_platformer2d::combat::components::DeathBeat;

fn beats(sim: &mut Platformer2dSimHarness) -> BTreeMap<SimId, (Beat, bool)> {
    let world = sim.world_mut();
    world
        .query::<(&SimId, &Beat, &Health)>()
        .iter(world)
        .map(|(id, beat, health)| (id.clone(), (*beat, beat.visible(health.alive()))))
        .collect()
}

/// How many bodies the guest holds whose `visible` differs from the host's in
/// the snapshot.
fn beats_differ(guest: &mut Platformer2dSimHarness, sent: &BTreeMap<SimId, (Beat, bool)>) -> usize {
    let world = guest.world_mut();
    world
        .query::<(&SimId, &Beat, &Health)>()
        .iter(world)
        .filter(|(id, beat, health)| sent.get(*id).is_some_and(|(_, visible)| beat.visible(health.alive()) != *visible))
        .count()
}

/// Write the host's beats onto the bodies the guest holds, and return how
/// many differed in `visible` before ([`beats_differ`]) and after.
fn apply_beats(guest: &mut Platformer2dSimHarness, sent: &BTreeMap<SimId, (Beat, bool)>) -> (usize, usize) {
    let before = beats_differ(guest, sent);
    let world = guest.world_mut();
    let held: Vec<(bevy::prelude::Entity, SimId)> =
        world.query::<(bevy::prelude::Entity, &SimId)>().iter(world).map(|(entity, id)| (entity, id.clone())).collect();
    for (entity, id) in &held {
        if let Some((beat, _)) = sent.get(id) {
            world.entity_mut(*entity).insert(*beat);
        }
    }
    (before, beats_differ(guest, sent))
}

/// NA-2b with a death: an enemy is spawned beside Alice on the host at frame
/// 5 with one point of health, and her shots kill it. The guest applies each
/// late snapshot by the general rule (build what it does not hold, remove what
/// is no longer named). Prints when the enemy's id entered and left the
/// host's snapshot, what the guest built and removed, and the frames its ids
/// differ from the snapshot's after applying it.
///
/// The snapshot carries the defeat (each body's health, [`HEALTH_FIELD`]):
/// it prints the frames the guest's enemy stood defeated, the frames any held
/// body's life differed from the snapshot's after applying it, and the link
/// bytes the field costs. It carries the death's beat too ([`BEAT_FIELD`]):
/// it prints when the enemy stopped being drawn on the host and on the
/// guest, and the frames any held body's `visible` differed from the host's
/// after applying.
#[test]
#[ignore = "measurement: prints what a death on the host does to a guest; run with --ignored --nocapture --test-threads=1"]
fn probe_what_a_death_on_the_host_does_to_a_guest() {
    const ENEMY: &str = "na2b_death_probe";
    let mut host = booted(ROOM);
    let mut guest = booted(ROOM);
    simulate_nothing(&mut guest);
    let mut in_flight: VecDeque<(i32, BTreeMap<SimId, [f32; 5]>, BTreeMap<SimId, i32>, BTreeMap<SimId, (Beat, bool)>)> = VecDeque::new();
    let (mut drawn_differed, mut drawn_differs, mut host_hidden, mut guest_hidden, mut beat_bytes) = (0, 0, None::<i32>, None::<i32>, 0u64);
    let (mut built, mut removed_built, mut removed_booted, mut unmatched_frames) = (Vec::new(), Vec::new(), Vec::new(), 0);
    let (mut life_differs, mut guest_defeated, mut health_bytes) = (0, None::<i32>, 0u64);
    let (mut enemy_id, mut entered, mut left, mut died) = (None::<SimId>, None, None, None);
    for frame in 1..=FRAMES {
        if frame == 5 {
            let at = {
                let world = host.world_mut();
                world
                    .query_filtered::<&BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("Alice has a body")
                    .pos
            };
            host.spawn_enemy_character_at(
                ENEMY,
                "Target",
                (at.x + 72.0, at.y),
                (12.0, 16.0),
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
                "npc_puppy_slug",
            );
        }
        alice_fires_often(&mut host, frame);
        host.drive_seat(0, Default::default());
        host.drive_seat(1, Default::default());
        host.app_mut().update();
        {
            let world = host.world_mut();
            let found = world
                .query::<(&ambition_platformer2d::combat::components::FeatureId, Option<&SimId>, Option<&mut ambition_platformer2d::characters::actor::BodyHealth>)>()
                .iter_mut(world)
                .find(|(feature, ..)| feature.0 == ENEMY)
                .map(|(_, id, health)| {
                    if let Some(mut health) = health {
                        if health.health.current > 1 {
                            health.health.current = 1;
                        }
                        if health.health.current <= 0 {
                            died.get_or_insert(frame);
                        }
                    }
                    id.cloned()
                });
            if let Some(Some(id)) = found {
                enemy_id.get_or_insert(id);
            }
        }
        let rows = bodies(&mut host, None);
        if let Some(id) = &enemy_id {
            if rows.contains_key(id) {
                entered.get_or_insert(frame);
            } else if entered.is_some() {
                left.get_or_insert(frame);
            }
        }
        let lives = healths(&mut host);
        health_bytes += lives.len() as u64 * HEALTH_FIELD;
        let drawn = beats(&mut host);
        beat_bytes += drawn.len() as u64 * BEAT_FIELD;
        if let Some((_, visible)) = enemy_id.as_ref().and_then(|id| drawn.get(id)) {
            if !visible && died.is_some() {
                host_hidden.get_or_insert(frame);
            }
        }
        in_flight.push_back((frame, rows, lives, drawn));
        if in_flight.front().is_some_and(|(sent, ..)| *sent <= frame - LATENCY) {
            let (_, rows, lives, drawn) = in_flight.pop_front().expect("a snapshot arrived");
            let done = apply_all(&mut guest, &rows);
            if apply_healths(&mut guest, &lives) > 0 {
                life_differs += 1;
            }
            let (before, after) = apply_beats(&mut guest, &drawn);
            if before > 0 {
                drawn_differed += 1;
            }
            if after > 0 {
                drawn_differs += 1;
            }
            if let Some(id) = &enemy_id {
                let world = guest.world_mut();
                let hidden = world
                    .query::<(&SimId, &Beat, &Health)>()
                    .iter(world)
                    .any(|(held, beat, health)| held == id && !health.alive() && !beat.visible(health.alive()));
                if hidden {
                    guest_hidden.get_or_insert(frame);
                }
            }
            if let Some(id) = &enemy_id {
                let world = guest.world_mut();
                let dead = world.query::<(&SimId, &Health)>().iter(world).any(|(held, health)| held == id && !health.alive());
                if dead {
                    guest_defeated.get_or_insert(frame);
                }
            }
            built.extend(done.built);
            removed_built.extend(done.removed_built);
            removed_booted.extend(done.removed_booted);
            if done.unmatched > 0 {
                unmatched_frames += 1;
            }
        }
        guest.app_mut().update();
    }
    println!("[death] {ROOM}: {FRAMES} frames, a snapshot {LATENCY} frames late");
    println!(
        "[death]   the enemy {enemy_id:?}: in the host's snapshot from frame {entered:?}, dead from frame {died:?}, gone from frame {left:?}"
    );
    let shots = |ids: &Vec<SimId>| ids.iter().filter(|id| Some(*id) != enemy_id.as_ref()).count();
    let enemy = |ids: &Vec<SimId>| ids.iter().any(|id| Some(id) == enemy_id.as_ref());
    println!(
        "[death]   the guest built {} (the enemy: {}, others: {}); removed {} it built (the enemy: {}) and {} it booted with {removed_booted:?}",
        built.len(),
        enemy(&built),
        shots(&built),
        removed_built.len(),
        enemy(&removed_built),
        removed_booted.len(),
    );
    println!("[death]   frames whose ids differ after applying {unmatched_frames}");
    println!(
        "[death]   with the defeat in the row: the guest's enemy defeated from frame {guest_defeated:?}; frames a held body's life differs after applying {life_differs}; the field costs {:.2} link bytes a frame",
        health_bytes as f64 / FRAMES as f64
    );
    println!(
        "[death]   with the beat in the row: the enemy stopped being drawn on the host from frame {host_hidden:?}, on the guest from frame {guest_hidden:?}; frames a held body's `visible` differs from the host's before applying (the guest's own beat) {drawn_differed} and after {drawn_differs}; the field costs {:.2} link bytes a frame",
        beat_bytes as f64 / FRAMES as f64
    );
    let world = host.world_mut();
    let last = world
        .query::<(&ambition_platformer2d::combat::components::FeatureId, Option<&ambition_platformer2d::characters::actor::BodyHealth>, Option<&BodyKinematics>)>()
        .iter(world)
        .find(|(feature, ..)| feature.0 == ENEMY)
        .map(|(_, health, body)| (health.map(|health| health.health.current), body.map(|body| body.pos)));
    println!("[death]   the enemy at the end on the host (health, position): {last:?}");
    assert!(entered.is_some(), "premise: the enemy reached the host's snapshot");
    assert!(died.is_some(), "premise: Alice's shots killed the enemy");
}

/// What one camera shake costs on the link: its amplitude (an `f32`) and its
/// room (a `u32`, or none).
const SHAKE_FIELD: u64 = 4 + 1 + 4;

/// The largest shake any view of `sim` holds now.
fn shaking(sim: &mut Platformer2dSimHarness) -> f32 {
    let shake = sim.world_mut().resource::<ambition_platformer2d::platformer::camera_ease::CameraShakeState>();
    shake.rooms.values().copied().fold(shake.amplitude_px, f32::max)
}

/// NA-2b, the kill as a presentation event: the death probe's run, with two
/// guests. Both apply the body rows, healths and beats; one is also sent
/// each frame's camera shakes (the presentation effects a gated guest never
/// makes: `for_each_presentation_effect`), and the other, the control, is
/// not. Prints when the host, the guest and the control first shake after
/// the kill, the frames each guest's shake differs from the host's shake
/// [`LATENCY`] frames earlier, and how many of the other presentation
/// effects (sounds, visuals) the host made: what the same channel would carry.
#[test]
#[ignore = "measurement: prints what a guest needs to show a kill's shake; run with --ignored --nocapture --test-threads=1"]
fn probe_what_a_kill_on_the_host_shows_a_guest() {
    use ambition_platformer2d::platformer::camera_ease::CameraShakeRequest;
    use bevy::ecs::message::{MessageCursor, Messages};
    const ENEMY: &str = "na2b_kill_probe";
    let mut host = booted(ROOM);
    let mut guest = booted(ROOM);
    let mut control = booted(ROOM);
    simulate_nothing(&mut guest);
    simulate_nothing(&mut control);
    let mut shakes = MessageCursor::<CameraShakeRequest>::default();
    let mut sounds = MessageCursor::<ambition_platformer2d::sfx::OwnedSfxMessage>::default();
    let mut visuals = MessageCursor::<ambition_platformer2d::vfx::VfxInRoom>::default();
    type Sent = (i32, BTreeMap<SimId, [f32; 5]>, BTreeMap<SimId, i32>, BTreeMap<SimId, (Beat, bool)>, Vec<CameraShakeRequest>);
    let mut in_flight: VecDeque<Sent> = VecDeque::new();
    let (mut died, mut enemy_id) = (None::<i32>, None::<SimId>);
    let mut host_shakes: Vec<f32> = Vec::new();
    let (mut first_host, mut first_guest, mut first_control) = (None::<i32>, None::<i32>, None::<i32>);
    let (mut guest_differs, mut control_differs, mut shake_count, mut sound_count, mut visual_count) = (0, 0, 0u64, 0u64, 0u64);
    let mut rooms_named = BTreeSet::new();
    for frame in 1..=FRAMES {
        if frame == 5 {
            let at = {
                let world = host.world_mut();
                world
                    .query_filtered::<&BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("Alice has a body")
                    .pos
            };
            host.spawn_enemy_character_at(
                ENEMY,
                "Target",
                (at.x + 72.0, at.y),
                (12.0, 16.0),
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
                "npc_puppy_slug",
            );
        }
        alice_fires_often(&mut host, frame);
        host.drive_seat(0, Default::default());
        host.drive_seat(1, Default::default());
        host.app_mut().update();
        {
            let world = host.world_mut();
            let found = world
                .query::<(&ambition_platformer2d::combat::components::FeatureId, Option<&SimId>, Option<&mut Health>)>()
                .iter_mut(world)
                .find(|(feature, ..)| feature.0 == ENEMY)
                .map(|(_, id, health)| {
                    if let Some(mut health) = health {
                        if health.health.current > 1 {
                            health.health.current = 1;
                        }
                        if health.health.current <= 0 {
                            died.get_or_insert(frame);
                        }
                    }
                    id.cloned()
                });
            if let Some(Some(id)) = found {
                enemy_id.get_or_insert(id);
            }
        }
        let made: Vec<CameraShakeRequest> = {
            let world = host.world();
            let made: Vec<CameraShakeRequest> = shakes.read(world.resource::<Messages<CameraShakeRequest>>()).copied().collect();
            sound_count += sounds.read(world.resource::<Messages<ambition_platformer2d::sfx::OwnedSfxMessage>>()).count() as u64;
            visual_count += visuals.read(world.resource::<Messages<ambition_platformer2d::vfx::VfxInRoom>>()).count() as u64;
            made
        };
        shake_count += made.len() as u64;
        rooms_named.extend(made.iter().map(|shake| shake.room));
        let host_shake = shaking(&mut host);
        host_shakes.push(host_shake);
        if died.is_some() && host_shake > 0.0 {
            first_host.get_or_insert(frame);
        }
        let rows = bodies(&mut host, None);
        let lives = healths(&mut host);
        let drawn = beats(&mut host);
        in_flight.push_back((frame, rows, lives, drawn, made));
        if in_flight.front().is_some_and(|(sent, ..)| *sent <= frame - LATENCY) {
            let (_, rows, lives, drawn, made) = in_flight.pop_front().expect("a snapshot arrived");
            for sim in [&mut guest, &mut control] {
                apply_all(sim, &rows);
                apply_healths(sim, &lives);
                apply_beats(sim, &drawn);
            }
            for shake in made {
                guest.world_mut().write_message(shake);
            }
        }
        guest.app_mut().update();
        control.app_mut().update();
        let (now_guest, now_control) = (shaking(&mut guest), shaking(&mut control));
        if died.is_some() && now_guest > 0.0 {
            first_guest.get_or_insert(frame);
        }
        if died.is_some() && now_control > 0.0 {
            first_control.get_or_insert(frame);
        }
        if frame > LATENCY {
            let then = host_shakes[(frame - LATENCY - 1) as usize];
            if (now_guest - then).abs() > 1e-3 {
                guest_differs += 1;
            }
            if (now_control - then).abs() > 1e-3 {
                control_differs += 1;
            }
        }
    }
    let shaken = host_shakes.iter().filter(|amplitude| **amplitude > 0.0).count();
    println!("[kill] {ROOM}: {FRAMES} frames, a snapshot {LATENCY} frames late; the enemy {enemy_id:?} dead from frame {died:?}");
    println!(
        "[kill]   the host made {shake_count} shakes (rooms {rooms_named:?}), {sound_count} sounds and {visual_count} visuals; it shook on {shaken} frames"
    );
    println!("[kill]   after the kill first shaking: host frame {first_host:?}, guest {first_guest:?}, control {first_control:?}");
    println!(
        "[kill]   frames a view's shake differs from the host's {LATENCY} frames earlier: guest {guest_differs}, control {control_differs}; the shakes cost {:.3} link bytes a frame",
        (shake_count * SHAKE_FIELD) as f64 / FRAMES as f64
    );
    assert!(died.is_some(), "premise: Alice's shots killed the enemy");
}

/// The shape of a minted id: its text with every run of digits written `#`.
fn id_shape(id: &SimId) -> String {
    let mut shape = String::new();
    let mut in_digits = false;
    for ch in id.as_str().chars() {
        if ch.is_ascii_digit() {
            if !in_digits {
                shape.push('#');
            }
            in_digits = true;
        } else {
            shape.push(ch);
            in_digits = false;
        }
    }
    shape
}

/// NA-2b, what else a host spawns: the death probe's run (Alice's shots kill
/// an enemy held at one point of health), and a census of every `SimId` that appears on the host after
/// the room is booted. For each shape of id: how many appeared, how many
/// frames each lived, whether the body rows already carry it (it has a
/// `BodyKinematics`), and the game's components on the first of them: what
/// a spawn row for that kind would have to build. And what the simulation
/// keeps that appears with no id (room- or session-scoped, or a pickup), by
/// its components.
#[test]
#[ignore = "measurement: prints what kinds of thing a host spawns besides bodies; run with --ignored --nocapture --test-threads=1"]
fn probe_what_else_a_host_spawns() {
    const ENEMY: &str = "na2b_census_probe";
    let mut host = booted(ROOM);
    let mut seen: BTreeMap<SimId, (i32, Option<i32>)> = BTreeMap::new();
    let booted_with: BTreeSet<SimId> = {
        let world = host.world_mut();
        world.query::<&SimId>().iter(world).cloned().collect()
    };
    // shape -> (count, a body row carries it, components of the first, lifetimes)
    let mut shapes: BTreeMap<String, (usize, bool, Vec<String>, Vec<i32>)> = BTreeMap::new();
    // What appears with no id: what the simulation keeps, by its components.
    let anonymous_at_boot: BTreeSet<bevy::prelude::Entity> = {
        let world = host.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::Without<SimId>>()
            .iter(world)
            .collect()
    };
    let mut anonymous: BTreeMap<bevy::prelude::Entity, String> = BTreeMap::new();
    let mut anonymous_kinds: BTreeMap<String, usize> = BTreeMap::new();
    for frame in 1..=FRAMES {
        if frame == 5 {
            let at = {
                let world = host.world_mut();
                world
                    .query_filtered::<&BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("Alice has a body")
                    .pos
            };
            host.spawn_enemy_character_at(
                ENEMY,
                "Target",
                (at.x + 72.0, at.y),
                (12.0, 16.0),
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
                "npc_puppy_slug",
            );
        }
        alice_fires_often(&mut host, frame);
        host.drive_seat(0, Default::default());
        host.drive_seat(1, Default::default());
        {
            // One point of health, as in the death probe: the first shot kills.
            let world = host.world_mut();
            for (feature, mut health) in world.query::<(&ambition_platformer2d::combat::components::FeatureId, &mut Health)>().iter_mut(world) {
                if feature.0 == ENEMY && health.health.current > 1 {
                    health.health.current = 1;
                }
            }
        }
        host.app_mut().update();
        let world = host.world_mut();
        let now: Vec<(bevy::prelude::Entity, SimId, bool)> = world
            .query::<(bevy::prelude::Entity, &SimId, bevy::prelude::Has<BodyKinematics>)>()
            .iter(world)
            .map(|(entity, id, body)| (entity, id.clone(), body))
            .collect();
        let held: BTreeSet<SimId> = now.iter().map(|(_, id, _)| id.clone()).collect();
        for (entity, id, body) in &now {
            if booted_with.contains(id) || seen.contains_key(id) {
                continue;
            }
            seen.insert(id.clone(), (frame, None));
            let entry = shapes.entry(id_shape(id)).or_insert_with(|| {
                let mut names: Vec<String> = world
                    .inspect_entity(*entity)
                    .map(|infos| {
                        infos
                            .map(|info| info.name().to_string())
                            .filter(|name| name.starts_with("ambition_"))
                            .map(|name| name.rsplit("::").next().unwrap_or(&name).to_string())
                            .collect()
                    })
                    .unwrap_or_default();
                names.sort();
                (0, *body, names, Vec::new())
            });
            entry.0 += 1;
        }
        let unnamed: Vec<bevy::prelude::Entity> = world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::Without<SimId>>()
            .iter(world)
            .filter(|entity| !anonymous_at_boot.contains(entity) && !anonymous.contains_key(entity))
            .collect();
        for entity in unnamed {
            let mut names: Vec<String> = world
                .inspect_entity(entity)
                .map(|infos| {
                    infos
                        .map(|info| info.name().to_string())
                        .filter(|name| name.starts_with("ambition_"))
                        .map(|name| name.rsplit("::").next().unwrap_or(&name).to_string())
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            let kind = names.join(" ");
            // Only what the simulation keeps: what a room or session sweep takes
            // back, or a pickup. Presentation entities are the guest's own.
            if !(kind.contains("RoomScopedEntity") || kind.contains("SessionScopedEntity") || kind.contains("Pickup") || kind.contains("GroundItem")) {
                anonymous.insert(entity, String::new());
                continue;
            }
            *anonymous_kinds.entry(kind.clone()).or_insert(0) += 1;
            anonymous.insert(entity, kind);
        }
        for (id, (born, gone)) in seen.iter_mut() {
            if gone.is_none() && !held.contains(id) {
                *gone = Some(frame);
                if let Some(entry) = shapes.get_mut(&id_shape(id)) {
                    entry.3.push(frame - *born);
                }
            }
        }
    }
    println!("[census] {ROOM}: {FRAMES} frames; {} ids at boot, {} minted after", booted_with.len(), seen.len());
    for (shape, (count, body, names, lives)) in &shapes {
        let still = count - lives.len();
        println!(
            "[census]   {shape}: {count} (in the body rows: {body}); lived {:?} frames, {still} still there; components ({}): {}",
            (lives.iter().min(), lives.iter().max()),
            names.len(),
            names.join(" ")
        );
    }
    println!("[census]   with no id, {} simulation entities appeared:", anonymous_kinds.values().sum::<usize>());
    for (kind, count) in &anonymous_kinds {
        println!("[census]     {count} x {kind}");
    }
}

type Pickup = ambition_platformer2d::item_runtime::pickups::PickupFeature;
type Collected = ambition_platformer2d::item_runtime::pickups::Collected;
type Origin = ambition_platformer2d::platformer::construction::SpawnOrigin;
type Aabb = ambition_platformer2d::combat::components::CenteredAabb;

/// A pickup the guest built from a pickup spawn row.
#[derive(bevy::prelude::Component)]
struct ReplicatedPickup;

/// A dropped pickup's peer-stable key: the `SimId` of what it fell out of
/// and the drop's sequence (`SpawnOrigin::Dynamic`). On the link, the
/// parent's network id and a byte.
type DropKey = (SimId, u64);
const DROP_KEY: u64 = 4 + 1;
/// One pickup spawn row: its key, its kind (a byte) and amount (an `i16`),
/// and its centre. Its size and art are its kind's.
const PICKUP_SPAWN: u64 = DROP_KEY + 1 + 2 + 8;
/// One pickup motion row (a magnet draws a drop toward its collector): its key
/// and its centre, sent only on a frame it moved.
const PICKUP_MOTION: u64 = DROP_KEY + 8;

/// The host's uncollected dropped pickups, by key: the pickup and its box.
fn dropped(sim: &mut Platformer2dSimHarness) -> BTreeMap<DropKey, (Pickup, Aabb)> {
    let world = sim.world_mut();
    world
        .query_filtered::<(&Origin, &Pickup, &Aabb), bevy::prelude::Without<Collected>>()
        .iter(world)
        .filter_map(|(origin, pickup, aabb)| match origin {
            Origin::Dynamic { parent, sequence } => Some(((parent.clone(), *sequence), (pickup.clone(), *aabb))),
            _ => None,
        })
        .collect()
}

/// The dropped pickups a view would draw: `rebuild_dynamic_feature_views`'
/// "Dropped pickup" rows, run on `sim`'s world (the guest gates the phase
/// that runs it), by position.
fn drawn_drops(sim: &mut Platformer2dSimHarness) -> Vec<bevy::math::Vec2> {
    use bevy::ecs::system::RunSystemOnce as _;
    let world = sim.world_mut();
    world.init_resource::<ambition_platformer2d::sim_view::DynamicFeatureViews>();
    world
        .run_system_once(ambition_platformer2d::sim_view::rebuild_dynamic_feature_views)
        .expect("the dynamic feature view runs");
    world
        .resource::<ambition_platformer2d::sim_view::DynamicFeatureViews>()
        .0
        .iter()
        .filter(|fact| fact.family == "Dropped pickup")
        .map(|fact| fact.pos)
        .collect()
}

/// What one pickup snapshot did to the guest.
#[derive(Default)]
struct PickupsApplied {
    built: u64,
    moved: u64,
    removed: u64,
    /// Keys on one side only after applying.
    unmatched: usize,
}

/// Build each dropped pickup the snapshot names and the guest does not hold,
/// move the ones it holds, and remove the ones it no longer names.
fn apply_pickups(guest: &mut Platformer2dSimHarness, sent: &BTreeMap<DropKey, (Pickup, Aabb)>) -> PickupsApplied {
    let mut applied = PickupsApplied::default();
    let world = guest.world_mut();
    let held: Vec<(bevy::prelude::Entity, DropKey)> = world
        .query_filtered::<(bevy::prelude::Entity, &Origin), bevy::prelude::With<ReplicatedPickup>>()
        .iter(world)
        .filter_map(|(entity, origin)| match origin {
            Origin::Dynamic { parent, sequence } => Some((entity, (parent.clone(), *sequence))),
            _ => None,
        })
        .collect();
    let mut holding = BTreeSet::new();
    for (entity, key) in &held {
        match sent.get(key) {
            Some((_, aabb)) => {
                let mut mine = world.get_mut::<Aabb>(*entity).expect("a replicated pickup has a box");
                if mine.center != aabb.center {
                    mine.center = aabb.center;
                    applied.moved += 1;
                }
                holding.insert(key.clone());
            }
            None => {
                world.despawn(*entity);
                applied.removed += 1;
            }
        }
    }
    for (key, (pickup, aabb)) in sent.iter().filter(|(key, _)| !holding.contains(*key)) {
        let (parent, sequence) = key.clone();
        world.spawn((
            ambition_platformer2d::combat::components::FeatureId::new(format!("drop:{}:{sequence}", parent.as_str())),
            ambition_platformer2d::combat::components::FeatureName::new("Dropped pickup"),
            *aabb,
            pickup.clone(),
            Origin::Dynamic { parent, sequence },
            ReplicatedPickup,
        ));
        applied.built += 1;
    }
    let now: BTreeSet<DropKey> = world
        .query_filtered::<&Origin, bevy::prelude::With<ReplicatedPickup>>()
        .iter(world)
        .filter_map(|origin| match origin {
            Origin::Dynamic { parent, sequence } => Some((parent.clone(), *sequence)),
            _ => None,
        })
        .collect();
    let sent_keys: BTreeSet<DropKey> = sent.keys().cloned().collect();
    applied.unmatched = now.symmetric_difference(&sent_keys).count();
    applied
}

/// NA-2b, a pickup by its origin: the death probe's run, long enough for the
/// enemy's bounty coin to fall, drift to Alice on its magnet and be
/// collected. Each snapshot also carries the host's uncollected dropped
/// pickups keyed by `SpawnOrigin::Dynamic` (the parent's `SimId` and the
/// drop's sequence), since a drop has no `SimId`. The guest builds each one
/// it does not hold, moves the ones it holds, and removes what the snapshot
/// no longer names. Prints when the coin appeared and was collected on the
/// host and on the guest, the frames the guest's drop keys differ from the
/// snapshot's after applying it, the frames the drops a view would draw
/// differ from the host's three frames earlier, and the link bytes a frame.
#[test]
#[ignore = "measurement: prints what replicating a dropped pickup by its origin costs; run with --ignored --nocapture --test-threads=1"]
fn probe_what_a_guest_needs_to_draw_a_drop() {
    const ENEMY: &str = "na2b_drop_probe";
    const RUN: i32 = 360;
    let mut host = booted(ROOM);
    let mut guest = booted(ROOM);
    simulate_nothing(&mut guest);
    let mut in_flight: VecDeque<(i32, BTreeMap<SimId, [f32; 5]>, BTreeMap<DropKey, (Pickup, Aabb)>, Vec<bevy::math::Vec2>)> = VecDeque::new();
    let (mut host_dropped, mut host_gone, mut guest_built, mut guest_gone) = (None::<i32>, None::<i32>, None::<i32>, None::<i32>);
    let (mut unmatched_frames, mut drawn_differs, mut drawn_frames, mut moves) = (0, 0, 0, 0u64);
    let (mut spawn_bytes, mut motion_bytes, mut despawn_bytes) = (0u64, 0u64, 0u64);
    let mut last_sent: BTreeMap<DropKey, bevy::math::Vec2> = BTreeMap::new();
    for frame in 1..=RUN {
        if frame == 5 {
            let at = {
                let world = host.world_mut();
                world
                    .query_filtered::<&BodyKinematics, bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
                    .single(world)
                    .expect("Alice has a body")
                    .pos
            };
            host.spawn_enemy_character_at(
                ENEMY,
                "Target",
                (at.x + 72.0, at.y),
                (12.0, 16.0),
                ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
                "npc_puppy_slug",
            );
        }
        alice_fires_often(&mut host, frame);
        host.drive_seat(0, Default::default());
        host.drive_seat(1, Default::default());
        {
            let world = host.world_mut();
            for (feature, mut health) in world.query::<(&ambition_platformer2d::combat::components::FeatureId, &mut Health)>().iter_mut(world) {
                if feature.0 == ENEMY && health.health.current > 1 {
                    health.health.current = 1;
                }
            }
        }
        host.app_mut().update();
        let drops = dropped(&mut host);
        if !drops.is_empty() {
            host_dropped.get_or_insert(frame);
        } else if host_dropped.is_some() {
            host_gone.get_or_insert(frame);
        }
        // The link: a spawn row for a key the guest has not been sent, a motion
        // row for one that moved, a despawn row for one no longer named.
        for (key, (_, aabb)) in &drops {
            match last_sent.get(key) {
                None => spawn_bytes += PICKUP_SPAWN,
                Some(at) if *at != aabb.center => motion_bytes += PICKUP_MOTION,
                Some(_) => {}
            }
        }
        despawn_bytes += last_sent.keys().filter(|key| !drops.contains_key(*key)).count() as u64 * DROP_KEY;
        last_sent = drops.iter().map(|(key, (_, aabb))| (key.clone(), aabb.center)).collect();
        let drawn = drawn_drops(&mut host);
        in_flight.push_back((frame, bodies(&mut host, None), drops, drawn));
        if in_flight.front().is_some_and(|(sent, ..)| *sent <= frame - LATENCY) {
            let (_, rows, drops, drawn) = in_flight.pop_front().expect("a snapshot arrived");
            apply_all(&mut guest, &rows);
            let done = apply_pickups(&mut guest, &drops);
            moves += done.moved;
            if done.built > 0 {
                guest_built.get_or_insert(frame);
            }
            if done.removed > 0 {
                guest_gone.get_or_insert(frame);
            }
            if done.unmatched > 0 {
                unmatched_frames += 1;
            }
            let mine = drawn_drops(&mut guest);
            if !mine.is_empty() {
                drawn_frames += 1;
            }
            if mine != drawn {
                drawn_differs += 1;
            }
        }
        guest.app_mut().update();
    }
    println!("[drop] {ROOM}: {RUN} frames, a snapshot {LATENCY} frames late");
    println!("[drop]   the host: a drop from frame {host_dropped:?}, collected (no uncollected drop) from frame {host_gone:?}");
    println!(
        "[drop]   the guest: built from frame {guest_built:?}, removed from frame {guest_gone:?}; {moves} moves applied; frames its keys differ after applying {unmatched_frames}"
    );
    println!("[drop]   frames a view would draw a drop on the guest {drawn_frames}; frames its drawn drops differ from the host's in the snapshot {drawn_differs}");
    println!(
        "[drop]   link a frame: spawn {:.3}, motion {:.3}, despawn {:.3} bytes",
        spawn_bytes as f64 / RUN as f64,
        motion_bytes as f64 / RUN as f64,
        despawn_bytes as f64 / RUN as f64,
    );
    assert!(host_dropped.is_some(), "premise: the enemy dropped something on the host");
}
