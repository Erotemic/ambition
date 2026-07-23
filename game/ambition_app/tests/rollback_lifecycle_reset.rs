//! **Track B, T1: the IN-PLACE room reset survives forced rollback (coverage
//! the exit oracle deliberately dodged).**
//!
//! The track-0 exit oracle pins the player at HP=200 so death never triggers a
//! same-room reset (`rollback_exit_oracle.rs` HP comment), and the campaign doc
//! recorded "sim-triggered room reset inside a rollback window is a guaranteed
//! divergence" (observed at frame ~2147 as a mid-brawl enemy full-heal). These
//! tests were written to REPRODUCE that — and, per the reproduce-first
//! discipline, they CORRECTED the diagnosis instead:
//!
//! - a manual same-room reset (`AgentAction::reset()`) stays clean, and
//! - a PLAYER-DEATH reset with damaged enemies + in-flight striker projectiles
//!   stays checksum-healthy for 2400 frames (past the ~2147 where divergence
//!   was originally seen).
//!
//! The in-place reset path (`reset_ecs_room_features`: despawn transients,
//! revive enemies via `health.reset()`, reset actors to spawn — ops 1/2a/3 in
//! the Track B map) is therefore rollback-safe in the current tree: the
//! intervening lifecycle-boundary hardening (`PendingPlayerHitEvents` voiding
//! `fd7ddbc0c`, strike-volume registration, the Combat chain reorder) closed
//! the recorded boundary for it. These tests keep it closed.
//!
//! Track B's REMAINING target is RECONSTRUCTION — the paths that despawn+respawn
//! a whole room's entities via Commands (op 2b full sandbox reset, op 4 room
//! transition, op 5 snapshot), which these tests do not exercise. That
//! reproduction needs an input-driven transition/full-reset scenario and is
//! still owed; see the campaign doc Track B section.

#![cfg(feature = "rl_sim")]

use ambition::characters::actor::BodyHealth;
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::{Entity, With, Without};

fn repro_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Stage the player on the arena floor (past the spawn-corner parkour ledge and
/// east of the hazard band), like the exit oracle, and drop HP low — folded into
/// the rollback baseline by the rebase that follows.
fn stage_low_hp_on_floor(sim: &mut SandboxSim, hp: i32) {
    let world = sim.world_mut();
    let player = {
        let mut q =
            world.query_filtered::<Entity, With<ambition::platformer::markers::PrimaryPlayer>>();
        q.single(world).expect("one primary player")
    };
    {
        let mut kin = world
            .get_mut::<ambition::platformer::body::BodyKinematics>(player)
            .expect("player kinematics");
        kin.pos = ambition::engine_core::Vec2::new(720.0, kin.pos.y);
        kin.vel = ambition::engine_core::Vec2::ZERO;
    }
    if let Some(mut health) = world.get_mut::<BodyHealth>(player) {
        health.health.max = hp;
        health.health.current = hp;
    }
    sim.rebase_rollback_history()
        .expect("low-HP arena staging becomes the rollback baseline");
}

fn player_hp(sim: &mut SandboxSim) -> i32 {
    let world = sim.world_mut();
    let mut q =
        world.query_filtered::<&BodyHealth, With<ambition::platformer::markers::PrimaryPlayer>>();
    q.single(world).map(|h| h.health.current).unwrap_or(0)
}

fn living_enemies(sim: &mut SandboxSim) -> Vec<(f32, f32)> {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<(
        &ambition::platformer::body::BodyKinematics,
        &BodyHealth,
    ), Without<ambition::platformer::markers::PrimaryPlayer>>();
    q.iter(world)
        .filter(|(_, h)| h.health.current > 0)
        .map(|(kin, _)| (kin.pos.x, kin.pos.y))
        .collect()
}

/// A manual same-room reset with full-health enemies stays clean under forced
/// rollback — the simplest witness that the in-place reset path is rollback-safe.
#[test]
fn a_manual_reset_with_full_health_enemies_is_clean() {
    let mut sim = repro_sim();
    for frame in 0..40 {
        sim.step(AgentAction::move_x(1.0));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("pre-reset frame {frame}: {error}"));
    }
    sim.step(AgentAction::reset());
    for frame in 0..40 {
        sim.step(AgentAction::default());
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("post-reset frame {frame}: {error}"));
    }
}

/// A player death fires a same-room reset while enemies are mid-brawl. The reset
/// revives them (`health.reset()`) inside the rollback window. This PASSES to
/// 2400 frames — the reproduce-first proof that the in-place reset boundary the
/// campaign doc recorded is now closed. Keeps it closed.
#[test]
fn a_player_death_reset_survives_the_rollback_window() {
    let mut sim = repro_sim();
    stage_low_hp_on_floor(&mut sim, 3);

    let mut saw_death = false;
    let mut prev_hp = player_hp(&mut sim);
    for frame in 0..2400 {
        // Walk toward the nearest living enemy and melee it (damage enemies,
        // and stand in the strikers' reach so they whittle the 3 HP down).
        let px = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<&ambition::platformer::body::BodyKinematics, With<ambition::platformer::markers::PrimaryPlayer>>();
            q.single(world).map(|k| k.pos.x).unwrap_or(0.0)
        };
        let action = match living_enemies(&mut sim)
            .into_iter()
            .map(|(x, _)| (x, (x - px).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        {
            Some((x, d)) if d > 12.0 => AgentAction::move_x((x - px).signum()),
            Some(_) => AgentAction {
                attack: true,
                ..AgentAction::default()
            },
            None => AgentAction::default(),
        };
        sim.step(action);

        let hp = player_hp(&mut sim);
        if hp > prev_hp {
            // HP jumped back up: the death reset (or revive) fired.
            saw_death = true;
        }
        prev_hp = hp;

        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame} (saw_death={saw_death}): {error}"));
    }

    assert!(
        saw_death,
        "the 3-HP player should have died and reset at least once in 2400 frames"
    );
}
