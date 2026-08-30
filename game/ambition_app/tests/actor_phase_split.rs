//! The actor monolith is split into explicit phases — this pins the SEAM between
//! them through the real headless schedule.
//!
//!   tick_actor_brains      — snapshot + brain → frame-local decision output
//!   publish_actor_decision_frames — decision output → `ActorControl`
//!   integrate_sim_bodies   — reads that `ActorControl` → moves `BodyKinematics`
//!   sync_actor_read_model  — mirrors integrated state onto the read-model comps
//!   apply_actor_contact_damage — observes post-move overlap → HitEvent
//!
//! `ActorControl` is the seam between the brain phase and the movement phase. These
//! tests drive the real sim and assert (1) the decision + publish phases commit
//! a body's intent into `ActorControl`, and (2) the movement phase turns that SAME
//! `ActorControl` into position change — i.e. brain → intent → body flows through
//! the separated phases. The isolation of each phase is structural: the movement
//! query carries no `Brain` (it cannot tick one) and the brain loop calls no
//! `em.update` (it cannot move a body); this pins the composed seam that guarantee
//! produces.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{
    AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::{Entity, World};

const ENEMY_ID: &str = "phase_split_enemy";

fn player_pos(world: &mut World) -> ambition_platformer2d::engine_core::Vec2 {
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

/// A hostile actor's brain produces movement intent, the publish phase commits it
/// into `ActorControl`, and the movement phase turns that same
/// `ActorControl` into position change — the brain→body seam across the split.
///
/// Every sim plugin registers into `SimSchedule` rather than naming a schedule, so the graph is
/// the same graph and the phase seam must hold identically. If threading the label broke an
/// ordering edge, exactly one of these two fails.
fn brain_intent_seam_holds(fixed_tick: bool) {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_fixed_tick(fixed_tick),
    )
    .expect("sandbox sim builds");
    // Drop the enemy a stride to the player's RIGHT; a chasing brain wants to move
    // LEFT toward the player, so its intent has a definite sign we can assert.
    let p = player_pos(sim.world_mut());
    // ⛔ SPAWN BY CHARACTER, NOT BY ARCHETYPE. A `Custom(..)` row that no longer
    // exists falls back to a generic `combatant`, so the fixture keeps passing
    // while asserting on a body that is not the one it names.

    sim.spawn_enemy_character_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 120.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let enemy = enemy_entity(sim.world_mut());
    let x_before = sim.world_mut().get::<BodyKinematics>(enemy).unwrap().pos.x;

    // Step once: DECIDE produces the frame and PUBLISH commits it to `ActorControl`.
    sim.step(AgentAction::default());
    let control = sim
        .world_mut()
        .get::<ActorControl>(enemy)
        .expect("enemy carries ActorControl written by the decision publish phase")
        .0;
    assert!(
        control.locomotion.x < -0.1,
        "the decision publish phase committed leftward chase intent into ActorControl \
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

#[test]
fn brain_intent_lands_in_actor_control_and_the_movement_phase_consumes_it() {
    brain_intent_seam_holds(false);
}

/// The same seam, with the whole sim threaded into `FixedUpdate` (N0.1).
#[test]
fn brain_intent_seam_holds_under_fixed_tick() {
    brain_intent_seam_holds(true);
}
