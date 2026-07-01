//! Phase C / C1 — possession works END-TO-END through the real headless sim.
//!
//! The keystone payoff of the control-unification arc: a human can take over a
//! normal actor because possession is BRAIN TRANSFER — `Brain::Player(PRIMARY)`
//! moves onto the actor, which then reads slot input through the SAME universal
//! brain path every controlled body uses (`SlotControls` → its own
//! `ActorControlFrame` → `update_ecs_actors`). The vacated home avatar has no
//! player brain, so it is inert. This pins the whole loop driving REAL inputs
//! through `SandboxSim::step`:
//!
//! 1. Hold Down+Interact ~2s next to an actor → its brain is replaced with
//!    `Brain::Player(PRIMARY)` (recorded in `PossessionState.possessed`). Its
//!    AUTHORED faction is NOT mutated — effective allegiance makes combat treat it
//!    as player-aligned while it carries the player brain.
//! 2. Driving `move_x` then moves the POSSESSED body (its own body path at its own
//!    run capability) while the vacated home avatar stays put (it has neutral
//!    input, no player brain — no `not_possessing` gate needed).
//! 3. A fresh Down+Interact press releases — the actor's authored brain is
//!    restored and the home avatar reclaims `Brain::Player`.

#![cfg(feature = "rl_sim")]

use ambition_app::{AgentAction, SandboxSim, TimestepMode};
use ambition_characters::actor::CharacterBrain;
use ambition_characters::brain::ActorControl;
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

/// Possess the actor `stride` px to the player's right, returning its entity.
/// Shared setup for the possession tests below.
fn spawn_and_possess(sim: &mut SandboxSim) -> Entity {
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_at(
        ACTOR_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
    );
    let actor = actor_entity(sim.world_mut());
    for i in 0..150 {
        sim.step(down_interact(i == 0));
    }
    assert_eq!(
        possessed(sim),
        Some(actor),
        "setup: the ~2s hold should have possessed the actor"
    );
    actor
}

/// SAME-FRAME slot input (schedule invariant): a possessed actor ticks inside
/// `WorldPrep`, which runs BEFORE `PlayerInput`. `SlotControls` +
/// `ControlledSubject` must be published even earlier (before `WorldPrep`), so a
/// SINGLE `move_x` step must show up in the possessed body's `ActorControl` THIS
/// frame — not next frame. Before the schedule fix this read last frame's input.
#[test]
fn possessed_actor_reads_this_frame_slot_input() {
    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let actor = spawn_and_possess(&mut sim);

    // The possess gesture drove `move_y` (down); horizontal was zero, so the
    // actor's control x is ~0 going into this step. ONE step of move_x(1.0):
    sim.step(AgentAction::move_x(1.0));

    let control = sim
        .world_mut()
        .get::<ActorControl>(actor)
        .expect("possessed actor carries ActorControl")
        .0;
    assert!(
        control.locomotion.x > 0.5,
        "the possessed actor's ActorControl must reflect THIS frame's move_x \
         (same-frame slot input); got locomotion.x = {} — the WorldPrep actor tick \
         read a stale SlotControls",
        control.locomotion.x,
    );
}

/// THE reported-bug invariant, end-to-end through the real sim: pressing Attack
/// while possessing starts the melee lifecycle on the POSSESSED actor (its
/// `BodyMelee` swings and, at the active edge, it OWNS the spawned strike
/// hitbox), while the vacated home avatar's melee never starts. Attack authority
/// follows `Brain::Player`, not the home body.
#[test]
fn attack_while_possessing_starts_the_possessed_actors_melee_not_the_home() {
    use ambition_gameplay_core::features::{BodyMelee, Hitbox};

    let mut sim =
        SandboxSim::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    let home = {
        let mut q = sim
            .world_mut()
            .query_filtered::<Entity, PrimaryPlayerOnly>();
        q.single(sim.world_mut()).expect("primary player").clone()
    };
    let actor = spawn_and_possess(&mut sim);

    // A swing's melee lifecycle ENGAGED this frame: either it is mid-swing, or its
    // recovery cooldown is armed (a swing began and — under the fixed-timestep
    // harness, where one `sim.step` can advance many sim frames — may already have
    // run to completion). Robust to catch-up; still proves "attack started this
    // body's melee".
    let melee_engaged = |sim: &mut SandboxSim, e: Entity| {
        sim.world_mut()
            .get::<BodyMelee>(e)
            .map(|m| m.is_swinging() || m.cooldown > 0.0)
            .unwrap_or(false)
    };

    // Hold Attack across a window. The possessed actor carries Brain::Player, so
    // its melee ActionSet resolves an ActorActionMessage::Melee addressed to
    // ITSELF, which enters the ONE body melee lifecycle (`start_body_melee` →
    // `advance_body_melee`) and, at the active edge, spawns a strike it OWNS. The
    // vacated home avatar has no player brain, so its melee never engages and it
    // owns no strike. Observed over a window (not one frame) to be robust to the
    // fixed-timestep catch-up.
    let mut actor_engaged = false;
    let mut home_engaged = false;
    let mut actor_owns_strike = false;
    let mut home_owns_strike = false;
    for _ in 0..30 {
        sim.step(AgentAction {
            attack: true,
            ..AgentAction::default()
        });
        actor_engaged |= melee_engaged(&mut sim, actor);
        home_engaged |= melee_engaged(&mut sim, home);
        let mut q = sim.world_mut().query::<&Hitbox>();
        for hb in q.iter(sim.world_mut()) {
            if hb.owner == actor {
                actor_owns_strike = true;
            }
            if hb.owner == home {
                home_owns_strike = true;
            }
        }
    }

    assert!(
        actor_engaged,
        "the POSSESSED actor's melee lifecycle engaged on Attack"
    );
    assert!(
        !home_engaged,
        "the vacated home avatar's melee did NOT engage — attack authority is the \
         body carrying Brain::Player, not the home body"
    );
    assert!(
        actor_owns_strike,
        "the possessed actor's swing spawned a strike hitbox OWNED by the actor",
    );
    assert!(
        !home_owns_strike,
        "the vacated home avatar spawned no strike",
    );
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
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
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
        ActorFaction::Enemy,
        "possession does NOT mutate the authored faction — effective allegiance \
         (carrying Brain::Player) is what makes combat treat it as player-aligned"
    );

    // 2. Drive right. The POSSESSED body should move — it now integrates through
    //    the SAME unified `integrate_sim_bodies` phase every body uses. The vacated
    //    home avatar stays put because it carries no `Brain::Player` (its
    //    `ActorControl` is neutral), NOT because of any movement run-condition gate.
    let player_before = player_pos(sim.world_mut());
    let actor_before = sim.world_mut().get::<BodyKinematics>(actor).unwrap().pos;
    // A short burst — long enough to clearly travel, short enough to stay on the
    // platform (driven far enough at the body's own run speed it would walk off a
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
    // The guarantee is "the same input doesn't drive BOTH bodies": the vacated
    // home avatar does NOT run right with `move_x` — it has no `Brain::Player`, so
    // its `ActorControl` is neutral. Its x stays put while the possessed body
    // travels. (Vertically the abandoned body may settle a little under gravity /
    // ground-snap — not input-driven, so we pin the horizontal axis the input
    // actually targets.)
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
