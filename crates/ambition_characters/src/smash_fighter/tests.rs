//! what is asked here is that PREPARATION LOSES NOTHING. A facet is only
//! worth having if the numbers an author wrote come back out of the runtime
//! moves unchanged; a lowering that dropped `hold_offset` would compile, pass
//! every shape assertion, and put every captive in the wrong place.
//!
//! deliberately a local fixture rather than `include_str!` of a shipped file.
//! `ambition_characters` is an engine crate and `game/ambition_demo_smash` is a
//! game; a test path climbing out of one into the other is a dependency the
//! crate graph does not have. George's own facet is guarded where it lives.

use super::*;
use ambition_entity_catalog::{MoveEventKind, WindowTag};

fn kit() -> CaptureKitAuthoring {
    CaptureKitAuthoring {
        grab: GrabAuthoring {
            id: "test_grab".to_string(),
            clip: "grab".to_string(),
            startup_s: 0.16,
            active_s: 0.06,
            recover_s: 0.30,
            reach: CaptureAttemptParams {
                offset: (18.0, 0.0),
                half_extents: (26.0, 13.0),
                hold_offset: (20.0, -2.0),
            },
        },
        pummel: PummelAuthoring {
            id: "test_pummel".to_string(),
            clip: "attack".to_string(),
            duration_s: 0.24,
            impact_at_s: 0.11,
            impact: CapturePummelParams { damage: 4 },
        },
        forward_throw: ThrowAuthoring {
            id: "test_fthrow".to_string(),
            clip: "attack".to_string(),
            duration_s: 0.34,
            release_at_s: 0.20,
            launch: CaptureThrowParams {
                damage: 11,
                knockback: 138.0,
                knockback_growth: 1.9,
                launch_dir: (1.0, -0.35),
            },
        },
        back_throw: None,
        up_throw: None,
        down_throw: None,
    }
}

fn facet() -> SmashFighterFacet {
    SmashFighterFacet {
        body: None,
        knockback_weight: None,
        character: "test_fighter".to_string(),
        capture: kit(),
    }
}

/// ⛔ A WEIGHT THE KNOCKBACK TERM DIVIDES BY MAY NOT BE ZERO OR NEGATIVE.
///
/// `scaled_knockback` divides the growth term by this, so 0.0 is a division by
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

/// THE ONE THAT MATTERS: every authored number survives preparation.
///
/// Read back out of the prepared moves rather than compared against the struct
/// it came from — a test that asserted `kit.grab.reach == kit.grab.reach` would
/// pass with the lowering deleted.
#[test]
fn preparation_carries_every_authored_number_into_the_moves() {
    let repertoire = kit().into_repertoire();

    assert_eq!(repertoire.grab.id, "test_grab");
    assert_eq!(repertoire.grab.clip.clip, "grab");
    // 0.16 startup + 0.06 active + 0.30 recovery.
    assert!((repertoire.grab.duration_s - 0.52).abs() < 1e-5);
    let active = repertoire
        .grab
        .windows
        .iter()
        .find(|w| w.tag == WindowTag::Active)
        .expect("a grab has an Active window");
    assert!((active.start_s - 0.16).abs() < 1e-5);
    assert!((active.end_s - 0.22).abs() < 1e-5);
    let effect = active
        .sustain_effect
        .as_ref()
        .expect("the Active window sustains the attempt");
    assert_eq!(effect.key, crate::smash_capture::CAPTURE_ATTEMPT);
    let reach: CaptureAttemptParams = effect.params.hydrate().expect("the attempt hydrates");
    assert_eq!(reach.offset, (18.0, 0.0));
    assert_eq!(reach.half_extents, (26.0, 13.0));
    assert_eq!(reach.hold_offset, (20.0, -2.0));

    let pummel = &repertoire.pummel;
    assert_eq!(pummel.id, "test_pummel");
    assert!((pummel.duration_s - 0.24).abs() < 1e-5);
    let (at_s, impact) = one_effect(pummel, crate::smash_capture::CAPTURE_PUMMEL);
    assert!((at_s - 0.11).abs() < 1e-5);
    let impact: CapturePummelParams = impact.hydrate().expect("the impact hydrates");
    assert_eq!(impact.damage, 4);

    let throw = &repertoire.forward_throw;
    assert_eq!(throw.id, "test_fthrow");
    assert!((throw.duration_s - 0.34).abs() < 1e-5);
    let (at_s, launch) = one_effect(throw, crate::smash_capture::CAPTURE_THROW);
    assert!((at_s - 0.20).abs() < 1e-5);
    let launch: CaptureThrowParams = launch.hydrate().expect("the launch hydrates");
    assert_eq!(launch.damage, 11);
    assert_eq!(launch.knockback, 138.0);
    assert_eq!(launch.knockback_growth, 1.9);
    assert_eq!(launch.launch_dir, (1.0, -0.35));

    assert!(repertoire.back_throw.is_none());
    assert!(repertoire.up_throw.is_none());
    assert!(repertoire.down_throw.is_none());
}

