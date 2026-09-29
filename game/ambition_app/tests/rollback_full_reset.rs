//! Track B — op 2b (full sandbox reset) under rollback.
//!
//! A New Game is a checkpoint restore to the fresh baseline at the start room.
//! The request is a host intent, released inside the timeline on its stamped
//! tick; the room is rebuilt by the confirmed-frame lifecycle commit, as a
//! death's is. This drives that road through the sync-test window and checks
//! both that it agrees with itself and that it really rebuilt the room.
//!
//! It used to be a single-tick reconstruction in the simulation, and this file
//! folded the pending request into the rollback baseline so the rebuild ran on
//! the baseline frame. Written the way the menu writes it, that road desynced
//! (NEW-GAME-RESYNC): a rewind across the rebuild frame brought the bodies back
//! without their derived components.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};
use bevy::prelude::{Entity, With};
use std::collections::HashSet;

fn repro_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

fn active_room(sim: &Platformer2dSimHarness) -> String {
    // The live room root names its definition in the session's room set.
    ambition_platformer2d::world::rooms::sole_live_room_spec(sim.world())
    .map(|spec| spec.id.clone())
    .unwrap_or_default()
}

/// The room-scoped roster. A full reset despawns this whole set and respawns
/// from the start-room plan; despawn bumps the generation, so a reconstruction
/// that actually ran leaves NO original `Entity` value behind.
fn feature_roster(sim: &mut Platformer2dSimHarness) -> HashSet<Entity> {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<Entity, With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>>();
    q.iter(world).collect()
}

#[test]
fn a_full_sandbox_reset_survives_the_rollback_window() {
    let mut sim = repro_sim();
    sim.step(AgentAction::default());
    let before = feature_roster(&mut sim);
    assert!(!before.is_empty(), "the room has a roster before the reset");

    // The menu's road: a host intent, with no rebase.
    ambition_platformer2d::actors::session::host_intents::write_host_intent(
        sim.world_mut(),
        ambition_platformer2d::actors::session::reset::NewGameRequested,
    );

    // Drive the window: the admission runs on the stamped tick and every
    // re-simulation of it, and the commit runs once on a confirmed frame.
    for frame in 0..180 {
        sim.step(AgentAction::default());
        sim.rollback_health().unwrap_or_else(|error| {
            panic!("frame {frame} (active={}): {error}", active_room(&sim))
        });
    }

    // The reset MUST actually have reconstructed — otherwise "clean" is a vacuous
    // pass over a reset that early-returned. Despawn+respawn  disjoint ids.
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
