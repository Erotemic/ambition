//! NET-AUTHORITY-EXPERIMENT NA-2: the smallest player-hosted authoritative
//! fixture, beside the GGRS pair it is compared with.
//!
//! The host is one ordinary App with both seats and no rollback session. The
//! guest's input reaches it [`LATENCY`] updates late and is applied once, in
//! order, when it arrives. What the host would send back is a snapshot of the
//! bodies in the guest's room. Nothing is sent: the fixture counts what a
//! snapshot would cost. The GGRS pair runs the same scene with the same
//! latency, and both report bytes and update time.
//!
//! This is a measurement, not a contract: no part of the hosted path ships,
//! and the numbers move with the room. The owner is
//! `docs/planning/engine/hosted-world-networking-experiment.md`.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{BodyKinematics, ControlFrame};
use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
use ambition_platformer2d::platformer::sim_id::SimId;
use ambition_platformer2d::rollback::{
    loopback_transports, start_peer_session, stop_session, LinkTraffic, PeerSessionSettings, RollbackFrameCount,
    SaveWorld,
};

use crate::common::{door_to, doors_cross_at_once, fixed_60hz_room_options};
use crate::two_players_two_live_rooms::{bob_beside_alice, where_they_are, ROOM};

/// Updates a message waits on the link, as in `two_peers`.
const LATENCY: i32 = 3;
/// The width of one input on the guest-to-host link: a bincode `ControlFrame`
/// (72 bytes) and a 4-byte sequence number.
const INPUT_RECORD: u64 = 72 + 4;
/// One body in a snapshot: a 4-byte network id, position, velocity and facing
/// as `f32`. A lower bound: animation, health, items and room state are not in
/// it.
const BODY_RECORD: u64 = 4 + 8 + 8 + 4;

/// Bob's input: he turns at his own rhythm and jumps, so a GGRS peer
/// mispredicts him and rolls back.
fn bobs(frame: i32) -> ControlFrame {
    let right = (frame / 13) % 2 == 0;
    ControlFrame {
        axis_x: if right { 1.0 } else { -1.0 },
        jump_pressed: frame % 29 == 0,
        jump_held: frame % 29 < 6,
        ..Default::default()
    }
}

/// Alice holds right and presses attack every ten frames from frame 20.
fn alice_fights(frame: i32) -> ControlFrame {
    ControlFrame { axis_x: 1.0, attack_pressed: frame >= 20 && frame % 10 == 0, ..Default::default() }
}

/// Alice presses interact on frame 30: she leaves through the hub door and
/// Bob stays, so the rest of the run has two live rooms.
pub(crate) fn alice_leaves(frame: i32) -> ControlFrame {
    ControlFrame { interact_pressed: frame == 30, interact_held: (30..33).contains(&frame), ..Default::default() }
}

/// Put Alice on the door to the hub, which crosses at once.
pub(crate) fn alice_on_the_hub_door(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    doors_cross_at_once(sim);
    let center = door_to(sim, "central_hub_complex").aabb.center();
    sim.teleport_player((center.x, center.y));
}

struct Scene {
    name: &'static str,
    room: &'static str,
    frames: i32,
    alice: fn(i32) -> ControlFrame,
    bob: fn(i32) -> ControlFrame,
    prepare: fn(&mut Platformer2dSimHarness),
}

/// Neither input changes after its first frame, so a GGRS peer's prediction
/// is right and it rolls back (almost) never: the control that separates what
/// GGRS pays every frame (a snapshot, a checksum) from what it pays to correct.
fn holds_right(_frame: i32) -> ControlFrame {
    ControlFrame { axis_x: 1.0, ..Default::default() }
}

const CONTROL: &str = "one room, inputs never change";

