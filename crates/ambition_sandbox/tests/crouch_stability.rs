//! Pin Crouching stability under continuous Down input.
//!
//! Per Jon's 2026-05-07 report: "When the character crouches they
//! start blinking ... shifting between standing and crouch sprites.
//! Holding down causes the camera and sprite to look like they are
//! shaking."
//!
//! This test runs the full SandboxSim (engine + progression chain
//! including body_mode driver) for 60 frames with axis_y pinned at
//! 1.0 (Down held) and asserts the player stays Crouching for the
//! tail of the run, with no per-frame Standing↔Crouching flips
//! producing camera-shake-sized pos.y deltas.

#![cfg(feature = "rl")]

use ambition_sandbox::rl::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};

#[test]
fn holding_down_on_flat_ground_does_not_flicker_body_mode() {
    // Use a room with a flat floor and no overhead obstructions.
    // central_hub_complex's spawn area is open enough.
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("central_hub_complex"),
    )
    .expect("sim builds");

    // Run a few frames idle so the player settles to the ground.
    for _ in 0..15 {
        sim.step(AgentAction::default());
    }
    let settled_obs = sim.observation();
    let settled_pos_y = settled_obs.player_pos.1;

    // Now hold Down for 30 frames. Track body_mode + pos.y each frame.
    let mut body_modes = Vec::with_capacity(30);
    let mut pos_ys = Vec::with_capacity(30);
    for _ in 0..30 {
        let obs = sim.step(AgentAction {
            move_y: 1.0,
            ..AgentAction::default()
        });
        body_modes.push(obs.body_mode.clone());
        pos_ys.push(obs.player_pos.1);
    }

    // The player should enter Crouching within a few frames AND
    // stay Crouching for the tail. If body_mode flickers, this fails.
    let last_5: Vec<&String> = body_modes.iter().rev().take(5).collect();
    let all_crouching = last_5
        .iter()
        .all(|m| m.contains("Crouching") || m.contains("Crawling"));
    assert!(
        all_crouching,
        "body_mode should stay Crouching/Crawling under continuous Down input; \
         last 5 modes were {last_5:?}, full sequence: {body_modes:?}"
    );

    // Camera-shake check: per-frame pos.y delta in the tail should
    // be tiny (not the ~10px crouch resize delta repeating).
    let max_pos_y_delta = pos_ys
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        max_pos_y_delta < 5.0,
        "per-frame pos.y delta should be small once crouched; \
         max delta = {max_pos_y_delta}, full y trace: {pos_ys:?}"
    );

    // Sanity: pos.y end is plausible relative to settled (crouch
    // adjusts pos.y by ~10 to keep feet planted, but it's a one-shot
    // adjustment, not a per-frame oscillation).
    let final_pos_y = pos_ys.last().copied().unwrap_or(settled_pos_y);
    let net_delta = (final_pos_y - settled_pos_y).abs();
    assert!(
        net_delta < 30.0,
        "net pos.y change settled->crouched should be a single body-resize step (~10 px), \
         not 30+ px; settled={settled_pos_y}, final={final_pos_y}, delta={net_delta}"
    );
}
