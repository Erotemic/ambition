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

/// Put every simulation phase of `guest` to sleep.
fn simulate_nothing(guest: &mut Platformer2dSimHarness) {
    let app = guest.app_mut();
    let schedule = app.sim_schedule();
    for phase in [
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
    ] {
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
        // The same guest through `App::update`, which also runs sub-apps.
        let mut times = UpdateTimes::default();
        for _ in 0..MEASURED {
            times.time(&mut guest);
        }
        let sub_apps = guest.app_mut().sub_apps().iter().count();
        println!("[guest-split]   App::update {:.2} ms an update ({sub_apps} sub-app(s))", times.mean_ms());
    }
}
