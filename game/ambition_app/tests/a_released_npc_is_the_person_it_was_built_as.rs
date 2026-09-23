#![cfg(feature = "rl_sim")]
//! A released NPC is the person it was built as.
//!
//! Provocation changes who is DECIDING — the brain policy and its read-model —
//! and nothing else about the body. So `<<restore_brain>>` has exactly that to
//! undo, and every other fact about the body must come back as construction
//! built it, not as a re-derivation of what an NPC is in general.

use crate::common::{base, fixed_60hz_sim};
use ambition_platformer2d::combat::components::{ActorDisposition, ActorIdentity, ActorInteraction};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::*;

fn body_facts(
    sim: &mut ambition_app::Platformer2dSimHarness,
    npc: Entity,
) -> (
    ambition_platformer2d::combat::actor_tuning::ActorTuning,
    ambition_platformer2d::combat::components::CombatCapabilities,
    ActorDisposition,
    ambition_platformer2d::characters::brain::BrainProfile,
) {
    let world = sim.world();
    let config = world
        .get::<ambition_platformer2d::actor::ActorConfig>(npc)
        .expect("an NPC carries its config");
    (
        config.tuning.clone(),
        world
            .get::<ambition_platformer2d::combat::components::CombatCapabilities>(npc)
            .cloned()
            .unwrap_or_default(),
        *world.get::<ActorDisposition>(npc).expect("an NPC carries a disposition"),
        config.brain_profile,
    )
}

#[test]
fn a_challenged_then_released_npc_keeps_the_body_it_was_built_with() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);

    let npcs: Vec<(Entity, String)> = {
        let mut q = sim
            .world_mut()
            .query_filtered::<(Entity, &ActorIdentity), With<ActorInteraction>>();
        q.iter(sim.world()).map(|(e, id)| (e, id.id.clone())).collect()
    };
    assert!(!npcs.is_empty(), "the start room authors no talkable NPC to provoke");
    let player_sim = {
        let mut q = sim.world_mut().query_filtered::<&SimId, PrimaryPlayerOnly>();
        q.single(sim.world()).cloned().expect("one primary body")
    };

    let mut checked = 0;
    for (npc, id) in npcs {
        let Some(npc_sim) = sim.world().get::<SimId>(npc).cloned() else {
            continue;
        };
        let (tuning, caps, disposition, profile) = body_facts(&mut sim, npc);
        if disposition != ActorDisposition::Peaceful {
            continue;
        }
        sim.world_mut()
            .write_message(ambition_platformer2d::actors::features::ChallengeRequested {
                target: npc_sim.clone(),
                challenger: Some(player_sim.clone()),
            });
        sim.step_n(base(), 60 * 3);
        assert_eq!(
            body_facts(&mut sim, npc).2,
            ActorDisposition::Hostile,
            "premise: the challenge never turned {id}, so the release undoes nothing"
        );
        sim.world_mut()
            .write_message(ambition_platformer2d::actors::features::ReleaseProvocation::new(npc_sim));
        sim.step_n(base(), 5);
        let (after_tuning, after_caps, after_disposition, after_profile) = body_facts(&mut sim, npc);
        assert_eq!(after_disposition, ActorDisposition::Peaceful, "premise: {id} was not released");
        assert_eq!(
            after_tuning, tuning,
            "{id}: releasing the provocation rewrote the tuning it was built with"
        );
        assert_eq!(
            after_caps, caps,
            "{id}: releasing the provocation rewrote the capabilities it was built with"
        );
        assert_eq!(
            after_profile, profile,
            "{id}: releasing the provocation did not restore the policy it was built with"
        );
        checked += 1;
    }
    assert!(checked > 0, "no peaceful NPC with a SimId was found to provoke and release");
}
