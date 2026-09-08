//! Project the authored fighter difficulty rung onto newly inserted fighter brains.
//!
//! An Update-time projection over LIVE `Added<Brain>` components — actor-kernel
//! policy, not construction. The spawn capability builds the brain; this decides
//! which rung a brain in the world fights at.

use ambition_characters::brain::{Brain, StateMachineCfg};

/// Project the game's authored fighter difficulty rung into newly inserted brains.
///
/// Rebuild `FighterState` so profile-cached perception and habit fields match the
/// authored rung, while preserving the fighter's existing noise-stream position.
/// The projection is idempotent and only rewrites when the rung differs.
pub fn project_authored_fighter_ladder(
    ladder: Option<bevy::prelude::Res<ambition_characters::brain::fighter::AuthoredFighterLadder>>,
    mut brains: bevy::prelude::Query<&mut Brain, bevy::prelude::Added<Brain>>,
) {
    let Some(ladder) = ladder else {
        // No ladder shipped: the engine floor is the answer, which is the rule
        // `profile_for_level` states.
        return;
    };
    for mut brain in &mut brains {
        let Brain::StateMachine(StateMachineCfg::Fighter { cfg, state }) = &mut *brain else {
            continue;
        };
        let level = cfg.profile.level;
        let Some(rung) = ladder.0.level(level) else {
            continue;
        };
        if cfg.profile == *rung {
            continue;
        }
        cfg.profile = *rung;
        // the stream this fighter was CONSTRUCTED on, carried across the
        // rebuild. See the note above: reseeding here is what would undo
        // `fighter_cognition_seed`.
        let stream = state.noise;
        **state = ambition_characters::brain::fighter::FighterState::new(cfg, stream);
    }
}

#[cfg(test)]
mod ladder_projection_tests {
    use super::*;
    use ambition_characters::brain::fighter::{AuthoredFighterLadder, FighterBrainLadder};
    use bevy::prelude::*;

    /// Two rungs that differ from the engine floor in the way the SHIPPED ladder
    /// does: a lower `apm_cap`, and — the one that matters — weights a beginner
    /// does not have.
    const LADDER: &str = "[
        (level: 1, reaction_ms: 500.0, apm_cap: 60.0, execution_noise: 0.40,
         rollout_depth: 0, rollout_k: 0, read_weight: 0.0,
         utility_weights: (reach_fit: 1.0, frame_advantage: 0.10, kill_potential: 0.00, stage_risk: -0.10, expected_payoff: 0.00)),
        (level: 2, reaction_ms: 450.0, apm_cap: 90.0, execution_noise: 0.35,
         rollout_depth: 0, rollout_k: 0, read_weight: 0.0,
         utility_weights: (reach_fit: 1.0, frame_advantage: 0.20, kill_potential: 0.00, stage_risk: -0.20, expected_payoff: 0.00)),
    ]";

    const CONSTRUCTED_STREAM: u64 = 0xC0FF_EE00_D15E_A5E5;

    fn fighter_brain(level: u8) -> Brain {
        let cfg = ambition_characters::brain::fighter::FighterCfg::new(
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(level),
        );
        let state =
            ambition_characters::brain::fighter::FighterState::new(&cfg, CONSTRUCTED_STREAM);
        Brain::StateMachine(StateMachineCfg::Fighter {
            cfg: Box::new(cfg),
            state: Box::new(state),
        })
    }

    fn stream_of(brain: &Brain) -> u64 {
        match brain {
            Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) => state.noise,
            other => panic!("not a fighter brain: {other:?}"),
        }
    }

    fn profile_of(brain: &Brain) -> ambition_characters::brain::fighter::FighterBrainProfile {
        match brain {
            Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) => cfg.profile,
            other => panic!("not a fighter brain: {other:?}"),
        }
    }

    /// **A spawned fighter reads the game's rung.**
    ///
    /// So a level-1 CPU priced a kill move exactly as the hardest one did.
    #[test]
    fn a_spawned_fighter_takes_the_authored_rung_over_the_floor() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);

        let floor = ambition_characters::brain::fighter::FighterBrainProfile::for_level(1);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();

        let projected = profile_of(app.world().get::<Brain>(entity).expect("brain"));
        assert_ne!(
            projected, floor,
            "the spawned fighter kept the engine floor, so the authored ladder \
             reached nothing"
        );
        assert!(
            projected.utility_weights.kill_potential < floor.utility_weights.kill_potential,
            "a level-1 CPU still values a kill move as highly as the hardest rung \
             does — floor {:?}, projected {:?}",
            floor.utility_weights,
            projected.utility_weights,
        );
        assert_eq!(projected.apm_cap, 60.0, "the authored action cap");
    }

    /// **No ladder means the floor, which is the engine's stated rule.**
    #[test]
    fn without_a_ladder_the_engine_floor_stands() {
        let mut app = App::new();
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(1),
            "a game that shipped no rows had its fighter rewritten anyway"
        );
    }

    /// **idempotent**, which is what makes it safe to run on a change-detection
    /// filter that does not rewind. A second pass must land on the same value.
    #[test]
    fn projecting_twice_lands_on_the_same_brain() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(2)).id();
        app.update();
        let once = profile_of(app.world().get::<Brain>(entity).expect("brain"));
        // Force it to be seen as freshly added again.
        let brain = app.world().get::<Brain>(entity).expect("brain").clone();
        app.world_mut().entity_mut(entity).insert(brain);
        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            once,
            "a second projection moved the brain, so the pass is not idempotent"
        );
    }

    /// **THE PROJECTION MUST NOT RE-CHOOSE THE COGNITIVE STREAM.**
    ///
    /// This is the second half of the same-character CPU symmetry defect, and it is the half that
    /// would have silently undone the first.
    #[test]
    fn the_projection_carries_the_stream_it_was_handed() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();

        let brain = app.world().get::<Brain>(entity).expect("brain");
        // Non-vacuity: the pass must actually have DONE its job, or "the stream
        // survived" is only saying that nothing ran.
        assert_ne!(
            profile_of(brain),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(1),
            "the ladder did not project, so this test is not observing the rebuild \
             it exists to constrain"
        );
        assert_eq!(
            stream_of(brain),
            CONSTRUCTED_STREAM,
            "the ladder projection reseeded the fighter's noise stream, which is \
             what made every CPU on one rung think identical thoughts"
        );
    }

    /// **a level the ladder does not author keeps the floor** rather than
    /// failing — the same fallback `profile_for_level` states, so the two agree.
    #[test]
    fn an_unauthored_level_keeps_the_floor() {
        let mut app = App::new();
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(7)).id();
        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(7),
            "level 7 is not in the two-rung fixture and must keep the floor"
        );
    }
}
