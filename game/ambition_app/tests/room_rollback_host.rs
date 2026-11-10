//! SIMULATION-ISLANDS: what it costs a hosted world to keep a room correct by
//! rollback.
//!
//! NA-2's host applies the guest's input when it arrives, [`LATENCY`] frames
//! late. Here the host predicts it instead (the guest's last arrived input),
//! and when the real input for frame g arrives and differs, it rewinds only
//! the guest's room to g ([`RoomSnapshotRows`], one save per frame) and
//! replays it to now with the corrected input. The result is compared with an
//! oracle run in which the guest's input is on time: the room-rollback host
//! must end where the oracle ends.
//!
//! A measurement, not a contract. The owner is
//! `docs/planning/engine/island-rollback-and-compact-peer-input.md`.

use std::collections::BTreeMap;

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::lifecycle::LiveRoomInstance;
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::rollback::{RoomSnapshot, RoomSnapshotRows};

use crate::a_room_rewinds_alone::{clock, of_room, set_clock, Clock};
use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_fights, bobs, bodies, holds_right, UpdateTimes, LATENCY};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

/// Empty the rollback-cleared message channels at a rewind, as `bevy_ggrs`
/// does on a load.
const CLEAR_MESSAGES: bool = true;

struct Scene {
    name: &'static str,
    room: &'static str,
    frames: i32,
    alice: fn(i32) -> ControlFrame,
    bob: fn(i32) -> ControlFrame,
}

const SCENES: [Scene; 3] = [
    Scene { name: "one room, inputs never change", room: ROOM, frames: 240, alice: holds_right, bob: holds_right },
    Scene { name: "one room, players only", room: ROOM, frames: 240, alice: alice_fights, bob: bobs },
    Scene { name: "one room with its NPCs", room: "central_hub_complex", frames: 240, alice: alice_fights, bob: bobs },
];

fn booted(scene: &Scene) -> (Platformer2dSimHarness, LiveRoomInstance) {
    let mut host = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(scene.room)).expect("the room boots");
    let room = bob_beside_alice(&mut host, scene.room, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    (host, room)
}

/// Every body at the end, by peer-stable id.
type Ending = BTreeMap<SimId, [f32; 5]>;

/// The oracle: Bob's input reaches the host on time.
fn on_time(scene: &Scene) -> Ending {
    let (mut host, _) = booted(scene);
    for frame in 1..=scene.frames {
        host.drive_seat(0, (scene.alice)(frame));
        host.drive_seat(1, (scene.bob)(frame));
        host.app_mut().update();
    }
    bodies(&mut host, None)
}

struct RoomRollbackRun {
    times: UpdateTimes,
    rollbacks: u32,
    replayed: u32,
    save: std::time::Duration,
    restore: std::time::Duration,
    ending: Ending,
}

/// Run frame `frame` from the state the host is in: save the room first.
fn step(
    host: &mut Platformer2dSimHarness,
    rows: &RoomSnapshotRows,
    room: LiveRoomInstance,
    saved: &mut BTreeMap<i32, (RoomSnapshot, Clock)>,
    run: &mut RoomRollbackRun,
    frame: i32,
    alice: ControlFrame,
    bob: ControlFrame,
) {
    let started = std::time::Instant::now();
    let entities = of_room(host.world_mut(), room);
    let snapshot = rows.save(host.world(), &entities);
    run.save += started.elapsed();
    saved.insert(frame, (snapshot, clock(host.world())));
    saved.retain(|kept, _| *kept > frame - LATENCY - 2);
    host.drive_seat(0, alice);
    host.drive_seat(1, bob);
    run.times.time(host);
}