/// The unauthored throws are absent, and an authored one is present with its
/// own numbers — the same slot answering both ways.
#[test]
fn an_authored_back_throw_arrives_and_the_others_stay_absent() {
    let mut kit = kit();
    kit.back_throw = Some(ThrowAuthoring {
        id: "test_bthrow".to_string(),
        clip: "attack".to_string(),
        duration_s: 0.40,
        release_at_s: 0.24,
        launch: CaptureThrowParams {
            damage: 13,
            knockback: 150.0,
            knockback_growth: 2.0,
            launch_dir: (-1.0, -0.6),
        },
    });
    let repertoire = kit.into_repertoire();
    let back = repertoire
        .back_throw
        .expect("the authored back throw arrives");
    assert_eq!(back.id, "test_bthrow");
    let (_, launch) = one_effect(&back, crate::smash_capture::CAPTURE_THROW);
    let launch: CaptureThrowParams = launch.hydrate().expect("the launch hydrates");
    assert_eq!(launch.launch_dir, (-1.0, -0.6));
    assert!(repertoire.up_throw.is_none());
    assert!(repertoire.down_throw.is_none());
}

/// the grab that plays and catches nobody. Every field is a plausible
/// number and the move is a recovery animation. `author_standing_grab` asserts
/// on it, so without this the failure would be a panic during preparation
/// rather than a diagnostic naming the file.
#[test]
fn a_grab_that_is_never_live_is_refused_before_it_can_panic() {
    let mut facet = facet();
    facet.capture.grab.active_s = 0.0;
    let problems = facet.problems();
    assert!(
        problems.iter().any(|p| p.contains("never asked about")),
        "{problems:?}"
    );
}

/// A reach with no area cannot overlap anything, so the grab is live and still
/// catches nobody — the same symptom from the other side.
#[test]
fn a_reach_with_no_area_is_refused() {
    let mut facet = facet();
    facet.capture.grab.reach.half_extents = (26.0, 0.0);
    assert!(
        facet.problems().iter().any(|p| p.contains("no area")),
        "{:?}",
        facet.problems()
    );
}

/// the sharpest one: a throw whose release is past its own end. The move
/// plays, the wind-up reads, and the captive is still held when it finishes —
/// which in a match looks like the grab being unbreakable rather than like a
/// number being wrong.
#[test]
fn a_release_past_the_end_of_its_own_throw_is_refused() {
    let mut facet = facet();
    facet.capture.forward_throw.release_at_s = 0.40;
    assert!(
        facet
            .problems()
            .iter()
            .any(|p| p.contains("never released")),
        "{:?}",
        facet.problems()
    );
}

#[test]
fn an_impact_past_the_end_of_its_own_pummel_is_refused() {
    let mut facet = facet();
    facet.capture.pummel.impact_at_s = 0.30;
    assert!(
        facet
            .problems()
            .iter()
            .any(|p| p.contains("damage never lands")),
        "{:?}",
        facet.problems()
    );
}

/// Two moves under one id: reachable by verb, unreachable by everything that
/// looks a move up by name.
#[test]
fn two_capture_moves_sharing_an_id_are_refused() {
    let mut facet = facet();
    facet.capture.pummel.id = facet.capture.grab.id.clone();
    assert!(
        facet
            .problems()
            .iter()
            .any(|p| p.contains("one id per move")),
        "{:?}",
        facet.problems()
    );
}

/// A throw that launches nowhere: damage lands, and the captive is dropped at
/// the captor's feet with the knockback road given no direction to scale.
#[test]
fn a_throw_with_no_launch_direction_is_refused() {
    let mut facet = facet();
    facet.capture.forward_throw.launch.launch_dir = (0.0, 0.0);
    assert!(
        facet.problems().iter().any(|p| p.contains("no direction")),
        "{:?}",
        facet.problems()
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

fn one_effect<'a>(
    spec: &'a ambition_entity_catalog::MoveSpec,
    key: &str,
) -> (f32, &'a ambition_entity_catalog::ParamValue) {
    let mut found = spec.events.iter().filter_map(|event| match &event.kind {
        MoveEventKind::Effect(effect) if effect.key == key => Some((event.at_s, &effect.params)),
        _ => None,
    });
    let first = found
        .next()
        .unwrap_or_else(|| panic!("move `{}` carries no `{key}` effect", spec.id));
    assert!(
        found.next().is_none(),
        "move `{}` carries more than one `{key}` effect",
        spec.id
    );
    first
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
