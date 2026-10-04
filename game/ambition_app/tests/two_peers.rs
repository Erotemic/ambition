//! Two peers in one process (netcode N2). Each peer is an App with a P2P
//! session over an in-memory link that delivers late, so each peer predicts
//! the other's input and rolls back when the prediction was wrong. Alice
//! drives slot 0 on peer A; Bob drives slot 1 on peer B.
//!
//! A sync test compares one App with itself. This compares two Apps, frame by
//! frame: the peer checksum (GGRS reports a desync), and every probed row,
//! including the rows outside the peer checksum (S7 in
//! `docs/planning/engine/simulation-authority-and-determinism.md`), which no
//! peer ever compares at run time.

use std::collections::BTreeMap;
use std::time::Duration;

use ambition_app::{AmbitionSim as _, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{ConfirmedFrameBoundary, ControlFrame};
use ambition_platformer2d::rollback::{
    loopback_pair, start_peer_session, stop_session, GgrsSchedule, PeerSessionSettings,
    RollbackChecksumProbes, RollbackFrameCount, SaveWorld,
};
use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase;
use bevy::prelude::*;

use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options, strengthen_the_float_rows};
use crate::two_players_two_live_rooms::{bob_beside_alice, ROOM};

/// Frames both peers must confirm.
const CONFIRMED: i32 = 240;
/// Updates a message waits on the link: the remote input arrives this late.
const LATENCY: u32 = 3;

/// Each probed row's peer census, by the frame it was saved at. A frame that
/// is simulated again after a rollback is recorded again, so a confirmed
/// frame holds the state the timeline agreed on.
#[derive(Resource, Default)]
struct CensusByFrame(BTreeMap<i32, BTreeMap<&'static str, (usize, u64)>>);

/// Each save of a frame this peer had saved before: (the frame, the highest
/// frame saved until then). A rollback loads a frame and saves each frame
/// after it again, so an entry `(f, h)` is a rewind from `h` to before `f`.
#[derive(Resource, Default)]
struct SavedAgain(Vec<(i32, i32)>);

fn record_the_census(world: &mut World) {
    let frame = world.resource::<RollbackFrameCount>().0;
    let highest = world.resource::<CensusByFrame>().0.keys().next_back().copied();
    if let Some(highest) = highest.filter(|highest| frame <= *highest) {
        world.resource_mut::<SavedAgain>().0.push((frame, highest));
    }
    let probes = world.resource::<RollbackChecksumProbes>().clone();
    let census = probes
        .census_all_as_peers_compare(world)
        .into_iter()
        .map(|(row, reading)| (row, (reading.count, reading.xor)))
        .collect();
    world.resource_mut::<CensusByFrame>().0.insert(frame, census);
}

/// What peer B alone does to its own world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Poison {
    None,
    /// Alice's position, from frame 90: a value in the peer checksum.
    Checksummed,
    /// Alice's persona baseline id, from frame 90: a value outside it that
    /// its probe reads, and that only a re-wear reads. (`Transform` and
    /// `ActorRenderSize` are outside it too, but their probes count entities
    /// only, so no census sees their values.)
    Unchecksummed,
}

#[derive(Resource, Clone, Copy)]
struct ThisPeersPoison(Poison);

fn poison_alices_position(
    poison: Res<ThisPeersPoison>,
    frame: Res<RollbackFrameCount>,
    mut alice: Query<
        &mut ambition_platformer2d::engine_core::BodyKinematics,
        With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >,
) {
    if poison.0 == Poison::Checksummed && frame.0 >= 90 {
        for mut kinematics in &mut alice {
            kinematics.pos.x += 0.5;
        }
    }
}

type Baseline = ambition_platformer2d::body_seed::PersonaBaseline;

