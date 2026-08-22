//! Integration test that the authored fighter difficulty ladder reaches the
//! runtime fighter brain.
//!
//! Level 5 is used because it differs from the engine floor on every asserted
//! axis, avoiding endpoint values that could pass without the authored ladder.
//! The full app composition is required because it brings the authored ladder and
//! Smash fighter consumers together.

use ambition_platformer2d::characters::actor::character_catalog::{brain_from_preset, BrainPreset};
use ambition_platformer2d::characters::brain::fighter::{
    AuthoredFighterLadder, FighterBrainProfile,
};
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};

/// The rung the smash roster's `duelist` actually ships on, and the one where the
/// ladder and the floor disagree on every axis.
const LEVEL: u8 = 5;

fn profile_of(brain: &Brain) -> FighterBrainProfile {
    match brain {
        Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) => cfg.profile,
        other => panic!("the fighter preset built a {other:?}, not a fighter brain"),
    }
}

#[test]
fn a_fighter_in_the_host_runs_on_the_authored_rung_not_the_engine_floor() {
    let mut sim = crate::common::fixed_60hz_sim();
    // One step so the app finishes startup and the content plugin's resources
    // are in the world.
    sim.step(crate::common::base());

    let floor = FighterBrainProfile::for_level(LEVEL);
    let authored = {
        let world = sim.world_mut();
        let ladder = world.get_resource::<AuthoredFighterLadder>().expect(
            "ambition_content inserts AuthoredFighterLadder from the lowered pack; \
             without it every fighter silently runs on the engine floor",
        );
        *ladder
            .0
            .level(LEVEL)
            .expect("the authored ladder has a row for every level 1..=9")
    };

    // ANTI-VACUITY, and it is not ceremony. If someone retunes the ladder
    // so this rung matches the floor, every assertion below passes without any
    // of the wiring existing. That is exactly how the level-9 version of this
    // test would have been born green.
    assert_ne!(
        authored, floor,
        "level {LEVEL}'s authored rung is identical to the engine floor, so this \
         test can no longer tell whether the ladder is wired at all — move it to \
         a rung that differs, or the probe is decoration"
    );

    // The REAL construction path: this is the function the character catalog's
    // resolver calls for `brain_template: Fighter`, not a hand-rolled equivalent.
    let brain = brain_from_preset(
        &BrainPreset::Fighter {
            level: LEVEL,
            decision_interval_ticks: 6,
        },
        0.0,
    );
    assert_eq!(
        profile_of(&brain),
        floor,
        "the builder is supposed to start at the floor and be corrected by the \
         projection; if it already reads the ladder, the assertion below stops \
         proving that the projection runs"
    );

    let entity = sim.world_mut().spawn(brain).id();
    // `Added<Brain>` is live for exactly this step.
    sim.step(crate::common::base());

    let got = profile_of(
        sim.world_mut()
            .get::<Brain>(entity)
            .expect("the spawned brain survives a step"),
    );

    assert_eq!(
        got, authored,
        "a freshly-built fighter still carries the engine floor after a full \
         step of the real host app. The ladder is loaded but not APPLIED: check \
         that `project_authored_fighter_ladder` is registered in Update and \
         chained before `tick_actor_brains`"
    );
}
