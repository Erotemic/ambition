//! NA-3 in production: a guest that predicts its own body
//! (`ambition_hosted_world::PredictsOwnBody`).
//!
//! Both Apps build `switch_lab` with Bob beside Alice. Bob is driven only on
//! the guest; the host (which rewinds guest rooms, as the visible app's host
//! does) drives Alice. Every message between them, both ways, arrives
//! [`LATENCY`] frames after it was sent. The predicting guest must move Bob on
//! the frame his input changes, agree with the host's rows for every tick it
//! is told of once its run-ahead has settled while nothing else touches Bob,
//! and, with Alice striking him, be corrected by replays and end where the
//! host ends. The control is the sleeping guest, which shows Bob only where
//! the host's rows put him: a round trip late.

use std::collections::VecDeque;

use ambition_app::Platformer2dSimHarness;
use ambition_hosted_world::bevy_replicon::prelude::*;
use ambition_hosted_world::bevy_replicon::test_app::{ServerTestAppExt as _, TestClientEntity};
use ambition_hosted_world::{GuestPredictionStats, HostedRole, HostedWorldPlugin, LocalSeat, PredictsOwnBody};
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core::ControlFrame;
use bevy::prelude::{PluginGroup as _, PostUpdate};

use crate::a_guest_app_applies_the_host::simulate_nothing_but_room_changes;
use crate::common::fixed_60hz_room_options;
use crate::hosted_world::{alice_fights, bobs, LATENCY};
use crate::two_players_two_live_rooms::{bob_beside_alice, ROOM};

/// Frames Bob is driven, and frames after for the link to drain.
const DRIVEN: i32 = 180;
const FRAMES: i32 = 240;

const BOB: PlayerSlot = PlayerSlot(1);
/// The frame Bob's input first turns (`bobs`).
const FIRST_TURN: i32 = 13;

fn booted(role: HostedRole) -> Platformer2dSimHarness {
    let mut sim = Platformer2dSimHarness::build(fixed_60hz_room_options(ROOM), |app, options| {
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        app.add_plugins((RepliconPlugins.set(ServerPlugin::new(PostUpdate)), HostedWorldPlugin { role }));
        app.finish();
        app.cleanup();
        Ok(())
    })
    .expect("the room boots");
    bob_beside_alice(&mut sim, ROOM, Some(BOB));
    sim
}

/// Bob's row on `sim`: place, velocity, facing.
fn bob(sim: &mut Platformer2dSimHarness) -> [f32; 5] {
    let world = sim.world_mut();
    world
        .query::<(&ambition_platformer2d::engine_core::BodyKinematics, &ambition_platformer2d::characters::control::DrivingParticipant)>()
        .iter(world)
        .find(|(_, driver)| driver.0 == BOB)
        .map(|(body, _)| [body.pos.x, body.pos.y, body.vel.x, body.vel.y, body.facing])
        .expect("Bob stands")
}

struct Run {
    /// After the run has settled (its second half of driving), and at the end.
    settled: GuestPredictionStats,
    stats: GuestPredictionStats,
    /// The frames from each turn of Bob's input to his turn on the guest.
    turn_delays: Vec<i32>,
    /// Bob on the host and on the guest after the link drained.
    on_host: [f32; 5],
    on_guest: [f32; 5],
}

