//! Replay-fixture regression test.
//!
//! Loads the in-tree fixture trace (`tests/fixtures/replay_central_hub_60f_v1.json`),
//! lifts its recorded controls into a typed
//! [`InputStream`](ambition::engine_core::InputStream) (netcode N0.2), drives a
//! fresh `SandboxSim` at fixed-60Hz with it, and asserts ZERO divergence from
//! the recorded `player.pos` at every tick.
//!
//! This pins many gameplay invariants in one shot: any change that shifts player
//! position determinism in the central_hub_complex Startup→tick path will fail
//! this test with a clear "frame N diverged" message.
//!
//! **What it does NOT pin.** The fixture's sixty ticks are entirely NEUTRAL
//! input, so on its own it can only prove that a falling body falls the same
//! way. The input pipeline — capture, transport, replay — is covered by
//! `input_stream_replay.rs`, which scripts real movement. Keep both: this one
//! guards the trajectory of a specific room from a specific recorded moment;
//! that one guards the artifact.
//!
//! Regenerate the fixture (e.g. when an intentional gameplay change shifts the
//! trajectory):
//!
//!     cargo run -p ambition_app --bin headless -- 60 --dump-trace /tmp/t/
//!     cp /tmp/t/ambition_gameplay_trace_*.json \
//!        game/ambition_app/tests/fixtures/replay_central_hub_60f_v1.json

#![cfg(feature = "rl_sim")]

use serde::Deserialize;

use ambition::engine_core::{ControlFrame, InputStream};
use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::SandboxSim;

const FIXTURE_PATH: &str = "tests/fixtures/replay_central_hub_60f_v1.json";
const TICK_HZ: u32 = 60;
const TOLERANCE: f32 = 0.001;

/// The subset of the gameplay-trace dump this test reads. The dump carries far
/// more (velocity, collision neighborhood, moving platforms); `serde` ignores
/// what we do not name, and `ControlFrame`'s `#[serde(default)]` fills the
/// fields this fixture predates (`left_pressed`, `aim_*`, the projectile verbs).
///
/// Deserializing straight into `ControlFrame` is the point of N0.2: the trace's
/// controls ARE control frames, and the old hand-written `f32_field` /
/// `bool_field` pokes into an untyped `serde_json::Value` — via an `AgentAction`
/// that cannot even carry `shield_held` or `aim_x` — were a lossy re-encoding of
/// data that already had a type.
#[derive(Deserialize)]
struct TraceDump {
    frames: Vec<TraceFrame>,
}

#[derive(Deserialize)]
struct TraceFrame {
    tick: u64,
    player: TracePlayer,
    controls: ControlFrame,
}

#[derive(Deserialize)]
struct TracePlayer {
    pos: TracePoint,
}

#[derive(Deserialize)]
struct TracePoint {
    x: f32,
    y: f32,
}

#[test]
fn fixture_replays_with_zero_divergence() {
    let text = std::fs::read_to_string(FIXTURE_PATH).expect("fixture trace JSON exists");
    let dump: TraceDump = serde_json::from_str(&text).expect("fixture parses");
    assert!(
        !dump.frames.is_empty(),
        "fixture must contain at least one frame"
    );

    // Promote the recording to the versioned artifact, and hold it to the
    // artifact's rules: a readable version, contiguous ticks, one slot throughout.
    let mut stream = InputStream::recording_at(TICK_HZ);
    for frame in &dump.frames {
        stream.push(frame.tick, [frame.controls]);
    }
    assert_eq!(
        stream.validate(),
        Ok(()),
        "the fixture's recorded ticks must form a valid input stream"
    );

    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("SandboxSim builds");

    let mut max_dx: f32 = 0.0;
    let mut max_dy: f32 = 0.0;
    for (i, (frame, recorded)) in stream
        .primary_frames()
        .zip(dump.frames.iter().map(|f| &f.player.pos))
        .enumerate()
    {
        let live = sim.step_frame(frame);
        let dx = (live.player_pos.0 - recorded.x).abs();
        let dy = (live.player_pos.1 - recorded.y).abs();
        max_dx = max_dx.max(dx);
        max_dy = max_dy.max(dy);
        assert!(
            dx + dy <= TOLERANCE,
            "fixture replay diverged at frame {i}: live=({:.3},{:.3}) recorded=({:.3},{:.3}) \
             delta=({dx:.3},{dy:.3})",
            live.player_pos.0,
            live.player_pos.1,
            recorded.x,
            recorded.y,
        );
    }
    // Sanity: max delta should be 0 for a deterministic round-trip (the fixture
    // was recorded at fixed-60Hz, same as the replay).
    assert!(max_dx <= TOLERANCE, "max dx {max_dx} exceeds {TOLERANCE}");
    assert!(max_dy <= TOLERANCE, "max dy {max_dy} exceeds {TOLERANCE}");
}
