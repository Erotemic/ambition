// Portal integration test: only built with the portal mechanic + RL stepping
// API. Compiled out (empty test binary) when `portal` is disabled.
#![cfg(all(feature = "portal", feature = "rl_sim"))]
//! Regression guard for "walk into a floor portal, press nothing, bounce a few
//! times, then lose all momentum / FALL THROUGH THE FLOOR" (Jon, 2026-06-09).
//!
//! Station A in `portal_lab` is a purple↔yellow ground↔ground pair. A body that
//! enters the purple floor portal with no further input should ping-pong between
//! the two floor portals forever, carrying its momentum through each crossing —
//! it must NEVER ground on the (open) portal floor and must NEVER fall through it
//! and trip the fall-out-of-world death reset.
//!
//! The bug: the host-surface carve lagged a frame behind transit, so on every
//! re-contact the floor was still SOLID when collision ran — the body thudded
//! onto it (landing sfx), its entry velocity was zeroed, and it then re-sank from
//! rest (popping out at only the gravity-from-rest speed, never the real entry
//! speed). On less forgiving geometry the same grounding race let the body tunnel
//! through. The fix opens the carve the same frame the body reaches the opening
//! (keyed off the body's current position, before collision), so the body sinks
//! straight through carrying its momentum.
//!
//! Driven through the public SandboxSim API, asserting only on observed player
//! position / velocity / on-ground / reset counter.

use ambition_sandbox::rl_sim::TimestepMode;
use ambition_sandbox::{AgentAction, SandboxSim, SandboxSimOptions};

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

struct BounceStats {
    died_at: Option<usize>,
    grounded_frames: usize,
}

fn run_bounce(dt: f32) -> BounceStats {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::Fixed { dt })
        .with_start_room("portal_lab");
    let mut sim = SandboxSim::new_with_options(opts).expect("SandboxSim::new in portal_lab");

    let spawn = sim.observation().player_pos;

    // Phase 1: walk right onto the purple floor portal center, then STOP.
    let mut prev = spawn;
    for _ in 0..480 {
        let obs = sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
        let cur = obs.player_pos;
        let jump = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
        prev = cur;
        if cur.0 >= 285.0 || jump > 150.0 {
            break;
        }
    }

    let resets_after_entry = sim.observation().resets;

    // Phase 2: release ALL input and let the body bounce between the floor portals.
    let mut died_at = None;
    let mut grounded_frames = 0usize;
    for frame in 0..240 {
        let obs = sim.step(base());
        if obs.on_ground {
            grounded_frames += 1;
        }
        if obs.resets != resets_after_entry {
            died_at = Some(frame);
            break;
        }
    }
    BounceStats {
        died_at,
        grounded_frames,
    }
}

/// The fall-through guard: across a range of frame rates (the real game runs a
/// variable wall-clock dt), a no-input floor-portal bounce must never die.
#[test]
fn floor_portal_bounce_never_falls_through() {
    for hz in [60.0_f32, 45.0, 30.0, 24.0, 20.0, 15.0] {
        let stats = run_bounce(1.0 / hz);
        assert_eq!(
            stats.died_at, None,
            "FELL THROUGH / DIED while bouncing between the two floor portals with \
             no input at {hz} Hz (frame {:?}). The body should ping-pong forever.",
            stats.died_at,
        );
    }
}

/// The momentum / landing-thud guard: the body must SINK THROUGH the open portal
/// floor, not repeatedly land on a still-solid floor. With the carve lagging a
/// frame the player grounded ~10+ frames per 240 (one thud per bounce, killing
/// its momentum); once the carve opens the same frame, grounding is near zero.
#[test]
fn floor_portal_bounce_does_not_thud_onto_the_open_floor() {
    let stats = run_bounce(1.0 / 60.0);
    assert!(
        stats.grounded_frames <= 4,
        "player grounded on the portal floor {} frames out of 240 while bouncing — \
         it should sink straight through the open aperture, not thud onto a \
         still-solid floor (the one-frame carve lag is back: momentum is being \
         killed on every entry and the landing sfx fires).",
        stats.grounded_frames,
    );
}
