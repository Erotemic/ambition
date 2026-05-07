//! Investigate Jon's report: "a grounded dash (i.e. slide) causes
//! the camera to shake in a weird way."
//!
//! Root cause was: `try_change_body_mode` keeps feet planted by
//! adjusting `pos.y` by half the height delta on every body-shape
//! change. When a grounded dash + down ends, the body_mode driver
//! transitions the player into Crouching the next frame and the
//! center pops 10.35px (= 0.225 * base_size.y). The camera tracked
//! pos.y directly, so it visibly snapped.
//!
//! Fix lives in `rendering::camera_follow`: subtract the resize
//! offset `(base_size.y - size.y) * 0.5` from the camera target so
//! the camera follows a stable "standing-pose center" point that
//! doesn't move with body resizes. This test asserts on the feet
//! position `pos.y + size.y * 0.5`, which is the same invariant
//! (feet planted ⇔ standing-pose center stable).

#![cfg(feature = "rl")]

use ambition_sandbox::rl::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};

#[test]
fn grounded_horizontal_dash_does_not_oscillate_pos_y() {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("central_hub_complex"),
    )
    .expect("sim builds");

    // Settle on the ground for a few frames.
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }

    // Hold right + dash for 30 frames. dash_timer is small (<0.5s
    // = 30 frames at 60Hz) so most of this is the dash itself plus
    // a few post-dash frames.
    let mut feet_ys = Vec::with_capacity(30);
    for i in 0..30 {
        // Press dash on first frame, hold direction throughout.
        let action = AgentAction {
            move_x: 1.0,
            dash: i == 0,
            ..AgentAction::default()
        };
        let obs = sim.step(action);
        // Feet position (+Y down convention, AABB center + half-height).
        feet_ys.push(obs.player_pos.1 + obs.player_size.1 * 0.5);
    }

    // Compute per-frame |feet dy|. The camera follows a "standing-
    // pose center" derived from feet, so feet stability == camera
    // stability. During a clean grounded dash the floor is stable
    // and the body might resize, but the feet should not jump.
    let max_feet_y_delta = feet_ys
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0_f32, f32::max);

    // Print full feet trace so a regression tells the developer
    // exactly which frame oscillated.
    if max_feet_y_delta >= 5.0 {
        for (i, y) in feet_ys.iter().enumerate() {
            eprintln!("  frame {i:>3}: feet_y = {y}");
        }
    }
    assert!(
        max_feet_y_delta < 5.0,
        "grounded dash should not oscillate feet_y (camera stability proxy); max |dy| = {max_feet_y_delta}"
    );
}

#[test]
fn grounded_horizontal_plus_down_dash_does_not_oscillate_pos_y() {
    // The user's specific report mentioned "slide" -- which on
    // this engine is dash + down. Dash impulse gets a downward
    // y component which is immediately absorbed by floor contact.
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("central_hub_complex"),
    )
    .expect("sim builds");

    for _ in 0..15 {
        sim.step(AgentAction::default());
    }

    let mut feet_ys = Vec::with_capacity(30);
    for i in 0..30 {
        let action = AgentAction {
            move_x: 1.0,
            move_y: 1.0, // Down: slide
            dash: i == 0,
            ..AgentAction::default()
        };
        let obs = sim.step(action);
        feet_ys.push(obs.player_pos.1 + obs.player_size.1 * 0.5);
    }
    let max_feet_y_delta = feet_ys
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0_f32, f32::max);
    if max_feet_y_delta >= 5.0 {
        for (i, y) in feet_ys.iter().enumerate() {
            eprintln!("  frame {i:>3}: feet_y = {y}");
        }
    }
    assert!(
        max_feet_y_delta < 5.0,
        "grounded slide (dash + down) should not oscillate feet_y (camera stability proxy); max |dy| = {max_feet_y_delta}"
    );
}