const SCENES: [Scene; 4] = [
    Scene { name: CONTROL, room: ROOM, frames: 240, alice: holds_right, bob: holds_right, prepare: |_| {} },
    Scene { name: "one room, players only", room: ROOM, frames: 240, alice: alice_fights, bob: bobs, prepare: |_| {} },
    Scene { name: "one room with its NPCs", room: "central_hub_complex", frames: 240, alice: alice_fights, bob: bobs, prepare: |_| {} },
    Scene { name: "two rooms after a crossing", room: ROOM, frames: 220, alice: alice_leaves, bob: bobs, prepare: alice_on_the_hub_door },
];

#[derive(Default)]
struct UpdateTimes {
    total: Duration,
    worst: Duration,
    count: u32,
}

impl UpdateTimes {
    fn time(&mut self, sim: &mut Platformer2dSimHarness) {
        let start = Instant::now();
        sim.app_mut().update();
        let spent = start.elapsed();
        self.total += spent;
        self.worst = self.worst.max(spent);
        self.count += 1;
    }

    fn mean_ms(&self) -> f64 {
        self.total.as_secs_f64() * 1e3 / f64::from(self.count.max(1))
    }
}

/// The bodies in `room` (all rooms with `None`), by peer-stable id.
pub(crate) fn bodies(sim: &mut Platformer2dSimHarness, room: Option<LiveRoomInstance>) -> BTreeMap<SimId, [f32; 5]> {
    let world = sim.world_mut();
    world
        .query::<(&SimId, &BodyKinematics, Option<&InRoomInstance>)>()
        .iter(world)
        .filter(|(_, _, at)| room.is_none() || at.map(|at| at.0) == room)
        .map(|(id, body, _)| (id.clone(), [body.pos.x, body.pos.y, body.vel.x, body.vel.y, body.facing]))
        .collect()
}

/// How many peer-stable simulated entities stand in `room`: bodies, and also
/// projectiles, items, switches and every other thing with a `SimId`. A
/// replication of the room would name each of them.
fn identified(sim: &mut Platformer2dSimHarness, room: Option<LiveRoomInstance>) -> usize {
    let world = sim.world_mut();
    world
        .query::<(&SimId, Option<&InRoomInstance>)>()
        .iter(world)
        .filter(|(_, at)| room.is_none() || at.map(|at| at.0) == room)
        .count()
}

/// What the host would send the guest, by the snapshot policy.
#[derive(Default, Debug)]
struct SnapshotBytes {
    /// Every body in the world, every frame.
    whole_world: u64,
    /// The bodies in the guest's room, every frame.
    guests_room: u64,
    /// The bodies in the guest's room whose state changed since the last
    /// snapshot, plus a spawn record (id string and network id) the first time
    /// the guest sees a body.
    guests_room_changed: u64,
}

struct HostedRun {
    times: UpdateTimes,
    input_bytes: u64,
    snapshots: SnapshotBytes,
    bodies_in_guests_room: usize,
    bodies_in_world: usize,
    /// The most identified entities in the guest's room on any frame.
    identified_in_guests_room: usize,
}

fn hosted(scene: &Scene) -> HostedRun {
    let mut host = Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(scene.room)).expect("the room boots");
    bob_beside_alice(&mut host, scene.room, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    (scene.prepare)(&mut host);
    let mut link: VecDeque<(i32, ControlFrame)> = VecDeque::new();
    let mut times = UpdateTimes::default();
    let mut snapshots = SnapshotBytes::default();
    let mut input_bytes = 0;
    let mut sent: BTreeMap<SimId, [f32; 5]> = BTreeMap::new();
    let mut known: BTreeMap<SimId, ()> = BTreeMap::new();
    let mut identified_in_guests_room = 0;
    for frame in 1..=scene.frames {
        link.push_back((frame + LATENCY, (scene.bob)(frame)));
        input_bytes += INPUT_RECORD;
        let mut bobs_input = ControlFrame::default();
        while link.front().is_some_and(|(arrives, _)| *arrives <= frame) {
            bobs_input = link.pop_front().expect("checked").1;
        }
        host.drive_seat(0, (scene.alice)(frame));
        host.drive_seat(1, bobs_input);
        times.time(&mut host);
        let bobs_room = where_they_are(&mut host).1.flatten();
        let all = bodies(&mut host, None);
        let his = bodies(&mut host, bobs_room);
        identified_in_guests_room = identified_in_guests_room.max(identified(&mut host, bobs_room));
        snapshots.whole_world += all.len() as u64 * BODY_RECORD;
        snapshots.guests_room += his.len() as u64 * BODY_RECORD;
        for (id, state) in &his {
            if known.insert(id.clone(), ()).is_none() {
                snapshots.guests_room_changed += serde_json::to_string(id).expect("an id serializes").len() as u64 + 4;
            }
            if sent.get(id) != Some(state) {
                snapshots.guests_room_changed += BODY_RECORD;
            }
        }
        sent = his;
    }
    let bobs_room = where_they_are(&mut host).1.flatten();
    HostedRun {
        times,
        input_bytes,
        snapshots,
        bodies_in_guests_room: bodies(&mut host, bobs_room).len(),
        bodies_in_world: bodies(&mut host, None).len(),
        identified_in_guests_room,
    }
}

