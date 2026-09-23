#![cfg(feature = "rl_sim")]
//! A body the save remembers is BUILT that way — dead, or provoked — by the
//! construction that makes it, witnessed in the shipped app.
//!
//! ⛔⛔ MEASURED 2026-09-07 (`dev/installer_call_coverage.json`): deleting the
//! save mirror's installer left ALL 583 `app_it` tests green, although two
//! shipped behaviours rode it. Both are construction now
//! (`construction::PersistedFates`), so both witnesses are a REBUILD: the
//! mechanism is what a commit reads, not what a later tick corrects.
//!
//! ⛔ DO NOT REPLACE THESE WITH A PRESENCE ASSERTION. A marker the construction
//! road inserts would go green while the fate stayed unbuilt.

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

/// A persisted provocation is BUILT: the room a replay rebuilds carries the
/// person hostile from the first frame they exist, with a grudge against
/// whoever plays — the body that struck the blow is gone, and at startup the
/// room is built before any player body exists to name.
#[test]
fn a_room_rebuilt_after_a_persisted_provocation_builds_that_person_hostile() {
    use ambition_platformer2d::combat::components::{ActorDisposition, ActorFaction, Grudge};

    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    // ⚠ ANTI-VACUITY: with no talkable actor in the room, every assertion below
    // is about an empty set and the test certifies nothing.
    let (npc, id, mode) = talkable_actors(&mut sim)
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
    let flag = ambition_platformer2d::actors::fate_flags::npc_flag_id(&id);
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
        let mut query = sim.world_mut().query_filtered::<(
            Entity,
            &ActorIdentity,
            &ActorAggression,
            &ActorDisposition,
            Option<&ambition_platformer2d::characters::actor::character_catalog::BrainBinding>,
        ), With<ActorInteraction>>();
        let world = sim.world();
        let Some((_, _, aggression, disposition, binding)) = query
            .iter(world)
            .find(|(entity, identity, _, _, _)| identity.id == id && *entity != npc)
        else {
            continue;
        };
        rebuilt_frames += 1;
        assert_eq!(
            (aggression.mode, *disposition, aggression.grudge),
            (
                AggressionMode::Hostile,
                ActorDisposition::Hostile,
                Some(Grudge::Faction(ActorFaction::Player)),
            ),
            "frame {frame}: the save carries `{flag}` and the rebuilt {id} is not the \
             person it records (mode, disposition, grudge)",
        );
        // The binding is what a rewind resolves the provoked mind from, so a
        // body built provoked must record it as a live provocation does.
        let binding = binding.unwrap_or_else(|| {
            panic!("{id} names a catalog character, so it carries a brain binding")
        });
        {
            assert!(
                binding.is_provoked()
                    || matches!(
                        binding.source,
                        ambition_platformer2d::characters::actor::character_catalog::AutonomousSource::ProvokedProfile { .. }
                    ),
                "frame {frame}: {id} is built hostile but its brain binding says {:?}, \
                 so a rewind would restore the peaceful mind",
                binding.source
            );
        }
    }
    assert!(
        rebuilt_frames > 0,
        "the replay never rebuilt {id}, so nothing about construction was checked"
    );
}

/// ⛔ A `<<challenge>>` IS A PROVOCATION THE REPLAY MUST REMEMBER.
///
/// Construction builds a person from the save's provocation fact, and only the
/// damage road used to write it: a challenged NPC turned hostile live and came
/// back PEACEFUL after a room replay. Driven through `ChallengeRequested`, the
/// message the `<<challenge>>` command sends, naming the playing body as
/// challenger — so the faction gate is answered by the real player body.
#[test]
fn a_challenged_npc_is_still_hostile_after_a_room_replay() {
    use ambition_platformer2d::combat::components::ActorDisposition;
    use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
    use ambition_platformer2d::platformer::sim_id::SimId;

    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    let (npc, id, mode) = talkable_actors(&mut sim)
        .first()
        .cloned()
        .expect("the start room authors at least one talkable NPC to challenge");
    assert_ne!(mode, AggressionMode::Hostile, "{id} is hostile before the challenge");
    let flag = ambition_platformer2d::actors::fate_flags::npc_flag_id(&id);
    let npc_sim = sim
        .world()
        .get::<SimId>(npc)
        .cloned()
        .expect("a placed NPC carries a SimId");
    let player_sim = {
        let mut query = sim.world_mut().query_filtered::<&SimId, PrimaryPlayerOnly>();
        query
            .single(sim.world())
            .cloned()
            .expect("the shipped app plays exactly one primary body")
    };
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::features::ChallengeRequested {
            target: npc_sim,
            challenger: Some(player_sim),
        });
    // The challenge is ARMED and fires after its grace in `Playing`.
    sim.step_n(base(), 60 * 3);
    assert_eq!(
        sim.world().get::<ActorDisposition>(npc).copied(),
        Some(ActorDisposition::Hostile),
        "premise: the challenge never flipped {id} live, so the replay says nothing"
    );
    assert!(
        sim.world()
            .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>()
            .data()
            .flag(&flag),
        "{id} was challenged and turned hostile, and the save does not carry `{flag}`"
    );

    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    let mut rebuilt = None;
    for _ in 0..60 {
        sim.step(base());
        rebuilt = talkable_actors(&mut sim)
            .into_iter()
            .find(|(entity, other, _)| *other == id && *entity != npc);
        if rebuilt.is_some() {
            break;
        }
    }
    let (_, _, rebuilt_mode) = rebuilt.expect("the replay never rebuilt the challenged NPC");
    assert_eq!(
        rebuilt_mode,
        AggressionMode::Hostile,
        "{id} was challenged, and the room replay rebuilt it peaceful"
    );
}

/// A persisted death is BUILT: the room a replay rebuilds carries the
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
        ambition_platformer2d::actors::fate_flags::ENEMY_DEAD_UNTIL_REST_SUFFIX
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
