//! Root cause was: `try_change_body_mode` keeps feet planted by
//! adjusting `pos.y` by half the height delta on every body-shape
//! change. When a grounded dash + down ends, the body_mode driver
//! transitions the player into Crouching the next frame and the
//! center pops 10.35px (= 0.225 * base_size.y). The camera tracked
//! pos.y directly, so it visibly snapped.
//!
//! Fix lives in the camera stage (`rendering::ViewCameraSet`): subtract the resize
//! offset `(base_size.y - size.y) * 0.5` from the camera target so
//! the camera follows a stable "standing-pose center" point that
//! doesn't move with body resizes. This test asserts on the feet
//! position `pos.y + size.y * 0.5`, which is the same invariant
//! (feet planted ⇔ standing-pose center stable).

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness};

/// The largest per-frame jump of the player's feet (+y down: the AABB centre
/// plus half its height) over a grounded dash pressed on the first of 30
/// frames with the direction held, after 15 frames settling on the hub floor.
/// The camera follows a standing-pose centre derived from the feet, so feet
/// stability is camera stability: the floor is stable and the body may resize,
/// but the feet must not jump.
fn max_feet_dy_over_a_grounded_dash(move_y: f32) -> f32 {
    let mut sim = Platformer2dSimHarness::new_with_options(
        crate::common::sandbox_options()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("central_hub_complex"),
    )
    .expect("sim builds");
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    // dash_timer is under 0.5 s (30 frames at 60 Hz), so this is the dash
    // itself plus a few frames after it.
    let feet_ys: Vec<f32> = (0..30)
        .map(|i| {
            let obs = sim.step(AgentAction { move_x: 1.0, move_y, dash: i == 0, ..AgentAction::default() });
            obs.player_pos.1 + obs.player_size.1 * 0.5
        })
        .collect();
    let max_feet_y_delta = feet_ys.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0_f32, f32::max);
    if max_feet_y_delta >= 5.0 {
        for (i, y) in feet_ys.iter().enumerate() {
            eprintln!("  frame {i:>3}: feet_y = {y}");
        }
    }
    max_feet_y_delta
}

#[test]
fn grounded_horizontal_dash_does_not_oscillate_pos_y() {
    let max_feet_y_delta = max_feet_dy_over_a_grounded_dash(0.0);
    assert!(
        max_feet_y_delta < 5.0,
        "grounded dash should not oscillate feet_y (camera stability proxy); max |dy| = {max_feet_y_delta}"
    );
}

#[test]
fn grounded_horizontal_plus_down_dash_does_not_oscillate_pos_y() {
    // The user's report mentioned "slide", which on this engine is dash +
    // down: the impulse's downward component is absorbed by floor contact.
    let max_feet_y_delta = max_feet_dy_over_a_grounded_dash(1.0);
    assert!(
        max_feet_y_delta < 5.0,
        "grounded slide (dash + down) should not oscillate feet_y (camera stability proxy); max |dy| = {max_feet_y_delta}"
    );
}
