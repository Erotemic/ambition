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

use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options, strengthen_the_float_rows};
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
/// His script presses Jump, so he joins with a body of seat 1 (the Q153
/// default join).
fn two_peers_in(room: &str, alices: fn(i32) -> ControlFrame, poison: Poison) -> Outcome {
    two_peers_with(room, alices, |frame| script(1, frame), poison)
}

/// Bob's script without its Jump presses: he moves the stick, so a peer still
/// has a changing remote input to predict, and he never joins with a body.
fn bob_without_a_body(frame: i32) -> ControlFrame {
    ControlFrame {
        jump_pressed: false,
        jump_held: false,
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
    // ⚠ ONE WALK ENDS ITS SESSION. In `portal_bridge` Alice walks off the
    // bridge, and her replay of the room is a lifecycle operation: under a
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
            ("portal_bridge", 2),
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
    alice_on_the_hub_door(sim);
    let center = door_of(sim, ROOM, HUB).aabb.center();
    put_bob_at(sim, center);
}

/// The frame of a session on which its crossing is recorded (`R`): the first
/// press of [`opens_the_door`].
const RECORDED_ON: i32 = 30;
/// The last frame that simulates while the crossing waits. The state saved
/// at this frame is the state of each later frame of that session.
const LAST_LIVE: i32 =
    RECORDED_ON + ambition_platformer2d::rollback::PEER_COMMIT_FREEZE_DELAY - 1;

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

/// THE POISON OF THE FREEZE: frames on which the peer session does not hold
/// the simulation, so a frame that must be frozen simulates.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum SimulatesWhileFrozen {
    Never,
    OnFrame(i32),
    /// No freeze at all. The commit rule stays, so each peer commits on the
    /// world of the frame it is at.
    Always,
}

impl SimulatesWhileFrozen {
    fn on(self, frame: i32) -> bool {
        match self {
            Self::Never => false,
            Self::OnFrame(poisoned) => poisoned == frame,
            Self::Always => true,
        }
    }
}

type Ownership = ambition_platformer2d::rollback::RollbackSessionOwnership;

type LoadState = ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;
type LoadPhase = ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase;

/// Updates Bob's machine takes longer than Alice's to prepare a room.
const BOBS_MACHINE_IS_SLOWER_BY: u32 = 6;

/// Bob's machine prepares each room [`BOBS_MACHINE_IS_SLOWER_BY`] updates
/// after Alice's machine.
///
/// A peer commits when its own plan is authorized and the other peer said in
/// its input that it prepared the operation. So Alice's peer commits a link
/// delay after Bob's verdict arrives, Bob's peer commits at once, and the two
/// commit at two frames: the case the freeze is for. With two equal machines
/// each verdict arrives one link delay after the other was sent, and the
/// peers committed at one frame (measured 2026-10-04: frames 36 and 36).
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

fn lift_the_freeze(
    poison: Res<SimulatesWhileFrozen>,
    frame: Res<RollbackFrameCount>,
    mut ownership: ResMut<Ownership>,
) {
    if poison.on(frame.0) {
        *ownership = Ownership::External;
    }
}

fn put_the_freeze_back(
    poison: Res<SimulatesWhileFrozen>,
    frame: Res<RollbackFrameCount>,
    mut ownership: ResMut<Ownership>,
) {
    if poison.on(frame.0) {
        *ownership = Ownership::Peer;
    }
}

/// What one peer saw of the crossing that ended one of its sessions.
#[derive(Clone, Debug, Default, PartialEq)]
struct CrossingOnOnePeer {
    /// The frame the crossing was recorded on.
    recorded_on: Option<i32>,
    /// The update whose advance first reached the freeze frame.
    froze_on_update: Option<u32>,
    /// The update that committed, and the frame the peer was at before it.
    committed: Option<(u32, i32)>,
    /// The update that ran the first frame of the next session.
    resumed_on_update: Option<u32>,
    /// The live rooms, and the live room of Alice and of Bob, after the
    /// commit.
    after: Option<(Vec<(LiveRoomInstance, String)>, (Option<LiveRoomInstance>, Option<Option<LiveRoomInstance>>))>,
    /// Whether the lifecycle slot was empty after the commit.
    slot_free_after: Option<bool>,
}

