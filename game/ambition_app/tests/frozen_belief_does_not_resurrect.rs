//! A brain that stops perceiving forgets, and a brain that resumes does not
//! inherit what the old one believed — in the real sim, through the real
//! command road.
//!
//! The invariant landed as a pure helper with three poison-verified arms
//! (`enforce_empty_belief_for_none`, review P2 #2, 2026-09-02); the reviewer
//! also asked for the behaviour end to end: observe a hostile → switch the
//! brain to one that does not perceive → move the hostile away → switch back
//! → the OLD hostile must not come back as a belief. Driving `tick_actor_brains`
//! alone was priced at a 145-line parameter list and declined; the RL sim
//! harness ticks the whole kernel for free, so the fixture is a spawn, a brain
//! swap and a teleport.

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::actors::features::ecs::perception::PerceptionMemory;
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::World;

const ENEMY_ID: &str = "believer";

fn player_pos(world: &mut World) -> ambition_platformer2d::engine_core::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

/// The believer's brain label and what it believes about hostiles.
fn believer(world: &mut World) -> (&'static str, bool, Option<String>) {
    let mut q = world.query::<(
        &FeatureId,
        &ambition_platformer2d::characters::brain::Brain,
        &PerceptionMemory,
    )>();
    let (_, brain, memory) = q
        .iter(world)
        .find(|(feature, _, _)| feature.as_str() == ENEMY_ID)
        .expect("the believer is alive with a perception memory");
    (
        brain.label(),
        memory.0.is_empty(),
        memory
            .0
            .last_known_hostile()
            .map(|actor| format!("{actor:?}")),
    )
}

/// Replace the believer's brain, returning the one it had.
fn swap_brain(
    world: &mut World,
    brain: ambition_platformer2d::characters::brain::Brain,
) -> ambition_platformer2d::characters::brain::Brain {
    let mut q = world.query::<(
        &FeatureId,
        &mut ambition_platformer2d::characters::brain::Brain,
    )>();
    let (_, mut current) = q
        .iter_mut(world)
        .find(|(feature, _)| feature.as_str() == ENEMY_ID)
        .expect("the believer has a brain");
    std::mem::replace(&mut *current, brain)
}

#[test]
fn a_brain_switched_away_and_back_does_not_resurrect_the_hostile_it_saw() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let p = player_pos(sim.world_mut());
    // A PERCEIVING brain: the fighter profile builds a world view and remembers
    // hostiles; a state-machine preset never would.
    sim.spawn_enemy_character_at(
        ENEMY_ID,
        "Perfect Cellular Automaton",
        (p.x + 60.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    for _ in 0..90 {
        sim.step(AgentAction::default());
    }
    let (label, empty, hostile) = believer(sim.world_mut());
    assert!(
        !empty && hostile.is_some(),
        "premise: after 90 frames within 60 px the fighter ({label}) remembers the player as a \
         hostile (memory empty={empty}, last hostile={hostile:?})"
    );

    // Switch to a brain that does not perceive. The invariant: a `None`
    // perception need EMPTIES the memory, so nothing ages in the dark.
    //
    // ⛔ SWAPPED ON THE COMPONENT, NOT THROUGH `BrainCommand`: a body the harness
    // spawns by `CharacterBrain::Custom` carries no `BrainBinding`, and
    // `apply_brain_commands` requires one, so a command to it is silently
    // skipped (found writing this test — the premise assertion caught it). The
    // invariant lives in `tick_actor_brains` and does not care which road
    // changed the brain; swapping the component is the same fact.
    let perceiving = swap_brain(
        sim.world_mut(),
        ambition_platformer2d::characters::brain::Brain::StateMachine(
            ambition_platformer2d::characters::brain::StateMachineCfg::StandStill,
        ),
    );
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    let (label, empty, hostile) = believer(sim.world_mut());
    assert_eq!(label, "stand_still", "premise: the swap took");
    assert!(
        empty,
        "a brain that stopped perceiving kept its belief: last hostile {hostile:?}"
    );

    // Move the hostile out of every perception radius while the brain is blind,
    // then give the fighter its perceiving brain back.
    sim.teleport_player((p.x + 4000.0, p.y));
    for _ in 0..5 {
        sim.step(AgentAction::default());
    }
    swap_brain(sim.world_mut(), perceiving);
    for _ in 0..30 {
        sim.step(AgentAction::default());
    }
    let (label, empty, hostile) = believer(sim.world_mut());
    assert_ne!(label, "stand_still", "premise: the default brain came back");
    assert!(
        empty,
        "the restored brain ({label}) resurrected a hostile the old brain saw before it went blind — \
         the player is 4000 px away and nothing can have perceived it since: {hostile:?}"
    );
}