struct PeerRun {
    times: [UpdateTimes; 2],
    traffic: [LinkTraffic; 2],
    frames: i32,
    rollbacks: u64,
    resimulated: u64,
    /// Milliseconds one run of a peer's `SaveWorld` takes at the end: the
    /// snapshot and checksum bevy_ggrs takes of every frame it simulates.
    save_ms: f64,
    /// Systems in `SaveWorld`: about one per registered rollback type.
    save_systems: usize,
}

/// A GGRS peer with no census: the cost of the shipped path, not of the
/// `two_peers` instruments.
fn peer(scene: &Scene, local: usize, remote: (usize, std::net::SocketAddr), transport: ambition_platformer2d::rollback::LoopbackTransport) -> Platformer2dSimHarness {
    let options = fixed_60hz_room_options(scene.room).with_sync_test_rollback_settings(4, 10).with_rollback_players(2);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the room boots");
    bob_beside_alice(&mut sim, scene.room, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    (scene.prepare)(&mut sim);
    let settings = PeerSessionSettings {
        players: 2,
        local: vec![local],
        remote: vec![remote],
        max_prediction_window: 8,
        input_delay: 0,
        desync_interval: 1,
        disconnect_timeout: Duration::from_secs(600),
    };
    let world = sim.world_mut();
    stop_session(world);
    start_peer_session(world, &settings, transport).expect("each peer starts at the same world");
    sim
}

fn confirmed(sim: &Platformer2dSimHarness) -> i32 {
    sim.world()
        .get_resource::<ambition_platformer2d::engine_core::ConfirmedFrameBoundary>()
        .map_or(-1, |boundary| boundary.confirmed)
}

fn peers(scene: &Scene) -> PeerRun {
    let (a, b) = ("127.0.0.1:7101".parse().unwrap(), "127.0.0.1:7102".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY as u32);
    let ends = [to_bob.clone(), to_alice.clone()];
    let mut alice = peer(scene, 0, (1, b), to_bob);
    let mut bob = peer(scene, 1, (0, a), to_alice);
    let mut times: [UpdateTimes; 2] = Default::default();
    let mut updates = 0;
    while confirmed(&alice).min(confirmed(&bob)) < scene.frames {
        updates += 1;
        assert!(updates < 20 * scene.frames, "{}: the peers stopped confirming", scene.name);
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let input = if index == 0 { (scene.alice)(next) } else { (scene.bob)(next) };
            sim.drive_seat(index as u8, input);
            times[index].time(sim);
        }
        assert_eq!(alice.rollback_health(), Ok(()), "{}: Alice's session failed", scene.name);
        assert_eq!(bob.rollback_health(), Ok(()), "{}: Bob's session failed", scene.name);
    }
    let stats: Vec<_> = [&alice, &bob].iter().filter_map(|sim| sim.rollback_execution_stats()).collect();
    // Run Alice's save on its own, after the session is done with her. It
    // overwrites the snapshot of her current frame with an equal one.
    let saves = 60;
    let start = Instant::now();
    for _ in 0..saves {
        alice.world_mut().run_schedule(SaveWorld);
    }
    let save_ms = start.elapsed().as_secs_f64() * 1e3 / f64::from(saves);
    let save_systems = alice
        .app_mut()
        .get_schedule_mut(SaveWorld)
        .map_or(0, |schedule| schedule.systems_len());
    PeerRun {
        save_ms,
        save_systems,
        times,
        traffic: ends.map(|end| end.sent()),
        frames: confirmed(&alice).min(confirmed(&bob)),
        // Per-session counters: the sync test the harness ran before the peer
        // session started is in the lifetime totals.
        rollbacks: stats.iter().map(|stats| stats.load_runs).sum(),
        resimulated: stats
            .iter()
            .map(|stats| stats.advance_runs.saturating_sub(stats.highest_simulated_frame.map_or(0, |frame| frame as u64 + 1)))
            .sum(),
    }
}

