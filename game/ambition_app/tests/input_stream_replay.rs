//! **Netcode N0.2's exit check**: the input stream is the artifact, and it is
//! sufficient.
//!
//! Record a real session's per-tick input through the engine's own recorder,
//! validate it, round-trip it through JSON, then replay the DECODED stream into
//! a fresh simulation and demand the trajectory match tick for tick.
//!
//! Every link in that chain is one of the four jobs the stream exists for:
//!
//! - the recorder is what an **RL trajectory** captures;
//! - the JSON round-trip is what a **replay fixture** stores and a **wire
//!   format** will carry;
//! - the fresh-sim replay with zero divergence is what a **desync canary**
//!   (N0.4) compares against.
//!
//! This is a stronger determinism canary than the legacy
//! `replay_fixture_regression` fixture, which records sixty ticks of entirely
//! NEUTRAL input — it can only prove that a falling body falls the same way.
//! Here the player runs, jumps, reverses, and dashes, so the trajectory depends
//! on the input actually surviving capture and transport.

#![cfg(feature = "rl_sim")]

use ambition::engine_core::{ControlFrame, InputStream};
use ambition::runtime::InputStreamRecorder;
use ambition_app::AmbitionSim;
use ambition_app::rl_sim::TimestepMode;
use ambition_app::SandboxSim;

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
            frame.dash_pressed = true;
        }
        _ => {
            frame.axis_x = -0.35;
        }
    }
    frame
}

fn new_sim() -> SandboxSim {
    SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("SandboxSim builds")
}

fn arm_recorder(sim: &mut SandboxSim) {
    sim.world_mut()
        .resource_mut::<InputStreamRecorder>()
        .arm_single_player(TICK_HZ);
}

fn finish_recording(sim: &mut SandboxSim) -> InputStream {
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

/// The recorder captures the input the SIM consumed, not the input a driver
/// wrote. Those differ wherever the input phase rewrites the frame, and a
/// recording of the wrong one replays into a different trajectory.
#[test]
fn the_recorder_captures_the_frame_the_sim_consumed() {
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
            "tick {tick}: the recorded axis must be the one the sim saw"
        );
    }
    assert!(
        frames[0].right_pressed,
        "the press edge on tick 0 must survive into the recording"
    );
}

/// Ticks are the timeline, and the recording says so.
///
/// The stream does NOT begin at tick 0: `SandboxSim::new` runs one sim step so
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
