// Movement-axis regression test: only built with the RL stepping API. Compiled
// out (empty test binary) when `rl_sim` is disabled.
#![cfg(feature = "rl_sim")]
//! Headless reproduction of the "Move axis is dead/sticky" input regression.
//!
//! The symptom from manual play: discrete button actions (jump/fly/fire) flow
//! fine, but the analog Move axis (`ControlFrame.axis_x` / `axis_y`) does not
//! reach player movement. This test drives the RL/agent seam — writing
//! `ControlFrame` directly each tick via `SandboxSim::step` (no devices, no
//! leafwing) — and asserts the player actually translates a real distance in the
//! commanded direction. It encodes the contract "axis input -> actual movement"
//! headlessly, independent of rendering or input devices.
//!
//! If this FAILS, the regression is DOWNSTREAM of `ControlFrame` (the
//! ControlFrame -> PlayerInputFrame -> brain -> movement-integration path, or a
//! system clobbering `ControlFrame.axis` between populate and consume). If it
//! PASSES, the live device->ControlFrame populate path is the only remaining
//! suspect (not exercised by this seam).

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions};

/// A fully-neutral action; build real ones with struct update:
/// `AgentAction { move_x: -1.0, ..base() }`.
fn base() -> AgentAction {
    AgentAction {
        move_x: 0.0,
        move_y: 0.0,
        up_pressed: false,
        down_pressed: false,
        jump: false,
        jump_held: false,
        jump_released: false,
        dash: false,
        attack: false,
        blink: false,
        blink_held: false,
        blink_released: false,
        pogo: false,
        interact: false,
        projectile: false,
        projectile_held: false,
        projectile_released: false,
        fly_toggle: false,
        reset: false,
        start: false,
        aim_x: 0.0,
        aim_y: 0.0,
    }
}

/// Minimum horizontal distance (px) we require over ~50 ticks of full-axis
/// walk. Walk speed is hundreds of px/s, so over ~0.83s at 60Hz this is a very
/// loose floor that still rules out "axis is dead" (~0 movement).
const MIN_TRAVEL_PX: f32 = 20.0;

fn fresh_sim() -> SandboxSim {
    // Default room spawns the player on flat floor; fixed 60Hz keeps the
    // trajectory deterministic.
    let opts = SandboxSimOptions::default().with_timestep(TimestepMode::fixed_60hz());
    SandboxSim::new_with_options(opts).expect("SandboxSim::new")
}

#[test]
fn move_axis_left_moves_the_player_left() {
    let mut sim = fresh_sim();
    // Let the spawn settle (gravity to floor) before commanding movement.
    sim.step_n(base(), 5);
    let start_x = sim.observation().player_pos.0;

    // Hold full LEFT (axis_x = -1.0) for ~55 ticks, writing ControlFrame
    // directly each tick via the agent seam.
    let obs = sim.step_n(
        AgentAction {
            move_x: -1.0,
            ..base()
        },
        55,
    );

    let dx = obs.player_pos.0 - start_x;
    assert!(
        dx < -MIN_TRAVEL_PX,
        "axis_x=-1.0 should move the player LEFT a real distance, but dx={dx} \
         (start_x={start_x}, end_x={}). A near-zero dx means the Move axis is \
         dead (not reaching movement).",
        obs.player_pos.0
    );
}

#[test]
fn move_axis_right_moves_the_player_right() {
    let mut sim = fresh_sim();
    sim.step_n(base(), 5);
    let start_x = sim.observation().player_pos.0;

    let obs = sim.step_n(
        AgentAction {
            move_x: 1.0,
            ..base()
        },
        55,
    );

    let dx = obs.player_pos.0 - start_x;
    assert!(
        dx > MIN_TRAVEL_PX,
        "axis_x=+1.0 should move the player RIGHT a real distance, but dx={dx} \
         (start_x={start_x}, end_x={}). A near-zero dx means the Move axis is \
         dead (not reaching movement).",
        obs.player_pos.0
    );
}