impl CrossingOnOnePeer {
    /// (updates from the freeze to the commit, updates from the commit to the
    /// first frame of the next session).
    fn freeze_in_updates(&self) -> Option<(u32, u32)> {
        let (froze, (committed, _), resumed) =
            (self.froze_on_update?, self.committed?, self.resumed_on_update?);
        Some((committed - froze, resumed - committed))
    }
}

fn pending_recorded_on(sim: &Platformer2dSimHarness) -> Option<i32> {
    sim.world()
        .resource::<ambition_platformer2d::actors::session::lifecycle_commit::PendingLifecycleCommit>()
        .pending
        .as_ref()
        .map(|intent| intent.frame)
}

struct DoorOutcome {
    healths: [Result<(), String>; 2],
    /// On each peer, the crossing that ended each session, by generation.
    crossings: [BTreeMap<u32, CrossingOnOnePeer>; 2],
    /// The settled frames of each generation that both peers saved.
    compared: BTreeMap<u32, usize>,
    /// Rows whose census differs between the peers, each with the first
    /// (generation, frame) it differs at.
    differing: RowsThatDiffer,
    /// Load runs of the two peers, in all sessions.
    rollbacks: u64,
    /// On each peer, in generation 0: the first frame after [`LAST_LIVE`]
    /// whose census is not the census of [`LAST_LIVE`], with the rows that
    /// moved.
    moved_after_the_freeze: [Option<(i32, Vec<&'static str>)>; 2],
    /// On each peer: the frames of generation 0 saved after [`LAST_LIVE`].
    frozen_frames_saved: [usize; 2],
    /// On each peer: whether the census of [`LAST_LIVE`] differs from the
    /// census of the frame before it. The control: the world moved until the
    /// freeze.
    moved_before_the_freeze: [bool; 2],
    /// On each peer: the rewinds of generation 0 from a frozen frame to a
    /// frame at or before the recording frame, as (the frame saved again,
    /// the highest frame).
    rewinds_across_the_freeze: [Vec<(i32, i32)>; 2],
    /// Parcels of an ended session that the link dropped.
    dropped_old_parcels: u64,
    /// The live room both players start in.
    first: LiveRoomInstance,
}

/// Alice and Bob stand on the door from `switch_lab` to the hub, on each
/// peer. In the first session Alice presses interact from frame 30. In the
/// second, Bob does. Each crossing ends its session, and the walk ends when
/// each peer has confirmed `after` frames of session `generations`.
fn doors_under_a_peer_session(generations: u32, after: i32, poison: SimulatesWhileFrozen) -> DoorOutcome {
    let (a, b) = ("127.0.0.1:7011".parse().unwrap(), "127.0.0.1:7012".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let link = to_bob.clone();
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, each_player_on_the_hub_door);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, each_player_on_the_hub_door);
    let root = ambition_platformer2d::platformer::schedule::GameplaySimulationRoot;
    for sim in [&mut alice, &mut bob] {
        let app = sim.app_mut();
        app.insert_resource(poison);
        app.add_systems(GgrsSchedule, (lift_the_freeze.before(root), put_the_freeze_back.after(root)));
    }
    bob.app_mut().add_systems(
        Update,
        bobs_machine_is_slower
            .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system),
    );

    let first = *ambition_platformer2d::platformer::lifecycle::sole_live_room_component::<LiveRoomInstance>(
        alice.world_mut(),
    )
    .expect("the session has one live room before a crossing");

