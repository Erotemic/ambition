//! Probes of `SmashFighterFacet::problems` and of the authored form.
//!
//! Deliberately a local fixture rather than `include_str!` of a shipped file.
//! `ambition_characters` is an engine crate and `game/ambition_demo_smash` is a
//! game; a test path climbing out of one into the other is a dependency the
//! crate graph does not have. George's own facet is guarded where it lives.

use super::*;
use crate::move_damage::{move_damage_over, MoveDamage};

fn facet() -> SmashFighterFacet {
    SmashFighterFacet {
        body: None,
        knockback_weight: None,
        move_damage: MoveDamage::new(),
        character: "test_fighter".to_string(),
    }
}

/// ⛔ A WEIGHT THE KNOCKBACK TERM DIVIDES BY MAY NOT BE ZERO OR NEGATIVE.
///
/// The launch law divides the growth term by this, so 0.0 is a division by
/// zero and a negative sends the victim TOWARD the attacker. Both are the class
/// this list is for: authored values whose consequence is invisible until
/// somebody launches the fighter.
///
/// ⚠ NOT a balance filter. 40.0 is a fighter nothing can launch and 0.01 is one
/// a jab kills, and both are refused here only if this file forgets what it is
/// for — see `problems`'s own doc.
#[test]
fn a_knockback_weight_the_launch_term_divides_by_must_be_positive() {
    for bad in [0.0, -1.35, f32::NAN] {
        let mut facet = facet();
        facet.knockback_weight = Some(bad);
        assert!(
            facet
                .problems()
                .iter()
                .any(|problem| problem.contains("knockback_weight")),
            "`{bad}` was accepted as a knockback weight: {:?}",
            facet.problems()
        );
    }
    // ⭐ AND THE CONTROL. A heavy and a light are both fine; the list refuses
    // the impossible, not the unusual.
    for good in [0.5, 1.0, 1.35, 8.0] {
        let mut facet = facet();
        facet.knockback_weight = Some(good);
        assert!(
            facet.problems().is_empty(),
            "`{good}` is an ordinary weight and was refused: {:?}",
            facet.problems()
        );
    }
}

#[test]
fn a_well_formed_facet_has_nothing_to_report() {
    assert!(
        facet().problems().is_empty(),
        "the fixture itself is malformed, so every negative case below proves \
         nothing: {:?}",
        facet().problems()
    );
}

/// The authored form round-trips. A facet that serialises to RON the schema
/// cannot read back is a facet nobody can hand-edit, which is the whole point of
/// it being content.
#[test]
fn the_authored_form_round_trips_through_ron() {
    let facet = facet();
    let text = ron::ser::to_string(&facet).expect("a facet serialises");
    let back: SmashFighterFacet = ron::from_str(&text).expect("and reads back");
    assert_eq!(back, facet);
}

/// A FIGHTER STATES ITS DIFFERENCES, and everything it does not state it keeps.
///
/// ⭐ The patch shape is the whole point: a heavy authors a gravity and a fall
/// speed and says nothing about its jump, so a later change to the shared
/// numbers still reaches it. A full body per fighter would freeze every author's
/// copy of them.
#[test]
fn an_authored_fighter_body_replaces_only_what_it_names() {
    let base = ambition_platformer2d_core::DEFAULT_TUNING;
    let heavy = super::FighterBodyAuthoring {
        gravity: Some(3100.0),
        max_fall_speed: Some(2400.0),
        ..super::FighterBodyAuthoring::default()
    };
    let body = heavy.over(base);
    assert_eq!(body.gravity, 3100.0);
    assert_eq!(body.max_fall_speed, 2400.0);
    assert_eq!(
        (body.jump_speed, body.run_accel, body.max_run_speed),
        (base.jump_speed, base.run_accel, base.max_run_speed),
        "a body that named a gravity and a fall speed also moved a number it \
         never mentioned, so a fighter cannot state one difference without \
         freezing the rest of the shared body"
    );
}

