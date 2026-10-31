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
        verbs: Default::default(),
        move_timing: Default::default(),
        move_knockback: Default::default(),
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
        problems.iter().any(|p| p.contains("`combo`") && p.contains("2 thing")),
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

/// ⭐ A GAME'S CLOCK MAPS THE MOVE'S OWN. The fixture's jab is active 0.1–0.2
/// of 0.4 with its sound at 0.1; timed to hit 0.05, last 0.4, total 0.7 the
/// window edges and the event follow the same piecewise map (0.1 -> 0.05,
/// 0.2 -> 0.4, end 0.4 -> 0.7), the landing lag is set, an aerial with lag
/// gets its autocancel, the move's own sound keeps the derived swing out, and
/// a move the map does not name is untouched. Poison: the map applied to the
/// events removed, red at the event assertion.
#[test]
fn move_timing_maps_the_moves_own_clock_and_nothing_else() {
    use crate::move_damage::{move_timing_over, MoveTiming, MoveTimings};
    let timing = MoveTimings::from([(
        "jab".to_string(),
        MoveTiming {
            hit_s: 0.05,
            last_s: 0.4,
            total_s: 0.7,
            landing_lag_s: Some(0.1),
            swing: Some("swing.aerial".to_string()),
        },
    )]);
    let moveset = move_timing_over(&timing, two_moves()).expect("the map names a move with a swing");
    let jab = moveset.move_by_id("jab").expect("kept");
    let close = |a: f32, b: f32| (a - b).abs() < 1e-5;
    assert!(close(jab.duration_s, 0.7));
    assert!(close(jab.windows[0].start_s, 0.05) && close(jab.windows[0].end_s, 0.4), "{:?}", jab.windows[0]);
    assert!(close(jab.events[0].at_s, 0.05), "the authored sound rode the map: {}", jab.events[0].at_s);
    assert_eq!(jab.events.len(), 1, "a move with its own sound before the strike gets no derived swing");
    assert_eq!(jab.landing_lag_s, Some(0.1));
    assert!(close(jab.autocancel_after_s.expect("an aerial with lag autocancels"), 0.4 + 0.6 * 0.3));
    let combo = moveset.move_by_id("combo").expect("kept");
    assert!(close(combo.duration_s, 0.4) && close(combo.windows[0].start_s, 0.1), "an unnamed move keeps its clock");

    // A silent move gets the swing a third of the way to its first strike.
    let mut silent = two_moves();
    silent.moves[0].events.clear();
    let moveset = move_timing_over(&timing, silent).expect("fits");
    let jab = moveset.move_by_id("jab").expect("kept");
    assert_eq!(jab.events.len(), 1);
    assert!(close(jab.events[0].at_s, 0.05 * 0.3));
    assert!(matches!(&jab.events[0].kind, ambition_entity_catalog::MoveEventKind::Sfx { cue } if cue == "swing.aerial"));

    // A move the moveset lacks, or one with no Active window, is refused whole.
    let missing = MoveTimings::from([("nope".to_string(), timing["jab"].clone())]);
    assert!(move_timing_over(&missing, two_moves()).is_err());
}

/// ⭐ A GAME'S KNOCKBACK FACTOR SCALES ONLY THE BASE OF THE MOVES IT NAMES: each
/// striking volume's `knockback` is multiplied, a windbox (damage 0) and the
/// growth are untouched, an unnamed move keeps its launch, and a factor that
/// is not positive or a move the moveset lacks refuses the map whole.
#[test]
fn move_knockback_scales_the_named_moves_base_launch_and_nothing_else() {
    use crate::move_damage::{move_knockback_over, MoveKnockback};
    let before = two_moves();
    let base = |m: &ambition_entity_catalog::MovesetContract, id: &str| -> Vec<(i32, f32, Option<f32>)> {
        m.move_by_id(id)
            .expect("move")
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter().map(|v| (v.damage, v.knockback, v.knockback_growth)))
            .collect()
    };
    let scaled = move_knockback_over(&MoveKnockback::from([("combo".to_string(), 1.25)]), before.clone()).expect("fits");
    for ((dmg, kb_before, growth_before), (_, kb_after, growth_after)) in base(&before, "combo").into_iter().zip(base(&scaled, "combo")) {
        let expected = if dmg > 0 { kb_before * 1.25 } else { kb_before };
        assert!((kb_after - expected).abs() < 1e-5, "damage {dmg}: {kb_before} -> {kb_after}");
        assert_eq!(growth_before, growth_after, "growth is untouched");
    }
    assert_eq!(base(&before, "jab"), base(&scaled, "jab"), "an unnamed move keeps its launch");
    assert!(move_knockback_over(&MoveKnockback::from([("nope".to_string(), 1.25)]), two_moves()).is_err());
    assert!(move_knockback_over(&MoveKnockback::from([("jab".to_string(), 0.0)]), two_moves()).is_err());
}
