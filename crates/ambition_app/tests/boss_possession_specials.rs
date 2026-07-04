//! Boss possession END-TO-END through the real headless sim — the "bosses are in
//! scope" mandate (unified-actors I2/I7): a possessed boss consumes slot input
//! AND commands its OWN authored specials, not movement only.
//!
//! Possession is brain transfer: `Brain::Player(PRIMARY)` moves onto the boss,
//! which then reads slot input through the SAME universal-brain path every
//! controlled body uses. The boss tick (`tick_boss_brains_system`) maps attack /
//! special input onto the boss's persisted `BossCapability` (its authored strike
//! repertoire, body data that survives the brain swap) and publishes a per-frame
//! `BossAttackIntent` the shared moveset trigger reads (§A1 intent/projection split).
//! A SPECIAL runs through the moveset like the autonomous pattern's — its move fires
//! the content technique (with the possessor's effective Player faction) and projects
//! the read-model. Capability is body data; the human is the policy choosing from it.
//!
//! This pins, driving REAL inputs through `SandboxSim::step`:
//! 1. A ~2s Down+Interact hold next to a boss possesses it (its brain becomes
//!    `Brain::Player`), without mutating its authored `Boss` faction.
//! 2. Pressing Attack (the boss's geometry-primary) is SUPPRESSED while possessed —
//!    §A1 slice 1b made `BossAttackState` a pure projection, and a possessed geometry
//!    strike starts no move, so it projects no read-model (a BLIND pose loss pending
//!    the effective-faction follow-up; its damage was already suppressed).
//! 3. Pressing Special (blink button) fires the boss's SIGNATURE content special —
//!    it runs through the moveset, so its `active_profile` read-model is projected.
//! 4. Releasing restores the boss's autonomous `BossPattern` brain.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_characters::actor::BossBrain;
use ambition_characters::brain::{BossAttackProfile, BossAttackState, BossCapability, Brain};
use ambition_gameplay_core::abilities::traversal::possession::PossessionState;
use ambition_gameplay_core::actor::{BodyKinematics, PrimaryPlayerOnly};
use ambition_gameplay_core::features::{ActorFaction, FeatureId};
use bevy::prelude::{Entity, World};

const BOSS_ID: &str = "possess_boss";

fn player_pos(world: &mut World) -> ambition_engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

fn boss_entity(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == BOSS_ID)
        .map(|(e, _)| e)
        .expect("the spawned boss is present")
}

fn possessed(sim: &mut SandboxSim) -> Option<Entity> {
    sim.world_mut().resource::<PossessionState>().possessed
}

fn active_profile(world: &mut World, boss: Entity) -> Option<BossAttackProfile> {
    world
        .get::<BossAttackState>(boss)
        .and_then(|s| s.active_profile.clone())
}

/// Hold Down (`move_y > 0.35`) + Interact — the possession gesture (see
/// `possession_end_to_end.rs`).
fn down_interact(edge: bool) -> AgentAction {
    AgentAction {
        move_y: 1.0,
        interact: edge,
        interact_held: true,
        ..AgentAction::default()
    }
}

/// Spawn a boss one short stride from the player and possess it (~2s hold).
/// GNU-ton is a `StationaryGiant` (it stays put through the hold, unlike an
/// airborne swooping boss) whose scripted repertoire is
/// `[HandSlam, HandSweep, HeadDescent, ConvergingShockwave, Special("apple_rain")]`
/// — a geometry primary (slot 0) AND a content signature special, so both
/// mapping arms are exercised.
fn spawn_and_possess_boss(sim: &mut SandboxSim) -> Entity {
    let p = player_pos(sim.world_mut());
    sim.spawn_boss_at(
        BOSS_ID,
        "gnu_ton",
        (p.x + 60.0, p.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "gnu_ton".to_string(),
        },
    );
    let boss = boss_entity(sim.world_mut());
    for i in 0..150 {
        sim.step(down_interact(i == 0));
    }
    assert_eq!(
        possessed(sim),
        Some(boss),
        "setup: the ~2s Down+Interact hold should have possessed the boss"
    );
    assert_eq!(
        sim.world_mut().get::<ActorFaction>(boss).copied(),
        Some(ActorFaction::Boss),
        "possession does NOT mutate the boss's authored faction (effective allegiance)"
    );
    boss
}

