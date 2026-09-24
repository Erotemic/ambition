//! Which difficulty ladder this demo's fighters get.
//!
//! The standalone demo app gives every CPU rung the same utility weights.
//! `Res<AuthoredFighterLadder>` is inserted by `ambition_content`, which
//! neither `ambition_demo_smash` nor `ambition_demo_smash_app` depends on. So
//! `profile_for_level` falls back to `FighterBrainProfile::for_level`, whose
//! `utility_weights` is `UtilityWeights::default()` (`v1()`, the level-9 row)
//! for every level. The shipped game composes `ambition_content` and gets the
//! authored rows.
//!
//! These tests pin that, so whoever installs a ladder here is told to update
//! the rig's claims about what it measures.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::characters::brain::fighter::{
    AuthoredFighterLadder, FighterBrainProfile,
};

/// The floor gives EVERY rung the level-9 utility weights.
///
/// If `for_level` starts authoring real per-level weights, this fails.
#[test]
fn the_engine_floor_gives_every_rung_the_same_utility_weights() {
    let weights: Vec<_> = (1..=9)
        .map(|level| FighterBrainProfile::for_level(level).utility_weights)
        .collect();
    let first = weights[0];
    assert!(
        weights.iter().all(|w| *w == first),
        "the floor's per-level weights now differ, so the rig is no longer \
         measuring one flat scoring policy across the ladder — re-read \
         `fighter-brain.md`'s ladder section, its conclusions depend on this"
    );
    // Anti-vacuity: they are specifically v1, the level-9 row of
    // `fighter_brain_ladder.ron`, not a neutral all-zero default.
    assert_eq!(
        first,
        ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1(),
        "the floor's shared weights are no longer v1, so a level-1 CPU is no \
         longer wearing the top rung's priorities"
    );
}

/// The demo app this crate's rig measures does NOT install the authored ladder.
///
/// When that changes (by composing `ambition_content`, or by Smash shipping
/// its own rows), update the rig's `report_which_ladder_is_in_play` line and
/// `fighter-brain.md`'s ladder section in the same change.
#[test]
fn the_demo_app_still_runs_its_fighters_on_the_engine_floor() {
    let mut app = build_demo_app();
    app.update();
    assert!(
        app.world().get_resource::<AuthoredFighterLadder>().is_none(),
        "an AuthoredFighterLadder is installed now — good, but the ladder rig \
         still prints that it measured the floor, and every recorded ladder \
         number was taken without this. Update the rig's report and the \
         fighter-brain ladder section, then delete this test."
    );
}
