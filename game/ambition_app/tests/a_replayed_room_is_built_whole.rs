#![cfg(feature = "rl_sim")]
//! A replayed room's bodies are whole on the tick they are rebuilt.
//!
//! A replay rebuilds the room inside the transition commit, which runs late in
//! the tick, after the persona derive. So a body that construction left for the
//! derive to finish spends a whole tick unfinished on this road, while on a room
//! load the same derive happens to run after the spawn and hides it. Measured in
//! the Hall of Characters on 2026-09-25: the four characters placed without
//! authored locomotion (`mary_o`, `mary_o_tall`, `sanic`, `super_sanic`) were
//! rebuilt at the unauthored 4 hit points with an empty kit, and became the
//! 1-hit fighters the room load had built one tick later.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::characters::brain::action_set::IdentityKit;
use ambition_platformer2d::combat::moveset::ActorMoveset;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;
use std::collections::BTreeMap;

/// What the persona derive would write, per worn body, keyed by stable id.
fn persona_facts(world: &mut World) -> BTreeMap<String, (Entity, String, String)> {
    let mut bodies = world.query::<(
        Entity,
        &SimId,
        &WornCharacter,
        &BodyHealth,
        &IdentityKit,
        &ActorMoveset,
    )>();
    bodies
        .iter(world)
        .map(|(entity, sim_id, worn, health, kit, moveset)| {
            (
                sim_id.as_str().to_string(),
                (
                    entity,
                    worn.id().to_string(),
                    format!("{:?} {kit:?} {moveset:?}", health.health.max),
                ),
            )
        })
        .collect()
}

#[test]
fn a_replayed_room_is_rebuilt_with_the_persona_the_room_load_built() {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    sim.step_n(base(), 10);
    let loaded = persona_facts(sim.world_mut());
    assert!(
        loaded.values().any(|(_, worn, _)| worn == "sanic"),
        "the Hall must still place `sanic`, a character with vitals and no \
         locomotion, or this test is about a different population"
    );

    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    // The first tick on which the room's bodies are new entities.
    let rebuilt = (0..12)
        .find_map(|_| {
            sim.step(base());
            let now = persona_facts(sim.world_mut());
            let replaced = now
                .iter()
                .any(|(id, (entity, ..))| loaded.get(id).is_some_and(|(old, ..)| old != entity));
            replaced.then_some(now)
        })
        .expect("the replay rebuilt the room within 12 ticks");
    sim.step_n(base(), 3);
    let settled = persona_facts(sim.world_mut());

    let mut compared = 0;
    for (id, (entity, worn, facts)) in &rebuilt {
        let Some((old_entity, _, loaded_facts)) = loaded.get(id) else {
            continue;
        };
        if old_entity == entity {
            continue;
        }
        compared += 1;
        assert_eq!(
            facts, loaded_facts,
            "{id} wearing `{worn}` was rebuilt with different persona facts than \
             the room load built (health max, identity kit, moveset)"
        );
        assert_eq!(
            Some(facts),
            settled.get(id).map(|(_, _, facts)| facts),
            "{id} wearing `{worn}` changed persona facts after it was rebuilt: \
             construction left them for a later tick"
        );
    }
    assert!(
        compared > 100,
        "the Hall's worn bodies must be rebuilt by the replay ({compared} compared)"
    );
}
