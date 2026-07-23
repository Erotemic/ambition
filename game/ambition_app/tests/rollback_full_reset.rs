//! **Track B — op 2b (full sandbox reset) under rollback: reproduce-first.**
//!
//! Unlike the room TRANSITION (which diverged via its not-rollback-registered
//! MULTI-TICK load machinery, `RoomTransitionLoadState` et al.), a full sandbox
//! reset (`process_sandbox_reset_request`) is SINGLE-TICK Commands reconstruction:
//! despawn the whole `RoomScopedEntity` set + respawn a start-room plan, plus the
//! registry/save/player resets, all in one `SandboxSet::ResetProcessing` pass.
//! The transition result does NOT answer whether that diverges — the in-place
//! reset proved single-tick Commands resets can be perfectly rollback-safe — so
//! this asks op 2b directly.
//!
//! `SandboxResetRequested` is rollback state, so folding a pending request into
//! the baseline makes the reconstruction run on the baseline frame AND on every
//! re-simulation of it inside the sync-test window. If this is RED, op 2b needs
//! the same confirmed-frame deferral the transition got; if GREEN, the single-tick
//! reconstruction is already rollback-safe and reproduce-first says leave it be.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, With};
use std::collections::HashSet;

fn repro_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

fn active_room(sim: &SandboxSim) -> String {
    // RoomSet is a session-world component, read via the same accessor the
    // harness observation uses.
    ambition::platformer::lifecycle::session_world_component::<ambition::actors::rooms::RoomSet>(
        sim.world(),
    )
    .map(|set| set.active_spec().id.clone())
    .unwrap_or_default()
}

/// The room-scoped roster. A full reset despawns this whole set and respawns
/// from the start-room plan; despawn bumps the generation, so a reconstruction
/// that actually ran leaves NO original `Entity` value behind.
fn feature_roster(sim: &mut SandboxSim) -> HashSet<Entity> {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<Entity, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
    q.iter(world).collect()
}

#[test]
fn a_full_sandbox_reset_survives_the_rollback_window() {
    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let before = feature_roster(&mut sim);
    assert!(!before.is_empty(), "the room has a roster before the reset");

    // Request a full sandbox reset and fold it into the rollback baseline.
    {
        let world = sim.world_mut();
        world
            .resource_mut::<ambition::actors::session::reset::SandboxResetRequested>()
            .request = true;
    }
    sim.rebase_rollback_history()
        .expect("the pending full reset folds into the rollback baseline");

    // Drive the window: the reconstruction runs on the baseline frame and every
    // re-simulation of it. A single-tick reconstruction that is not rollback-safe
    // diverges here.
    for frame in 0..180 {
        sim.step(AgentAction::default());
        sim.rollback_health().unwrap_or_else(|error| {
            panic!("frame {frame} (active={}): {error}", active_room(&sim))
        });
    }

    // The reset MUST actually have reconstructed — otherwise "clean" is a vacuous
    // pass over a reset that early-returned. Despawn+respawn ⇒ disjoint ids.
    let after = feature_roster(&mut sim);
    assert!(!after.is_empty(), "the reset respawned a roster");
    assert!(
        before.is_disjoint(&after),
        "the full sandbox reset actually despawned+respawned the room \
         (before={} after={} shared={})",
        before.len(),
        after.len(),
        before.intersection(&after).count(),
    );
}
