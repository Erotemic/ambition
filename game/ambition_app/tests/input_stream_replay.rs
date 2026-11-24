//! Netcode N0.2's exit check: the input stream is the artifact, and it is
//! sufficient.
//!
//! Record a real session's per-tick input through the engine's own recorder,
//! validate it, round-trip it through JSON, then replay the DECODED stream into
//! a fresh simulation and demand the trajectory match tick for tick.
//!
//! Every link in that chain is one of the four jobs the stream exists for:
//!
//! - the recorder is what an RL trajectory captures;
//! - the JSON round-trip is what a replay fixture stores and a wire
//!   format will carry;
//! - the fresh-sim replay with zero divergence is what a desync canary
//!   (N0.4) compares against.
//!
//! This is a stronger determinism canary than the legacy
//! `replay_fixture_regression` fixture, which records sixty ticks of entirely
//! NEUTRAL input — it can only prove that a falling body falls the same way.
//! Here the player runs, jumps, reverses, and dashes, so the trajectory depends
//! on the input actually surviving capture and transport.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::Platformer2dSimHarness;
use ambition_platformer2d::engine_core::{ControlFrame, InputStream};
use ambition_platformer2d::runtime::InputStreamRecorder;

const TICK_HZ: u32 = 60;
const TICKS: usize = 90;

/// A scripted session with enough shape that a dropped field changes the path:
/// run right, jump, reverse, dash, hold a direction.
fn scripted_input(tick: usize) -> ControlFrame {
    let mut frame = ControlFrame::default();
    match tick {
        0..=19 => {
            frame.axis_x = 1.0;
            frame.right_pressed = tick == 0;
        }
        20 => {
            frame.axis_x = 1.0;
            frame.jump_pressed = true;
            frame.jump_held = true;
        }
        21..=34 => {
            frame.axis_x = 1.0;
            frame.jump_held = true;
        }
        35 => {
            frame.axis_x = 1.0;
            frame.jump_released = true;
        }
        36..=49 => {
            frame.axis_x = -1.0;
            frame.left_pressed = tick == 36;
        }
        50 => {
            frame.axis_x = -1.0;
            frame.burst_pressed = true;
        }
        _ => {
            frame.axis_x = -0.35;
        }
    }
    frame
}

fn new_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(crate::common::sandbox_options().with_timestep(TimestepMode::fixed_60hz()))
        .expect("Platformer2dSimHarness builds")
}

fn arm_recorder(sim: &mut Platformer2dSimHarness) {
    sim.world_mut()
        .resource_mut::<InputStreamRecorder>()
        .arm_single_player(TICK_HZ);
}

fn finish_recording(sim: &mut Platformer2dSimHarness) -> InputStream {
    sim.world_mut()
        .resource_mut::<InputStreamRecorder>()
        .finish()
        .expect("the recorder was armed")
}

#[test]
fn a_recorded_input_stream_replays_a_fresh_sim_with_zero_divergence() {
    // ── Record ────────────────────────────────────────────────────────────
    let mut sim = new_sim();
    arm_recorder(&mut sim);
    let mut recorded_path = Vec::with_capacity(TICKS);
    for tick in 0..TICKS {
        let obs = sim.step_frame(scripted_input(tick));
        recorded_path.push(obs.player_pos);
    }
    let stream = finish_recording(&mut sim);

    assert_eq!(stream.validate(), Ok(()), "a live recording must validate");
    assert_eq!(
        stream.len(),
        TICKS,
        "the recorder appends exactly one frame per sim step"
    );
    assert_eq!(stream.slot_count(), Some(1));
    assert_eq!(stream.tick_hz, TICK_HZ);

    // The path must actually go somewhere, or the assertions below are vacuous.
    let start = recorded_path[0];
    let end = *recorded_path.last().unwrap();
    assert!(
        (end.0 - start.0).abs() > 8.0,
        "the scripted session must MOVE the player (start={start:?} end={end:?}); \
         a stationary trajectory would pass this test with the input dropped"
    );

    // ── Transport ─────────────────────────────────────────────────────────
    let text = serde_json::to_string(&stream).expect("stream serializes");
    let decoded: InputStream = serde_json::from_str(&text).expect("stream deserializes");
    assert_eq!(decoded, stream, "JSON is lossless for the artifact");
    assert_eq!(decoded.validate(), Ok(()));

    // ── Replay ────────────────────────────────────────────────────────────
    let mut replay = new_sim();
    for (tick, frame) in decoded.primary_frames().enumerate() {
        let obs = replay.step_frame(frame);
        let expected = recorded_path[tick];
        assert_eq!(
            obs.player_pos, expected,
            "replay diverged at tick {tick}: replayed={:?} recorded={expected:?}. \
             Same build, same platform, same input stream — the trajectory is a \
             function of the stream alone (ADR 0023).",
            obs.player_pos,
        );
    }
}

