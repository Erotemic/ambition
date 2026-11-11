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
//! - whether a gated guest moved a body on its own.
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
            origin: at + bevy::math::Vec2::new(24.0, 0.0),
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

/// NA-2b with a death: an enemy is spawned beside Alice on the host at frame
/// 5 with one point of health, and her shots kill it. The guest applies each
/// late snapshot by the general rule (build what it does not hold, remove what
/// is no longer named). Prints when the enemy's id entered and left the
/// host's snapshot, what the guest built and removed, and the frames its ids
/// differ from the snapshot's after applying it.
#[test]
#[ignore = "measurement: prints what a death on the host does to a guest; run with --ignored --nocapture --test-threads=1"]
fn probe_what_a_death_on_the_host_does_to_a_guest() {
    const ENEMY: &str = "na2b_death_probe";
    let mut host = booted(ROOM);
    let mut guest = booted(ROOM);
    simulate_nothing(&mut guest);
    let mut in_flight: VecDeque<(i32, BTreeMap<SimId, [f32; 5]>)> = VecDeque::new();
    let (mut built, mut removed_built, mut removed_booted, mut unmatched_frames) = (Vec::new(), Vec::new(), Vec::new(), 0);
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
        in_flight.push_back((frame, rows));
        if in_flight.front().is_some_and(|(sent, _)| *sent <= frame - LATENCY) {
            let (_, rows) = in_flight.pop_front().expect("a snapshot arrived");
            let done = apply_all(&mut guest, &rows);
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
