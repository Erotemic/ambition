//! **Ambition's fighter-brain ladder, validated.** `fighter-brain.md` §4 makes the
//! nine rows content; §3's humanity checks make them checkable.
//!
//! Every assertion here would otherwise surface as *"the levels do not order
//! correctly"* after hours of self-play. `problems()` says it at startup.

use ambition_characters::brain::fighter::FighterBrainLadder;

const LADDER_RON: &str = include_str!("../assets/data/fighter_brain_ladder.ron");

fn ladder() -> FighterBrainLadder {
    FighterBrainLadder::from_ron(LADDER_RON).expect("fighter_brain_ladder.ron parses")
}

#[test]
fn the_shipped_ladder_is_well_formed() {
    let problems = ladder().problems();
    assert!(problems.is_empty(), "{problems:#?}");
    assert_eq!(ladder().rungs().len(), 9);
}

/// **§1.3, kept.** *"Level 9 = small numbers, never zero."* A shipped difficulty
/// that reacted instantly would be a cheating CPU wearing a level's name — and the
/// perception seam would let it, because `DelayedPerception::new(0)` is a legal
/// buffer for RL rigs.
#[test]
fn no_shipped_level_reacts_instantly() {
    for r in ladder().rungs() {
        assert!(r.reaction_ms > 0.0, "level {}", r.level);
        assert!(
            r.delay(60.0).delay_ticks() > 0,
            "level {} would see the live world",
            r.level
        );
    }
    // The doc's endpoints, at 60 Hz.
    assert_eq!(ladder().level(9).unwrap().delay(60.0).delay_ticks(), 9);
    assert_eq!(ladder().level(1).unwrap().delay(60.0).delay_ticks(), 30);
}

/// **L3 is an upgrade, never a dependency** (§1). Every shipped row runs with
/// rollouts OFF, because N3.1's `restore` does not exist. The ladder still plays,
/// on L2's scores alone, and FB6 turns these on without touching a difficulty's
/// identity.
#[test]
fn the_whole_shipped_ladder_plays_without_l3() {
    for r in ladder().rungs() {
        assert!(!r.uses_rollouts(), "level {} expects L3", r.level);
    }
}