    let mut crossings: [BTreeMap<u32, CrossingOnOnePeer>; 2] = Default::default();
    let done = |sim: &Platformer2dSimHarness| generation(sim.world()) >= generations && confirmed(sim) >= after;
    let mut updates = 0;
    while !(done(&alice) && done(&bob)) {
        updates += 1;
        assert!(
            updates < 400 * (generations + 1) + 20 * after as u32,
            "after {updates} updates the peers are at generations {} and {}, confirmed to {} and {}; \
             the crossings: {crossings:#?}",
            generation(alice.world()),
            generation(bob.world()),
            confirmed(&alice),
            confirmed(&bob)
        );
        for (index, sim) in [&mut alice, &mut bob].into_iter().enumerate() {
            let before = generation(sim.world());
            let frame_before = sim.world().resource::<RollbackFrameCount>().0;
            let next = frame_before + 1;
            // Session `g` ends with the crossing of seat `g`. The other seat,
            // and each seat after its crossing, changes its input and stands.
            let input = if before as usize == index { opens_the_door(next) } else { changes_and_stands(next) };
            sim.drive_seat(index as u8, input);
            sim.app_mut().update();

            let now = generation(sim.world());
            let frame = sim.world().resource::<RollbackFrameCount>().0;
            if now > before {
                let after_the_commit = (live_rooms(sim), where_they_are(sim));
                let slot_free = pending_recorded_on(sim).is_none();
                let crossing = crossings[index].entry(before).or_default();
                crossing.committed = Some((updates, frame_before));
                crossing.after = Some(after_the_commit);
                crossing.slot_free_after = Some(slot_free);
            } else if let Some(recorded_on) = pending_recorded_on(sim) {
                let crossing = crossings[index].entry(now).or_default();
                crossing.recorded_on = Some(recorded_on);
                let frozen_from = recorded_on + ambition_platformer2d::rollback::PEER_COMMIT_FREEZE_DELAY;
                if crossing.froze_on_update.is_none() && frame >= frozen_from {
                    crossing.froze_on_update = Some(updates);
                }
            }
            if now > 0 && frame >= 1 {
                if let Some(ended) = crossings[index].get_mut(&(now - 1)) {
                    ended.resumed_on_update.get_or_insert(updates);
                }
            }
        }
        if alice.rollback_health().is_err() || bob.rollback_health().is_err() {
            break;
        }
    }

    let censuses = [
        &alice.world().resource::<CensusByFrame>().0,
        &bob.world().resource::<CensusByFrame>().0,
    ];
    let (compared, differing) = compare(&alice, &bob);
    let frozen = |census: &BTreeMap<(u32, i32), BTreeMap<&'static str, (usize, u64)>>| {
        census
            .range((0, LAST_LIVE + 1)..=(0, i32::MAX))
            .map(|((_, frame), now)| (*frame, now.clone()))
            .collect::<Vec<_>>()
    };
    let moved_after_the_freeze = censuses.map(|census| {
        let held = &census[&(0, LAST_LIVE)];
        frozen(census).into_iter().find(|(_, now)| now != held).map(|(frame, now)| {
            let rows = now
                .iter()
                .filter(|(row, reading)| held.get(*row) != Some(reading))
                .map(|(row, _)| *row)
                .collect();
            (frame, rows)
        })
    });
    let frozen_frames_saved = censuses.map(|census| frozen(census).len());
    let moved_before_the_freeze = censuses.map(|census| census[&(0, LAST_LIVE - 1)] != census[&(0, LAST_LIVE)]);
    let rewinds_across_the_freeze = [&alice, &bob].map(|sim| {
        sim.world()
            .resource::<SavedAgain>()
            .0
            .iter()
            .filter(|(generation, frame, highest)| {
                *generation == 0 && *frame <= RECORDED_ON && *highest > LAST_LIVE
            })
            .map(|(_, frame, highest)| (*frame, *highest))
            .collect()
    });
    let rollbacks = [&alice, &bob]
        .iter()
        .filter_map(|sim| sim.rollback_execution_stats())
        .map(|stats| stats.lifetime_load_runs)
        .sum();
    DoorOutcome {
        healths: [alice.rollback_health(), bob.rollback_health()],
        crossings,
        compared,
        differing,
        rollbacks,
        moved_after_the_freeze,
        frozen_frames_saved,
        moved_before_the_freeze,
        rewinds_across_the_freeze,
        dropped_old_parcels: link.dropped_old_parcels(),
        first,
    }
}

