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
use ambition_platformer2d::platformer::lifecycle::LiveRoomInstance;
use ambition_platformer2d::rollback::{
    loopback_transports, start_peer_session, stop_session, GgrsSchedule, LoopbackTransport,
    PeerLineage, PeerSessionSettings, RollbackChecksumProbes, RollbackFrameCount, SaveWorld,
};
use bevy::prelude::*;

use crate::common::{fixed_60hz_room_options, strengthen_the_float_rows};
use crate::two_players_two_live_rooms::{
    bob_beside_alice, door_of, live_rooms, put_bob_at, where_they_are, ROOM,
};

/// Frames both peers must confirm.
const CONFIRMED: i32 = 240;
/// Updates a message waits on the link: the remote input arrives this late.
const LATENCY: u32 = 3;

/// Each probed row's peer census, by the generation of the peer session and
/// the frame it was saved at. A frame that is simulated again after a
/// rollback is recorded again, so a confirmed frame holds the state the
/// timeline agreed on.
#[derive(Resource, Default)]
struct CensusByFrame(BTreeMap<(u32, i32), BTreeMap<&'static str, (usize, u64)>>);

/// Each save of a frame this peer had saved before, in one generation: (the
/// generation, the frame, the highest frame saved until then). A rollback
/// loads a frame and saves each frame after it again, so an entry
/// `(g, f, h)` is a rewind from `h` to before `f`.
#[derive(Resource, Default)]
struct SavedAgain(Vec<(u32, i32, i32)>);

/// The generation of this peer's session: 0 until a lifecycle commit starts
/// the next one.
fn generation(world: &World) -> u32 {
    world.get_resource::<PeerLineage>().map_or(0, PeerLineage::generation)
}

fn record_the_census(world: &mut World) {
    let frame = world.resource::<RollbackFrameCount>().0;
    let generation = generation(world);
    let highest = world
        .resource::<CensusByFrame>()
        .0
        .range((generation, i32::MIN)..=(generation, i32::MAX))
        .next_back()
        .map(|((_, frame), _)| *frame);
    if let Some(highest) = highest.filter(|highest| frame <= *highest) {
        world.resource_mut::<SavedAgain>().0.push((generation, frame, highest));
    }
    let probes = world.resource::<RollbackChecksumProbes>().clone();
    let census = probes
        .census_all_as_peers_compare(world)
        .into_iter()
        .map(|(row, reading)| (row, (reading.count, reading.xor)))
        .collect();
    world.resource_mut::<CensusByFrame>().0.insert((generation, frame), census);
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
    transport: LoopbackTransport,
    poison: Poison,
) -> (Platformer2dSimHarness, Vec<&'static str>) {
    peer_prepared_by(room, local, remote, transport, poison, |_| {})
}

/// [`peer`], with `prepare` run on the world before the P2P session starts.
/// Each peer runs the same `prepare`, so the two worlds are still equal at
/// frame zero.
fn peer_prepared_by(
    room: &str,
    local: usize,
    remote: (usize, std::net::SocketAddr),
    transport: LoopbackTransport,
    poison: Poison,
    prepare: fn(&mut Platformer2dSimHarness),
) -> (Platformer2dSimHarness, Vec<&'static str>) {
    peer_with_prediction(room, local, remote, transport, poison, prepare, 8)
}

fn peer_with_prediction(
    room: &str,
    local: usize,
    remote: (usize, std::net::SocketAddr),
    transport: LoopbackTransport,
    poison: Poison,
    prepare: fn(&mut Platformer2dSimHarness),
    max_prediction_window: usize,
) -> (Platformer2dSimHarness, Vec<&'static str>) {
    let options = fixed_60hz_room_options(room)
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
        max_prediction_window,
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
    start_peer_session(world, &settings, transport).expect("each peer starts at the same world");
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
        // Bob's Start with his Jump: the join gesture. Seated, it does nothing.
        start_pressed: slot == 1 && frame % jump == 0,
        ..Default::default()
    }
}

fn confirmed(sim: &Platformer2dSimHarness) -> i32 {
    sim.world()
        .get_resource::<ConfirmedFrameBoundary>()
        .map_or(-1, |boundary| boundary.confirmed)
}

/// The frames of this peer that are settled: each frame of a session that a
/// lifecycle commit ended, and the confirmed frames of the live session.
///
/// A frame of an ended session is settled also past its confirmed frame. A
/// frame before the freeze was corrected before the commit, and a frozen
/// frame has the frozen state.
fn settled(sim: &Platformer2dSimHarness) -> usize {
    let (live, confirmed) = (generation(sim.world()), confirmed(sim));
    sim.world()
        .resource::<CensusByFrame>()
        .0
        .keys()
        .filter(|(generation, frame)| *generation < live || *frame <= confirmed)
        .count()
}

type RowsThatDiffer = BTreeMap<&'static str, (u32, i32)>;

/// The settled frames that both peers saved, counted by generation, and the
/// rows whose census differs between the peers, each with the first
/// (generation, frame) it differs at.
fn compare(alice: &Platformer2dSimHarness, bob: &Platformer2dSimHarness) -> (BTreeMap<u32, usize>, RowsThatDiffer) {
    let (left, right) = (
        &alice.world().resource::<CensusByFrame>().0,
        &bob.world().resource::<CensusByFrame>().0,
    );
    let live = generation(alice.world()).min(generation(bob.world()));
    let last = confirmed(alice).min(confirmed(bob));
    let mut compared: BTreeMap<u32, usize> = BTreeMap::new();
    let mut differing = RowsThatDiffer::new();
    for (&(generation, frame), left) in left {
        if generation > live || (generation == live && frame > last) {
            continue;
        }
        let Some(right) = right.get(&(generation, frame)) else {
            continue;
        };
        *compared.entry(generation).or_default() += 1;
        for (row, reading) in left {
            if right.get(row) != Some(reading) {
                differing.entry(*row).or_insert((generation, frame));
            }
        }
    }
    (compared, differing)
}