/// The host predicts Bob, and rewinds his room when a late input says
/// otherwise.
fn with_room_rollback(scene: &Scene) -> RoomRollbackRun {
    let (mut host, room) = booted(scene);
    let rows = host.world_mut().remove_resource::<RoomSnapshotRows>().expect("the host keeps the room rows");
    let mut run = RoomRollbackRun {
        times: UpdateTimes::default(),
        rollbacks: 0,
        replayed: 0,
        save: Default::default(),
        restore: Default::default(),
        ending: Ending::new(),
    };
    let mut saved: BTreeMap<i32, (RoomSnapshot, Clock)> = BTreeMap::new();
    let mut used: BTreeMap<i32, ControlFrame> = BTreeMap::new();
    let mut known = ControlFrame::default();

    // Bob's input for `arrived` reaches the host; rewind if it was predicted
    // wrong, and replay to `now` (exclusive).
    let arrive = |host: &mut Platformer2dSimHarness,
                  saved: &mut BTreeMap<i32, (RoomSnapshot, Clock)>,
                  used: &mut BTreeMap<i32, ControlFrame>,
                  run: &mut RoomRollbackRun,
                  known: &mut ControlFrame,
                  arrived: i32,
                  now: i32| {
        let actual = (scene.bob)(arrived);
        *known = actual;
        if used.get(&arrived) == Some(&actual) {
            return;
        }
        run.rollbacks += 1;
        let started = std::time::Instant::now();
        let (mut snapshot, at) = saved.remove(&arrived).expect("the room was saved before the frame it rewinds to");
        let entities = of_room(host.world_mut(), room);
        rows.restore(host.world_mut(), &mut snapshot, &entities);
        if CLEAR_MESSAGES {
            ambition_platformer2d::platformer::lifecycle::clear_channels_for_a_rewind(host.world_mut());
        }
        set_clock(host.world_mut(), at);
        run.restore += started.elapsed();
        for frame in arrived..now {
            // Frames after `arrived` have no arrived input yet: predict it.
            let bob = if frame == arrived { actual } else { *known };
            used.insert(frame, bob);
            step(host, &rows, room, saved, run, frame, (scene.alice)(frame), bob);
            run.replayed += 1;
        }
    };

    for frame in 1..=scene.frames {
        let arrived = frame - LATENCY;
        if arrived >= 1 {
            arrive(&mut host, &mut saved, &mut used, &mut run, &mut known, arrived, frame);
        }
        used.insert(frame, known);
        step(&mut host, &rows, room, &mut saved, &mut run, frame, (scene.alice)(frame), known);
    }
    // The last inputs arrive after the last frame: settle them, run nothing new.
    for arrived in (scene.frames - LATENCY + 1).max(1)..=scene.frames {
        arrive(&mut host, &mut saved, &mut used, &mut run, &mut known, arrived, scene.frames + 1);
    }
    host.world_mut().insert_resource(rows);
    run.ending = bodies(&mut host, None);
    let _ = where_they_are(&mut host);
    run
}

#[test]
#[ignore = "measurement: prints what room-level rollback costs a hosted world; run with --ignored --nocapture --test-threads=1"]
fn probe_what_room_rollback_costs_a_hosted_world() {
    for scene in &SCENES {
        let oracle = on_time(scene);
        let run = with_room_rollback(scene);
        let frames = f64::from(scene.frames);
        let differing: Vec<String> = oracle
            .iter()
            .filter(|(id, state)| run.ending.get(*id) != Some(*state))
            .map(|(id, state)| format!("{id:?}: oracle {state:?}, host {:?}", run.ending.get(id)))
            .collect();
        println!("[room-rollback] {} ({}): {} frames", scene.name, scene.room, scene.frames);
        println!(
            "[room-rollback]   host update {:.2} ms a frame ({} updates, {} of them replays); {} rollbacks",
            run.times.total.as_secs_f64() * 1e3 / frames,
            run.times.count,
            run.replayed,
            run.rollbacks
        );
        println!(
            "[room-rollback]   room saves {:.3} ms a frame, restores {:.3} ms a rollback",
            run.save.as_secs_f64() * 1e3 / f64::from(run.times.count.max(1)),
            run.restore.as_secs_f64() * 1e3 / f64::from(run.rollbacks.max(1))
        );
        println!(
            "[room-rollback]   ends where the on-time oracle ends: {} ({} of {} bodies differ)",
            differing.is_empty(),
            differing.len(),
            oracle.len()
        );
        for line in differing.iter().take(4) {
            println!("[room-rollback]     {line}");
        }
    }
}
