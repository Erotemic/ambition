//! ONE melee lifecycle for every body — the convergence proof for the
//! melee-driver unification. There is no longer a player melee driver
//! (`attack_advance_system`) and an actor melee driver
//! (`start_enemy_melee_from_brain_actions` + the inline `update_ecs_actors`
//! active-edge spawn). Instead, EVERY body — the human player, a possessed actor,
//! an autonomous hostile — runs the SAME two body-generic phases through the real
//! schedule:
//!
//!   `ActorActionMessage::Melee` (from `emit_brain_action_messages`)
//!     → `combat::attack::start_body_melee`  (begins the ONE `BodyMelee` swing)
//!     → `combat::attack::advance_body_melee` (ticks it, spawns the active-edge
//!        strike through the shared `spawn_melee_strike`, owned by the body)
//!
//! This pins, through `SandboxSim::step`, that BOTH the player and an autonomous
//! hostile actor enter that identical `BodyMelee` lifecycle and own the strike
//! their swing spawns. The possessed-actor case is pinned by
//! `possession_end_to_end.rs`; the peaceful-NPC-with-kit case is the same path
//! gated by its ActionSet (capability) + brain (policy) — a peaceful brain never
//! presses attack, but a possessing human drives the identical lifecycle.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_entity_catalog::placements::CharacterBrain;
use ambition_gameplay_core::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::features::{BodyMelee, FeatureId, Hitbox};
use bevy::prelude::{Entity, World};

fn player_entity(world: &mut World) -> Entity {
    let mut q = world.query_filtered::<Entity, PrimaryPlayerOnly>();
    q.single(world).expect("primary player")
}

fn player_pos(world: &mut World) -> ambition_engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

/// A body's melee lifecycle engaged: mid-swing, or its recovery cooldown is armed
/// (a swing began this window — robust to fixed-timestep catch-up completing a
/// short swing within one `sim.step`).
fn melee_engaged(world: &mut World, e: Entity) -> bool {
    world
        .get::<BodyMelee>(e)
        .map(|m| m.is_swinging() || m.cooldown > 0.0)
        .unwrap_or(false)
}

fn owns_a_strike(world: &mut World, e: Entity) -> bool {
    let mut q = world.query::<&Hitbox>();
    q.iter(world).any(|hb| hb.owner == e)
}

/// The PLAYER's own melee now flows through the body-generic lifecycle (the
/// deleted `attack_advance_system` is gone): pressing Attack engages its
/// `BodyMelee` and spawns a strike it OWNS.
#[test]
fn the_player_enters_the_body_melee_lifecycle_and_owns_its_strike() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let player = player_entity(sim.world_mut());

    let mut engaged = false;
    let mut owns_strike = false;
    for _ in 0..30 {
        sim.step(AgentAction {
            attack: true,
            ..AgentAction::default()
        });
        engaged |= melee_engaged(sim.world_mut(), player);
        owns_strike |= owns_a_strike(sim.world_mut(), player);
    }

    assert!(
        engaged,
        "the player's BodyMelee lifecycle engages on Attack (via start_body_melee)"
    );
    assert!(
        owns_strike,
        "the player's swing spawns a strike hitbox it OWNS (via advance_body_melee \
         → spawn_melee_strike)"
    );
}

/// An autonomous hostile actor enters the SAME `BodyMelee` lifecycle from the SAME
/// `ActorActionMessage::Melee` path — no separate actor melee driver.
#[test]
fn a_hostile_actor_enters_the_same_body_melee_lifecycle() {
    const ENEMY_ID: &str = "unified_melee_enemy";
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
    );
    let enemy = {
        let mut q = sim.world_mut().query::<(Entity, &FeatureId)>();
        q.iter(sim.world_mut())
            .find(|(_, f)| f.as_str() == ENEMY_ID)
            .map(|(e, _)| e)
            .expect("spawned enemy present")
    };

    // Stand still; the in-range hostile fighter commits swings on its own.
    let mut engaged = false;
    let mut owns_strike = false;
    for _ in 0..240 {
        sim.step(AgentAction::default());
        engaged |= melee_engaged(sim.world_mut(), enemy);
        owns_strike |= owns_a_strike(sim.world_mut(), enemy);
    }

    assert!(
        engaged,
        "the hostile actor's BodyMelee lifecycle engages (same start_body_melee path)"
    );
    assert!(
        owns_strike,
        "the hostile actor's swing spawns a strike hitbox it OWNS (same \
         advance_body_melee path as the player)"
    );
}