/// ⛔ A `body:` THAT STATES NOTHING IS A DECLARATION THAT MEANS NOTHING, and it
/// is worth a diagnostic rather than a silent no-op: an author who wrote the key
/// meant to say something.
#[test]
fn a_fighter_body_must_state_something_and_must_state_it_positive() {
    let mut facet = facet();
    facet.body = Some(super::FighterBodyAuthoring::default());
    let problems = facet.problems();
    assert!(
        problems.iter().any(|p| p.contains("states no number")),
        "an empty authored body passed validation, so a fighter file can \
         declare a body and get none: {problems:?}"
    );

    // ⭐ AND THE ARMS STRADDLE THE RULE. Every field here is a magnitude the
    // kernel multiplies by, so zero is not a slow fighter — it is one that
    // cannot move.
    facet.body = Some(super::FighterBodyAuthoring {
        max_run_speed: Some(0.0),
        ..super::FighterBodyAuthoring::default()
    });
    assert!(
        facet.problems().iter().any(|p| p.contains("max_run_speed")),
        "a gait of zero passed validation"
    );
    facet.body = Some(super::FighterBodyAuthoring {
        max_run_speed: Some(240.0),
        ..super::FighterBodyAuthoring::default()
    });
    assert!(
        facet.problems().is_empty(),
        "an ordinary authored gait was refused: {:?}",
        facet.problems()
    );
}

/// Two moves: a one-hit `jab`, and a `combo` whose second window holds a
/// damaging volume and a WINDBOX (damage 0) that has no damage to replace.
fn two_moves() -> ambition_entity_catalog::MovesetContract {
    use crate::prepared_fixtures::{moveset_with, slash};
    let jab = slash("jab", "swing", "hit");
    let mut combo = slash("combo", "swing", "hit");
    let mut second = combo.windows[0].clone();
    second.volumes[0].damage = 2;
    let mut gust = second.volumes[0].clone();
    gust.damage = 0;
    second.volumes.push(gust);
    combo.windows.push(second);
    moveset_with(&[("attack", "jab")], vec![jab, combo])
}

fn damages(moveset: &ambition_entity_catalog::MovesetContract, id: &str) -> Vec<i32> {
    moveset
        .moves
        .iter()
        .find(|spec| spec.id == id)
        .expect("the fixture has the move")
        .windows
        .iter()
        .flat_map(|window| window.volumes.iter().map(|volume| volume.damage))
        .collect()
}

/// ⭐ A GAME'S DAMAGE REPLACES ONLY THE NUMBERS IT NAMES. Each damaging volume
/// gets its value in order, the windbox stays a push, and a move the map does
/// not name keeps its moveset damage.
#[test]
fn move_damage_replaces_each_damaging_volume_and_nothing_else() {
    let damage = MoveDamage::from([("combo".to_string(), vec![6, 9])]);
    let moveset = move_damage_over(&damage, two_moves()).expect("the map fits the moves");
    assert_eq!(damages(&moveset, "combo"), vec![6, 9, 0]);
    assert_eq!(damages(&moveset, "jab"), vec![1], "an unnamed move changed");
}

/// ⛔ A MAP THAT DOES NOT FIT IS REFUSED WHOLE. An unknown move and a list of
/// the wrong length are both named, and the moveset that comes back on `Ok`
/// never exists: a body must not deal one game's damage on half its moves.
#[test]
fn move_damage_that_does_not_fit_the_moves_is_refused_whole() {
    let damage = MoveDamage::from([
        ("jab".to_string(), vec![3]),
        ("combo".to_string(), vec![6]),
        ("uppercut".to_string(), vec![12]),
    ]);
    let problems = move_damage_over(&damage, two_moves()).expect_err("two keys do not fit");
    assert!(
        problems.iter().any(|p| p.contains("`uppercut`")),
        "an unknown move was not named: {problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("`combo`") && p.contains("2 volume")),
        "a list of the wrong length was not named: {problems:?}"
    );
    assert_eq!(problems.len(), 2, "the key that fits was reported: {problems:?}");
}

/// ⛔ A VALUE BELOW 1 WOULD TURN A HIT INTO A PUSH, and an empty list names a
/// move and gives it nothing.
#[test]
fn move_damage_values_must_be_at_least_one() {
    for bad in [vec![0], vec![], vec![4, -1]] {
        let mut facet = facet();
        facet.move_damage = MoveDamage::from([("jab".to_string(), bad.clone())]);
        assert!(
            facet.problems().iter().any(|p| p.contains("move_damage.jab")),
            "`{bad:?}` was accepted: {:?}",
            facet.problems()
        );
    }
    let mut facet = facet();
    facet.move_damage = MoveDamage::from([("jab".to_string(), vec![3, 15])]);
    assert!(facet.problems().is_empty(), "{:?}", facet.problems());
}