/// The recorder captures the seat's ADMITTED input for the tick: what the driver
/// delivered through the input seam, before any stage interprets it.
#[test]
fn the_recorder_captures_the_admitted_frame() {
    let mut sim = new_sim();
    arm_recorder(&mut sim);
    for tick in 0..12 {
        sim.step_frame(scripted_input(tick));
    }
    let stream = finish_recording(&mut sim);

    let frames: Vec<ControlFrame> = stream.primary_frames().collect();
    assert_eq!(frames.len(), 12);
    for (tick, frame) in frames.iter().enumerate() {
        assert_eq!(
            frame.axis_x,
            scripted_input(tick).axis_x,
            "tick {tick}: the recorded axis must be the admitted one"
        );
    }
    assert!(
        frames[0].right_pressed,
        "the press edge on tick 0 must survive into the recording"
    );
}

/// Ticks are the timeline, and the recording says so.
///
/// The stream does NOT begin at tick 0: `Platformer2dSimHarness::new` runs one sim step so
/// the caller's first `observation()` is valid, and the recorder is armed after
/// it. A stream is contiguous from ITS OWN first tick — which is why `frame()`
/// offsets from `start_tick()` rather than assuming zero, and why a recording
/// that begins mid-session is a first-class artifact (desync forensics starts
/// recording when the desync is suspected, not when the game booted).
#[test]
fn recorded_ticks_are_contiguous_from_the_first_recorded_step() {
    let mut sim = new_sim();
    arm_recorder(&mut sim);
    for tick in 0..5 {
        sim.step_frame(scripted_input(tick));
    }
    let stream = finish_recording(&mut sim);

    let start = stream.start_tick().expect("five recorded ticks");
    assert!(start > 0, "the constructor already ran sim step 0");
    let ticks: Vec<u64> = stream.frames.iter().map(|f| f.tick).collect();
    assert_eq!(ticks, (start..start + 5).collect::<Vec<u64>>());
    assert_eq!(stream.validate(), Ok(()));

    // `frame()` is offset-from-start, not indexed-from-zero.
    assert!(stream.frame(start).is_some());
    assert!(stream.frame(start - 1).is_none(), "before the recording");
    assert_eq!(stream.control(start, 0).axis_x, scripted_input(0).axis_x);
}

/// An unarmed recorder costs nothing and records nothing — the default state
/// for every RL rollout and every headless run that does not want a trace.
#[test]
fn an_unarmed_recorder_records_nothing() {
    let mut sim = new_sim();
    for tick in 0..5 {
        sim.step_frame(scripted_input(tick));
    }
    assert!(sim
        .world_mut()
        .resource_mut::<InputStreamRecorder>()
        .finish()
        .is_none());
}

