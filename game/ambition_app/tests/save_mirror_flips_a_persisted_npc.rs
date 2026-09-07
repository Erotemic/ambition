#![cfg(feature = "rl_sim")]
//! A provoked NPC is still hostile after the save says so — the save mirror,
//! witnessed in the shipped app.
//!
//! ⛔⛔ MEASURED 2026-09-07 (`dev/installer_call_coverage.json`): deleting
//! `install_save_mirror(app, sim)` from `progression_schedule.rs:69` left ALL 583
//! `app_it` tests green. Two shipped behaviours ride that installer — a persisted
//! provoked NPC loading hostile, and a persisted non-respawning enemy death staying
//! dead — and neither had a witness at app level.
//!
//! ⚠ THE MIRROR RUNS EVERY SIM TICK, NOT AT LOAD, and its own source says so
//! (`save_sync.rs`: "which runs EVERY SIM TICK, not at load"). So this does not
//! need a room reload to observe it, and a test written around a reload would be
//! testing the reload.
//!
//! ⛔ DO NOT REPLACE THIS WITH A PRESENCE ASSERTION. A marker resource the
//! installer registers would go green while the flip stayed untested, which is
//! the failure mode `queue.md` names for all six of these holes.

use crate::common::{base, fixed_60hz_sim};

use ambition_platformer2d::characters::actor::BodyHealth;
use ambition_platformer2d::combat::components::{
    ActorAggression, ActorIdentity, ActorInteraction, AggressionMode,
};
use bevy::prelude::*;

/// Every talkable actor in the world, as (entity, id, aggression mode).
fn talkable_actors(
    sim: &mut ambition_app::Platformer2dSimHarness,
) -> Vec<(Entity, String, AggressionMode)> {
    let mut query = sim
        .world_mut()
        .query_filtered::<(Entity, &ActorIdentity, &ActorAggression), With<ActorInteraction>>();
    let world = sim.world();
    query
        .iter(world)
        .map(|(entity, identity, aggression)| (entity, identity.id.clone(), aggression.mode))
        .collect()
}

#[test]
fn a_save_flag_makes_a_talkable_npc_hostile_without_a_room_reload() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    let before = talkable_actors(&mut sim);
    // ⚠ ANTI-VACUITY: with no talkable actor in the room, every assertion below
    // is about an empty set and the test certifies nothing.
    let (npc, id, mode) = before
        .first()
        .cloned()
        .expect("the start room authors at least one talkable NPC to provoke");
    assert_ne!(
        mode,
        AggressionMode::Hostile,
        "{id} is ALREADY hostile before the save says anything, so a hostile \
         reading afterwards would prove nothing"
    );

    // The flag's ONE spelling, asked of the code that writes it rather than
    // re-derived here — a second `format!` is a rename waiting to go one-sided.
    let flag = ambition_platformer2d::actors::features::npc_flag_id(&id);
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag.clone(), true);

    sim.step_n(base(), 1);

    let after = talkable_actors(&mut sim)
        .into_iter()
        .find(|(entity, _, _)| *entity == npc)
        .map(|(_, _, mode)| mode)
        .expect("the NPC is still in the world");
    assert_eq!(
        after,
        AggressionMode::Hostile,
        "the save carries `{flag}` and {id} is still {after:?} one tick later. \
         `install_save_mirror` is what mirrors that flag onto the live actor; \
         deleting its call leaves every other app test green."
    );

    // The grudge is the other half of the flip: a persisted-hostile NPC
    // re-establishes it against the stable player slot, because the attacker
    // entity that earned it does not survive a save round-trip.
    let grudge = {
        let mut query = sim.world_mut().query::<&ActorAggression>();
        let world = sim.world();
        query
            .get(world, npc)
            .expect("the NPC still carries its aggression")
            .grudge
    };
    assert!(
        grudge.is_some(),
        "{id} loaded hostile with NO grudge: a hostile mode with nothing to be \
         hostile AT is a body that stands there, which is the bug the mirror's \
         `stable_player_grudge` exists to avoid"
    );
}

/// The mirror's OTHER half: a persisted death stays dead.
///
/// ⛔⛔ THE SUBJECT IS SCARCE AND THAT IS THE FINDING BEHIND THIS ROOM CHOICE.
/// A placement that authors NO respawn policy takes `UNDESCRIBED_BODY_RESPAWN`,
/// which is `OnRoomReenter` — a policy that writes no flag on death and reads
/// none on load. So the persisted-death arm is unreachable from most of the
/// world. MEASURED 2026-09-07 over every shipped `.ldtk`
/// (`scripts/measure_persisting_enemy_placements.py`): exactly two rooms author a
/// persisting policy, `pirate_sky_lookout` (4) and `pirate_sky_arena` (3), all of
/// them `OnRest` — and NOTHING in the shipped world authors `DeadStaysDead`,
/// despite it being the enum's `#[default]`.
///
/// ⚠ SO THE FLAG HERE IS THE `OnRest` SPELLING, `enemy_<id>_dead_until_rest`, and
/// a test written against `enemy_<id>_dead` would fail in this room for a reason
/// that has nothing to do with the mirror.
#[test]
fn a_persisted_on_rest_death_zeroes_the_body_on_the_next_tick() {
    let mut sim = crate::common::fixed_60hz_room_sim("pirate_sky_lookout");
    sim.step_n(base(), 120);

    let alive: Vec<(Entity, String)> = {
        let mut query = sim
            .world_mut()
            .query::<(Entity, &ActorIdentity, &BodyHealth)>();
        let world = sim.world();
        query
            .iter(world)
            .filter(|(_, identity, health)| {
                identity.id.starts_with("EnemySpawn") && health.health.current > 0
            })
            .map(|(entity, identity, _)| (entity, identity.id.clone()))
            .collect()
    };
    // ⚠ ANTI-VACUITY: this room is chosen for its `OnRest` placements. If it
    // stops authoring live ones, everything below is about an empty set.
    let (body, id) = alive
        .first()
        .cloned()
        .expect("pirate_sky_lookout authors live EnemySpawn placements");

    let flag = format!(
        "enemy_{id}{}",
        ambition_platformer2d::actors::features::ENEMY_DEAD_UNTIL_REST_SUFFIX
    );
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag.clone(), true);

    sim.step_n(base(), 1);

    let health = sim
        .world()
        .get::<BodyHealth>(body)
        .expect("the body is still in the world")
        .health
        .current;
    assert_eq!(
        health, 0,
        "the save carries `{flag}` and {id} still has {health} health one tick \
         later. `install_save_mirror` is what makes a persisted death survive; \
         deleting its call leaves every other app test green."
    );
}
