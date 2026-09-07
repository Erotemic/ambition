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
