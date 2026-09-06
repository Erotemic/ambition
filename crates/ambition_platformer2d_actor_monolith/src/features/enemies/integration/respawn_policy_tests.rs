//! A room reset must not resurrect a corpse whose respawn policy forbids it.
//!
//! These assert the value immediately after the reset call, which is the only
//! place the old behavior was observable.

use super::*;
use crate::features::ecs::actor_clusters::SeedActorMut;
use ambition_body_seed::ActorClusterSeed;
use ambition_entity_catalog::placements::{CharacterBrain, RespawnPolicy};

/// A dead body carrying `respawn`, reset once; returns whether it came back.
fn revived_by_room_reset(respawn: RespawnPolicy) -> bool {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "corpse".to_string(),
        "Corpse".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    seed.config.tuning.respawn = respawn;
    let mut model = ambition_platformer2d_core::movement::MotionModel::default();
    let mut em = seed.as_actor_mut();
    // Kill it the way combat does: drain the damage meter to zero HP.
    let pool = em.health.max();
    em.health.damage(pool);
    assert!(!em.health.alive(), "fixture must start dead");
    em.reset_to_spawn(&mut model);
    em.health.alive()
}

#[test]
fn a_room_reset_leaves_a_dead_stays_dead_corpse_dead() {
    assert!(
        !revived_by_room_reset(RespawnPolicy::DeadStaysDead),
        "a DeadStaysDead actor must stay dead through a room reset — reviving it \
         (even for the remainder of one frame, before save-sync re-zeroes the HP) \
         makes the reset a second, policy-blind liveness authority"
    );
}

#[test]
fn a_room_reset_leaves_an_on_rest_corpse_dead() {
    assert!(
        !revived_by_room_reset(RespawnPolicy::OnRest),
        "an OnRest actor returns at a save point, not on room re-entry"
    );
}

#[test]
fn a_room_reset_revives_an_on_room_reenter_mob() {
    assert!(
        revived_by_room_reset(RespawnPolicy::OnRoomReenter),
        "OnRoomReenter is precisely the policy that says 'fresh every time the \
         player enters the room' — the reset is its revive path"
    );
}

#[test]
fn a_room_reset_restores_a_live_actor_to_full_health_regardless_of_policy() {
    // The gate is about CORPSES. A living, damaged body still resets to full
    // under every policy — that is ordinary room-reset behavior and the change
    // must not narrow it.
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0));
    let mut seed = ActorClusterSeed::new(
        "survivor".to_string(),
        "Survivor".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    seed.config.tuning.respawn = RespawnPolicy::DeadStaysDead;
    let max = seed.health.max();
    let mut model = ambition_platformer2d_core::movement::MotionModel::default();
    let mut em = seed.as_actor_mut();
    em.health.damage(1);
    assert!(em.health.alive(), "fixture must still be alive");
    em.reset_to_spawn(&mut model);
    assert_eq!(
        em.health.current(),
        max,
        "a living actor resets to full health under every policy"
    );
}

/// Respawn is a body restart, not merely a transit: it resets maneuver state and
/// raises `BodyRestarted` while preserving the body's authored size.
#[test]
fn a_respawn_announces_a_restart_and_keeps_the_bodys_own_size() {
    let size = ae::Vec2::new(24.0, 40.0);
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, size);
    let mut seed = ActorClusterSeed::new(
        "reviver".to_string(),
        "Reviver".to_string(),
        aabb,
        CharacterBrain::Custom("cellular_automaton_fighter".into()),
        &[],
    );
    seed.config.tuning.respawn = RespawnPolicy::OnRoomReenter;
    let mut model = ambition_platformer2d_core::movement::MotionModel::default();
    let mut em = seed.as_actor_mut();
    let spawn_size = em.spawn.size;
    assert!(!em.lifetime.restart_pending);

    em.reset_to_spawn(&mut model);

    assert!(
        em.lifetime.restart_pending,
        "an enemy came back without the engine saying its body had restarted, so \
         every provider holding life-scoped state kept it"
    );
    // `BodyBaseSize::default()` is the default PLAYER size, and nothing on the
    // enemy spawn path wrote it — so a reset that restores `base_size` would
    // silently resize every enemy in the game to a player. The respawn records
    // the identity size before resetting; without that this assertion fails.
    assert_eq!(
        em.kin.size, spawn_size,
        "the respawn resized this body to somebody else's default"
    );
}
