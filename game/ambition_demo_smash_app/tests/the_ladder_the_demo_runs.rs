//! WHICH DIFFICULTY LADDER THIS DEMO'S FIGHTERS ACTUALLY GET.
//!
//! ⛔⛔ Found 2026-09-04, and it had been true for the whole life of the ladder
//! rig: **the standalone demo app gives every CPU rung the same utility
//! weights**, because `Res<AuthoredFighterLadder>` is inserted by
//! `ambition_content` and neither `ambition_demo_smash` nor
//! `ambition_demo_smash_app` depends on it. `profile_for_level` then falls back
//! to `FighterBrainProfile::for_level`, whose `utility_weights` is
//! `UtilityWeights::default()` — which IS `v1()`, the level-9 row — for every
//! level.
//!
//! ⇒ The shipped game composes `ambition_content` and DOES hand its fighters the
//! authored rows, so the rig has been measuring a different fighter from the one
//! a player fights. Every ladder number this project recorded is a measurement
//! of the floor.
//!
//! These tests do not repair that. They PIN it, so it cannot go back to being an
//! invisible property somebody has to re-derive from a null result — and so that
//! whoever installs a ladder here is told by a red test that the rig's claims
//! about what it measured need updating with it.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::characters::brain::fighter::{
    AuthoredFighterLadder, FighterBrainProfile,
};

/// The floor gives EVERY rung the level-9 utility weights.
///
/// This is the defect stated as an arithmetic fact about the engine floor, with
/// no app involved: if a future change makes `for_level` author real per-level
/// weights, this reddens and the story above stops being true.
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
    // ⚠ ANTI-VACUITY: and they are specifically v1, the LEVEL 9 row of
    // `fighter_brain_ladder.ron`. "All the same" would also be satisfied by a
    // neutral all-zero default, which would be a different (and less
    // misleading) situation.
    assert_eq!(
        first,
        ambition_platformer2d::characters::brain::fighter::UtilityWeights::v1(),
        "the floor's shared weights are no longer v1, so a level-1 CPU is no \
         longer wearing the top rung's priorities"
    );
}

/// The demo app this crate's rig measures does NOT install the authored ladder.
///
/// ⇒ When somebody fixes that — by composing `ambition_content`, or by Smash
/// shipping its own nine rows as `for_level`'s doc invites — this test is the
/// one that says so, and the rig's `report_which_ladder_is_in_play` line and
/// `fighter-brain.md`'s ladder section both need updating in the same change.
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
