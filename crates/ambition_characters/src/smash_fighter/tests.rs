//! Probes of `SmashFighterFacet::problems` and of the authored form.
//!
//! Deliberately a local fixture rather than `include_str!` of a shipped file.
//! `ambition_characters` is an engine crate and `game/ambition_demo_smash` is a
//! game; a test path climbing out of one into the other is a dependency the
//! crate graph does not have. George's own facet is guarded where it lives.

use super::*;

fn facet() -> SmashFighterFacet {
    SmashFighterFacet {
        body: None,
        knockback_weight: None,
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
