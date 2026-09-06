use super::*;

/// Nine rows, monotone in every axis. Reaction falls, APM rises, noise falls.
const LADDER: &str = r#"[
    (level: 1, reaction_ms: 500.0, apm_cap: 60.0,  execution_noise: 0.40, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.1, kill_potential: 0.0, stage_risk: -0.1, expected_payoff: 0.00)),
    (level: 2, reaction_ms: 450.0, apm_cap: 90.0,  execution_noise: 0.35, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.2, kill_potential: 0.0, stage_risk: -0.2, expected_payoff: 0.00)),
    (level: 3, reaction_ms: 400.0, apm_cap: 120.0, execution_noise: 0.30, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.3, kill_potential: 0.1, stage_risk: -0.3, expected_payoff: 0.10)),
    (level: 4, reaction_ms: 350.0, apm_cap: 160.0, execution_noise: 0.25, rollout_depth: 0, rollout_k: 0, read_weight: 0.1, utility_weights: (reach_fit: 1.0, frame_advantage: 0.4, kill_potential: 0.2, stage_risk: -0.4, expected_payoff: 0.20)),
    (level: 5, reaction_ms: 300.0, apm_cap: 200.0, execution_noise: 0.20, rollout_depth: 0, rollout_k: 0, read_weight: 0.2, utility_weights: (reach_fit: 1.0, frame_advantage: 0.5, kill_potential: 0.3, stage_risk: -0.5, expected_payoff: 0.30)),
    (level: 6, reaction_ms: 260.0, apm_cap: 240.0, execution_noise: 0.16, rollout_depth: 0, rollout_k: 0, read_weight: 0.3, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.6, expected_payoff: 0.40)),
    (level: 7, reaction_ms: 220.0, apm_cap: 280.0, execution_noise: 0.12, rollout_depth: 0, rollout_k: 0, read_weight: 0.5, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.7, expected_payoff: 0.45)),
    (level: 8, reaction_ms: 185.0, apm_cap: 320.0, execution_noise: 0.08, rollout_depth: 0, rollout_k: 0, read_weight: 0.7, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.8, expected_payoff: 0.50)),
    (level: 9, reaction_ms: 150.0, apm_cap: 360.0, execution_noise: 0.05, rollout_depth: 0, rollout_k: 0, read_weight: 1.0, utility_weights: (reach_fit: 1.0, frame_advantage: 0.6, kill_potential: 0.4, stage_risk: -0.8, expected_payoff: 0.50)),
]"#;

fn ladder() -> FighterBrainLadder {
    FighterBrainLadder::from_ron(LADDER).expect("the fixture ladder parses")
}

#[test]
fn a_nine_rung_ladder_parses_and_is_well_formed() {
    let l = ladder();
    assert_eq!(l.rungs().len(), 9);
    assert_eq!(l.problems(), Vec::<String>::new());
    assert_eq!(l.level(9).unwrap().reaction_ms, 150.0);
    assert_eq!(l.level(1).unwrap().reaction_ms, 500.0);
}

/// §1.3, as a check rather than a wish. *"Level 9 = small numbers, never
/// zero."* A shipped profile that reacts instantly is a cheating CPU wearing a
/// difficulty's name, and the `problems()` list says so at startup rather than
/// after a self-play run.
#[test]
fn no_shipped_profile_reacts_instantly() {
    for r in ladder().rungs() {
        assert!(r.reaction_ms > 0.0, "level {}", r.level);
    }

    // Reach in the only way a test can: rebuild the RON with a zeroed row.
    let bad = LADDER.replace("reaction_ms: 150.0", "reaction_ms: 0.0");
    let cheat = FighterBrainLadder::from_ron(&bad).unwrap();
    let problems = cheat.problems();
    assert!(
        problems
            .iter()
            .any(|p| p.contains("never reacts instantly")),
        "{problems:?}"
    );
}

/// The ladder's monotonicity is checkable BEFORE a single match. Every one of
/// these would otherwise surface as "the levels do not order correctly" after
/// hours of self-play.
#[test]
fn a_non_monotone_ladder_is_caught_at_startup_not_after_a_self_play_run() {
    for (needle, patch, expect) in [
        ("reaction_ms: 150.0", "reaction_ms: 260.0", "reacts slower"),
        ("apm_cap: 360.0", "apm_cap: 100.0", "may act less often"),
        ("execution_noise: 0.05", "execution_noise: 0.9", "sloppier"),
    ] {
        let bad = LADDER.replace(needle, patch);
        let l = FighterBrainLadder::from_ron(&bad).unwrap();
        assert!(
            l.problems().iter().any(|p| p.contains(expect)),
            "patching `{needle}` -> `{patch}` should report `{expect}`: {:?}",
            l.problems()
        );
    }
}

/// The ladder's endpoints convert to the delay buffers §5 names: 150 ms → 9
/// ticks at 60 Hz, 500 ms → 30. And every rung's buffer is a REAL buffer, not
/// a pass-through.
#[test]
fn every_rung_gets_a_real_delay_buffer() {
    let l = ladder();
    assert_eq!(l.level(9).unwrap().delay(60.0).delay_ticks(), 9);
    assert_eq!(l.level(1).unwrap().delay(60.0).delay_ticks(), 30);
    for r in l.rungs() {
        assert!(
            r.delay(60.0).delay_ticks() > 0,
            "level {} would see the live world",
            r.level
        );
    }
}

