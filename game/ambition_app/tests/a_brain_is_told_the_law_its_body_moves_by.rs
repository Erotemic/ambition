#![cfg(feature = "rl_sim")]
//! What a brain is told of its body's movement is what the body does.
//!
//! A brain that thinks in speeds turns one into a throttle: the speed it
//! wants, over the top speed it is told (`BrainSnapshot::locomotion_for`). The
//! integrator turns the throttle back into a speed with the body's real top
//! speed. So the two must be one number.
//!
//! They were not for a body that authors its movement feel
//! (`AuthoredMovementTuning`). The snapshot took the config's tuning alone.
//! Measured in the hall, 2026-10-09: of 138 bodies two author their feel,
//! Mary-O and tall Mary-O, and both were told run 270, jump 520 and one air
//! jump while they move at run 300, jump 450 and no air jump. A patrol that
//! asked Mary-O for 100 px/s got 111.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::characters::brain::state_machine::{PatrolCfg, PatrolState};
use ambition_platformer2d::characters::brain::{AuthoredWorldPatrolLane, Brain, StateMachineCfg};
use ambition_platformer2d::engine_core::{AuthoredMovementTuning, BodyKinematics};
use bevy::prelude::Entity;

const ASKED: f32 = 100.0;

/// The speed a hall body walks at when a patrol brain asks it for `ASKED`.
fn walks_at(character: &str) -> (f32, bool) {
    let mut sim = fixed_60hz_room_sim("hall_of_characters");
    sim.step_n(base(), 30);
    let (body, at, authors_its_feel) = {
        let world = sim.world_mut();
        let mut query = world.query::<(Entity, &WornCharacter, &BodyKinematics, Option<&AuthoredMovementTuning>)>();
        query
            .iter(world)
            .find(|(_, worn, ..)| worn.id() == character)
            .map(|(entity, _, kinematics, authored)| (entity, kinematics.pos, authored.is_some()))
            .unwrap_or_else(|| panic!("the hall stages `{character}`"))
    };
    *sim.world_mut().get_mut::<Brain>(body).expect("a hall body has a brain") =
        Brain::StateMachine(StateMachineCfg::Patrol {
            cfg: PatrolCfg {
                // A lane wide enough that the body does not turn in the measure.
                lane: AuthoredWorldPatrolLane::new(at.x, 4000.0),
                speed: ASKED,
                ..PatrolCfg::NPC_DEFAULT
            },
            state: PatrolState::default(),
        });
    // Up to speed, then the mean over a second.
    sim.step_n(base(), 60);
    let from = sim.world().get::<BodyKinematics>(body).expect("a live body").pos.x;
    sim.step_n(base(), 60);
    let to = sim.world().get::<BodyKinematics>(body).expect("a live body").pos.x;
    ((to - from).abs(), authors_its_feel)
}

#[test]
fn a_body_that_authors_its_feel_walks_at_the_speed_its_brain_asks_for() {
    // The control: a body with no authored feel. It was right before.
    let (plain, authors) = walks_at("npc_architect");
    assert!(!authors, "control: the architect authors its feel now; choose another body");
    assert!((plain - ASKED).abs() < 4.0, "control: a plain body asked for {ASKED} px/s walked at {plain:.1}");
    // The subject.
    let (mary, authors) = walks_at("mary_o");
    assert!(authors, "premise: Mary-O authors her movement feel");
    assert!(
        (mary - ASKED).abs() < 4.0,
        "Mary-O's brain asked for {ASKED} px/s and her body walked at {mary:.1}: \
         the brain is told a top speed the body does not have"
    );
}