/// Run each scene on both topologies and print the comparison. The only
/// assertions are the controls: both topologies ran, the GGRS peers rolled
/// back where Bob turns, and the last scene really ends in two rooms.
#[test]
#[ignore = "measurement: prints hosted-versus-GGRS bytes and update time; run with --ignored --nocapture --test-threads=1"]
fn what_a_hosted_world_costs_beside_ggrs() {
    for scene in &SCENES {
        let host = hosted(scene);
        let pair = peers(scene);
        if scene.name != CONTROL {
            assert!(pair.rollbacks > 0, "{}: no prediction was corrected, so GGRS paid nothing for latency", scene.name);
        }
        if scene.name.starts_with("two rooms") {
            assert!(
                host.bodies_in_guests_room < host.bodies_in_world,
                "{}: the guest's room holds every body ({}), so nothing was scoped",
                scene.name,
                host.bodies_in_world
            );
        }
        let frames = scene.frames as f64;
        println!(
            "[hosted] {} ({}): {} frames, {} bodies in the world and {} in the guest's room at the end; at most {} identified entities in his room",
            scene.name, scene.room, scene.frames, host.bodies_in_world, host.bodies_in_guests_room, host.identified_in_guests_room
        );
        println!(
            "[hosted]   host update {:.2} ms mean, {:.2} ms worst; guest runs no simulation",
            host.times.mean_ms(),
            host.times.worst.as_secs_f64() * 1e3
        );
        println!("[hosted]   guest -> host inputs: {:.1} bytes a frame", host.input_bytes as f64 / frames);
        println!(
            "[hosted]   host -> guest bodies: whole world {:.1}, guest's room {:.1}, guest's room changed-only {:.1} bytes a frame",
            host.snapshots.whole_world as f64 / frames,
            host.snapshots.guests_room as f64 / frames,
            host.snapshots.guests_room_changed as f64 / frames
        );
        println!(
            "[hosted]   host: {:.2} ms of update a frame",
            host.times.total.as_secs_f64() * 1e3 / frames
        );
        for (who, (times, sent)) in ["alice", "bob"].into_iter().zip(pair.times.iter().zip(pair.traffic)) {
            println!(
                "[ggrs]   {who}: update {:.2} ms mean, {:.2} ms worst, {:.2} ms a confirmed frame; sent {:.1} bytes a frame in {:.2} messages a frame",
                times.mean_ms(),
                times.worst.as_secs_f64() * 1e3,
                times.total.as_secs_f64() * 1e3 / f64::from(pair.frames),
                sent.bytes as f64 / f64::from(pair.frames),
                sent.messages as f64 / f64::from(pair.frames)
            );
        }
        println!(
            "[ggrs]   {} rollbacks, {} resimulated frames over {} confirmed frames (both peers); one SaveWorld run {:.2} ms over {} systems",
            pair.rollbacks, pair.resimulated, pair.frames, pair.save_ms, pair.save_systems
        );
    }
}
