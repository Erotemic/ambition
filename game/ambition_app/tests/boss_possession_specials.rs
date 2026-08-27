//! End-to-end boss possession through the real headless simulation.
//!
//! Possession redirects a driving participant onto the boss; the boss keeps its
//! authored faction and `BossCapability`. Human attack/special input chooses from
//! that body-owned repertoire, while emitted attacks use the possessor's
//! effective combat faction. Releasing possession restores the autonomous boss
//! brain.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
use ambition_platformer2d::actors::features::{ActorFaction, FeatureId};
use ambition_platformer2d::characters::brain::{
    BossAttackProfile, BossAttackState, BossCapability, Brain,
};
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::BossBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::vfx::HitSide;
use bevy::prelude::{Entity, World};

const BOSS_ID: &str = "possess_boss";

fn player_pos(world: &mut World) -> ambition_platformer2d::engine_core::Vec2 {
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

fn possessed(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
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
/// The GNU-ton rider is a `StationaryGiant` (it stays put through the hold,
/// unlike an airborne swooping boss) whose scripted repertoire is
/// `[HandSlam, HandSweep, HeadDescent, ConvergingShockwave, Special("apple_rain")]`
/// — a geometry primary (slot 0) AND a content signature special, so both
/// mapping arms are exercised.
fn spawn_and_possess_boss(sim: &mut Platformer2dSimHarness) -> Entity {
    let p = player_pos(sim.world_mut());
    sim.spawn_boss_at(
        BOSS_ID,
        "gnu_ton_rider",
        (p.x + 60.0, p.y),
        (30.0, 30.0),
        BossBrain::PhaseScript {
            script_id: "gnu_ton_rider".to_string(),
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
fn wait_out_strike(sim: &mut Platformer2dSimHarness, boss: Entity) {
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
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
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
            cap.slot(0)
                .expect("the gnu_ton rider has strikes")
                .0
                .clone(),
            cap.signature_special()
                .expect("the gnu_ton rider authors a content special")
                .0
                .clone(),
        )
    };
    assert_eq!(primary, BossAttackProfile::Strike("hand_slam".to_string()));
    assert_eq!(
        signature,
        BossAttackProfile::Special("apple_rain".to_string())
    );

    // No strike in flight before we press.
    assert_eq!(active_profile(sim.world_mut(), boss), None);

    // Since R1.4 a possessed boss's geometry strike FIRES like any other move
    // (possession grants the full kit, invariant I2): pressing Attack starts its
    // moveset move, so the projected `active_profile` read-model shows the strike.
    // The strike hitbox carries the possessor's EFFECTIVE faction (`Player`,
    // stamped in `advance_move_playback`), so it hits the boss's former allies
    // rather than the controlling player.
    //
    // WHICH strike is the boss's own authored choice. The rider authors a G5 `possessed_verbs` map
    // (`attack` -> `hand_sweep`), so plain Attack commands that move instead of falling back to
    // slot 0 — and because `hand_sweep` is limb-routed, the press drives the giant mount's
    // facing-side hand.
    let attack_move = BossAttackProfile::Strike("hand_sweep".to_string());
    assert!(
        !primary.is_special() && !attack_move.is_special(),
        "the rider's strikes are geometry profiles, not specials"
    );
    sim.step(AgentAction {
        attack: true,
        ..AgentAction::default()
    });
    assert_eq!(
        active_profile(sim.world_mut(), boss),
        Some(attack_move),
        "a possessed boss's geometry strike fires through the moveset (R1.4), \
         resolved through its authored possessed-verb map (G5)"
    );
    // The spawned strike hitbox carries the possessor's EFFECTIVE faction (Player),
    // not the boss's authored `Boss` — so a possessing player's geometry strike hurts
    // the boss's former allies, not the player. This is the load-bearing correctness
    // of routing the strike through the shared moveset instead of suppressing it.
    {
        // `HitSide` is the presentation-neutral side carried by vfx hitboxes.
        let mut q = sim
            .world_mut()
            .query::<&ambition_platformer2d::combat::hitbox::Hitbox>();
        let strike_sides: Vec<HitSide> = q
            .iter(sim.world_mut())
            .filter(|h| h.owner == boss)
            .map(|h| h.source)
            .collect();
        assert!(
            !strike_sides.is_empty(),
            "the possessed geometry strike spawned a hitbox"
        );
        assert!(
            strike_sides
                .iter()
                .all(|side| matches!(side, HitSide::Player)),
            "the possessed boss's strike hitbox carries the possessor's effective faction \
             (Player), not the authored Boss: {strike_sides:?}"
        );
    }

    // The geometry strike is a committed move (the Smash convention: it runs to
    // completion, uncancelable into a special mid-strike). Wait it out before the
    // next input so the boss is free to start a new move — exactly as real play
    // requires now that a possessed geometry strike actually occupies the body (R1.4).
    wait_out_strike(&mut sim, boss);

    // Special (the DEDICATED `Platformer2dInputActionMonolith::Special` slot — the
    // `special_pressed = blink_pressed` alias is retired, so blink no longer
    // fires it) → the boss's SIGNATURE content special fires (emitting an
    // `ActorActionMessage::Special` its content technique consumes).
    sim.step(AgentAction {
        special: true,
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
