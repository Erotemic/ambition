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
//! ⚠ The provoked-NPC half still runs as a mirror, every sim tick, so its test
//! needs no reload. The persisted-DEATH half is construction now — see the
//! second test, which is written around a rebuild because that is the mechanism.
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

/// A persisted death is BUILT, not mirrored: the room a replay rebuilds carries the
/// body in the state the save records, from the first frame it exists.
///
/// ⛔⛔ THE SUBJECT IS SCARCE AND THAT IS THE FINDING BEHIND THIS ROOM CHOICE.
/// A placement that authors NO respawn policy takes `UNDESCRIBED_BODY_RESPAWN`,
/// which is `OnRoomReenter` — a policy that writes no flag on death and reads
/// none on load. MEASURED 2026-09-07 over every shipped `.ldtk`
/// (`scripts/measure_persisting_enemy_placements.py`): exactly two rooms author a
/// persisting policy, `pirate_sky_lookout` (4) and `pirate_sky_arena` (3), all of
/// them `OnRest`. ⚠ So the flag here is the `OnRest` spelling,
/// `enemy_<id>_dead_until_rest`.
///
/// ⚠ This used to assert the body was zeroed ONE TICK after the flag was set,
/// in place — the mechanism then was a save mirror re-applying every flag every
/// sim tick. Construction now reads the record when a commit is requested
/// (`construction::PersistedFates`), so the witness is a rebuild.
#[test]
fn a_room_rebuilt_after_a_persisted_on_rest_death_builds_that_body_dead() {
    let mut sim = crate::common::fixed_60hz_room_sim("pirate_sky_lookout");
    sim.step_n(base(), 120);

    let on_rest_bodies = |sim: &mut ambition_app::Platformer2dSimHarness| -> Vec<(Entity, String, i32)> {
        let mut query = sim.world_mut().query::<(
            Entity,
            &ActorIdentity,
            &BodyHealth,
            &ambition_platformer2d::actor::ActorConfig,
        )>();
        let world = sim.world();
        query
            .iter(world)
            .filter(|(_, identity, _, config)| {
                identity.id.starts_with("EnemySpawn")
                    && matches!(
                        config.tuning.respawn,
                        ambition_platformer2d::actors::features::RespawnPolicy::OnRest
                    )
            })
            .map(|(entity, identity, health, _)| (entity, identity.id.clone(), health.health.current))
            .collect()
    };
    // ⚠ ANTI-VACUITY: this room is chosen for its `OnRest` placements.
    let (body, id, hp_before) = on_rest_bodies(&mut sim)
        .into_iter()
        .find(|(_, _, hp)| *hp > 0)
        .expect("pirate_sky_lookout authors live OnRest EnemySpawn placements");

    let flag = format!(
        "enemy_{id}{}",
        ambition_platformer2d::actors::features::ENEMY_DEAD_UNTIL_REST_SUFFIX
    );
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
        .data_mut()
        .set_flag(flag.clone(), true);
    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );

    let mut rebuilt_frames = 0;
    for frame in 0..60 {
        sim.step(base());
        let Some((entity, _, hp)) = on_rest_bodies(&mut sim)
            .into_iter()
            .find(|(_, other, _)| *other == id)
        else {
            continue;
        };
        if entity == body {
            continue;
        }
        rebuilt_frames += 1;
        assert_eq!(
            hp, 0,
            "frame {frame}: the save carries `{flag}` and the rebuilt {id} exists with \
             {hp} HP (it had {hp_before} before) — the room was built alive and left \
             for something else to correct",
        );
    }
    assert!(
        rebuilt_frames > 0,
        "the replay never rebuilt {id}, so nothing about construction was checked"
    );
}