/// A PEER SESSION COMMITS A CROSSING BEHIND THE PEER BARRIER, and the next
/// crossing too. This is the open-world "Remote peers" row.
///
/// Alice and Bob stand on the door from `switch_lab` to the hub, on each
/// peer. Alice presses interact from frame 30 of the first session, and Bob
/// from frame 30 of the second.
///
/// - Each peer records Alice's crossing on frame 30, and from the next frame
///   its simulation does not run (`a_peer_commit_holds_the_simulation`).
/// - Each peer commits alone, on its own update and at its own frame, when
///   its confirmed frame reaches the freeze frame and its plan is authorized
///   (`commit_confirmed_lifecycle`). Alice is in the hub on each peer, with
///   `switch_lab` live for Bob: two live rooms.
/// - Each peer starts the next generation of its session at frame zero. The
///   handshake of those sessions is the barrier: a peer that committed first
///   runs no more than its prediction window before the other peer is there.
/// - The lifecycle slot is free, so Bob's crossing in the second session is
///   accepted and commits the same way. He joins Alice's hub, and
///   `switch_lab`, which nobody holds, retires.
/// - The peers agree at each frame of the three sessions: no desync, and no
///   probed row differs.
///
/// ⚠ WHY THE FREEZE. The sync-test rule runs the crossing on the current
/// world when its recording frame is confirmed. On that update the two peers
/// are at different frames, each with frames the other peer has not
/// confirmed. With the freeze, those frames hold one state.
/// `a_peer_commit_with_no_freeze_runs_on_two_worlds` is that rule with no
/// freeze.
#[test]
fn a_door_under_a_peer_session_commits_on_each_peer_and_so_does_the_next() {
    /// Frames each peer must confirm in the third session.
    const AFTER: i32 = 60;
    let outcome = doors_under_a_peer_session(2, AFTER, SimulatesWhileFrozen::Never);
    // The peers agree, in each session.
    assert_eq!(
        (&outcome.healths, &outcome.differing),
        (&[Ok(()), Ok(())], &BTreeMap::new()),
        "(the peers' health, the rows that differ and the first (generation, frame) of each)"
    );
    assert!(
        outcome.compared.len() == 3 && outcome.compared[&2] > AFTER as usize,
        "the frames compared in each session: {:?}",
        outcome.compared
    );
    assert!(outcome.rollbacks > 0, "control: no peer rolled back, so no prediction was tested");

    // THE FREEZE, in the first session. No probed row moves after the last
    // live frame, on either peer. The controls: frozen frames were saved,
    // the rows moved until then, and a rewind crossed the freeze.
    assert!(
        outcome.frozen_frames_saved.iter().all(|saved| *saved >= 2),
        "control: the frozen frames each peer saved: {:?}",
        outcome.frozen_frames_saved
    );
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
    assert!(
        outcome.rewinds_across_the_freeze.iter().any(|rewinds| !rewinds.is_empty()),
        "control: no peer rewound from a frozen frame to before the recording \
         frame, so the freeze was not tested under a rewind"
    );

    // THE COMMITS. Each crossing was recorded on frame 30 of its session on
    // each peer, and left the same rooms on each peer.
    let first = outcome.first;
    let second = first.next();
    let alices_crossing = (
        vec![(first, ROOM.to_string()), (second, HUB.to_string())],
        (Some(second), Some(Some(first))),
    );
    let bobs_crossing = (vec![(second, HUB.to_string())], (Some(second), Some(Some(second))));
    for (peer, crossings) in outcome.crossings.iter().enumerate() {
        let seen: Vec<_> = crossings
            .iter()
            .map(|(generation, crossing)| {
                (*generation, crossing.recorded_on, crossing.slot_free_after, crossing.after.clone())
            })
            .collect();
        assert_eq!(
            seen,
            vec![
                (0, Some(RECORDED_ON), Some(true), Some(alices_crossing.clone())),
                (1, Some(RECORDED_ON), Some(true), Some(bobs_crossing.clone())),
            ],
            "peer {peer}: each crossing as (generation, the frame it was recorded on, the slot \
             is free after, (the live rooms, where Alice and Bob are))"
        );
    }

    // The peers committed Alice's crossing at different frames: the frozen
    // world, and not the frame, is what they share.
    let at: Vec<i32> = outcome.crossings.iter().map(|by| by[&0].committed.expect("asserted above").1).collect();
    assert!(
        at[0] != at[1] && at.iter().all(|frame| *frame > RECORDED_ON),
        "the peers committed at frames {at:?}. If they are equal now, the link \
         or the loop changed, and this run does not show two peers at two frames"
    );
    // A session started while parcels of the one before it were on the link.
    assert!(
        outcome.dropped_old_parcels > 0,
        "control: the link dropped no parcel of an ended session, so the \
         generation of a socket was not tested here"
    );

    // THE COST: how long the simulation of a peer is held for one crossing,
    // in updates of that peer, as (from the first frozen frame to the commit,
    // from the commit to the first frame of the next session).
    //
    // Measured 2026-10-04 at a link latency of 3 updates each way, over four
    // runs and both crossings: (2..7, 21..36), 23 to 43 updates in all, which
    // is 0.38 s to 0.72 s at 60 updates a second. The first part is the wait
    // for the confirmed frame and two updates of readiness. The second part
    // is the GGRS handshake (five round trips), and it is most of the cost.
    // GGRS times its handshake retries on the wall clock, so the second part
    // is not the same in each run; the band holds the measured range with
    // room, and it fails on a freeze of seconds.
    let held: Vec<(u32, u32)> = outcome
        .crossings
        .iter()
        .flat_map(|by| by.values())
        .map(|crossing| crossing.freeze_in_updates().expect("each crossing committed and resumed"))
        .collect();
    assert!(
        held.iter().all(|(to_commit, handshake)| (1..=14).contains(to_commit) && (10..=70).contains(handshake)),
        "(updates frozen until the commit, updates of the handshake) of each \
         crossing on each peer: {held:?}"
    );
}

