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

fn record_the_census(world: &mut World) {
    let frame = world.resource::<RollbackFrameCount>().0;
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
    local: usize,
    remote: (usize, std::net::SocketAddr),
    socket: ambition_platformer2d::rollback::LoopbackSocket,
    poison: Poison,
) -> (Platformer2dSimHarness, Vec<&'static str>) {
    let options = fixed_60hz_room_options(ROOM)
        .with_save(a_save_that_has_seen_the_hub_intro())
        .with_sync_test_rollback_settings(4, 10)
        .with_rollback_players(2);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("switch_lab boots");
    bob_beside_alice(&mut sim, ROOM, Some(ambition_platformer2d::characters::control::PlayerSlot(1)));
    // The float rows get value probes, or their census is a carrier count.
    let sharp = strengthen_the_float_rows(sim.world_mut());
    let app = sim.app_mut();
    app.init_resource::<CensusByFrame>();
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
    let (a, b) = ("127.0.0.1:7001".parse().unwrap(), "127.0.0.1:7002".parse().unwrap());
    let (to_bob, to_alice) = loopback_pair(a, b, LATENCY);
    let (mut alice, sharp) = peer(0, (1, b), to_bob, Poison::None);
    let (mut bob, _) = peer(1, (0, a), to_alice, poison);
    let mut updates = 0;
    while confirmed(&alice).min(confirmed(&bob)) < CONFIRMED {
        updates += 1;
        assert!(updates < 20 * CONFIRMED, "the peers confirmed only to {} and {}", confirmed(&alice), confirmed(&bob));
        for (sim, slot) in [(&mut alice, 0), (&mut bob, 1)] {
            if sim.rollback_health().is_err() {
                continue;
            }
            let next = sim.world().resource::<RollbackFrameCount>().0 + 1;
            sim.drive_seat(slot as u8, script(slot, next));
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
