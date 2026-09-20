//! Project the authored fighter difficulty rung onto fighter brains.
//!
//! A projection over the fighter brains in the world — actor-kernel policy, not
//! construction. The spawn capability builds the brain; this decides which rung
//! a brain in the world fights at.

use ambition_characters::brain::{Brain, StateMachineCfg};

/// Project the game's authored fighter difficulty rung into fighter brains.
///
/// Rebuild `FighterState` so profile-cached perception and habit fields match the
/// authored rung, while preserving the fighter's existing noise-stream position.
/// The projection is idempotent and only rewrites when the rung differs.
///
/// ⛔⛤ **IT USED TO FILTER ON `Added<Brain>`, AND THAT DOES NOT COMPOSE WITH A
/// DISABLING COMPONENT.** A candidate session builds its whole population
/// behind `InactiveCandidate`, which ordinary queries cannot see, while the
/// LIVE session keeps running this system every frame. Change detection
/// compares a component's added tick against the SYSTEM'S last run, so by the
/// time a candidate is adopted its brains are no longer newly added — and this
/// is the only production consumer of `AuthoredFighterLadder`, so such a
/// fighter kept `FighterBrainProfile::for_level`, the ENGINE FLOOR, in a game
/// that authored a ladder. Silent: a floor profile is a valid profile.
///
/// ⚠ THE FILTER IS GONE RATHER THAN WIDENED, because there is no tick-based
/// filter that a disabling component cannot step past. The pass is idempotent
/// and writes only when the rung differs, so the cost is one profile
/// comparison per fighter per tick.
///
/// ⛔ AND THE READ IS IMMUTABLE ON PURPOSE. `&mut *brain` on every fighter
/// every tick would mark `Brain` changed for every other change-detection
/// reader in the schedule, which is a busier thing than the projection it
/// would be reporting. The mutable borrow is taken only by the fighter that is
/// actually being rewritten.
///
/// ⛔⛤ **IT MUST STAY IN THE SIMULATION SCHEDULE, AND THAT IS A TRANSACTION
/// CONSTRAINT RATHER THAN A PERFORMANCE ONE.** A 2026-09-19 review raised it:
/// `AuthoredFighterLadder` is an App-global resource that
/// `commit_content_generation` republishes at generation N+1 partway through
/// `Update` — `.after(AmbitionGameShellSet::Pending)`, `.before(GameplaySessionSet::Providers)`
/// — and the candidate that owns N+1 is adopted at the far end of that span.
/// A projection running inside the span would rewrite the LIVE session's
/// fighters to a generation that is not yet authoritative, which is exactly
/// the split the reload architecture exists to prevent.
///
/// ⇒ It cannot, because this host advances the simulation from `PreUpdate`
/// (`RunGgrsSystems`) and the whole commit-to-adoption span is inside one
/// `Update` pass. **Moving this system into `Update` opens the hole**, and so
/// does moving the advance out of `PreUpdate`. Held by
/// `reload_publication_is_installed::no_simulation_tick_falls_between_the_generation_commit_and_its_adoption`,
/// poison-verified by registering this system in `Update` as well.
pub fn project_authored_fighter_ladder(
    ladder: Option<bevy::prelude::Res<ambition_characters::brain::fighter::AuthoredFighterLadder>>,
    mut brains: bevy::prelude::Query<&mut Brain>,
) {
    let Some(ladder) = ladder else {
        // No ladder shipped: the engine floor is the answer, which is the rule
        // `profile_for_level` states.
        return;
    };
    for mut brain in &mut brains {
        let wanted = {
            let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &*brain else {
                continue;
            };
            let Some(rung) = ladder.0.level(cfg.profile.level) else {
                continue;
            };
            if cfg.profile == *rung {
                continue;
            }
            *rung
        };
        let Brain::StateMachine(StateMachineCfg::Fighter { cfg, state }) = &mut *brain else {
            unreachable!("the immutable read above matched the fighter arm")
        };
        cfg.profile = wanted;
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

    /// **A FIGHTER BUILT INSIDE A HIDDEN CANDIDATE STILL TAKES THE RUNG.**
    ///
    /// ⛔⛤ **`Added<Brain>` AND A DISABLING COMPONENT DO NOT COMPOSE, AND THE
    /// FAILURE IS SILENT.** A candidate session builds its whole world behind
    /// `InactiveCandidate`, which is a real Bevy disabling component: ordinary
    /// queries cannot see those entities. This projection's filter is change
    /// detection, which compares the component's added tick against the
    /// SYSTEM'S last-run tick — and the system keeps running, on the live
    /// session, for every frame the candidate stays hidden. By the time the
    /// candidate is adopted its brains are no longer "added" relative to a
    /// system that has run since, so the one production consumer of
    /// `AuthoredFighterLadder` never sees them and the fighter keeps
    /// `FighterBrainProfile::for_level`, the ENGINE FLOOR, in a game that
    /// authored a ladder.
    ///
    /// ⚠ **LATENT ON THE SHIPPED SMASH DUEL AND THAT IS NOT A DEFENCE.**
    /// Measured 2026-09-19: the projection fires exactly twice in
    /// `two_cpus_in_the_shipped_composition_damage_each_other`, once per seat —
    /// smash seats are built into the live session after activation, so they
    /// are visible when their brains appear. The hole is in the road, not in
    /// that route, and the next fighter authored into candidate-built content
    /// falls through it with no symptom to read.
    ///
    /// ⇒ The filter is gone rather than widened. The pass is idempotent (see
    /// above) and only writes when the rung differs, so running it over every
    /// fighter costs one profile comparison per brain per tick and owes nothing
    /// to a tick counter that a disabling component can step past.
    #[test]
    fn a_fighter_built_behind_a_candidate_barrier_still_takes_the_rung() {
        use ambition_platformer2d_shared_tangle::construction::{
            hide_candidate_session_root, publish_candidate_session,
            register_inactive_candidate_filter,
        };
        use ambition_platformer2d_shared_tangle::lifecycle::{
            SessionScopeId, SessionScopedEntity,
        };

        let scope = SessionScopeId(7);
        let mut app = App::new();
        register_inactive_candidate_filter(app.world_mut());
        app.insert_resource(AuthoredFighterLadder(
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        ));
        app.add_systems(Update, project_authored_fighter_ladder);

        // Built hidden, exactly as a candidate session builds its population:
        // the real hiding road, so a change in what hides a candidate reaches
        // this arm instead of leaving it asserting against a hand-made marker.
        let entity = app
            .world_mut()
            .spawn((fighter_brain(1), SessionScopedEntity(scope)))
            .id();
        bevy::ecs::system::RunSystemOnce::run_system_once(
            app.world_mut(),
            move |mut commands: Commands| {
                hide_candidate_session_root(&mut commands, entity);
            },
        )
        .expect("the hiding system runs");
        // ⛔ THE LIVE SESSION KEEPS RUNNING WHILE THE CANDIDATE IS PREPARED, and
        // that is the whole mechanism: each of these advances the system's
        // last-run tick past the brain's added tick.
        for _ in 0..3 {
            app.update();
        }
        // Adoption, through the one road that performs it.
        publish_candidate_session(app.world_mut(), entity, scope);
        app.update();

        let floor = ambition_characters::brain::fighter::FighterBrainProfile::for_level(1);
        assert_ne!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")),
            floor,
            "a fighter prepared behind a candidate barrier came out on the ENGINE \
             FLOOR: the ladder projection never saw it, because `Added<Brain>` \
             was already stale by the time the candidate became visible"
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