struct Outcome {
    healths: (Result<(), String>, Result<(), String>),
    rollbacks: u64,
    /// What Alice's and Bob's ends sent, and the frames they settled.
    traffic: [ambition_platformer2d::rollback::LinkTraffic; 2],
    frames: usize,
    /// Rows whose census differs between the peers, each with the first
    /// (generation, frame) it differs at.
    differing: RowsThatDiffer,
    /// The sessions the walk ran: one, and one more for each lifecycle
    /// operation that committed.
    sessions: usize,
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
/// His script presses Start, so he joins with a body of seat 1 (the Q153
/// join gesture).
fn two_peers_in(room: &str, alices: fn(i32) -> ControlFrame, poison: Poison) -> Outcome {
    two_peers_with(room, alices, |frame| script(1, frame), poison)
}

/// Bob's script without its Jump and Start presses: he moves the stick, so a
/// peer still has a changing remote input to predict, and he never joins with
/// a body.
fn bob_without_a_body(frame: i32) -> ControlFrame {
    ControlFrame {
        jump_pressed: false,
        jump_held: false,
        start_pressed: false,
        ..script(1, frame)
    }
}

/// [`two_peers_in`], with Bob's input a function of the frame too.
fn two_peers_with(
    room: &str,
    alices: fn(i32) -> ControlFrame,
    bobs: fn(i32) -> ControlFrame,
    poison: Poison,
) -> Outcome {
    let (a, b) = ("127.0.0.1:7001".parse().unwrap(), "127.0.0.1:7002".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let ends = [to_bob.clone(), to_alice.clone()];
    let (mut alice, sharp) = peer(room, 0, (1, b), to_bob, Poison::None);
    let (mut bob, _) = peer(room, 1, (0, a), to_alice, poison);
    let mut updates = 0;
    // A lifecycle operation (a replay, a crossing) ends a session and the
    // next one starts at frame zero, so the walk counts settled frames, not
    // the confirmed frame of one session.
    while settled(&alice).min(settled(&bob)) < CONFIRMED as usize {
        updates += 1;
        assert!(
            updates < 20 * CONFIRMED,
            "in {room} the peers settled only {} and {} frames, in sessions {} and {}",
            settled(&alice),
            settled(&bob),
            generation(alice.world()),
            generation(bob.world())
        );
        for (sim, slot) in [(&mut alice, 0), (&mut bob, 1)] {
            if sim.rollback_health().is_err() {
                continue;
            }
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let input = if slot == 0 { alices(next) } else { bobs(next) };
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
    let left = &alice.world().resource::<CensusByFrame>().0;
    let carried = sharp
        .iter()
        .copied()
        .filter(|row| left.values().any(|census| census.get(row).is_some_and(|(count, _)| *count > 0)))
        .collect();
    let (compared, differing) = compare(&alice, &bob);
    let probes = alice.world().resource::<RollbackChecksumProbes>();
    let value_probed = &probes.type_names() - &probes.presence_only_type_names();
    Outcome {
        traffic: ends.map(|end| end.sent()),
        frames: settled(&alice).min(settled(&bob)),
        sessions: compared.len(),
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
            "DeathBeat",
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
    let mut sessions = Vec::new();
    for (room, alices) in walks {
        // Bob has no body, so Alice's fall in `portal_bridge` is the last
        // participant out of play in her room and the room goes back. A
        // joined Bob in play in her room would hold it (co-op), and the walk
        // would not cross a replay.
        let outcome = two_peers_with(room, alices, bob_without_a_body, Poison::None);
        sessions.push((room, outcome.sessions));
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
    // ⚠ ONE WALK ENDS ITS SESSION, TWICE. In `portal_bridge` Alice walks off
    // the bridge, and her replay of the room is a lifecycle operation: under a
    // peer session it freezes, commits on each peer and starts the next
    // session, as a door does. So this walk also shows that the peers agree
    // across a same-room replay. (Before the peer commit the replay waited
    // with no end, and from the freeze on the walk compared a held world.)
    assert_eq!(
        sessions,
        vec![
            ("blink_run", 1),
            ("portal_lab", 1),
            ("basement_hazards", 1),
            // Three since 2026-10-12: a swing of the robot at home does not
            // root it, so in the same frames she walks off the bridge twice.
            ("portal_bridge", 3),
            ("basement_boss", 1),
            ("pirate_sky_lookout", 1),
            ("goblin_encounter", 1),
        ],
        "the sessions each walk ran: one, and one more for each lifecycle operation that committed"
    );
}

/// The room each player leaves for, from `switch_lab`.
const HUB: &str = "central_hub_complex";

/// Put Alice on the door to the hub.
fn alice_on_the_hub_door(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let center = crate::common::door_to(sim, HUB).aabb.center();
    sim.teleport_player((center.x, center.y));
}

/// Put Alice and Bob on the door to the hub.
fn each_player_on_the_hub_door(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    // The passage starts on the scripted press on both peers.
    crate::common::doors_cross_at_once(sim);
    alice_on_the_hub_door(sim);
    let center = door_of(sim, ROOM, HUB).aabb.center();
    put_bob_at(sim, center);
}

/// The frame of a session on which its crossing is recorded (`R`): the first
/// press of [`opens_the_door`].
const RECORDED_ON: i32 = 30;
/// A seat presses interact from frame 30, as a press and a release.
fn opens_the_door(frame: i32) -> ControlFrame {
    ControlFrame {
        interact_pressed: frame >= RECORDED_ON && frame % 6 == 0,
        interact_held: frame >= RECORDED_ON && frame % 6 < 3,
        ..Default::default()
    }
}

/// An input that changes and moves nothing: a jump release with no jump. The
/// other peer predicts the last input again, so each change is a wrong
/// prediction and a rollback, and the body stays where it is. It changes on
/// frames 29, 30 and 33, so a rollback crosses the recording frame.
fn changes_and_stands(frame: i32) -> ControlFrame {
    ControlFrame {
        jump_released: (frame + 1) % 10 < 4 && (frame + 1) % 10 != 1,
        ..Default::default()
    }
}

type LoadState = ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;
type LoadPhase = ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase;

/// Updates Bob's machine takes longer than Alice's to prepare a room.
const BOBS_MACHINE_IS_SLOWER_BY: u32 = 6;

/// Hold Bob's host authorization for several updates. Simulation must continue.
fn bobs_machine_is_slower(mut state: ResMut<LoadState>, mut held: Local<(u64, u32)>) {
    let Some(active) = state.active.as_mut().filter(|active| active.phase == LoadPhase::CommitAuthorized) else {
        return;
    };
    if held.0 != active.sequence {
        *held = (active.sequence, 0);
    }
    if held.1 < BOBS_MACHINE_IS_SLOWER_BY {
        held.1 += 1;
        active.phase = LoadPhase::AwaitingReadiness;
    }
}

fn pending_recorded_on(sim: &Platformer2dSimHarness) -> Option<i32> {
    sim.world()
        .resource::<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>()
        .pending
        .as_ref()
        .map(|intent| intent.frame)
}

struct TimelineDoorOutcome {
    healths: [Result<(), String>; 2],
    differing: RowsThatDiffer,
    compared: BTreeMap<u32, usize>,
    applied: [BTreeMap<i32, (i32, Option<LiveRoomInstance>)>; 2],
    progressed_while_pending: [bool; 2],
    rollbacks: u64,
    rooms: [Vec<(LiveRoomInstance, String)>; 2],
    members: [(Option<LiveRoomInstance>, Option<Option<LiveRoomInstance>>); 2],
    first: LiveRoomInstance,
    retired_receipts: [usize; 2],
    replayed_commits: [bool; 2],
    settling: [Vec<i32>; 2],
    arrival_resets: [usize; 2],
    traffic: [ambition_platformer2d::rollback::LinkTraffic; 2],
    frames: i32,
}

fn crossings_on_one_timeline(count: u8, order: [usize; 2], configure: fn(&mut App)) -> TimelineDoorOutcome {
    let (a, b) = ("127.0.0.1:7051".parse().unwrap(), "127.0.0.1:7052".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let ends = [to_bob.clone(), to_alice.clone()];
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, each_player_on_the_hub_door);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, each_player_on_the_hub_door);
    for sim in [&mut alice, &mut bob] { configure(sim.app_mut()); }
    bob.app_mut().add_systems(Update, bobs_machine_is_slower
        .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
    let first = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(alice.world_mut()).unwrap();
    let mut applied: [BTreeMap<i32, (i32, Option<LiveRoomInstance>)>; 2] = Default::default();
    let mut progressed_while_pending = [false; 2];
    let mut pending_tick = [None; 2];
    let end = if count == 1 { 100 } else { 220 };
    let mut updates = 0;
    while confirmed(&alice) < end || confirmed(&bob) < end {
        updates += 1;
        assert!(updates < 2000, "peers did not confirm travel: {:?}, {:?}", alice.rollback_health(), bob.rollback_health());
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let press = if index == order[0] { RECORDED_ON } else { 150 };
            sim.drive_seat(index as u8, ControlFrame {
                interact_pressed: next == press,
                interact_held: (press..press + 3).contains(&next),
                jump_released: next % 2 == 0,
                ..Default::default()
            });
            sim.app_mut().update();
            assert_eq!(generation(sim.world()), 0, "ordinary travel must keep its peer timeline");
            let pending = sim.world().resource::<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>();
            let tick = sim_tick(sim);
            if pending.peek().is_some() {
                if pending_tick[index].is_some_and(|previous| tick > previous) { progressed_while_pending[index] = true; }
                pending_tick[index] = Some(tick);
            } else { pending_tick[index] = None; }
            if let Some(receipt) = pending.applied.as_ref().filter(|receipt| receipt.applied_on <= confirmed(sim)) {
                applied[index].insert(receipt.operation.frame, (receipt.applied_on, receipt.arrived_in));
            }
        }
        if alice.rollback_health().is_err() || bob.rollback_health().is_err() { break; }
    }
    let (compared, differing) = compare(&alice, &bob);
    TimelineDoorOutcome {
        traffic: ends.map(|end| end.sent()), frames: confirmed(&alice).min(confirmed(&bob)),
        healths: [alice.rollback_health(), bob.rollback_health()], differing, compared, applied,
        progressed_while_pending,
        rollbacks: [&alice, &bob].iter().filter_map(|sim| sim.rollback_execution_stats()).map(|stats| stats.lifetime_load_runs).sum(),
        rooms: [live_rooms(&mut alice), live_rooms(&mut bob)],
        members: [where_they_are(&mut alice), where_they_are(&mut bob)], first,
        retired_receipts: [&alice, &bob].map(|sim| sim.world().get_resource::<RetiredPromiseReceipts>().map_or(0, |count| count.0)),
        arrival_resets: [&alice, &bob].map(|sim| sim.world().get_resource::<ObservedRoomArrivalResets>().map_or(0, |observed| observed.0)),
        settling: [&alice, &bob].map(|sim| sim.world().resource::<LoadState>().presentations()
            .filter(|active| active.phase == LoadPhase::Committed).map(|active| active.recorded_on).collect()),
        replayed_commits: [&alice, &bob].map(|sim| sim.world().resource::<SavedAgain>().0.iter().any(|(generation, frame, highest)|
            *generation == 0 && sim.world().resource::<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>()
                .applied.as_ref().is_some_and(|receipt| *frame <= receipt.applied_on && *highest >= receipt.applied_on))),
    }
}

#[test]
fn ordinary_peer_crossings_apply_on_shared_input_frames_without_holding_or_rebasing() {
    for order in [[0, 1], [1, 0]] {
        let outcome = crossings_on_one_timeline(2, order, |_| {});
        assert_eq!(outcome.healths, [Ok(()), Ok(())]);
        assert!(outcome.differing.is_empty(), "confirmed rows differ: {:?}", outcome.differing);
        assert!(outcome.compared.get(&0).is_some_and(|frames| *frames >= 200));
        assert!(outcome.rollbacks > 0, "the link did not correct any prediction");
        assert!(outcome.replayed_commits.into_iter().any(|replayed| replayed), "no rollback crossed an application frame");
        assert_eq!(outcome.progressed_while_pending, [true; 2]);
        assert_eq!(outcome.applied[0], outcome.applied[1], "the input frame must select the same applications");
        assert_eq!(outcome.applied[0].keys().copied().collect::<Vec<_>>(), [RECORDED_ON, 150]);
        let destination = outcome.first.next();
        assert!(outcome.applied[0].values().all(|(_, room)| *room == Some(destination)));
        assert_eq!(outcome.rooms, [vec![(destination, HUB.into())], vec![(destination, HUB.into())]]);
        assert_eq!(outcome.members, [(Some(destination), Some(Some(destination))); 2]);
    }
}

/// NET-INPUT-WIDTH's measurement: what the link carries, beside the width of
/// one input. GGRS delta-encodes each input against the last acknowledged one
/// and run-length-compresses it, so the link is not 2 x the input width a
/// frame; a verdict that does not change between frames costs little on it.
/// Two scenes: steady play with a 3-update latency (so predictions roll back),
/// and two crossings agreed on one timeline.
#[test]
#[ignore = "measurement: prints link bytes per frame; run with --ignored --nocapture"]
fn how_many_bytes_the_link_carries_a_frame() {
    println!("[link] input wire identity {}", ambition_platformer2d::engine_core::CONTROL_FRAME_WIRE_IDENTITY);
    let steady = two_peers_in(ROOM, holds_right_and_fires, Poison::None);
    assert_eq!(steady.healths, (Ok(()), Ok(())));
    let crossing = crossings_on_one_timeline(2, [0, 1], |_| {});
    assert_eq!(crossing.healths, [Ok(()), Ok(())]);
    for (scene, traffic, frames, rollbacks) in [
        ("steady play", steady.traffic, steady.frames as u64, steady.rollbacks),
        ("two crossings", crossing.traffic, crossing.frames as u64, crossing.rollbacks),
    ] {
        for (who, sent) in ["alice", "bob"].into_iter().zip(traffic) {
            println!(
                "[link] {scene}: {who} sent {} messages, {} bytes over {frames} frames: {:.1} bytes a frame ({rollbacks} rollbacks)",
                sent.messages, sent.bytes, sent.bytes as f64 / frames.max(1) as f64
            );
        }
    }
}

/// The fixture acknowledges its cover but retains every committed presentation.
fn retain_room_covers(app: &mut App) {
    app.init_resource::<ambition_platformer2d::runtime::room_transition::RoomTransitionPresentationAvailable>()
        .init_resource::<ObservedRoomArrivalResets>()
        .add_systems(Update, observe_room_arrival_resets);
    app.add_systems(Update, (|mut state: ResMut<LoadState>| {
        if let Some(active) = state.active.as_mut() { active.cover_presented = true; }
    }).after(ambition_platformer2d::runtime::room_transition::begin_room_transition_load_system)
      .before(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
}

#[test]
fn an_old_cover_does_not_block_the_next_peers_preparation_or_commit() {
    for order in [[0, 1], [1, 0]] {
        let outcome = crossings_on_one_timeline(2, order, retain_room_covers);
        assert_eq!(outcome.healths, [Ok(()), Ok(())]);
        assert!(outcome.differing.is_empty(), "confirmed rows differ: {:?}", outcome.differing);
        assert_eq!(outcome.progressed_while_pending, [true; 2]);
        assert_eq!(outcome.applied[0], outcome.applied[1]);
        assert_eq!(outcome.applied[0].keys().copied().collect::<Vec<_>>(), [RECORDED_ON, 150]);
        assert_eq!(outcome.arrival_resets, [2; 2], "each confirmed crossing releases one arrival reset");
        assert_eq!(outcome.settling, [vec![RECORDED_ON, 150], vec![RECORDED_ON, 150]],
            "both confirmed crossings must keep their presentation receipts");
    }
}

const PEER_COMBAT_VICTIM: &str = "peer-room-combat-victim";

fn prepare_alice_returning_to_bobs_fight(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::{characters::actor::{BodyHealth, Health}, engine_core::AabbExt as _};
    crate::common::doors_cross_at_once(sim);
    let floor = {
        let world = sim.world();
        let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
            ambition_platformer2d::world::rooms::RoomSet>(world).unwrap();
        let room = rooms.spec(rooms.definition_by_id(ROOM).unwrap());
        room.world.blocks.iter().filter(|block|
            ambition_platformer2d::engine_core::collision_semantics::is_full_collision_surface(block.kind)
                && block.aabb.width() >= 256.0 && block.aabb.height() <= 64.0
                && block.aabb.top() >= 96.0 && block.aabb.top() < room.world.size.y
                && block.aabb.center().x >= 96.0 && block.aabb.center().x < room.world.size.x - 96.0)
            .map(|block| block.aabb).find(|floor| {
                [-48.0, 48.0].into_iter().all(|dx| room.embeds(ambition_platformer2d::engine_core::Aabb::new(
                    ambition_platformer2d::engine_core::Vec2::new(floor.center().x + dx, floor.top() - 32.0),
                    ambition_platformer2d::engine_core::Vec2::splat(24.0))).is_empty())
            }).expect("the source room has clear ground for the combat fixture")
    };
    let at = ambition_platformer2d::engine_core::Vec2::new(floor.center().x + 48.0, floor.top() - 64.0);
    sim.spawn_enemy_character_at(PEER_COMBAT_VICTIM, "Target", (at.x, at.y), (12.0, 16.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive, "npc_puppy_slug");
    {
        let world = sim.world_mut();
        let source = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(world).unwrap();
        let victim = world.query::<(Entity, &ambition_platformer2d::combat::components::FeatureId)>().iter(world)
            .find(|(_, id)| id.0 == PEER_COMBAT_VICTIM).unwrap().0;
        world.entity_mut(victim).insert(ambition_platformer2d::platformer::lifecycle::InRoomInstance(source));
        let mut bodies = world.query::<(&ambition_platformer2d::combat::components::FeatureId,
            &ambition_platformer2d::platformer::sim_id::SimId, &mut BodyHealth)>();
        let (id, mut health) = bodies.iter_mut(world).find(|(id, _, _)| id.0 == PEER_COMBAT_VICTIM)
            .map(|(_, id, health)| (id.clone(), health)).unwrap();
        *health = BodyHealth::new(Health::new(1));
        world.insert_resource(ScriptedCombatTarget(id));
    }
    {
        let world = sim.world_mut();
        let mut bodies = world.query::<(&ambition_platformer2d::combat::components::FeatureId,
            ambition_platformer2d::engine_core::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel)>();
        for (id, mut clusters, mut model) in bodies.iter_mut(world) {
            let dx = if id.0 == "ow1_bob" { -48.0 } else if id.0 == PEER_COMBAT_VICTIM { 48.0 } else { continue };
            let half = clusters.kinematics.size() * 0.5;
            let at = ambition_platformer2d::engine_core::Vec2::new(floor.center().x + dx, floor.top() - half.y);
            ambition_platformer2d::engine_core::movement::transit_body(&mut model, &mut clusters.as_clusters_mut(), at,
                ambition_platformer2d::engine_core::movement::TransitVelocity::Zero);
        }
    }
    assert_eq!(crate::common::walk_through_the_door_to(sim, HUB), HUB);
    let door = door_of(sim, HUB, ROOM).aabb.center();
    sim.teleport_player((door.x, door.y));
}

#[derive(Resource)]
struct ScriptedCombatTarget(ambition_platformer2d::platformer::sim_id::SimId);

fn bob_strikes_during_room_preparation(
    controls: Res<ambition_platformer2d::characters::control::SlotControls>,
    pending: Res<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>,
    target: Res<ScriptedCombatTarget>,
    bodies: Query<(Entity, &ambition_platformer2d::platformer::sim_id::SimId,
        Option<&ambition_platformer2d::characters::control::DrivingParticipant>,
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::platformer::lifecycle::InRoomInstance)>,
    mut hits: MessageWriter<ambition_platformer2d::combat::events::HitEvent>,
) {
    use ambition_platformer2d::{characters::control::PlayerSlot, combat::events::{HitEvent, HitSource, HitTarget, HitMode}};
    if !controls.get(PlayerSlot(1)).attack_pressed || pending.peek().is_none() { return; }
    let (attacker, _, _, _, room) = bodies.iter().find(|(_, _, driver, _, _)|
        driver.is_some_and(|driver| driver.0 == PlayerSlot(1))).unwrap();
    let Some((victim, _, _, body, target_room)) = bodies.iter().find(|(_, id, _, _, _)|
        **id == target.0) else { return };
    if room != target_room { return; }
    hits.write(HitEvent {
        contact: None,
        pieces: Vec::new(), volume: ambition_platformer2d::engine_core::Aabb::new(body.pos, body.size() * 0.5).into(),
        damage: 1, source: HitSource::Projectile, attacker: Some(attacker), room: Some(room.0),
        target: HitTarget::Body(victim), mode: HitMode::Knockback, knockback: None,
        ignored_targets: Vec::new(), strike_sfx: None, attacker_move_instance: None,
    });
}

#[derive(Clone, Debug, PartialEq)]
struct LiveCombatSample {
    pending_from_another_room: bool,
    bob_x: f32,
    victim_health: Option<i32>,
}

#[derive(Resource, Default)]
struct LiveCombatSamples(BTreeMap<i32, LiveCombatSample>);

fn sample_bobs_room_while_alice_prepares(
    frame: Res<RollbackFrameCount>,
    pending: Res<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>,
    bodies: Query<(&ambition_platformer2d::combat::components::FeatureId,
        &ambition_platformer2d::engine_core::BodyKinematics,
        &ambition_platformer2d::characters::actor::BodyHealth,
        &ambition_platformer2d::platformer::lifecycle::InRoomInstance)>,
    mut samples: ResMut<LiveCombatSamples>,
) {
    let (_, bob, _, room) = bodies.iter().find(|(id, _, _, _)| id.0 == "ow1_bob").unwrap();
    let hp = bodies.iter().find(|(id, _, _, _)| id.0 == PEER_COMBAT_VICTIM).map(|(_, _, health, _)| health.health.current);
    let another = pending.peek().and_then(|operation| operation.kind.subject())
        .is_some_and(|subject| subject.room != Some(room.0));
    samples.0.insert(frame.0, LiveCombatSample { pending_from_another_room: another, bob_x: bob.pos.x, victim_health: hp });
}

#[test]
fn a_peer_keeps_moving_and_fighting_in_a_live_destination_while_another_peer_prepares() {
    let (a, b) = ("127.0.0.1:7061".parse().unwrap(), "127.0.0.1:7062".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, prepare_alice_returning_to_bobs_fight);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, prepare_alice_returning_to_bobs_fight);
    for sim in [&mut alice, &mut bob] {
        sim.app_mut().init_resource::<LiveCombatSamples>()
            .add_systems(GgrsSchedule, bob_strikes_during_room_preparation
                .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::Combat)
                .before(ambition_platformer2d::platformer::schedule::CombatSet::Resolve))
            .add_systems(SaveWorld, sample_bobs_room_while_alice_prepares);
    }
    bob.app_mut().add_systems(Update, bobs_machine_is_slower
        .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
    let source = where_they_are(&mut alice).1.unwrap().unwrap();
    for update in 0..2000 {
        if confirmed(&alice) >= 100 && confirmed(&bob) >= 100 { break; }
        assert!(update < 1999, "the peers did not finish the crossing");
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            sim.drive_seat(index as u8, if index == 0 {
                ControlFrame { interact_pressed: next == RECORDED_ON,
                    interact_held: (RECORDED_ON..RECORDED_ON + 3).contains(&next), ..Default::default() }
            } else { ControlFrame { axis_x: if (30..45).contains(&next) { 1.0 } else { 0.0 },
                attack_pressed: next == 34, jump_released: next % 2 == 0, ..Default::default() } });
            sim.app_mut().update();
            assert_eq!(generation(sim.world()), 0);
        }
        assert!(alice.rollback_health().is_ok() && bob.rollback_health().is_ok());
    }
    let (compared, differing) = compare(&alice, &bob);
    assert!(compared.get(&0).is_some_and(|frames| *frames >= 90));
    assert!(differing.is_empty(), "confirmed state differs: {differing:?}");
    for sim in [&mut alice, &mut bob] {
        let samples = &sim.world().resource::<LiveCombatSamples>().0;
        let waiting: Vec<_> = samples.iter().filter(|(frame, sample)| **frame <= confirmed(sim) && sample.pending_from_another_room).collect();
        assert!(waiting.iter().any(|(_, sample)| sample.bob_x != waiting[0].1.bob_x), "no movement during preparation: {waiting:?}");
        assert!(waiting.iter().any(|(_, sample)| sample.victim_health.is_none_or(|hp| hp <= 0)), "no kill during preparation: {waiting:?}");
        assert!(samples[&29].victim_health.is_some_and(|hp| hp > 0));
        assert_eq!(live_rooms(sim), vec![(source, ROOM.into())]);
        assert_eq!(where_they_are(sim), (Some(source), Some(Some(source))));
    }
}

const LATE_HIT_FRAME: i32 = 58;
const ALICE_READY_FRAME: i32 = 60;

fn prepare_alice_waiting_for_a_late_hit(sim: &mut Platformer2dSimHarness) {
    each_player_on_the_hub_door(sim);
    let world = sim.world_mut();
    let alice = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .single(world).unwrap();
    world.get_mut::<ambition_platformer2d::characters::actor::BodyHealth>(alice).unwrap().health.current = 1;
    let target = world.get::<ambition_platformer2d::platformer::sim_id::SimId>(alice).unwrap().clone();
    world.insert_resource(ScriptedCombatTarget(target));
}

fn hold_alices_plan_until_the_late_hit(frame: Res<RollbackFrameCount>, mut loads: ResMut<LoadState>) {
    if frame.0 < ALICE_READY_FRAME {
        if let Some(active) = loads.active.as_mut().filter(|active| active.phase == LoadPhase::CommitAuthorized) {
            active.phase = LoadPhase::AwaitingReadiness;
        }
    }
}

/// Keep the later death restore waiting so the cancelled travel is observed alone.
fn delay_followup_checkpoint_preparation(mut loads: ResMut<LoadState>) {
    if let Some(active) = loads.active.as_mut().filter(|active|
        active.checkpoint_operation.is_some() && active.phase == LoadPhase::CommitAuthorized) {
        active.phase = LoadPhase::AwaitingReadiness;
    }
}

#[derive(Resource, Default)]
struct ObservedRoomArrivalResets(usize);

fn observe_room_arrival_resets(
    frame: Res<RollbackFrameCount>,
    mut sounds: MessageReader<ambition_platformer2d::sfx::OwnedSfxMessage>,
    mut observed: ResMut<ObservedRoomArrivalResets>,
) {
    for sound in sounds.read() {
        if frame.0 >= RECORDED_ON && matches!(sound.request, ambition_platformer2d::sfx::SfxMessage::Reset { .. }) {
            observed.0 += 1;
        }
    }
}

#[test]
fn corrected_combat_removes_a_speculative_crossing_and_its_arrival_effect() {
    use ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit;
    for attack in [false, true] {
        let (a, b) = ("127.0.0.1:7071".parse().unwrap(), "127.0.0.1:7072".parse().unwrap());
        let (to_bob, to_alice) = loopback_transports(a, b, 6);
        let (mut alice, _) = peer_with_prediction(ROOM, 0, (1, b), to_bob, Poison::None, prepare_alice_waiting_for_a_late_hit, 32);
        let (mut bob, _) = peer_with_prediction(ROOM, 1, (0, a), to_alice, Poison::None, prepare_alice_waiting_for_a_late_hit, 32);
        for sim in [&mut alice, &mut bob] {
            sim.app_mut().init_resource::<ObservedRoomArrivalResets>()
                .add_systems(GgrsSchedule, bob_strikes_during_room_preparation
                    .in_set(ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhase::Combat)
                    .before(ambition_platformer2d::platformer::schedule::CombatSet::Resolve))
                .add_systems(Update, observe_room_arrival_resets)
                .add_systems(Update, delay_followup_checkpoint_preparation
                    .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
        }
        alice.app_mut().add_systems(Update, hold_alices_plan_until_the_late_hit
            .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
        let mut speculative = [false; 2];
        let mut first_ready = [None; 2];
        for update in 0..2000 {
            if confirmed(&alice) >= 75 && confirmed(&bob) >= 75 { break; }
            assert!(update < 1999, "the corrected crossing did not settle");
            for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
                let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
                sim.drive_seat(index as u8, ControlFrame {
                    interact_pressed: index == 0 && next == RECORDED_ON,
                    interact_held: index == 0 && (RECORDED_ON..RECORDED_ON + 3).contains(&next),
                    attack_pressed: attack && index == 1 && next == LATE_HIT_FRAME,
                    jump_released: next % 2 == 0, ..Default::default()
                });
                sim.app_mut().update();
                let verdict = sim.world().resource::<ambition_platformer2d::rollback::ThisPeersVerdict>().0;
                if verdict.operation.recorded_on == RECORDED_ON
                    && verdict.said == ambition_platformer2d::rollback::PreparationVerdict::Prepared && first_ready[index].is_none() {
                    first_ready[index] = Some((sim.world().resource::<RollbackFrameCount>().0, confirmed(sim)));
                }
                assert_eq!(generation(sim.world()), 0);
                if sim.world().resource::<PendingLifecycleCommit>().applied.as_ref()
                    .is_some_and(|receipt| receipt.operation.frame == RECORDED_ON && receipt.applied_on > confirmed(sim)) {
                    speculative[index] = true;
                    let active = sim.world().resource::<LoadState>().active.as_ref().expect("an unconfirmed crossing must keep its loading receipt");
                    assert_eq!(active.phase, LoadPhase::CommitAuthorized);
                    assert!(active.arrived_in.is_none(), "a speculative crossing leaked its presentation receipt");
                }
            }
            assert!(alice.rollback_health().is_ok() && bob.rollback_health().is_ok());
        }
        let (compared, differing) = compare(&alice, &bob);
        assert!(compared.get(&0).is_some_and(|frames| *frames >= 70));
        assert!(differing.is_empty(), "corrected state differs: {differing:?}");
        assert!(speculative.into_iter().any(|applied| applied), "the run never applied speculatively; attack={attack}; first readiness={first_ready:?}");
        for sim in [&mut alice, &mut bob] {
            let applied = sim.world().resource::<PendingLifecycleCommit>().applied.as_ref()
                .is_some_and(|receipt| receipt.operation.frame == RECORDED_ON);
            assert_eq!(applied, !attack, "a corrected death must remove the original application");
            assert_eq!(sim.world().resource::<ObservedRoomArrivalResets>().0, usize::from(!attack));
            let expected_rooms = if attack { vec![(LiveRoomInstance::ACTIVATION, ROOM.into())] }
                else { vec![(LiveRoomInstance::ACTIVATION, ROOM.into()), (LiveRoomInstance::ACTIVATION.next(), HUB.into())] };
            assert_eq!(live_rooms(sim), expected_rooms);
        }
    }
}

const PREPARATION_ITEM: &str = "peer-continuity-pickup";

fn destination_with_a_continuity_item(app: &mut App) {
    use ambition_platformer2d::platformer::lifecycle::session_world_component_mut;
    let mut rooms = session_world_component_mut::<ambition_platformer2d::world::rooms::RoomSet>(app.world_mut()).unwrap();
    let destination = rooms.rooms.iter_mut().find(|room| room.id == HUB).unwrap();
    destination.ground_items.push(ambition_platformer2d::world::rooms::GroundItemSpec {
        id: PREPARATION_ITEM.into(), name: "Preparation item".into(), held_item: "gun_sword".into(),
        pos: destination.world.spawn, half_extent: ambition_platformer2d::engine_core::Vec2::splat(8.0),
    });
}

fn consume_destination_item_after_capture(
    frame: Res<RollbackFrameCount>,
    pending: Res<ambition_platformer2d::session::lifecycle_commit::PendingLifecycleCommit>,
    mut ledger: ResMut<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>,
) {
    if pending.peek().is_some_and(|operation| operation.frame == frame.0) {
        ledger.consume([ambition_platformer2d::platformer::sim_id::SimId::placement(PREPARATION_ITEM)]);
    }
}

#[test]
fn peers_refuse_a_destination_occurrence_changed_after_preparation_capture() {
    let control = crossings_on_one_timeline(1, [0, 1], destination_with_a_continuity_item);
    assert_eq!(control.healths, [Ok(()), Ok(())]);
    assert!(control.applied.iter().all(|applied| applied.len() == 1));
    assert!(control.members.iter().all(|members| members.0 == Some(control.first.next())));
    let changed = crossings_on_one_timeline(1, [0, 1], |app| {
        destination_with_a_continuity_item(app);
        app.add_systems(GgrsSchedule, consume_destination_item_after_capture.after(
            ambition_platformer2d::platformer::schedule::GameplaySimulationRoot));
    });
    assert_eq!(changed.healths, [Ok(()), Ok(())]);
    assert!(changed.differing.is_empty(), "confirmed room state differs: {:?}", changed.differing);
    assert!(changed.rollbacks > 0, "the delayed link did not exercise prediction");
    assert!(changed.applied.iter().all(BTreeMap::is_empty));
    assert_eq!(changed.rooms, [vec![(changed.first, ROOM.into())], vec![(changed.first, ROOM.into())]]);
    assert_eq!(changed.members, [(Some(changed.first), Some(Some(changed.first))); 2]);
}

#[derive(Resource, Default)]
struct RetiredPromiseReceipts(usize);

fn retire_a_promised_load_receipt(
    verdict: Res<ambition_platformer2d::rollback::ThisPeersVerdict>,
    mut loads: ResMut<ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState>,
    mut retired: ResMut<RetiredPromiseReceipts>,
) {
    if verdict.0.said == ambition_platformer2d::rollback::PreparationVerdict::Prepared
        && loads.active.as_ref().is_some_and(|active|
            active.phase == ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase::CommitAuthorized)
    {
        loads.active = None;
        retired.0 += 1;
    }
}

#[test]
fn a_peer_keeps_its_promised_room_plan_after_the_load_receipt_retires() {
    let outcome = crossings_on_one_timeline(2, [0, 1], |app| {
        app.init_resource::<RetiredPromiseReceipts>();
        app.add_systems(Update, retire_a_promised_load_receipt
            .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
    });
    assert!(outcome.retired_receipts.into_iter().sum::<usize>() > 0, "no promised receipt retired");
    assert_eq!(outcome.healths, [Ok(()), Ok(())]);
    assert!(outcome.differing.is_empty(), "{:?}", outcome.differing);
    assert!(outcome.compared.get(&0).is_some_and(|frames| *frames >= 200));
    assert!(outcome.applied.iter().all(|applied| applied.len() == 2));
}

/// Bob's machine cannot prepare a room: each transaction it opens fails, as a
/// room whose assets do not load on that machine would.
fn bobs_machine_cannot_prepare(mut state: ResMut<LoadState>) {
    if let Some(active) = state.active.as_mut().filter(|active| active.phase != LoadPhase::Committed) {
        active.phase = LoadPhase::Failed;
        active.failure = Some("this machine cannot prepare the room (two_peers)".to_string());
    }
}

/// What each peer did with the crossing, on each peer, after the walk.
#[derive(Debug, PartialEq)]
struct CrossingOutcome {
    /// The generation of each peer's session.
    generations: [u32; 2],
    /// Each peer still has the crossing waiting.
    waiting: [bool; 2],
    /// Each peer simulated in the last 30 updates.
    simulating: [bool; 2],
    /// The room Alice is in, on each peer.
    alice_in: [String; 2],
}

/// A PEER COMMITS ONLY WHEN EACH PEER SAID IT PREPARED THE OPERATION, AND A
/// PEER THAT COULD NOT ENDS IT ON EACH PEER (Q156).
///
/// Alice opens the door on frame 30, and each peer records the crossing.
/// Alice's machine prepares the hub. Bob's machine cannot: each transaction
/// it opens fails. Each peer says what it has in its input.
///
/// - Neither peer commits. Before the barrier, Alice's peer committed alone
///   and started its next session, and Bob's peer stayed in the old one:
///   neither session ran again, and nothing said why.
/// - The crossing ends on each peer, at the frame whose input says `Failed`,
///   and each peer simulates again, with Alice still in her room. The peers
///   agree at each confirmed frame (`rollback_health`), so each ended it on
///   the same frame: ended at two frames, the simulation would start again
///   on two frames and the checksums would differ. Alice presses the door
///   each 6 frames, so the crossing is recorded again, and ends again, many
///   times in the run. That is the default in force until Q156 is ruled: a
///   failed preparation ends the operation, as a failed respawn ends on one
///   machine. Measured before: both peers stayed frozen with no end.
/// - The control is in the same run: Alice's peer said `Prepared`, so its own
///   plan was authorized, and only the other peer's verdict held it.
///   `a_door_under_a_peer_session_commits_on_each_peer_and_so_does_the_next`
///   is the run where each machine prepares and each peer commits.
#[test]
fn a_peer_does_not_commit_a_crossing_the_other_peer_could_not_prepare() {
    refused_peer_crossing(|app| {
        app.add_systems(Update, bobs_machine_cannot_prepare
            .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system)
            .before(ambition_platformer2d::runtime::room_transition::finalize_unpresented_room_transition_failure_system));
    });
}

fn refused_peer_crossing(configure_bob: impl FnOnce(&mut App)) -> Platformer2dSimHarness {
    let (a, b) = ("127.0.0.1:7021".parse().unwrap(), "127.0.0.1:7022".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, each_player_on_the_hub_door);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, each_player_on_the_hub_door);
    configure_bob(bob.app_mut());
    // Far past the commit of the run where each machine prepares: that run
    // commits 1 to 14 updates after the freeze.
    for _ in 0..240 {
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let input = if index == 0 { opens_the_door(next) } else { changes_and_stands(next) };
            sim.drive_seat(index as u8, input);
            sim.app_mut().update();
        }
    }
    let ticks_before = [sim_tick(&alice), sim_tick(&bob)];
    for _ in 0..30 {
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let input = if index == 0 { opens_the_door(next) } else { changes_and_stands(next) };
            sim.drive_seat(index as u8, input);
            sim.app_mut().update();
        }
    }
    let seen = |sim: &mut Platformer2dSimHarness, ticks_before: u64| {
        (
            generation(sim.world()),
            pending_recorded_on(sim) == Some(RECORDED_ON),
            sim_tick(sim) > ticks_before,
            sim.observation().active_room,
        )
    };
    let (alices, bobs) = (seen(&mut alice, ticks_before[0]), seen(&mut bob, ticks_before[1]));
    assert_eq!(
        (&alice.rollback_health(), &bob.rollback_health()),
        (&Ok(()), &Ok(())),
        "the peers' health"
    );
    assert_eq!(
        CrossingOutcome {
            generations: [alices.0, bobs.0],
            waiting: [alices.1, bobs.1],
            simulating: [alices.2, bobs.2],
            alice_in: [alices.3, bobs.3],
        },
        CrossingOutcome {
            generations: [0, 0],
            waiting: [false; 2],
            simulating: [true; 2],
            alice_in: [ROOM.to_string(), ROOM.to_string()],
        },
        "neither peer commits, and the crossing ends on each peer, which then \
         simulates again"
    );
    bob
}

#[derive(Resource, Default)]
struct DifferentRoomPlans(usize);

fn bob_prepares_different_room_data(world: &mut World) {
    use ambition_platformer2d::construction::mechanics::{GenerationMechanics, SessionMechanics};
    use ambition_platformer2d::publication::{RoomConstructionPlan, ActiveContentBinding};
    use ambition_platformer2d::runtime::room_transition::{RoomTransitionLoadState, RoomTransitionLoadPhase};
    let Some(original) = world.get_resource::<RoomTransitionLoadState>()
        .and_then(|state| state.active.as_ref())
        .filter(|active| active.phase == RoomTransitionLoadPhase::CommitAuthorized)
        .and_then(|active| active.construction_plan.clone()) else { return };
    if original.spec().world.name.ends_with(" [different-plan]") {
        return;
    }
    let binding = world.query::<&ActiveContentBinding>().single(world).unwrap().0;
    let mechanics = GenerationMechanics::of(
        world.resource::<SessionMechanics>(),
        world.resource::<ambition_platformer2d::characters::prepared::ActiveSessionCast>(),
    );
    let mut spec = original.spec().clone();
    spec.world.name.push_str(" [different-plan]");
    let plan = RoomConstructionPlan::prepare_spec(
        original.target_index(), spec,
        world.resource::<ambition_platformer2d::construction::placements::PlacementLoweringRegistry>(),
        world.resource::<ambition_platformer2d::publication::construction::RoomContentStagingRegistry>(),
        mechanics.bosses(), original.session_scope(),
        ambition_platformer2d::publication::construction::ActorConstructionContext::for_live_room_construction(
            world.resource::<ambition_platformer2d::construction::ActorConstructionRegistry>(),
            world.resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>(),
            &mechanics, binding,
            world.get_resource::<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>(),
            None,
        ),
    ).expect("the alternate room plan prepares");
    assert_eq!(original.peer_identity().unwrap().incoming_content(), plan.peer_identity().unwrap().incoming_content());
    assert_ne!(original.peer_identity(), plan.peer_identity());
    world.resource_mut::<RoomTransitionLoadState>().active.as_mut().unwrap().construction_plan = Some(std::sync::Arc::new(plan));
    world.resource_mut::<DifferentRoomPlans>().0 += 1;
}

#[test]
fn peers_refuse_different_room_plans_under_the_same_content_identity() {
    let bob = refused_peer_crossing(|app| {
        app.init_resource::<DifferentRoomPlans>();
        app.add_systems(Update, bob_prepares_different_room_data
            .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system));
    });
    assert!(bob.world().resource::<DifferentRoomPlans>().0 > 0, "Bob never offered the different plan");
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
        if recorded.is_none() && pending_recorded_on(&sim).is_some() {
            recorded = Some((update, sim_tick(&sim)));
        }
        if sim.observation().active_room == HUB {
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

#[test]
fn peer_contract_waits_for_the_other_proposal_before_simulation() {
    let (a, b) = ("127.0.0.1:7031".parse().unwrap(), "127.0.0.1:7032".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let (mut alice, _) = peer(ROOM, 0, (1, b), to_bob, Poison::None);
    let before = sim_tick(&alice);
    for _ in 0..20 {
        alice.app_mut().update();
    }
    assert_eq!(sim_tick(&alice), before);
    assert_eq!(alice.world().resource::<RollbackFrameCount>().0, 0);
    alice.rollback_health().expect("a missing proposal waits without invalidation");

    fn another_local_activation(sim: &mut Platformer2dSimHarness) {
        use ambition_platformer2d::runtime::{ContentEpoch, PreparedContent, publish_session_content};
        let world = sim.world_mut();
        let mut roots = world.query_filtered::<
            &PreparedContent,
            With<ambition_platformer2d::platformer::lifecycle::SessionRoot>,
        >();
        let content = roots.single(world).expect("one prepared session").clone();
        let epoch = ContentEpoch(content.epoch().0 + 17);
        publish_session_content(world, content.with_epoch(epoch));
    }
    let (mut bob, _) = peer_prepared_by(
        ROOM, 1, (0, a), to_alice, Poison::None, another_local_activation,
    );
    let bobs_before = sim_tick(&bob);
    for _ in 0..120 {
        alice.app_mut().update();
        bob.app_mut().update();
        if sim_tick(&alice) > before && sim_tick(&bob) > bobs_before {
            break;
        }
    }
    alice.rollback_health().expect("equal contracts remain healthy");
    bob.rollback_health().expect("equal contracts remain healthy");
    assert!(sim_tick(&alice) > before);
    assert!(sim_tick(&bob) > bobs_before);
}

#[test]
fn peer_contract_mismatch_refuses_simulation_on_both_peers() {
    use ambition_platformer2d::runtime::{PreparedContentIdentity, ContentFingerprint};
    use ambition_platformer2d::runtime::rollback::{descriptor, RollbackEntryKind, RollbackRegistry};

    fn different_content(sim: &mut Platformer2dSimHarness) {
        let world = sim.world_mut();
        let mut roots = world.query_filtered::<
            &mut PreparedContentIdentity,
            With<ambition_platformer2d::platformer::lifecycle::SessionRoot>,
        >();
        let mut identity = roots.single_mut(world).expect("one prepared session");
        identity.fingerprint = ContentFingerprint::from_bytes([19; 32]);
    }
    fn different_schema(sim: &mut Platformer2dSimHarness) {
        sim.world_mut().resource_mut::<RollbackRegistry>().try_register(descriptor::<u64>(
            "test", "test.peer.contract.schema", RollbackEntryKind::ResourceCloneCustomChecksum,
            "peer has another resource schema",
        )).unwrap();
    }
    for prepare in [different_content as fn(&mut Platformer2dSimHarness), different_schema] {
        let (a, b) = ("127.0.0.1:7041".parse().unwrap(), "127.0.0.1:7042".parse().unwrap());
        let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
        let (mut alice, _) = peer(ROOM, 0, (1, b), to_bob, Poison::None);
        let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, prepare);
        for sim in [&mut alice, &mut bob] {
            let before = sim_tick(sim);
            sim.app_mut().update();
            let reason = sim.rollback_health().expect_err("different contracts must be refused");
            assert!(reason.contains("peer contract mismatch"), "{reason}");
            assert_eq!(sim_tick(sim), before);
            assert_eq!(sim.world().resource::<RollbackFrameCount>().0, 0);
        }
    }
}

#[test]
fn a_remote_leaders_party_preferences_agree_after_late_input_and_rollback() {
    use ambition_platformer2d::sim::{PartyControlPreference, PartyTravelMode};
    use ambition_platformer2d::characters::control::PlayerSlot;
    use ambition_platformer2d::session::{party_travel::{PartyTravel, PartyTravelRules}, participants::SessionParticipants};
    use ambition_platformer2d::platformer::lifecycle::{session_world_component, session_world_component_mut};
    fn prepare(sim: &mut Platformer2dSimHarness) {
        session_world_component_mut::<SessionParticipants>(sim.world_mut()).unwrap().admit(PlayerSlot(1));
        session_world_component_mut::<PartyTravel>(sim.world_mut()).unwrap().configure(PartyTravelRules {
            accept_preferences: true, leader: PlayerSlot(1), ..Default::default()
        }).unwrap();
    }
    let (a, b) = ("127.0.0.1:7001".parse().unwrap(), "127.0.0.1:7002".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, prepare);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, prepare);
    alice.world_mut().resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>()
        .gameplay.party_travel = PartyTravelMode::AnyMember;
    bob.world_mut().resource_mut::<ambition_platformer2d::persistence::settings::UserSettings>()
        .gameplay.party_travel = PartyTravelMode::Independent;
    for _ in 0..20 * CONFIRMED {
        for (sim, slot) in [(&mut alice, 0), (&mut bob, 1)] {
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            let (mode, returns) = if slot == 0 { (PartyTravelMode::AnyMember, false) }
                else if next < 60 { (PartyTravelMode::Independent, false) }
                else if next < 120 { (PartyTravelMode::Leader, true) }
                else if next < 180 { (PartyTravelMode::AnyMember, false) }
                else { (PartyTravelMode::AllMembers, true) };
            sim.drive_seat(slot, ControlFrame { party_preference: PartyControlPreference::new(mode, returns), ..Default::default() });
            sim.app_mut().update();
        }
        if confirmed(&alice).min(confirmed(&bob)) >= CONFIRMED { break; }
        assert!(alice.rollback_health().is_ok() && bob.rollback_health().is_ok(), "the party preference desynchronized the peers");
    }
    assert!(confirmed(&alice).min(confirmed(&bob)) >= CONFIRMED);
    for sim in [&alice, &bob] {
        let rules = session_world_component::<PartyTravel>(sim.world()).unwrap().rules();
        assert_eq!((rules.mode, rules.return_to_leader), (PartyTravelMode::AllMembers, true));
        assert!(sim.rollback_health().is_ok());
    }
    assert!([&alice, &bob].iter().any(|sim| sim.rollback_execution_stats().is_some_and(|stats| stats.lifetime_load_runs > 0)),
        "late changes must exercise a real rewind");
    let (compared, differing) = compare(&alice, &bob);
    assert!(compared.values().sum::<usize>() >= CONFIRMED as usize);
    assert!(differing.is_empty(), "the confirmed peer state differs: {differing:?}");
}