fn poison_alices_baseline(
    poison: Res<ThisPeersPoison>,
    frame: Res<RollbackFrameCount>,
    mut alice: Query<&mut Baseline, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>,
) {
    if poison.0 == Poison::Unchecksummed && frame.0 >= 90 {
        for mut baseline in &mut alice {
            baseline.id = "poisoned on one peer".to_string();
        }
    }
}

/// One peer: the same world on both, Bob seated on slot 1 beside Alice, and
/// a P2P session in place of the sync test the harness started.
fn peer(
    room: &str,
    local: usize,
    remote: (usize, std::net::SocketAddr),
    socket: ambition_platformer2d::rollback::LoopbackSocket,
    poison: Poison,
) -> (Platformer2dSimHarness, Vec<&'static str>) {
    peer_prepared_by(room, local, remote, socket, poison, |_| {})
}

/// [`peer`], with `prepare` run on the world before the P2P session starts.
/// Each peer runs the same `prepare`, so the two worlds are still equal at
/// frame zero.
fn peer_prepared_by(
    room: &str,
    local: usize,
    remote: (usize, std::net::SocketAddr),
    socket: ambition_platformer2d::rollback::LoopbackSocket,
    poison: Poison,
    prepare: fn(&mut Platformer2dSimHarness),
) -> (Platformer2dSimHarness, Vec<&'static str>) {
    let options = fixed_60hz_room_options(room)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the room boots");
    bob_beside_alice(&mut sim, room, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    prepare(&mut sim);
    // The float rows get value probes, or their census is a carrier count.
    let sharp = strengthen_the_float_rows(sim.world_mut());
    let app = sim.app_mut();
    app.init_resource::<CensusByFrame>();
    app.init_resource::<SavedAgain>();
    app.insert_resource(ThisPeersPoison(poison));
    app.add_systems(SaveWorld, record_the_census);
    app.add_systems(GgrsSchedule, (poison_alices_position, poison_alices_baseline));
    let settings = PeerSessionSettings {
        players: 2,
        local: vec![local],
        remote: vec![remote],
        max_prediction_window: 8,
        input_delay: 0,
        desync_interval: 1,
        // An update of this App can take far longer than a network round
        // trip; the measurement must not drop a peer for that.
        disconnect_timeout: Duration::from_secs(600),
    };
    // One world access: an update between the stop and the start would let
    // the local host start a sync test again.
    let world = sim.world_mut();
    stop_session(world);
    start_peer_session(world, &settings, socket).expect("each peer starts at the same world");
    (sim, sharp)
}

/// The input a slot holds at `frame`: each turns at its own rhythm, so a
/// prediction (the last input repeated) is wrong at each turn.
fn script(slot: usize, frame: i32) -> ControlFrame {
    let (period, jump) = if slot == 0 { (20, 37) } else { (13, 29) };
    let right = (frame / period) % 2 == 0;
    ControlFrame {
        axis_x: if right { 1.0 } else { -1.0 },
        jump_pressed: frame % jump == 0,
        jump_held: frame % jump < 6,
        ..Default::default()
    }
}

fn confirmed(sim: &Platformer2dSimHarness) -> i32 {
    sim.world()
        .get_resource::<ConfirmedFrameBoundary>()
        .map_or(-1, |boundary| boundary.confirmed)
}

struct Outcome {
    healths: (Result<(), String>, Result<(), String>),
    rollbacks: u64,
    /// Rows whose census differs between the peers, each with the first
    /// confirmed frame it differs at.
    differing: BTreeMap<&'static str, i32>,
    /// The rows whose probe reads a value, not only presence.
    value_probed: std::collections::BTreeSet<&'static str>,
    /// The float rows (S7) given a value probe for this run.
    sharp: Vec<&'static str>,
    /// The float rows with at least one carrier at some confirmed frame. A
    /// row with none agrees the way two empty sets agree.
    carried: std::collections::BTreeSet<&'static str>,
}

fn two_peers(poison: Poison) -> Outcome {
    two_peers_in(ROOM, |frame| script(0, frame), poison)
}

/// [`two_peers`] in `room`, with Alice's input a function of the frame. Bob
/// always runs his script, so a peer always has a remote input to predict.
fn two_peers_in(room: &str, alices: fn(i32) -> ControlFrame, poison: Poison) -> Outcome {
    let (a, b) = ("127.0.0.1:7001".parse().unwrap(), "127.0.0.1:7002".parse().unwrap());
    let (to_bob, to_alice) = loopback_pair(a, b, LATENCY);
    let (mut alice, sharp) = peer(room, 0, (1, b), to_bob, Poison::None);
    let (mut bob, _) = peer(room, 1, (0, a), to_alice, poison);
    let mut updates = 0;
    while confirmed(&alice).min(confirmed(&bob)) < CONFIRMED {
        updates += 1;
        assert!(updates < 20 * CONFIRMED, "the peers confirmed only to {} and {}", confirmed(&alice), confirmed(&bob));
        for (sim, slot) in [(&mut alice, 0), (&mut bob, 1)] {
            if sim.rollback_health().is_err() {
                continue;
            }
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let input = if slot == 0 { alices(next) } else { script(slot, next) };
            sim.drive_seat(slot as u8, input);
            sim.app_mut().update();
        }
        if alice.rollback_health().is_err() || bob.rollback_health().is_err() {
            break;
        }
    }
    let rollbacks = [&alice, &bob]
        .iter()
        .filter_map(|sim| sim.rollback_execution_stats())
        .map(|stats| stats.lifetime_load_runs)
        .sum();
    let last = confirmed(&alice).min(confirmed(&bob));
    let (left, right) = (
        &alice.world().resource::<CensusByFrame>().0,
        &bob.world().resource::<CensusByFrame>().0,
    );
    let carried = sharp
        .iter()
        .copied()
        .filter(|row| left.values().any(|census| census.get(row).is_some_and(|(count, _)| *count > 0)))
        .collect();
    let mut differing = BTreeMap::new();
    for frame in 0..=last {
        let (Some(left), Some(right)) = (left.get(&frame), right.get(&frame)) else {
            continue;
        };
        for (row, reading) in left {
            if right.get(row) != Some(reading) {
                differing.entry(*row).or_insert(frame);
            }
        }
    }
    let probes = alice.world().resource::<RollbackChecksumProbes>();
    let value_probed = &probes.type_names() - &probes.presence_only_type_names();
    Outcome {
        carried,
        sharp,
        value_probed,
        healths: (alice.rollback_health(), bob.rollback_health()),
        rollbacks,
        differing,
    }
}

/// Two peers that run the same inputs from the same world agree at every
/// confirmed frame on every probed row, and GGRS reports no desync. The
/// control: the late link made both peers roll back.
#[test]
fn two_peers_agree_at_every_confirmed_frame() {
    let outcome = two_peers(Poison::None);
    assert!(
        outcome.sharp.iter().all(|row| outcome.value_probed.contains(row)),
        "precondition: every float row has a value probe, or the census compares carrier counts"
    );
    // The population: the float rows `switch_lab` and two bodies carry. The
    // portal, hazard, boss, mount, ground-item and camera-zoom rows have no
    // carrier here; `two_local_histories_agree_about_the_sharp_unchecksummed_rows`
    // walks the rooms that author them, on one host each.
    assert_eq!(
        outcome.carried.iter().map(|row| row.rsplit("::").next().unwrap()).collect::<Vec<_>>(),
        vec![
            "BodyAnimFacts",
            "CombatTuning",
            "ActorRenderSize",
            "ActorSpriteOffset",
            "SpawnBaseline",
            "PlayerBlinkCameraState",
            "SpritePosedBody",
            "Transform",
        ],
        "the float rows this run carries changed; a row that left is coverage lost"
    );
    assert!(outcome.rollbacks > 0, "control: no peer rolled back, so no prediction was tested");
    assert_eq!(
        (outcome.healths, outcome.differing),
        ((Ok(()), Ok(())), BTreeMap::new()),
        "(the peers' health, the rows that differ and the first frame they differ at)"
    );
}

/// A value in the peer checksum that one peer alone changes is a desync GGRS
/// reports, and the session stops.
#[test]
fn a_checksummed_difference_is_a_desync() {
    let outcome = two_peers(Poison::Checksummed);
    assert!(
        outcome.healths.0.is_err() || outcome.healths.1.is_err(),
        "one peer moved Alice and no peer reported a desync"
    );
}

/// A value outside the peer checksum that one peer alone changes is no
/// desync: GGRS cannot see it. Only this census can.
#[test]
fn an_unchecksummed_difference_is_seen_only_by_the_census() {
    let outcome = two_peers(Poison::Unchecksummed);
    let row = std::any::type_name::<Baseline>();
    assert!(
        outcome.value_probed.contains(row),
        "precondition: `{row}` has a value probe, or no census can see the poison"
    );
    assert_eq!(
        (outcome.healths, outcome.differing.keys().collect::<Vec<_>>()),
        ((Ok(()), Ok(())), vec![&row]),
        "(the peers' health, the rows the census saw differ)"
    );
}

/// Alice stands still.
fn stands(_frame: i32) -> ControlFrame {
    ControlFrame::default()
}

/// Alice holds right.
fn holds_right(_frame: i32) -> ControlFrame {
    ControlFrame {
        axis_x: 1.0,
        ..Default::default()
    }
}

/// Alice holds right and, from frame 20, presses attack every ten frames:
/// the first press takes the portal gun beside her, and each later one fires.
fn holds_right_and_fires(frame: i32) -> ControlFrame {
    ControlFrame {
        axis_x: 1.0,
        attack_pressed: frame >= 20 && frame % 10 == 0,
        ..Default::default()
    }
}

/// The rooms that carry the float rows `switch_lab` does not: the ground
/// item, the portal rows, the hazard, the boss rows, the shark's mount rows
/// and the encounter zoom. Each pair of peers agrees at every confirmed frame,
/// and together the rooms carry every float row that has a production writer.
#[test]
fn two_peers_agree_in_the_rooms_that_carry_the_float_rows() {
    let walks: [(&str, fn(i32) -> ControlFrame); 7] = [
        ("blink_run", stands),
        ("portal_lab", holds_right),
        ("basement_hazards", stands),
        ("portal_bridge", holds_right_and_fires),
        ("basement_boss", stands),
        ("pirate_sky_lookout", stands),
        ("goblin_encounter", holds_right),
    ];
    let mut carried = std::collections::BTreeSet::new();
    let mut sharp = Vec::new();
    for (room, alices) in walks {
        let outcome = two_peers_in(room, alices, Poison::None);
        assert_eq!(
            (outcome.healths, outcome.differing),
            ((Ok(()), Ok(())), BTreeMap::new()),
            "in {room}: (the peers' health, the rows that differ and the first frame they differ at)"
        );
        carried.extend(outcome.carried);
        sharp = outcome.sharp;
    }
    // `MountedSize` has no production writer (measured 2026-10-03: no
    // `MountedSize(` outside its definition), so no room carries it. A
    // writer arriving makes this red, and the walk that carries it belongs
    // above.
    let missing: Vec<&str> = sharp
        .iter()
        .copied()
        .filter(|row| !carried.contains(row))
        .map(|row| row.rsplit("::").next().unwrap())
        .collect();
    assert_eq!(missing, vec!["MountedSize"], "the float rows no walk carries");
}

/// The room Alice leaves for, from `switch_lab`.
const HUB: &str = "central_hub_complex";

/// Put Alice on the door to the hub.
fn alice_on_the_hub_door(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let center = crate::common::door_to(sim, HUB).aabb.center();
    sim.teleport_player((center.x, center.y));
}

/// Alice presses interact from frame 30, as a press and a release.
fn opens_the_door(frame: i32) -> ControlFrame {
    ControlFrame {
        interact_pressed: frame >= 30 && frame % 6 == 0,
        interact_held: frame >= 30 && frame % 6 < 3,
        ..Default::default()
    }
}

/// What one peer holds about Alice's crossing.
#[derive(Clone, Debug, PartialEq)]
struct Crossing {
    /// The frame the crossing was recorded on, while it waits.
    recorded_on: Option<i32>,
    room: String,
    live_rooms: usize,
    /// The phase of the readiness transaction of the crossing, when one is
    /// open. It is host state: each peer has its own.
    readiness: Option<RoomTransitionLoadPhase>,
}

fn crossing(sim: &mut Platformer2dSimHarness) -> Crossing {
    let recorded_on = sim
        .world()
        .resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .pending
        .as_ref()
        .map(|intent| intent.frame);
    let live_rooms = {
        let world = sim.world_mut();
        world
            .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>>()
            .iter(world)
            .count()
    };
    let readiness = sim
        .world()
        .resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>()
        .active
        .as_ref()
        .map(|transaction| transaction.phase);
    Crossing {
        recorded_on,
        room: sim.observation().active_room.clone(),
        live_rooms,
        readiness,
    }
}

/// THE POISON OF THE FREEZE: on this frame the peer session does not hold the
/// simulation, so one frame that must be frozen simulates. `None`: no poison.
#[derive(Resource, Clone, Copy)]
struct SimulatesOnOneFrozenFrame(Option<i32>);

type Ownership = ambition_platformer2d::rollback::RollbackSessionOwnership;

fn lift_the_freeze_on_one_frame(
    poison: Res<SimulatesOnOneFrozenFrame>,
    frame: Res<RollbackFrameCount>,
    mut ownership: ResMut<Ownership>,
) {
    if poison.0 == Some(frame.0) {
        *ownership = Ownership::External;
    }
}

fn put_the_freeze_back(
    poison: Res<SimulatesOnOneFrozenFrame>,
    frame: Res<RollbackFrameCount>,
    mut ownership: ResMut<Ownership>,
) {
    if poison.0 == Some(frame.0) {
        *ownership = Ownership::Peer;
    }
}

/// The frame Alice's crossing is recorded on (`R`): her first press.
const RECORDED_ON: i32 = 30;
/// The last frame that simulates while the crossing waits. The state saved
/// at this frame is the state of each later frame.
const LAST_LIVE: i32 =
    RECORDED_ON + ambition_platformer2d::rollback::PEER_COMMIT_FREEZE_DELAY - 1;

struct DoorOutcome {
    /// What each peer holds about the crossing at the end.
    crossings: [Crossing; 2],
    /// The frame each peer was at when it first saw the recording frame
    /// confirmed.
    at_confirmation: [Option<i32>; 2],
    /// Confirmed frames that both peers saved.
    compared: usize,
    /// Rows whose census differs between the peers, each with the first
    /// confirmed frame it differs at.
    differing: BTreeMap<&'static str, i32>,
    rollbacks: u64,
    /// On each peer: the first frame after [`LAST_LIVE`] whose census is not
    /// the census of [`LAST_LIVE`], with the rows that moved.
    moved_after_the_freeze: [Option<(i32, Vec<&'static str>)>; 2],
    /// On each peer: whether the census of [`LAST_LIVE`] differs from the
    /// census of the frame before it. The control: the world moved until the
    /// freeze.
    moved_before_the_freeze: [bool; 2],
    /// On each peer: the rewinds from a frozen frame to a frame at or before
    /// the recording frame, as (the frame saved again, the highest frame).
    rewinds_across_the_freeze: [Vec<(i32, i32)>; 2],
    /// On each peer: the ticks the simulation ran, and the frames GGRS ran.
    ticks_and_frames: [(u64, i32); 2],
}

/// Alice stands on the door to the hub on each peer and presses interact from
/// frame 30. Bob runs his script. Each peer confirms `after` frames past the
/// recording frame.
fn a_door_under_a_peer_session(after: i32, poison: Option<i32>) -> DoorOutcome {
    let (a, b) = ("127.0.0.1:7011".parse().unwrap(), "127.0.0.1:7012".parse().unwrap());
    let (to_bob, to_alice) = loopback_pair(a, b, LATENCY);
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, alice_on_the_hub_door);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, alice_on_the_hub_door);
    let root = ambition_platformer2d::platformer::schedule::GameplaySimulationRoot;
    for sim in [&mut alice, &mut bob] {
        let app = sim.app_mut();
        app.insert_resource(SimulatesOnOneFrozenFrame(poison));
        app.add_systems(
            GgrsSchedule,
            (lift_the_freeze_on_one_frame.before(root), put_the_freeze_back.after(root)),
        );
    }
    let ticks_at_frame_zero = [sim_tick(&alice), sim_tick(&bob)];

    let mut at_confirmation: [Option<i32>; 2] = [None, None];
    let mut updates = 0;
    while confirmed(&alice).min(confirmed(&bob)) < RECORDED_ON + after {
        updates += 1;
        assert!(
            updates < 20 * (RECORDED_ON + after),
            "the peers confirmed only to {} and {}",
            confirmed(&alice),
            confirmed(&bob)
        );
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let input = if index == 0 { opens_the_door(next) } else { script(index, next) };
            sim.drive_seat(index as u8, input);
            sim.app_mut().update();
            let recorded_on = crossing(sim).recorded_on;
            if at_confirmation[index].is_none()
                && recorded_on.is_some_and(|frame| confirmed(sim) >= frame)
            {
                at_confirmation[index] = Some(sim.world().resource::<RollbackFrameCount>().0);
            }
        }
        assert_eq!(
            (alice.rollback_health(), bob.rollback_health()),
            (Ok(()), Ok(())),
            "a peer reported a desync after {updates} updates"
        );
    }

    let last = confirmed(&alice).min(confirmed(&bob));
    let crossings = [crossing(&mut alice), crossing(&mut bob)];
    let censuses = [
        &alice.world().resource::<CensusByFrame>().0,
        &bob.world().resource::<CensusByFrame>().0,
    ];
    let mut compared = 0;
    let mut differing: BTreeMap<&'static str, i32> = BTreeMap::new();
    for frame in 0..=last {
        let (Some(left), Some(right)) = (censuses[0].get(&frame), censuses[1].get(&frame)) else {
            continue;
        };
        compared += 1;
        for (row, reading) in left {
            if right.get(row) != Some(reading) {
                differing.entry(*row).or_insert(frame);
            }
        }
    }
    let moved_after_the_freeze = censuses.map(|census| {
        let frozen = &census[&LAST_LIVE];
        census.range(LAST_LIVE + 1..=last).find(|(_, now)| *now != frozen).map(|(frame, now)| {
            let rows = now
                .iter()
                .filter(|(row, reading)| frozen.get(*row) != Some(reading))
                .map(|(row, _)| *row)
                .collect();
            (*frame, rows)
        })
    });
    let moved_before_the_freeze = censuses.map(|census| census[&(LAST_LIVE - 1)] != census[&LAST_LIVE]);
    let rewinds_across_the_freeze = [&alice, &bob].map(|sim| {
        sim.world()
            .resource::<SavedAgain>()
            .0
            .iter()
            .copied()
            .filter(|(frame, highest)| *frame <= RECORDED_ON && *highest > LAST_LIVE)
            .collect()
    });
    let rollbacks = [&alice, &bob]
        .iter()
        .filter_map(|sim| sim.rollback_execution_stats())
        .map(|stats| stats.lifetime_load_runs)
        .sum();
    let ticks_and_frames = [(&alice, ticks_at_frame_zero[0]), (&bob, ticks_at_frame_zero[1])]
        .map(|(sim, zero)| (sim_tick(sim) - zero, sim.world().resource::<RollbackFrameCount>().0));
    DoorOutcome {
        crossings,
        at_confirmation,
        compared,
        differing,
        rollbacks,
        moved_after_the_freeze,
        moved_before_the_freeze,
        rewinds_across_the_freeze,
        ticks_and_frames,
    }
}

