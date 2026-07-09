//! Replay-fixture regression test.
//!
//! Loads the in-tree fixture trace (`tests/fixtures/replay_central_hub_60f_v1.json`),
//! drives a fresh `SandboxSim` at fixed-60Hz with the recorded
//! `ControlFrame` sequence, and asserts ZERO divergence from the
//! recorded `player.pos` at every tick.
//!
//! This pins many gameplay invariants in one shot: any change that
//! shifts player position determinism in the central_hub_complex
//! Startup→tick path will fail this test with a clear "frame N
//! diverged" message + reproducible seed.
//!
//! Regenerate the fixture (e.g. when an intentional gameplay change
//! shifts the trajectory):
//!
//!     cargo run -p ambition_app --bin headless -- 60 --dump-trace /tmp/t/
//!     cp /tmp/t/ambition_gameplay_trace_*.json \
//!        game/ambition_app/tests/fixtures/replay_central_hub_60f_v1.json
//!
//! Mirrors the `trace_replay` binary's logic but inline so the test
//! doesn't shell out.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim};

const FIXTURE_PATH: &str = "tests/fixtures/replay_central_hub_60f_v1.json";

fn f32_field(value: &serde_json::Value, key: &str) -> f32 {
    value.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32
}

fn bool_field(value: &serde_json::Value, key: &str) -> bool {
    value.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn agent_action_from_json_controls(c: &serde_json::Value) -> AgentAction {
    AgentAction {
        move_x: f32_field(c, "axis_x"),
        move_y: f32_field(c, "axis_y"),
        left_pressed: bool_field(c, "left_pressed"),
        right_pressed: bool_field(c, "right_pressed"),
        up_pressed: bool_field(c, "up_pressed"),
        down_pressed: bool_field(c, "down_pressed"),
        jump: bool_field(c, "jump_pressed"),
        jump_held: bool_field(c, "jump_held"),
        jump_released: bool_field(c, "jump_released"),
        dash: bool_field(c, "dash_pressed"),
        attack: bool_field(c, "attack_pressed"),
        blink: bool_field(c, "blink_pressed"),
        blink_held: bool_field(c, "blink_held"),
        blink_released: bool_field(c, "blink_released"),
        pogo: bool_field(c, "pogo_pressed"),
        interact: bool_field(c, "interact_pressed"),
        interact_held: bool_field(c, "interact_pressed"),
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: bool_field(c, "fly_toggle_pressed"),
        reset: bool_field(c, "reset_pressed"),
        start: bool_field(c, "start_pressed"),
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

#[test]
fn fixture_replays_with_zero_divergence() {
    let text = std::fs::read_to_string(FIXTURE_PATH).expect("fixture trace JSON exists");
    let json: serde_json::Value = serde_json::from_str(&text).expect("fixture parses");
    let frames = json
        .get("frames")
        .and_then(|v| v.as_array())
        .expect("fixture has frames array");
    assert!(
        !frames.is_empty(),
        "fixture must contain at least one frame"
    );

    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("SandboxSim builds");

    let tolerance = 0.001_f32;
    let mut max_dx: f32 = 0.0;
    let mut max_dy: f32 = 0.0;
    for (i, frame) in frames.iter().enumerate() {
        let controls = frame.get("controls").expect("frame has controls");
        let action = agent_action_from_json_controls(controls);
        let live = sim.step(action);
        let recorded = frame
            .get("player")
            .and_then(|v| v.get("pos"))
            .expect("frame has player.pos");
        let recorded_x = f32_field(recorded, "x");
        let recorded_y = f32_field(recorded, "y");
        let dx = (live.player_pos.0 - recorded_x).abs();
        let dy = (live.player_pos.1 - recorded_y).abs();
        if dx > max_dx {
            max_dx = dx;
        }
        if dy > max_dy {
            max_dy = dy;
        }
        assert!(
            dx + dy <= tolerance,
            "fixture replay diverged at frame {i}: live=({:.3},{:.3}) recorded=({:.3},{:.3}) delta=({:.3},{:.3})",
            live.player_pos.0,
            live.player_pos.1,
            recorded_x,
            recorded_y,
            dx,
            dy
        );
    }
    // Sanity: max delta should be 0 for a deterministic round-trip
    // (the fixture was recorded at fixed-60Hz, same as the replay).
    assert!(
        max_dx <= tolerance,
        "max dx {} exceeds tolerance {}",
        max_dx,
        tolerance
    );
    assert!(
        max_dy <= tolerance,
        "max dy {} exceeds tolerance {}",
        max_dy,
        tolerance
    );
}