/// A rung that reacts faster gets a shallower buffer. This is the reaction-time
/// distribution check §3 asks for, in the form it can take before a rig exists:
/// the ONLY thing that decides how late a brain sees the world is `reaction_ms`.
#[test]
fn the_buffer_depth_is_monotone_in_the_reaction_time() {
    let l = ladder();
    let depths: Vec<usize> = l
        .rungs()
        .iter()
        .map(|r| r.delay(60.0).delay_ticks())
        .collect();
    for w in depths.windows(2) {
        assert!(w[1] <= w[0], "depths not monotone: {depths:?}");
    }
    assert!(
        depths[0] > depths[8],
        "level 9 must react faster than level 1"
    );
}

/// L3 is an upgrade, never a dependency (§1). The whole shipped ladder runs
/// with rollouts OFF, because N3.1's `restore` does not exist yet — and every
/// rung still plays, on L2's scores alone.
#[test]
fn the_whole_ladder_degrades_gracefully_without_l3() {
    for r in ladder().rungs() {
        assert!(!r.uses_rollouts(), "level {} expects L3", r.level);
    }
}

/// A game that shipped rows gets ITS rows, not the engine's floor.
///
///  this is the rule that was written down and not enforced.
/// `FighterBrainProfile::for_level`'s own doc says a game shipping nine rows
/// means the floor "is never consulted" — and both production call sites
/// consulted it anyway, because a doc comment on the losing source cannot
/// arbitrate. Ambition ships `fighter_brain_ladder.ron` and had never read a row
/// of it.
///
///  the numbers here are the ones that were actually wrong in the game, not
/// invented ones: `for_level` gives EVERY rung `UtilityWeights::default()`, and
/// `default()` is `v1()` — which is the authored level 9 verbatim. So a level-1
/// CPU priced a smash exactly as a level-9 did, and the ladder's whole second
/// axis (*"not noticing which move hits harder IS a difficulty statement"*) did
/// not exist in the shipped game.
#[test]
fn a_shipped_ladder_beats_the_engine_floor() {
    use super::profile_for_level;

    let ladder = ladder();

    // The floor, when a game shipped nothing.
    let floor = profile_for_level(1, None);
    assert_eq!(
        floor,
        FighterBrainProfile::for_level(1),
        "with no ladder the engine floor is what a level means"
    );

    // The authored rung, when it did.
    let authored = profile_for_level(1, Some(&ladder));
    assert_eq!(
        authored.level, 1,
        "the rung for the level asked for, not the first row"
    );
    assert_ne!(
        authored, floor,
        "the authored level-1 rung is identical to the floor, so this test cannot \
         tell whether the ladder was consulted at all"
    );

    //  the difference that matters, and the one the game was losing.
    assert!(
        authored.utility_weights.kill_potential < floor.utility_weights.kill_potential,
        "the floor hands level 1 the level-9 weight set ({:?}), so a beginner CPU \
         prices a kill move exactly as the hardest one does",
        floor.utility_weights,
    );

    //  a level the ladder does not author still gets a body rather than a panic:
    // `problems()` is where a malformed ladder is reported, at load, all at once.
    assert_eq!(
        profile_for_level(200, Some(&ladder)),
        FighterBrainProfile::for_level(200),
        "an unauthored level falls back to the floor"
    );
}

/// ⭐⭐ THE FORK, PINNED: the two authorities disagree, and nothing at the call
/// site says which one answered.
///
/// ⛔ `profile_for_level` is `ladder.level(n)` else `FighterBrainProfile::for_level(n)`
/// — **two answers to "what does rung N mean", both shipping.** A composition that
/// installs an `AuthoredFighterLadder` gets the game's rows; one that does not
/// gets the floor, silently, with a profile that looks exactly as authoritative.
///
/// ⇒ **Four separate defects on 2026-09-04 were symptoms of this one fork**
/// (`docs/planning/engine/fighter-brain.md`): a rig measured the floor's weights
/// while its header claimed the authored ladder; `UtilityWeights::default()` turned
/// out to BE the level-9 row so every rung scored identically; the floor arms the
/// L3 rollout at level 6 while the shipped rows disable it everywhere; and the
/// rollout's `Dodge`/`Shield` suppression was characterised at length before
/// anyone noticed no player could reach it.
///
/// ⚠ **This test does not argue the floor should go.** It exists so the DISAGREEMENT
/// is a fact in the test suite rather than a discovery each time: the floor is a
/// real requirement (a demo with no authored content still runs), and its cost is
/// that a caller cannot tell which authority answered.
#[test]
fn the_floor_and_an_authored_ladder_disagree_and_the_caller_cannot_tell() {
    // ⭐ The module's own fixture ladder, which mirrors the shipped one on the
    // field that caused the most trouble: `rollout_depth: 0` on every rung.
    // Using it rather than a hand-built row keeps this test honest if the
    // fixture is ever retuned.
    let ladder = ladder();
    let floor = profile_for_level(6, None);
    let shipped = profile_for_level(6, Some(&ladder));

    // ⛔ The floor arms the L3 rollout at level 6. The authored row does not.
    assert!(
        floor.uses_rollouts(),
        "the floor stopped arming the rollout at level 6 — if that is deliberate, \
         this test and the four findings it records need re-deriving, because \
         they all turn on the floor and the shipped rows disagreeing HERE"
    );
    assert!(
        !shipped.uses_rollouts(),
        "the authored row's zeroed rollout fields did not survive the lookup"
    );
    assert_ne!(
        floor, shipped,
        "the two authorities agreed, which would make this fork harmless — and \
         if they now agree by construction, the four findings this test records \
         are no longer reachable and it should be deleted rather than kept"
    );

    // ⭐ THE POINT: both are `FighterBrainProfile` and carry no provenance. A
    // caller holding one cannot ask which authority produced it, which is why
    // every one of those four findings needed an instrument change to see.
    assert_eq!(
        floor.level, shipped.level,
        "both answers claim the same rung — that is the whole hazard, and if the \
         level ever disagreed a caller would at least have a signal"
    );
}