/// ⛔ TODAY A DOOR UNDER A PEER SESSION IS ACCEPTED, FREEZES THE SIMULATION,
/// AND IS NEVER COMMITTED. The freeze is the first half of the peer barrier
/// (the open-world "Remote peers" row); the commit is the second, and it
/// changes the last assertion below.
///
/// Alice stands on the door to the hub on each peer and presses interact from
/// frame 30.
///
/// - Each peer records the crossing on frame 30 (`PendingLifecycleCommit`).
/// - From the next frame the gameplay simulation does not run on either
///   peer (`a_peer_commit_holds_the_simulation`). GGRS frames go on, and each
///   saved frame from 30 on has the census of frame 30. A rollback that
///   loads a frame before 30 runs into the same freeze.
/// - The peers do not diverge: no desync, and no probed row differs at a
///   confirmed frame.
/// - No peer commits the crossing: `commit_confirmed_lifecycle` runs only
///   for a `LocalSyncTest` session. Each peer's readiness transaction opens
///   and does not reach its authorization.
///
/// ⚠ WHY THE FREEZE. The sync-test rule runs the crossing on the current
/// world when its recording frame is confirmed. On that update the two peers
/// are at different frames (measured before the freeze: 36 and 31), each with
/// frames the other peer has not confirmed. With the freeze, those frames
/// hold one state.
#[test]
fn a_door_under_a_peer_session_freezes_the_simulation_and_is_not_committed_yet() {
    /// Frames each peer must confirm after the crossing was recorded.
    const AFTER: i32 = 270;
    let outcome = a_door_under_a_peer_session(AFTER, None);
    assert!(outcome.compared > AFTER as usize, "only {} confirmed frames were compared", outcome.compared);
    assert_eq!(outcome.differing, BTreeMap::new(), "the rows that differ, and the first frame of each");
    assert!(outcome.rollbacks > 0, "control: no peer rolled back, so no prediction was tested");

    // Accepted on each peer, on the same frame.
    assert_eq!(
        outcome.crossings.each_ref().map(|crossing| crossing.recorded_on),
        [Some(RECORDED_ON); 2],
        "the frame each peer recorded Alice's crossing on"
    );

    // THE FREEZE. No probed row moves after the last live frame, on either
    // peer. The control: the rows moved until then.
    assert_eq!(
        outcome.moved_before_the_freeze,
        [true; 2],
        "control: the census did not move in the frame before the freeze, so a \
         census that does not move after it shows nothing"
    );
    assert_eq!(
        outcome.moved_after_the_freeze,
        [None, None],
        "on each peer: the first frame after frame {LAST_LIVE} whose census moved, and its rows"
    );
    // A rewind across the freeze happened, so the frozen frames above were
    // also simulated again from a frame before the crossing existed.
    assert!(
        outcome.rewinds_across_the_freeze.iter().any(|rewinds| !rewinds.is_empty()),
        "control: no peer rewound from a frozen frame to before the recording \
         frame, so the freeze was not tested under a rewind"
    );

    // The two peers were at different frames when the recording frame was
    // confirmed, each past it.
    let [Some(alice_at), Some(bob_at)] = outcome.at_confirmation else {
        panic!("a peer never saw the recording frame confirmed: {:?}", outcome.at_confirmation);
    };
    assert!(
        alice_at != bob_at && alice_at > RECORDED_ON && bob_at > RECORDED_ON,
        "the peers were at frames {alice_at} and {bob_at} when frame {RECORDED_ON} was \
         confirmed. If they are equal now, the link or the loop changed; the \
         measured values before the freeze were 36 and 31"
    );

    // ⛔ THE STATE THE COMMIT CHANGES. Not committed: each peer is in the
    // room it started in, with one live room.
    //
    // ⚠ THE READINESS TRANSACTION WAITS TOO, and the commit must settle this
    // first. `authorize_ready_room_transition_system` authorizes on a tick
    // after the one that opened the transaction (`commit_not_before_tick`).
    // Each peer opens its transaction after the freeze began, so that tick
    // does not come. (A sync test given the same freeze by hand does not
    // commit for the same reason.)
    let stuck = Crossing {
        recorded_on: Some(RECORDED_ON),
        room: ROOM.to_string(),
        live_rooms: 1,
        readiness: Some(RoomTransitionLoadPhase::AwaitingReadiness),
    };
    assert_eq!(
        outcome.crossings,
        [stuck.clone(), stuck],
        "a peer session committed the crossing, or dropped it. If the peer \
         commit landed, this arm becomes its witness: Alice in `{HUB}` on each \
         peer, two live rooms, and the census above across the commit"
    );
}