/// IN1: a stage that SPENDS an input must not erase it from the recording. The
/// reset consumer clears `reset_pressed` and requests a room replay; the stream
/// keeps the press, and the decoded replay resets at the same tick.
#[test]
fn a_recorded_reset_replays_the_same_room_replay() {
    const RESET_TICK: usize = 40;
    let input = |tick: usize| {
        let mut frame = scripted_input(tick);
        if tick == RESET_TICK {
            frame.reset_pressed = true;
        }
        frame
    };
    let mut sim = new_sim();
    arm_recorder(&mut sim);
    let mut recorded_path = Vec::with_capacity(TICKS);
    for tick in 0..TICKS {
        recorded_path.push(sim.step_frame(input(tick)).player_pos);
    }
    let stream = finish_recording(&mut sim);
    let frames: Vec<ControlFrame> = stream.primary_frames().collect();
    assert!(
        frames[RESET_TICK].reset_pressed,
        "the admitted reset press is in the recording, though the reset consumer spent it"
    );
    // The reset took effect: the body left its run and returned toward the room's start.
    let before = recorded_path[RESET_TICK - 1];
    let after = recorded_path[RESET_TICK + 1];
    assert!(
        (after.0 - before.0).abs() > 8.0 || (after.1 - before.1).abs() > 8.0,
        "the reset must move the body (before={before:?} after={after:?})"
    );

    let decoded: InputStream =
        serde_json::from_str(&serde_json::to_string(&stream).unwrap()).unwrap();
    let mut replay = new_sim();
    for (tick, frame) in decoded.primary_frames().enumerate() {
        let obs = replay.step_frame(frame);
        assert_eq!(
            obs.player_pos, recorded_path[tick],
            "replay diverged at tick {tick} (reset at {RESET_TICK})"
        );
    }
}

/// IN1, the portal half: a held stick carried through a real transit. The guard
/// and the warp act after the admitted cut, so the stream holds the plain held
/// input and the replay runs both again. Compare the whole body pose each tick,
/// not only a final position, so the tick after the transit reads the same input.
#[cfg(feature = "portal")]
#[test]
fn a_recorded_held_stick_replays_the_same_portal_transit() {
    use ambition_platformer2d::engine_core::{self as ae, BodyKinematics};
    use ambition_platformer2d::platformer::markers::{PlayerEntity, PrimaryPlayer};
    use ambition_platformer2d::portal::{EditablePortalTuning, PortalConvention, PortalTuning};
    use bevy::prelude::*;

    fn pose(sim: &mut Platformer2dSimHarness) -> BodyKinematics {
        let world = sim.world_mut();
        *world
            .query_filtered::<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>()
            .single(world)
            .unwrap()
    }
    fn lab() -> Platformer2dSimHarness {
        let mut sim = Platformer2dSimHarness::new_with_options(
            crate::common::sandbox_options()
                .with_required_start_room("portal_lab")
                .with_timestep(TimestepMode::fixed_60hz()),
        )
        .unwrap();
        sim.world_mut().resource_mut::<PortalTuning>().convention = PortalConvention::Reflection;
        sim.world_mut().resource_mut::<EditablePortalTuning>().convention = PortalConvention::Reflection;
        for _ in 0..12 {
            sim.step_frame(ControlFrame::default());
        }
        let world = sim.world_mut();
        let (entity, mut body) = world
            .query_filtered::<(Entity, &BodyKinematics), (With<PlayerEntity>, With<PrimaryPlayer>)>()
            .single(world)
            .map(|(e, b)| (e, *b))
            .unwrap();
        body.pos = ae::Vec2::new(1860.0, 896.0 - body.half_extent().y);
        body.vel = ae::Vec2::ZERO;
        world.entity_mut(entity).insert((body, ae::SweepSample::at_rest(body, ae::Vec2::Y)));
        sim
    }
    let held = |tick: usize| ControlFrame {
        axis_x: if tick < 140 { 1.0 } else { 0.0 },
        right_pressed: tick == 0,
        ..ControlFrame::default()
    };

    let mut sim = lab();
    arm_recorder(&mut sim);
    let mut recorded = Vec::new();
    for tick in 0..180 {
        sim.step_frame(held(tick));
        recorded.push(pose(&mut sim));
    }
    let stream = finish_recording(&mut sim);
    assert!(
        recorded.iter().any(|b| b.material.scale() != 1.0),
        "the held walk must carry the body through the scaling pair; poses {:?}",
        recorded.iter().step_by(20).map(|b| (b.pos, b.material.scale())).collect::<Vec<_>>()
    );

    let decoded: InputStream =
        serde_json::from_str(&serde_json::to_string(&stream).unwrap()).unwrap();
    let mut replay = lab();
    for (tick, frame) in decoded.primary_frames().enumerate() {
        replay.step_frame(frame);
        let body = pose(&mut replay);
        let expected = recorded[tick];
        assert_eq!(
            (body.pos, body.vel, body.material),
            (expected.pos, expected.vel, expected.material),
            "replay diverged at tick {tick}"
        );
    }
}
