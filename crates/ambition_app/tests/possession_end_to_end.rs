//! Phase C / C1 — possession works END-TO-END through the real headless sim.
//!
//! The keystone payoff of the actor-unification arc: a human can take over a
//! normal actor and drive it through the SAME body code path the actor's own
//! brain uses (`tick_player_brain_from_control` → its own `ActorControlFrame` →
//! `update_ecs_actors`), while the player's own body is suppressed. The trigger
//! gesture, faction flip, input-sync, and camera-follow are all already wired;
//! this pins the whole loop driving REAL inputs through `SandboxSim::step`:
//!
//! 1. Hold Down+Interact ~2s next to an actor → it becomes `Possessed` and flips
//!    to the player's faction.
//! 2. Driving `move_x` then moves the POSSESSED body (its own update path) while
//!    the player's own body stays frozen (`player_body_tick` is gated
//!    `not_possessing`).
//! 3. A fresh Down+Interact press releases — the actor reverts to its own faction
//!    and its own brain.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_characters::actor::EnemyBrain;
use ambition_engine_core as ae;
use ambition_gameplay_core::abilities::traversal::possession::PossessionState;
use ambition_gameplay_core::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::features::{ActorFaction, FeatureId};
use bevy::prelude::{Entity, World};

const ACTOR_ID: &str = "possess_target";

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

fn actor_entity(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == ACTOR_ID)
        .map(|(e, _)| e)
        .expect("the spawned actor is present")
}

fn possessed(sim: &mut SandboxSim) -> Option<Entity> {
    sim.world_mut().resource::<PossessionState>().possessed
}

fn faction(world: &mut World, e: Entity) -> ActorFaction {
    *world.get::<ActorFaction>(e).expect("actor faction")
}

/// Hold Down (`move_y > 0.35`) + Interact — the possession gesture. The HOLD
/// accumulates on `interact_held` (the real binding is `pressed`, i.e. held);
/// the single-frame `interact` edge fires only when `edge` is set (frame one of
/// a press), exactly as the device pipeline reports a real button hold.
fn down_interact(edge: bool) -> AgentAction {
    AgentAction {
        move_y: 1.0,
        interact: edge,
        interact_held: true,
        ..AgentAction::default()
    }
}

#[test]
fn a_player_can_possess_drive_and_release_an_actor_end_to_end() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");

    // Drop a normal actor one short stride from the player — inside POSSESS_RADIUS
    // (150px). Same known-good melee archetype the enemy-attacks test uses.
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_at(
        ACTOR_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        EnemyBrain::Custom("cellular_automaton_fighter".to_string()),
    );
    let actor = actor_entity(sim.world_mut());
    assert_eq!(
        faction(sim.world_mut(), actor),
        ActorFaction::Enemy,
        "the actor starts on its own (Enemy) faction"
    );

    // 1. Hold Down+Interact past the ~2s commit threshold (fixed 60hz → ~120
    //    frames; hold 150 for margin). The trigger runs on real time.
    for i in 0..150 {
        sim.step(down_interact(i == 0));
    }
    assert_eq!(
        possessed(&mut sim),
        Some(actor),
        "a full ~2s Down+Interact hold next to the actor possesses it"
    );
    assert_eq!(
        faction(sim.world_mut(), actor),
        ActorFaction::Player,
        "the possessed actor flips to the player's side"
    );

    // 2. Drive right. The POSSESSED body should move (its own update path); the
    //    player's own body is frozen (`player_body_tick` gated `not_possessing`).
    let player_before = player_pos(sim.world_mut());
    let actor_before = sim.world_mut().get::<BodyKinematics>(actor).unwrap().pos;
    // A short burst — long enough to clearly travel, short enough to stay on the
    // platform (driven far enough at POSSESSED_MOVE_SPEED it would walk off a
    // ledge and despawn OOB, which is realistic but not what this test isolates).
    for _ in 0..30 {
        sim.step(AgentAction::move_x(1.0));
    }
    let player_after = player_pos(sim.world_mut());
    let actor_after = sim.world_mut().get::<BodyKinematics>(actor).unwrap().pos;

    assert!(
        actor_after.x - actor_before.x > 20.0,
        "the possessed body moves right under player input: {actor_before:?} -> {actor_after:?}"
    );
    // The guarantee is "the same input doesn't drive BOTH bodies": the player's
    // own body does NOT run right with `move_x` (its control is gated off while
    // possessing). Its x stays put while the possessed body travels. (Vertically
    // the abandoned body may settle a little under gravity / ground-snap — that's
    // not input-driven, so we pin the horizontal axis the input actually targets.)
    assert!(
        (player_after.x - player_before.x).abs() < 1.0,
        "the player's OWN body does not respond to the drive input while possessing: \
         {player_before:?} -> {player_after:?}"
    );

    // 3. A fresh Down+Interact press releases possession. `prev_down_interact` is
    //    false after the move phase, so this frame is the rising edge.
    sim.step(down_interact(true));
    assert_eq!(
        possessed(&mut sim),
        None,
        "a fresh Down+Interact press releases possession"
    );
    assert_eq!(
        faction(sim.world_mut(), actor),
        ActorFaction::Enemy,
        "on release the actor reverts to its original faction (its own brain again)"
    );
}
