//! Project the authored fighter difficulty rung onto fighter brains.
//!
//! A projection over the fighter brains in the world — actor-kernel policy, not
//! construction. The spawn capability builds the brain; this decides which rung
//! a brain in the world fights at.

use ambition_characters::brain::{Brain, StateMachineCfg};

/// Apply each live room's authored difficulty ladder to its fighters.
///
/// Rooms without a ladder keep their constructed fighter profile. Update
/// `FighterState` only when a rung differs, preserving the noise-stream
/// position. Do not filter on `Added<Brain>`: hidden candidates can become
/// live after their added tick is no longer visible to this system.
///
/// Keep this projection in the simulation schedule. Running it between
/// content-generation commit and candidate adoption could apply a new
/// ladder to fighters in the outgoing session.
pub fn project_authored_fighter_ladder(
    ladders: crate::session::governing_rules::RulesOf<ambition_characters::brain::fighter::FighterBrainLadder>,
    mut brains: bevy::prelude::Query<(bevy::prelude::Entity, &mut Brain)>,
) {
    // One resolution for each room, not one for each fighter: a resolution
    // clones the ladder's rows.
    let mut by_room: Vec<(
        Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
        Option<ambition_characters::brain::fighter::FighterBrainLadder>,
    )> = Vec::new();
    for (entity, mut brain) in &mut brains {
        let wanted = {
            let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &*brain else {
                continue;
            };
            let room = ladders.room_of(entity);
            let index = match by_room.iter().position(|(seen, _)| *seen == room) {
                Some(index) => index,
                None => {
                    by_room.push((room, ladders.in_live_room(room)));
                    by_room.len() - 1
                }
            };
            // No ladder governs this room: the engine floor is the answer,
            // which is the rule `profile_for_level` states.
            let Some(ladder) = &by_room[index].1 else {
                continue;
            };
            let Some(rung) = ladder.level(cfg.profile.level) else {
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
    use ambition_characters::brain::fighter::FighterBrainLadder;
    use ambition_combat::scoped_rules::{DeclareRulesExt as _, RulesScope};
    use bevy::prelude::*;

    /// Two rungs that differ from the engine floor in the way the SHIPPED ladder
    /// does: a lower `apm_cap`, and — the one that matters — weights a beginner
    /// does not have.
    const LADDER: &str = "[
        (level: 1, reaction_ms: 500.0, apm_cap: 60.0, execution_noise: 0.40,
         rollout_depth: 0, rollout_k: 0,
         utility_weights: (reach_fit: 1.0, frame_advantage: 0.10, kill_potential: 0.00, stage_risk: -0.10, expected_payoff: 0.00)),
        (level: 2, reaction_ms: 450.0, apm_cap: 90.0, execution_noise: 0.35,
         rollout_depth: 0, rollout_k: 0,
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
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
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

    /// ⭐ **EACH FIGHTER READS THE LADDER OF ITS OWN ROOM.** One binary hosts
    /// several games (Ambition hosts Smash), and the ladder is the game's
    /// rule (Q88). As one App-global resource, the second installer would
    /// have won in every room. Here the hall (no mode) declares `LADDER` and
    /// the Smash stage declares a ladder whose level 1 caps at 30 actions; a
    /// fighter in each room takes its own room's row, and a fighter in no live
    /// room keeps the floor.
    #[test]
    fn each_fighter_takes_the_rung_of_its_own_rooms_ladder() {
        use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
        let mut app = App::new();
        app.declare_rules(
            RulesScope::UntaggedRooms,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
        app.declare_rules(
            RulesScope::Mode("smash"),
            FighterBrainLadder::from_ron(&LADDER.replace("apm_cap: 60.0", "apm_cap: 30.0"))
                .expect("the stage's ladder parses"),
        );
        let hall = crate::session::governing_rules::tests::two_game_session(&mut app, true);
        app.add_systems(Update, project_authored_fighter_ladder);
        let in_hall = app.world_mut().spawn((fighter_brain(1), InRoomInstance(hall))).id();
        let on_stage = app.world_mut().spawn((fighter_brain(1), InRoomInstance(hall.next()))).id();
        let nowhere = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();

        let apm = |entity| profile_of(app.world().get::<Brain>(entity).expect("brain")).apm_cap;
        assert_eq!(apm(in_hall), 60.0, "the fighter in the hall did not take the hall's ladder");
        assert_eq!(apm(on_stage), 30.0, "the fighter on the Smash stage did not take Smash's ladder");
        assert_eq!(
            profile_of(app.world().get::<Brain>(nowhere).expect("brain")),
            ambition_characters::brain::fighter::FighterBrainProfile::for_level(1),
            "a fighter in no live room took a ladder that governs no room it is in"
        );
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
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
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

    /// ⛔⛔ **THE PROJECTION IS A CONTINUOUS AUTHORITY OVER `cfg.profile`, AND A
    /// SECOND WRITER LOSES WITHIN ONE TICK WITHOUT BEING TOLD.**
    ///
    /// That is a consequence of the filter being GONE rather than widened — see
    /// the note on the system — and it is correct: a candidate session's brains
    /// must reach the authored rung however long they sat behind a disabling
    /// component. But it makes any other writer of a live fighter's profile a
    /// loser, silently, every tick, and nothing in the type system says so.
    ///
    /// ⚠ MEASURED COST, not a hypothesis. `ladder-rig`'s `--weight`, `--apm`,
    /// `--noise` and `--reaction-ms` all wrote live profiles, so on the shipped
    /// ladder — the road added specifically so the rig could measure the
    /// SHIPPED fighter — `--apm 1` and `--apm 600` produced byte-identical
    /// bouts, as did `reach_fit=0` against `reach_fit=999`. The rig now writes
    /// the ladder ROWS instead, which is the owner. ⇒ This test is what a
    /// future second writer reads before making the same mistake.
    #[test]
    fn a_profile_written_from_outside_is_reverted_on_the_very_next_tick() {
        let mut app = App::new();
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();
        let projected = profile_of(app.world().get::<Brain>(entity).expect("brain"));
        assert_eq!(projected.apm_cap, 60.0, "premise: rung 1 was projected");

        // A second writer, long after the brain stopped being newly added.
        for _ in 0..5 {
            app.update();
        }
        let mut live = app.world_mut().get_mut::<Brain>(entity);
        let Some(Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. })) = live.as_deref_mut()
        else {
            panic!("not a fighter brain")
        };
        cfg.profile.apm_cap = 999.0;
        drop(live);
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")).apm_cap,
            999.0,
            "premise: the outside write landed at all"
        );

        app.update();
        assert_eq!(
            profile_of(app.world().get::<Brain>(entity).expect("brain")).apm_cap,
            60.0,
            "an outside write to a live fighter's profile survived a tick — if \
             that is now true, `ladder-rig` and anything else routed through \
             `FighterBrainLadder::rungs_mut` to avoid this can go back to \
             poking brains, and this test should say so"
        );
    }

    /// ⛔⛔ **AND THE REVERT IS NOT A NO-OP: IT REBUILDS `FighterState`.**
    ///
    /// This is what makes losing expensive rather than merely futile. The
    /// projection's rewrite ends in `FighterState::new(cfg, stream)`, so a
    /// second writer that keeps trying does not just fail — it wipes that
    /// fighter's pending press, reaction ledger and habits on every tick it
    /// tries, which is a fighter that can never finish a thought. A reader who
    /// knows only "my write gets reverted" would expect a harmless no-op.
    #[test]
    fn the_revert_rebuilds_the_state_so_a_losing_writer_wipes_the_fighter_each_tick() {
        let mut app = App::new();
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
        app.add_systems(Update, project_authored_fighter_ladder);
        let entity = app.world_mut().spawn(fighter_brain(1)).id();
        app.update();

        // Something the brain is in the middle of, and a profile write beside it.
        let mut live = app.world_mut().get_mut::<Brain>(entity);
        let Some(Brain::StateMachine(StateMachineCfg::Fighter { cfg, state })) = live.as_deref_mut()
        else {
            panic!("not a fighter brain")
        };
        state.ticks_until_decision = 7;
        cfg.profile.apm_cap = 999.0;
        drop(live);

        app.update();

        let Some(Brain::StateMachine(StateMachineCfg::Fighter { state, .. })) =
            app.world().get::<Brain>(entity)
        else {
            panic!("not a fighter brain")
        };
        assert_ne!(
            state.ticks_until_decision, 7,
            "the profile was reverted without rebuilding the state — which would \
             make a losing writer harmless, and is a better world than the one \
             this test was written against"
        );
    }

    /// **THE PROJECTION MUST NOT RE-CHOOSE THE COGNITIVE STREAM.**
    ///
    /// This is the second half of the same-character CPU symmetry defect, and it is the half that
    /// would have silently undone the first.
    #[test]
    fn the_projection_carries_the_stream_it_was_handed() {
        let mut app = App::new();
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
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
    /// the authored ladder never sees them and the fighter keeps
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
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
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
        app.declare_rules(
            RulesScope::EveryRoom,
            FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses"),
        );
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
