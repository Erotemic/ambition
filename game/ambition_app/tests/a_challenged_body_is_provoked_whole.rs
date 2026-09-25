#![cfg(feature = "rl_sim")]
//! A challenged body is provoked whole, or not at all.
//!
//! Whether a body can be provoked has one answer: its character's resolved
//! provoked policy (its own, else its provider's declaration). Measured in the
//! Hall of Characters on 2026-09-25, two roads disagreed with it:
//!
//! - 89 of the Hall's 129 bodies wore a character with a catalog row and no
//!   authored definition. Nothing prepared those rows, so their provider's
//!   declarations never reached them, and a challenge left all 89 running
//!   `stand_still`.
//! - The aggression flip happened before the mind was chosen, so those 89, and
//!   the six whose providers declare no provoked policy, turned Hostile in
//!   aggression and standing while keeping their peaceful mind.
//!
//! And the mind a provocation lowers is lowered from the policy it installs
//! (AP32). The builders read the chase speed and the engagement off the
//! construction record, which provocation does not rewrite: 88 provoked Smash
//! drivers chased at the peaceful road's stored 60 where their policy says 270,
//! and two MeleeBrute bodies turned Hostile with an aggressiveness of zero.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::actor::{ActorConfig, ActorPolicy};
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::combat::components::ActorDisposition;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

#[test]
fn every_hall_body_is_provoked_into_its_policy_or_stays_peaceful() {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    sim.step_n(base(), 90);

    let catalog_only: Vec<String> = {
        let world = sim.world();
        let registry = world.resource::<ambition_platformer2d::character::PreparedCharacterRegistry>();
        world
            .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
            .iter()
            .map(|(id, _)| id.clone())
            .filter(|id| registry.get(id).is_none())
            .collect()
    };
    assert!(
        catalog_only.is_empty(),
        "catalog rows with no prepared character: {catalog_only:?}. The barrier \
         prepares every row, so its provider's declarations reach it"
    );

    // (body, character, the provoked policy its character resolved)
    let bodies: Vec<(Entity, SimId, String, Option<ambition_platformer2d::characters::brain::BrainProfile>)> = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &WornCharacter, &ActorDisposition)>();
        let peaceful: Vec<_> = q
            .iter(world)
            .filter(|(.., disposition)| **disposition == ActorDisposition::Peaceful)
            .map(|(entity, sim_id, worn, _)| (entity, sim_id.clone(), worn.id().to_string()))
            .collect();
        let registry = world.resource::<ambition_platformer2d::character::PreparedCharacterRegistry>();
        peaceful
            .into_iter()
            .map(|(entity, sim_id, worn)| {
                let policy = registry.get(&worn).and_then(|prepared| prepared.provoked_profile);
                (entity, sim_id, worn, policy)
            })
            .collect()
    };
    let provokable = bodies.iter().filter(|(.., policy)| policy.is_some()).count();
    assert!(
        provokable > 100 && provokable < bodies.len(),
        "the Hall must stage both kinds of body, or one arm below is vacuous \
         ({provokable} provokable of {} peaceful)",
        bodies.len()
    );

    let player = {
        let mut q = sim.world_mut().query_filtered::<&SimId, PrimaryPlayerOnly>();
        q.single(sim.world()).cloned().expect("one primary body")
    };
    for (_, target, ..) in &bodies {
        sim.world_mut()
            .write_message(ambition_platformer2d::actors::features::ChallengeRequested {
                target: target.clone(),
                challenger: Some(player.clone()),
            });
    }
    // Past the challenge grace (2s), with room for the flip to land.
    sim.step_n(base(), 60 * 3);

    for (entity, _, worn, policy) in &bodies {
        let world = sim.world();
        let disposition = world.get::<ActorDisposition>(*entity).copied();
        match policy {
            Some(policy) => {
                assert_eq!(
                    disposition,
                    Some(ActorDisposition::Hostile),
                    "`{worn}` resolves a provoked policy and a challenge did not provoke it"
                );
                assert_eq!(
                    world.get::<ActorPolicy>(*entity).map(|live| live.0),
                    Some(*policy),
                    "`{worn}` turned hostile without the provoked policy its character resolved"
                );
                let Some(Brain::StateMachine(mind)) = world.get::<Brain>(*entity) else {
                    panic!("`{worn}` was provoked into no state-machine mind");
                };
                assert!(
                    mind.is_hostile(),
                    "`{worn}` turned hostile with a peaceful mind: the builder read the \
                     construction record's engagement instead of the one provocation installs"
                );
                if let StateMachineCfg::Smash { cfg, .. } = mind {
                    let top = world
                        .get::<ActorConfig>(*entity)
                        .expect("a provoked body is a built actor")
                        .tuning
                        .max_run_speed;
                    assert_eq!(
                        cfg.chase_speed,
                        policy.chase_speed(top),
                        "`{worn}` chases at a speed its provoked policy did not choose"
                    );
                }
            }
            None => assert_eq!(
                disposition,
                Some(ActorDisposition::Peaceful),
                "`{worn}` resolves no provoked policy, so nothing may provoke it; a \
                 hostile standing here runs its peaceful mind"
            ),
        }
    }
}
