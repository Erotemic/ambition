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

/// Every worn body's physical facts against its PREPARED character, not
/// against another construction: "load equals replay" cannot tell both right
/// from both wrong, and the Hall's mites were built without their death traits
/// on both roads. Returns how many bodies author death traits, so a caller can
/// prove the capability arm saw one.
fn assert_built_as_prepared(world: &mut World, when: &str) -> usize {
    let mut bodies = world.query::<(
        &SimId,
        &WornCharacter,
        &BodyHealth,
        &ambition_platformer2d::combat::components::CombatCapabilities,
    )>();
    let built: Vec<_> = bodies
        .iter(world)
        .map(|(sim_id, worn, health, caps)| {
            (sim_id.as_str().to_string(), worn.id().to_string(), health.health.max, caps.clone())
        })
        .collect();
    let registry = world.resource::<ambition_platformer2d::character::PreparedCharacterRegistry>();
    let mut with_traits = 0;
    for (id, worn, max, caps) in built {
        let Some(prepared) = registry.get(&worn) else {
            continue;
        };
        with_traits += usize::from(prepared.death_traits.is_some());
        assert_eq!(
            caps,
            prepared
                .death_traits
                .as_ref()
                .map(ambition_platformer2d::combat::components::CombatCapabilities::from)
                .unwrap_or_default(),
            "{when}: {id} wearing `{worn}` was built without its authored death traits"
        );
        assert_eq!(
            max,
            prepared.vitals.max_health.map_or(
                ambition_platformer2d::characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH,
                |max| max.max(1)
            ),
            "{when}: {id} wearing `{worn}` was built without its authored health"
        );
    }
    with_traits
}

#[test]
fn a_replayed_room_is_rebuilt_with_the_persona_the_room_load_built() {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    sim.step_n(base(), 10);
    let loaded = persona_facts(sim.world_mut());
    assert!(
        assert_built_as_prepared(sim.world_mut(), "room load") > 0,
        "the Hall must place a character that authors death traits (the mites), \
         or the capability arm is vacuous"
    );
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
    assert_built_as_prepared(sim.world_mut(), "replay rebuild tick");
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

/// What the decision pass found unattached on one tick.
#[derive(Resource, Default)]
struct UnattachedAtDecision {
    /// Every body the decision pass saw, so the caller can find the rebuilt ones.
    seen: std::collections::HashSet<Entity>,
    /// Brained bodies with no belief memory for their senses to update.
    memoryless: Vec<Entity>,
    /// Brained bodies with no orientation to right themselves with.
    rollless: Vec<Entity>,
    /// Staged actors with no dormancy stance.
    stanceless: Vec<Entity>,
}

fn record_unattached_at_decision(
    mut out: ResMut<UnattachedAtDecision>,
    brained: Query<
        (
            Entity,
            Has<ambition_platformer2d::characters::perception::PerceptionMemory>,
            Has<ambition_platformer2d::platformer::orientation::ActorRoll>,
        ),
        (
            With<ambition_platformer2d::characters::brain::Brain>,
            With<ambition_platformer2d::platformer::lifecycle::FeatureSimEntity>,
            Without<ambition_platformer2d::platformer::markers::PlayerEntity>,
            Without<ambition_platformer2d::boss_encounter::BossConfig>,
        ),
    >,
    staged: Query<
        (
            Entity,
            &ambition_platformer2d::actor::ActorFaction,
            Has<ambition_platformer2d::actors::features::ecs::dormancy::DormancyPolicy>,
        ),
        With<ambition_platformer2d::actor::BodyKinematics>,
    >,
) {
    use ambition_platformer2d::actor::ActorFaction;
    for (entity, has_memory, has_roll) in &brained {
        out.seen.insert(entity);
        if !has_memory {
            out.memoryless.push(entity);
        }
        if !has_roll {
            out.rollless.push(entity);
        }
    }
    for (entity, faction, has_stance) in &staged {
        if !matches!(faction, ActorFaction::Player | ActorFaction::Neutral) && !has_stance {
            out.stanceless.push(entity);
        }
    }
}

/// A rebuilt body is complete before anything decides for it.
///
/// The transition commit rebuilds the room after every first-sight attacher has
/// run for the tick, so each rebuilt body spends the rest of that tick without a
/// dormancy stance. That is safe only because nothing after the commit reads it,
/// and on the next tick `declare_ambition_dormancy` runs before
/// `assess_dormancy` and the brain tick.
///
/// A body's senses are derived when it decides, so there is nothing of them to
/// attach. Its belief memory is built with its brain (`Brain` requires it), and
/// its roll with its movement frame (`ResolvedMotionFrame` requires it); the
/// memory and roll arms check that the rebuilt bodies arrive with both.
#[test]
fn a_rebuilt_body_is_complete_before_anything_decides_for_it() {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    {
        let app = sim.app_mut();
        let label = app.sim_schedule();
        app.init_resource::<UnattachedAtDecision>();
        app.add_systems(
            label,
            record_unattached_at_decision
                .in_set(ambition_platformer2d::platformer::schedule::ActorDecisionSet::Observe),
        );
    }
    sim.step_n(base(), 10);
    let loaded = std::mem::take(&mut sim.world_mut().resource_mut::<UnattachedAtDecision>().seen);

    sim.world_mut().write_message(
        ambition_platformer2d::actors::session::reset::RoomReplayRequested::manual(),
    );
    sim.step_n(base(), 12);

    let record = sim.world().resource::<UnattachedAtDecision>();
    let rebuilt = record.seen.difference(&loaded).count();
    assert!(
        rebuilt > 100,
        "the replay must rebuild the Hall's brained bodies and the decision pass \
         must see them, or this is not about rebuilt bodies ({rebuilt} seen)"
    );
    assert!(
        record.memoryless.is_empty(),
        "{} brained bodies reached the decision pass with no belief memory: {:?}",
        record.memoryless.len(),
        &record.memoryless[..record.memoryless.len().min(5)]
    );
    assert!(
        record.rollless.is_empty(),
        "{} brained bodies reached the decision pass with no `ActorRoll`: {:?}",
        record.rollless.len(),
        &record.rollless[..record.rollless.len().min(5)]
    );
    assert!(
        record.stanceless.is_empty(),
        "{} staged actors reached the decision pass without a dormancy stance: {:?}",
        record.stanceless.len(),
        &record.stanceless[..record.stanceless.len().min(5)]
    );
}
