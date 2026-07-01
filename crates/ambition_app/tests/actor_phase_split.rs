//! The actor monolith is split into explicit phases — this pins the SEAM between
//! them through the real headless schedule.
//!
//! `update_ecs_actors` used to fuse brain tick + movement integration + read-model
//! sync + contact damage in one system. It is now four scheduled phases:
//!
//!   tick_actor_brains      — snapshot + brain → `ActorControl` (intent), no move
//!   integrate_actor_bodies — reads that `ActorControl` → moves `BodyKinematics`
//!   sync_actor_read_model  — mirrors integrated state onto the read-model comps
//!   apply_actor_contact_damage — observes post-move overlap → HitEvent
//!
//! `ActorControl` is the seam between the brain phase and the movement phase. These
//! tests drive the real sim and assert (1) the brain phase publishes a body's
//! intent into `ActorControl`, and (2) the movement phase turns that SAME
//! `ActorControl` into position change — i.e. brain → intent → body flows through
//! the separated phases. The isolation of each phase is structural: the movement
//! query carries no `Brain` (it cannot tick one) and the brain loop calls no
//! `em.update` (it cannot move a body); this pins the composed seam that guarantee
//! produces.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_characters::actor::CharacterBrain;
use ambition_characters::brain::ActorControl;
use ambition_gameplay_core::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::features::FeatureId;
use bevy::prelude::{Entity, World};

const ENEMY_ID: &str = "phase_split_enemy";

fn player_pos(world: &mut World) -> ambition_engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

fn enemy_entity(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == ENEMY_ID)
        .map(|(e, _)| e)
        .expect("spawned enemy present")
}

/// A hostile actor's brain publishes movement intent into its `ActorControl`
/// (the brain phase's only output), and the movement phase turns that same
/// `ActorControl` into position change — the brain→body seam across the split.
#[test]
fn brain_intent_lands_in_actor_control_and_the_movement_phase_consumes_it() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    // Drop the enemy a stride to the player's RIGHT; a chasing brain wants to move
    // LEFT toward the player, so its intent has a definite sign we can assert.
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 120.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
    );
    let enemy = enemy_entity(sim.world_mut());
    let x_before = sim.world_mut().get::<BodyKinematics>(enemy).unwrap().pos.x;

    // Step once: the brain phase writes the enemy's intent into `ActorControl`.
    sim.step(AgentAction::default());
    let control = sim
        .world_mut()
        .get::<ActorControl>(enemy)
        .expect("enemy carries ActorControl written by tick_actor_brains")
        .0;
    assert!(
        control.locomotion.x < -0.1,
        "the brain phase published leftward chase intent into ActorControl \
         (locomotion.x = {}); the enemy is right of the player and wants to close",
        control.locomotion.x,
    );

    // Drive a short window: the movement phase reads that `ActorControl` and moves
    // the body left (toward the player) — no separate actor movement driver.
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    let x_after = sim.world_mut().get::<BodyKinematics>(enemy).unwrap().pos.x;
    assert!(
        x_after < x_before - 5.0,
        "the movement phase turned the brain's ActorControl intent into leftward \
         position change: x {x_before} -> {x_after}",
    );
}
