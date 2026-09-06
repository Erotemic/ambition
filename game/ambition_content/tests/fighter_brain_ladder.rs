//! Ambition's fighter-brain ladder, validated. `fighter-brain.md` §4 makes the
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

/// §1.3, kept. *"Level 9 = small numbers, never zero."* A shipped difficulty
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

/// L3 is an upgrade, never a dependency (§1). Every shipped row runs with
/// rollouts OFF, because N3.1's `restore` does not exist. The ladder still plays,
/// on L2's scores alone, and FB6 turns these on without touching a difficulty's
/// identity.
#[test]
fn the_whole_shipped_ladder_plays_without_l3() {
    for r in ladder().rungs() {
        assert!(!r.uses_rollouts(), "level {} expects L3", r.level);
    }
}

/// The pack LOWERS the ladder, which is the whole point of migrating it.
///
/// the tests above this one all passed while the game read none of it.
/// Every one of them parses `LADDER_RON` itself, so they were green for as long
/// as the file was well-formed — including the entire period when
/// `fighter_brain_ladder.ron` was authored content that nothing in the running
/// game had ever read, and `FighterBrainProfile::for_level` (which documents
/// itself as the floor a game overrides) was consulted at both production call
/// sites instead.
///
/// so this test asks the only question the others cannot: is it in the
/// pack? It reads the prepared pack rather than the file, so it fails if the
/// manifest stops declaring the source, if the schema stops being registered, or
/// if lowering breaks — none of which the parse tests can see.
#[test]
fn the_prepared_pack_lowers_the_shipped_ladder() {
    let lowered = ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(
        ambition_content::pack::prepared(),
    )
    .expect(
        "the prepared pack lowers no fighter ladder — the game is back on the \
         engine floor, where every difficulty scores moves with the level-9 \
         weight set",
    );

    assert_eq!(
        lowered.rungs().len(),
        9,
        "the lowered ladder is not nine rungs"
    );
    assert_eq!(
        lowered.rungs(),
        ladder().rungs(),
        "the pack lowered a DIFFERENT ladder than the file on disk"
    );
    // the property the floor gets wrong, asserted on what the game will read.
    let l1 = lowered.level(1).expect("level 1 is authored");
    let floor = ambition_characters::brain::fighter::FighterBrainProfile::for_level(1);
    assert!(
        l1.utility_weights.kill_potential < floor.utility_weights.kill_potential,
        "the lowered level-1 rung values a kill move as highly as the engine floor \
         does, so loading the ladder changed nothing about difficulty"
    );
}