/// Idle until the boss's strike window closes, so a fresh press isn't ignored
/// mid-strike (the active window is the body's fire-rate enforcement, I3).
fn wait_out_strike(sim: &mut SandboxSim, boss: Entity) {
    for _ in 0..200 {
        if active_profile(sim.world_mut(), boss).is_none() {
            return;
        }
        sim.step(AgentAction::default());
    }
    let rem = sim
        .world_mut()
        .get::<BossAttackState>(boss)
        .map(|s| s.active_remaining);
    panic!("boss strike never cleared; active_remaining={rem:?}");
}

#[test]
fn possessed_boss_commands_its_authored_specials_and_release_restores_the_pattern() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let boss = spawn_and_possess_boss(&mut sim);

    // The boss's authored repertoire is present as body capability, surviving the
    // brain swap. slot(0) is the primary strike; the signature special is the
    // first content Special.
    let (primary, signature) = {
        let cap = sim
            .world_mut()
            .get::<BossCapability>(boss)
            .expect("possessed boss retains its authored capability");
        (
            cap.slot(0).expect("gnu_ton has strikes").0.clone(),
            cap.signature_special()
                .expect("gnu_ton authors a content special")
                .0
                .clone(),
        )
    };
    assert_eq!(primary, BossAttackProfile::HandSlam);
    assert_eq!(
        signature,
        BossAttackProfile::Special("apple_rain".to_string())
    );

    // No strike in flight before we press.
    assert_eq!(active_profile(sim.world_mut(), boss), None);

    // The boss's PRIMARY strike (slot 0) is a GEOMETRY profile (HandSlam). §A1 slice 1b
    // made `BossAttackState` a pure PROJECTION of the live `MovePlayback`, and a
    // possessed boss's geometry strike is suppressed by `trigger_boss_attack_moves`
    // (parity with the retired `sync_boss_strike_hitboxes`, which never struck for a
    // player-controlled boss) — so it starts NO move and therefore projects NO strike
    // read-model. Its damage was already suppressed; it now shows no strike POSE either
    // (a BLIND presentation change, Jon feel-checks). Restoring it as a REAL strike —
    // routed through the moveset with the possessor's EFFECTIVE faction so it hits the
    // boss's former allies, not its possessor — is the effective-faction follow-up.
    assert!(!primary.is_special(), "the primary (HandSlam) is a geometry strike");
    sim.step(AgentAction {
        attack: true,
        ..AgentAction::default()
    });
    assert_eq!(
        active_profile(sim.world_mut(), boss),
        None,
        "a possessed geometry strike is suppressed (no move → no projected pose) \
         pending the effective-faction follow-up"
    );

    // Special (the blink button maps to `special_pressed` in the player brain) →
    // the boss's SIGNATURE content special fires (emitting an
    // `ActorActionMessage::Special` its content technique consumes).
    sim.step(AgentAction {
        blink: true,
        ..AgentAction::default()
    });
    assert_eq!(
        active_profile(sim.world_mut(), boss),
        Some(signature),
        "Special input fires the boss's signature content special"
    );

    // Release: a fresh Down+Interact press restores the boss's autonomous brain.
    wait_out_strike(&mut sim, boss);
    sim.step(down_interact(true));
    assert_eq!(
        possessed(&mut sim),
        None,
        "a fresh press releases possession"
    );
    assert!(
        matches!(
            sim.world_mut().get::<Brain>(boss),
            Some(Brain::StateMachine(_))
        ),
        "release restores the boss's autonomous BossPattern brain"
    );
}