fn run(alice: fn(i32) -> ControlFrame, predicts: bool) -> Run {
    let mut host = booted(HostedRole::Host);
    let mut guest = booted(HostedRole::Guest);
    host.world_mut().insert_resource(ambition_hosted_world::RewindsGuestRooms);
    if predicts {
        guest.world_mut().insert_resource(PredictsOwnBody);
    } else {
        simulate_nothing_but_room_changes(&mut guest);
    }
    guest.world_mut().insert_resource(LocalSeat(BOB));
    host.app_mut().connect_client(guest.app_mut());
    let client = **guest.world().resource::<TestClientEntity>();
    host.world_mut().entity_mut(client).insert(ambition_hosted_world::GuestSeat(BOB));
    // Messages on their way, each with the frame it arrives on.
    let mut up = VecDeque::new();
    let mut down = VecDeque::new();
    let mut settled = GuestPredictionStats::default();

    let (mut turn_delays, mut turned_at, mut facing) = (Vec::new(), None, None::<bool>);
    for frame in 1..=FRAMES {
        let input = if frame <= DRIVEN { bobs(frame) } else { ControlFrame::default() };
        // From the second turn on: the first comes while the host's first
        // rows are still on their way to the guest.
        if frame > FIRST_TURN && frame <= DRIVEN && (input.axis_x > 0.0) != (bobs(frame - 1).axis_x > 0.0) {
            turned_at = Some(frame);
        }
        guest.drive_seat(BOB.0, input);
        guest.app_mut().update();
        host.drive_seat(0, alice(frame));
        host.app_mut().update();
        // The link: what each App sent this frame leaves now and arrives
        // `LATENCY` frames on.
        for (channel, message) in guest.world_mut().resource_mut::<ClientMessages>().drain_sent() {
            up.push_back((frame + LATENCY, channel, message));
        }
        host.world_mut().resource_mut::<ServerMessages>().retain_sent(|(to, channel, message)| {
            if *to == client {
                down.push_back((frame + LATENCY, *channel, message.clone()));
                false
            } else {
                true
            }
        });
        while up.front().is_some_and(|(due, ..)| *due <= frame) {
            let (_, channel, message) = up.pop_front().expect("checked");
            host.world_mut().resource_mut::<ServerMessages>().insert_received(client, channel, message);
        }
        while down.front().is_some_and(|(due, ..)| *due <= frame) {
            let (_, channel, message) = down.pop_front().expect("checked");
            guest.world_mut().resource_mut::<ClientMessages>().insert_received(channel, message);
        }
        let right = bob(&mut guest)[2] > 0.0;
        if facing.is_some_and(|was| was != right) {
            if let Some(at) = turned_at.take() {
                turn_delays.push(frame - at);
            }
        }
        facing = Some(right);

        if frame == DRIVEN / 2 {
            settled = guest.world().get_resource::<GuestPredictionStats>().copied().unwrap_or_default();
        }
    }
    let stats = guest.world().get_resource::<GuestPredictionStats>().copied().unwrap_or_default();
    let since = |later: u32, earlier: u32| later - earlier;
    let settled = GuestPredictionStats {
        compared: since(stats.compared, settled.compared),
        corrected: since(stats.corrected, settled.corrected),
        replayed: since(stats.replayed, settled.replayed),
        not_corrected: since(stats.not_corrected, settled.not_corrected),
        correction_sum: stats.correction_sum - settled.correction_sum,
        largest_correction: stats.largest_correction,
        jumps: since(stats.jumps, settled.jumps),
    };
    Run { settled, stats, turn_delays, on_host: bob(&mut host), on_guest: bob(&mut guest) }
}

fn report(name: &str, run: &Run) {
    let s = &run.settled;
    println!(
        "[predict] {name}: Bob turns on the guest {:?} frames after his input; over the run {:?}; after settling: {} compared, {} corrected (mean {:.3}), {} ticks replayed, {} not corrected, {} jumps; end host {:?} guest {:?}",
        run.turn_delays,
        run.stats,
        s.compared,
        s.corrected,
        s.correction_sum / s.corrected.max(1) as f32,
        s.replayed,
        s.not_corrected,
        s.jumps,
        run.on_host,
        run.on_guest,
    );
}

fn agree(a: &[f32; 5], b: &[f32; 5]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() <= 1e-4)
}

/// With nothing touching Bob but his own input, the predicting guest moves
/// him on the frame the input turns, and once its run-ahead has settled every
/// row the host sends of him is where the guest predicted him for that tick.
/// The control (the sleeping guest) turns him a round trip late.
#[test]
fn a_guest_predicts_its_own_body_with_no_correction_while_nothing_touches_it() {
    let still: fn(i32) -> ControlFrame = |_| ControlFrame::default();
    let predicted = run(still, true);
    let asleep = run(still, false);
    report("Alice still, predicting", &predicted);
    report("Alice still, asleep", &asleep);
    assert!(!predicted.turn_delays.is_empty() && !asleep.turn_delays.is_empty(), "premise: Bob turns");
    assert!(predicted.turn_delays.iter().all(|delay| *delay <= 1), "the predicting guest turned Bob late: {:?}", predicted.turn_delays);
    assert!(
        asleep.turn_delays.iter().all(|delay| *delay >= 2 * LATENCY),
        "the control: the sleeping guest turned Bob sooner than a round trip, so the link is not late: {:?}",
        asleep.turn_delays
    );
    assert!(predicted.settled.compared > 30, "the guest compared too few rows: {:?}", predicted.settled);
    assert_eq!(predicted.settled.corrected + predicted.settled.not_corrected, 0, "a prediction differed with nothing touching Bob: {:?}", predicted.settled);
    assert!(agree(&predicted.on_host, &predicted.on_guest), "the guest ended elsewhere than the host");
}

/// With Alice striking Bob on the host (which the guest cannot predict: it
/// does not have her input), the host's rows differ from the guest's
/// prediction, each is corrected by a replay, and once the link has drained
/// the guest's Bob is where the host's is.
#[test]
fn a_guest_corrects_its_own_body_by_replaying_when_another_body_strikes_it() {
    let predicted = run(alice_fights, true);
    report("Alice fights, predicting", &predicted);
    assert!(predicted.settled.corrected > 0, "premise: Alice's strikes never moved Bob off his prediction: {:?}", predicted.settled);
    assert_eq!(predicted.settled.not_corrected, 0, "a difference could not replay: {:?}", predicted.settled);
    assert!(predicted.settled.replayed >= predicted.settled.corrected, "a correction replayed nothing: {:?}", predicted.settled);
    assert!(agree(&predicted.on_host, &predicted.on_guest), "the guest ended elsewhere than the host: {:?} against {:?}", predicted.on_guest, predicted.on_host);
}

