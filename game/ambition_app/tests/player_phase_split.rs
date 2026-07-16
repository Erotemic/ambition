//! The home/player body runs the SAME decomposed phase pipeline as actors — this
//! pins the player movement→presentation seam through the real schedule.
//!
//! `player_body_tick` used to fuse movement integration and presentation. It is now
//! two scheduled phases: the MOVEMENT phase integrates through the LITERAL same
//! engine entry actors use (`ae::step_motion`) and writes a
//! `PlayerBodyFrameOutput` hand-off; the separate PRESENTATION phase
//! (`sync_player_presentation`, the player counterpart of `sync_actor_read_model`)
//! reads that hand-off to emit anim/SFX/VFX/screen-shake. This drives real input
//! and asserts the movement phase publishes its `FrameEvents` into the hand-off the
//! presentation phase consumes — i.e. player movement and presentation are separate
//! phases joined by an explicit seam, not one fused tick.

#![cfg(feature = "rl_sim")]

use ambition::actors::actor::PrimaryPlayerOnly;
use ambition::engine_core::MovementOp;
use ambition_app::AmbitionSim;
use ambition_app::app::PlayerBodyFrameOutput;
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, World};

fn primary_player(world: &mut World) -> Entity {
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.single(world).expect("primary player")
}

/// The player body carries the movement→presentation hand-off, and the MOVEMENT
/// phase publishes this frame's `FrameEvents` into it (here: a Jump op from a
/// jump press) — the exact data the separate PRESENTATION phase consumes. Proves
/// the two player phases are joined by an explicit seam, not fused.
///
/// **This is half of netcode N0.1's exit check** — see `actor_phase_split.rs`.
/// The body runs both frame-stepped (`Update`) and fixed-tick (`FixedUpdate`).
fn player_handoff_seam_holds(fixed_tick: bool) {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_fixed_tick(fixed_tick),
    )
    .expect("sandbox sim builds");
    let player = primary_player(sim.world_mut());

    // The hand-off component exists on the player (a required component of every
    // player body) — the movement/presentation phases both key on it.
    assert!(
        sim.world_mut()
            .get::<PlayerBodyFrameOutput>(player)
            .is_some(),
        "the player body carries the PlayerBodyFrameOutput movement→presentation seam"
    );

    // Press Jump and scan a short window for the movement phase publishing a Jump
    // MovementOp into the hand-off (the grounded body leaves the floor on frame 1).
    let mut saw_jump_handoff = false;
    for _ in 0..10 {
        sim.step(AgentAction {
            jump: true,
            ..AgentAction::default()
        });
        if let Some(out) = sim.world_mut().get::<PlayerBodyFrameOutput>(player) {
            if out
                .events
                .operations
                .iter()
                .any(|op| matches!(op, MovementOp::Jump | MovementOp::DoubleJump))
            {
                saw_jump_handoff = true;
                break;
            }
        }
    }
    assert!(
        saw_jump_handoff,
        "the player MOVEMENT phase published a Jump event into PlayerBodyFrameOutput \
         for the PRESENTATION phase to consume — the movement→presentation seam is live"
    );
}

#[test]
fn player_movement_phase_hands_off_frame_events_to_presentation() {
    player_handoff_seam_holds(false);
}

/// The same seam, with the whole sim threaded into `FixedUpdate` (N0.1).
#[test]
fn player_handoff_seam_holds_under_fixed_tick() {
    player_handoff_seam_holds(true);
}
