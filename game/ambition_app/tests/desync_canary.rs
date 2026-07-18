//! GGRS rollback integration gates for the real Ambition simulation harness.
//!
//! `SyncTestSession` is the canary: each harness update saves the world, advances
//! it, rewinds a configurable distance, resimulates with the same inputs, and
//! compares checksums. These tests intentionally exercise the product simulation
//! rather than a toy counter or the retired custom snapshot engine.

#![cfg(feature = "rl_sim")]

use ambition::runtime::rollback::{Rollback, RollbackRegistry};
use ambition_app::rl_sim::{AgentAction, AmbitionSim, SandboxSim, SandboxSimOptions, TimestepMode};
use bevy::prelude::With;

fn rollback_sim() -> SandboxSim {
    SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds")
}

fn scripted_action(frame: usize) -> AgentAction {
    AgentAction {
        move_x: if frame % 24 < 12 { 1.0 } else { -1.0 },
        jump: frame % 17 == 0,
        jump_held: frame % 17 < 5,
        dash: frame % 29 == 3,
        attack: frame % 11 == 2,
        projectile: frame % 13 == 4,
        ..AgentAction::default()
    }
}

#[test]
fn sync_test_session_performs_real_rewinds_and_resimulation() {
    let mut sim = rollback_sim();
    for frame in 0..32 {
        sim.step(scripted_action(frame));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed");
    assert!(
        stats.load_runs > 0,
        "SyncTestSession issued LoadWorld requests"
    );
    assert!(
        stats.advance_runs > 32,
        "resimulation executes more GGRS frames than external harness steps: {stats:?}"
    );
}

#[test]
fn two_ggrs_harnesses_match_under_the_same_input_stream() {
    let mut left = rollback_sim();
    let mut right = rollback_sim();

    for frame in 0..48 {
        let action = scripted_action(frame);
        let a = left.step(action);
        let b = right.step(action);
        left.rollback_health().unwrap();
        right.rollback_health().unwrap();

        assert_eq!(
            a.player_pos, b.player_pos,
            "position diverged at frame {frame}"
        );
        assert_eq!(
            a.player_vel, b.player_vel,
            "velocity diverged at frame {frame}"
        );
        assert_eq!(a.hp, b.hp, "health diverged at frame {frame}");
        assert_eq!(
            a.active_room, b.active_room,
            "room diverged at frame {frame}"
        );
        assert_eq!(
            a.enemies.len(),
            b.enemies.len(),
            "entity population diverged at frame {frame}"
        );
        assert_eq!(
            a.pickups.len(),
            b.pickups.len(),
            "pickup population diverged at frame {frame}"
        );
    }
}

#[test]
fn dynamic_actor_churn_survives_ggrs_recreation() {
    let mut sim = rollback_sim();
    sim.spawn_boss_at(
        "ggrs_probe",
        "ggrs_probe",
        (640.0, 400.0),
        (24.0, 36.0),
        ambition::entity_catalog::placements::BossBrain::Dormant,
    );

    let rebased = sim.rollback_execution_stats().unwrap();
    assert_eq!(
        rebased.load_runs, 0,
        "setup discards pre-spawn rollback history"
    );
    assert_eq!(
        rebased.advance_runs, 0,
        "the fresh session has not advanced yet"
    );

    for frame in 0..24 {
        sim.step(scripted_action(frame));
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
    }

    let stats = sim.rollback_execution_stats().unwrap();
    assert!(stats.load_runs > 0);
    assert!(
        sim.observation().enemies.iter().any(|enemy| enemy.alive),
        "the dynamically spawned actor remains represented after repeated rollback recreation"
    );
}

#[test]
fn authoritative_entity_families_are_ggrs_anchors() {
    let mut sim = rollback_sim();
    let world = sim.world_mut();

    let mut bodies =
        world.query_filtered::<Option<&Rollback>, With<ambition::actors::actor::BodyKinematics>>();
    assert!(bodies.iter(world).all(|marker| marker.is_some()));

    let mut features = world.query_filtered::<Option<&Rollback>, With<ambition::platformer::lifecycle::FeatureSimEntity>>();
    assert!(features.iter(world).all(|marker| marker.is_some()));

    let mut projectiles = world.query_filtered::<Option<&Rollback>, With<ambition::platformer::projectile::ProjectileGameplay>>();
    assert!(projectiles.iter(world).all(|marker| marker.is_some()));

    let mut roots =
        world.query_filtered::<Option<&Rollback>, With<ambition::actors::rooms::RoomSet>>();
    assert!(roots.iter(world).all(|marker| marker.is_some()));
}

#[test]
fn registration_dump_and_fingerprint_are_stable() {
    let mut first = rollback_sim();
    let mut second = rollback_sim();
    let a = first.world().resource::<RollbackRegistry>();
    let b = second.world().resource::<RollbackRegistry>();
    assert_eq!(a.deterministic_dump(), b.deterministic_dump());
    assert_eq!(a.schema_fingerprint(), b.schema_fingerprint());
}

#[test]
fn ordinary_room_local_motion_does_not_invalidate_the_session_contract() {
    let mut sim = rollback_sim();
    let initial = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("prepared identity exists")
    };

    for frame in 0..20 {
        sim.step(scripted_action(frame));
        sim.rollback_health().unwrap();
    }

    let observed = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition::runtime::PreparedContentIdentity>();
        query
            .single(world)
            .copied()
            .expect("prepared identity remains attached")
    };
    assert_eq!(
        initial, observed,
        "simulation frames never change the content epoch"
    );
}
