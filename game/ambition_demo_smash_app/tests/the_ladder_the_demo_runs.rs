//! Which difficulty ladder this demo's fighters get.
//!
//! Smash's pack names the ladder file (`data/fighter_brain_ladder.ron`, the
//! file the shipped game's pack names too), and the demo declares its rows for
//! its mode. So the rig measures the ladder players meet, not the engine floor.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::characters::brain::fighter::{FighterBrainLadder, FighterBrainProfile};

/// The floor gives EVERY rung the level-9 utility weights.
///
/// A room that no game gives a ladder plays this floor. If `for_level` starts
/// authoring real per-level weights, this fails.
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

/// The demo's Smash rooms are governed by the rows of the ladder file.
///
/// The comparison is with the FILE, not with the pack's own lowering, so a
/// declaration of other rows (the floor, another file) fails here.
#[test]
fn the_demo_app_runs_its_fighters_on_smashs_ladder() {
    let mut app = build_demo_app();
    app.update();
    let declared = app
        .world()
        .get_resource::<ambition_platformer2d::combat::scoped_rules::DeclaredRules<FighterBrainLadder>>()
        .and_then(|rules| {
            rules.governing(ambition_platformer2d::combat::scoped_rules::ActiveRoom::Mode(
                ambition_demo_smash::SMASH_MODE,
            ))
        })
        .expect("Smash declares a fighter ladder for its rooms");
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ambition_content/assets/data/fighter_brain_ladder.ron");
    let file = FighterBrainLadder::from_ron(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(declared.rungs(), file.rungs(), "the Smash rooms play the rows of {}", path.display());
}