/// The freeze arm can fail: with the simulation run on one frozen frame, the
/// census moves on that frame, and the arm above names it.
#[test]
fn one_frozen_frame_that_simulates_moves_the_census() {
    /// The first frozen frame. Each peer runs it before it commits, because
    /// the commit waits for the world to be at a frozen frame.
    const POISONED: i32 = LAST_LIVE + 1;
    let outcome = doors_under_a_peer_session(1, 10, SimulatesWhileFrozen::OnFrame(POISONED));
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
}

/// THE POISON OF THE PEER COMMIT: the same commit rule with no freeze. Each
/// peer commits when its confirmed frame reaches the freeze frame, at the
/// frame it is at then, and the two peers are at different frames. So the
/// next sessions start from two worlds, and the peers do not agree.
#[test]
fn a_peer_commit_with_no_freeze_runs_on_two_worlds() {
    let outcome = doors_under_a_peer_session(1, 30, SimulatesWhileFrozen::Always);
    // The two committed worlds ran different numbers of ticks: the tick
    // differs at frame zero of the next session, and with it each row that
    // moves with time. Measured 2026-10-04: ten rows differ at (1, 0), and
    // GGRS reports the desync at frame 1.
    let tick = std::any::type_name::<ambition_platformer2d::time::SimTick>();
    assert_eq!(
        outcome.differing.get(tick),
        Some(&(1, 0)),
        "the first (generation, frame) the tick differs at. With no freeze the \
         peers agree, so the arm above does not show that the freeze is what \
         makes them agree. All the rows that differ: {:?}",
        outcome.differing
    );
    assert!(
        outcome.healths.iter().any(Result::is_err),
        "the next sessions started from two worlds and no peer reported a desync"
    );
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

    let (a, b) = ("127.0.0.1:7021".parse().unwrap(), "127.0.0.1:7022".parse().unwrap());
    let (to_bob, to_alice) = loopback_transports(a, b, LATENCY);
    let (mut alice, _) = peer_prepared_by(ROOM, 0, (1, b), to_bob, Poison::None, each_player_on_the_hub_door);
    let (mut bob, _) = peer_prepared_by(ROOM, 1, (0, a), to_alice, Poison::None, each_player_on_the_hub_door);
    bob.app_mut().add_systems(
        Update,
        bobs_machine_cannot_prepare
            .after(ambition_platformer2d::runtime::room_transition::authorize_ready_room_transition_system)
            .before(ambition_platformer2d::runtime::room_transition::finalize_unpresented_room_transition_failure_system),
    );
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
