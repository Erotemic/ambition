//! **Track B, T1b — the owed RECONSTRUCTION reproduction.**
//!
//! The in-place reset (`rollback_lifecycle_reset.rs`) mutates SURVIVING entities
//! and is rollback-safe. A room TRANSITION is different: it despawns the whole
//! source roster and spawns the target roster via Commands (reconstruction), and
//! the multi-tick transition transaction machinery
//! (`RoomTransitionLoadState` / `RoomTransitionContentEpoch` / `LoadCoordinator`,
//! installed as plain resources in `app/plugins.rs`) is NOT rollback-registered.
//! The entity/room state it drives (`RoomSet`, `RoomGeometry`, `FeatureSimEntity`,
//! `room_transition_cooldown`) IS rollback state. So a GGRS rollback that
//! straddles the transaction's tick span rewinds the world but not the
//! transaction phase/tick barrier — the commit can then fire at a different tick,
//! twice, or fail to reproduce, diverging the sync-test checksum. That is the
//! reconstruction divergence Track B names as still owed.
//!
//! This test walks the controlled body through the east `EdgeExit` of the
//! calibration lab into the adjacent boss room INSIDE a forced sync-test rollback
//! window. It encodes the TARGET invariant: the transition actually occurs (the
//! active room flips, the source roster is gone, a fresh target roster is present
//! ⇒ new entity generations) AND the sim stays checksum-clean across and past the
//! commit.
//!
//! **OBSERVED: RED (2026-07-23).** It fails — `GGRS sync-test checksum mismatch at
//! frames [9, 10, 11]` while `active == combat_calibration_lab` (the divergence
//! lands in the transition LOAD phase, as the body overlaps the exit and the
//! multi-tick transaction engages, BEFORE the room flips). The same room brawled
//! for 2400 frames stays clean (`rollback_lifecycle_reset`), so this is
//! transition-caused: the transaction machinery is executed eagerly on a
//! speculative frame while its own progress state is not rollback-registered.
//! This is the reproduce-first evidence that greenlights Track B's confirmed-frame
//! deferral. `#[ignore]`d so the suite stays green; UN-IGNORE when Track B lands
//! and this must pass (the transition then commits on a confirmed frame + session
//! rebase, so resim cannot diverge). See the campaign doc Track B section.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, With};
use std::collections::HashSet;

/// `combat_calibration_lab` is the harness room the in-place reset tests already
/// prove builds+runs; its right-edge `EdgeExit` (`combat_lab_to_boss`, LDtk px
/// [1264,560] w16×h176) leads to `first_system_boss`.
const SOURCE_ROOM: &str = "combat_calibration_lab";
const TARGET_ROOM: &str = "first_system_boss";

fn repro_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room(SOURCE_ROOM)
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Every room-scoped feature/actor entity currently spawned. A transition
/// despawns this whole set and spawns a fresh one; because despawn bumps the
/// generation, no source `Entity` value can equal a post-transition one.
fn feature_roster(sim: &mut SandboxSim) -> HashSet<Entity> {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<Entity, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
    q.iter(world).collect()
}

fn player_y(sim: &mut SandboxSim) -> f32 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition::platformer::body::BodyKinematics, With<ambition::platformer::markers::PrimaryPlayer>>();
    q.single(world).map(|k| k.pos.y).unwrap_or(0.0)
}

#[test]
#[ignore = "RED reproduction: the room-transition reconstruction diverges under \
            rollback (checksum mismatch in the load phase, ~frame 9-11) because \
            the transition transaction machinery is not rollback-registered and \
            runs eagerly on speculative frames. Un-ignore when Track B lands."]
fn a_room_transition_survives_the_rollback_window() {
    let mut sim = repro_sim();

    // Baseline: we start in the source room, checksum-clean.
    let start = sim.step(AgentAction::default());
    assert_eq!(
        start.active_room.as_str(),
        SOURCE_ROOM,
        "the sim starts in the source room"
    );
    sim.rollback_health().expect("clean before staging");

    // Stage the body just west of the east EdgeExit (past the hazard band that
    // sits west of x≈720); teleport auto-rebases, folding this into the baseline.
    let floor_y = player_y(&mut sim);
    sim.teleport_player((1200.0, floor_y));

    // Record the source roster so we can prove it is gone after the transition.
    let source_roster = feature_roster(&mut sim);
    assert!(!source_roster.is_empty(), "the source room has a roster");

    // Walk right into the EdgeExit. The transition commits ~2 sim ticks after the
    // overlap frame, all inside the forced rollback window.
    let mut transitioned_at = None;
    for frame in 0..240 {
        let obs = sim.step(AgentAction::move_x(1.0));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (active={}): {error}", obs.active_room));
        if obs.active_room.as_str() == TARGET_ROOM {
            transitioned_at = Some(frame);
            break;
        }
    }
    let transitioned_at = transitioned_at.expect(
        "the body should have crossed the east EdgeExit into the target room within 240 frames",
    );

    // Prove the reconstruction happened: target room active, source roster gone,
    // a fresh target roster present (despawn+respawn ⇒ disjoint entity ids).
    let target_roster = feature_roster(&mut sim);
    assert!(
        !target_roster.is_empty(),
        "the target room spawned a roster"
    );
    assert!(
        source_roster.is_disjoint(&target_roster),
        "the transition despawned the source roster and spawned fresh entities \
         (source={} target={} shared={})",
        source_roster.len(),
        target_roster.len(),
        source_roster.intersection(&target_roster).count(),
    );

    // Keep running well past the commit, still checksum-clean.
    for frame in 0..180 {
        let obs = sim.step(AgentAction::default());
        sim.rollback_health().unwrap_or_else(|error| {
            panic!(
                "post-transition frame {frame} (t+{transitioned_at}, active={}): {error}",
                obs.active_room
            )
        });
    }
}