/// The freeze arm can fail: with the simulation run on one frozen frame, the
/// census moves on that frame, and the arm above names it.
#[test]
fn one_frozen_frame_that_simulates_moves_the_census() {
    /// A frame after the freeze began, and before the frames this run confirms end.
    const POISONED: i32 = 45;
    let outcome = a_door_under_a_peer_session(40, Some(POISONED));
    let moved_on: Vec<Option<i32>> = outcome
        .moved_after_the_freeze
        .iter()
        .map(|moved| moved.as_ref().map(|(frame, _)| *frame))
        .collect();
    assert_eq!(
        moved_on,
        vec![Some(POISONED); 2],
        "on each peer: the first frame after the freeze whose census moved"
    );
    assert_eq!(
        outcome.ticks_and_frames.map(|(ticks, _)| ticks),
        [LAST_LIVE as u64 + 1; 2],
        "the ticks each peer simulated: the live frames and the poisoned one"
    );
}

fn sim_tick(sim: &Platformer2dSimHarness) -> u64 {
    sim.world().resource::<ambition_platformer2d::time::SimTick>().0
}

/// THE CONTROL FOR THE FREEZE: a host that commits alone does not freeze.
///
/// The same world and the same door as the peer arm, under the sync test the
/// harness starts (check distance 4). The crossing waits for its recording
/// frame to be confirmed, and the simulation runs on each frame of that wait.
#[test]
fn a_host_that_commits_alone_simulates_while_its_crossing_waits() {
    let options = fixed_60hz_room_options(ROOM)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    alice_on_the_hub_door(&mut sim);

    // (updates, the tick) when the crossing was first seen waiting, and when
    // Alice was first seen in the hub.
    let mut recorded: Option<(i32, u64)> = None;
    let mut committed: Option<(i32, u64)> = None;
    for update in 1..=200 {
        sim.drive_seat(0, opens_the_door(30 + update));
        sim.drive_seat(1, script(1, update));
        sim.app_mut().update();
        let now = crossing(&mut sim);
        if recorded.is_none() && now.recorded_on.is_some() {
            recorded = Some((update, sim_tick(&sim)));
        }
        if now.room == HUB {
            committed = Some((update, sim_tick(&sim)));
            break;
        }
    }
    let (Some(recorded), Some(committed)) = (recorded, committed) else {
        panic!("the crossing was recorded at {recorded:?} and committed at {committed:?}");
    };
    assert_eq!(
        (committed.0 - recorded.0, committed.1 - recorded.1),
        (6, 6),
        "(the updates, the ticks) from the recording to the commit: four frames \
         to confirm the recording frame and two for the readiness transaction. \
         (0 ticks: this host froze, and the freeze is for a peer session only)"
    );
}
