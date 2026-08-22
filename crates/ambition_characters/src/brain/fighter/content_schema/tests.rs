//! these test the SCHEMA, not the ladder. `FighterBrainLadder::problems` has
//! its own tests next door; what is asked here is whether a ladder the TYPE
//! calls broken is one the schema would have something to report — which is the
//! difference between a ladder that is well-formed and one that is checked.
//!
//! deliberately a local fixture rather than `include_str!` of the shipped
//! file. `ambition_characters` is an engine crate and `game/ambition_content` is
//! a game; a test path climbing six directories out of one into the other is a
//! dependency the crate graph does not have. The shipped ladder has its own test
//! where it lives.

use super::*;

/// Nine well-formed rungs, trimmed to the fields the ladder reads.
fn nine_rungs(reaction_at_five: f32) -> String {
    let mut rows = String::from("[\n");
    for level in 1..=9u8 {
        let t = (level - 1) as f32 / 8.0;
        let reaction = if level == 5 {
            reaction_at_five
        } else {
            500.0 - t * 350.0
        };
        rows.push_str(&format!(
            "  (level: {level}, reaction_ms: {reaction:.1}, apm_cap: {:.1}, \
             execution_noise: {:.3}, rollout_depth: 0, rollout_k: 0, read_weight: 0.0, \
             utility_weights: (reach_fit: 1.0, frame_advantage: 0.1, kill_potential: 0.0, \
             stage_risk: -0.1, expected_payoff: 0.0)),\n",
            60.0 + t * 300.0,
            0.45 - t * 0.4,
        ));
    }
    rows.push(']');
    rows
}

#[test]
fn a_well_formed_ladder_has_nothing_to_report() {
    let ron = nine_rungs(300.0);
    let ladder = FighterBrainLadder::from_ron(&ron).expect("the fixture parses");
    assert!(
        ladder.problems().is_empty(),
        "the fixture itself is malformed, so the negative case below proves \
         nothing: {:?}",
        ladder.problems()
    );
    assert_eq!(ladder.rungs().len(), 9);
}

/// the case that looks fine row by row. A ladder whose level 5 reacts
/// SLOWER than level 4 parses cleanly — every row is a valid profile — and is
/// nonsense as a ladder. This is what the schema exists to say at load, in one
/// place, instead of as "the levels do not order correctly" after hours of
/// self-play.
#[test]
fn a_ladder_that_is_not_monotone_parses_and_is_still_wrong() {
    let ron = nine_rungs(600.0);
    let ladder = FighterBrainLadder::from_ron(&ron).expect("it parses — that is the point");
    let problems = ladder.problems();
    assert!(
        !problems.is_empty(),
        "level 5 reacts slower than level 4 and `problems()` is silent, so the \
         schema has nothing to report and the check is decorative"
    );
    assert!(
        problems.iter().any(|p| p.contains("monotone")),
        "the reported problems do not mention monotonicity: {problems:?}"
    );
}
