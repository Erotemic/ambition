//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod movement_tuning_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::{resolve_movement_for, CharacterRosterAssemblyError};
use crate::combat::{BodyMovementPatch, BodyMovementTuning};
use ambition_entity_catalog::placements::CharacterBrain;
use std::collections::BTreeMap;

/// The composition primitive: `Some` knobs override, `None` knobs inherit.
#[test]
fn patch_apply_onto_overrides_only_specified_knobs() {
    let patch = BodyMovementPatch {
        gravity: Some(700.0),
        ..Default::default()
    };
    let r = patch.apply_onto(BodyMovementTuning::BASELINE);
    assert_eq!(r.gravity, 700.0, "specified knob overrides");
    assert_eq!(
        r.max_fall_speed,
        BodyMovementTuning::BASELINE.max_fall_speed,
        "unspecified knob inherits the base",
    );
}

/// The hierarchy folds BASELINE <- parent <- child: a child inherits its
/// parent's overrides AND the baseline, then layers its own.
#[test]
fn inheritance_chain_composes() {
    let mut raw: BTreeMap<String, (BodyMovementPatch, Option<String>)> = BTreeMap::new();
    raw.insert(
        "parent".to_string(),
        (
            BodyMovementPatch {
                gravity: Some(700.0),
                ..Default::default()
            },
            None,
        ),
    );
    raw.insert(
        "child".to_string(),
        (
            BodyMovementPatch {
                jump_speed: Some(900.0),
                ..Default::default()
            },
            Some("parent".to_string()),
        ),
    );
    let owners = BTreeMap::from([
        ("parent".to_string(), "p".to_string()),
        ("child".to_string(), "p".to_string()),
    ]);
    let child = resolve_movement_for(
        &raw,
        &owners,
        "child",
        &mut vec!["child".to_string()],
    )
    .unwrap();
    assert_eq!(child.gravity, 700.0, "inherited from the parent's override");
    assert_eq!(child.jump_speed, 900.0, "the child's own override");
    assert_eq!(
        child.run_accel,
        BodyMovementTuning::BASELINE.run_accel,
        "knob neither set inherits the baseline",
    );
}

/// A cyclic / self-referential `inherits` refuses publication and reports the
/// complete loop instead of silently degrading to the baseline.
#[test]
fn inheritance_cycle_is_an_error() {
    let raw = BTreeMap::from([
        (
            "a".to_string(),
            (BodyMovementPatch::default(), Some("b".to_string())),
        ),
        (
            "b".to_string(),
            (BodyMovementPatch::default(), Some("a".to_string())),
        ),
    ]);
    let owners = BTreeMap::from([
        ("a".to_string(), "p".to_string()),
        ("b".to_string(), "p".to_string()),
    ]);
    let error = resolve_movement_for(&raw, &owners, "a", &mut vec!["a".to_string()])
        .expect_err("cycle must reject the candidate roster");
    assert_eq!(
        error,
        CharacterRosterAssemblyError::MovementInheritanceCycle {
            provider_id: "p".to_string(),
            chain: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        }
    );
}

/// End-to-end through the real roster loader: an archetype with no movement
/// overrides resolves to the baseline (behavior-preserving data move), and the
/// resolved tuning is what the runtime `ActorTuning` carries.
#[test]
fn roster_resolves_baseline_for_unauthored_movement() {
    let roster = super::test_roster();
    let combatant = roster
        .spec_for_brain(&CharacterBrain::Custom("combatant".to_string()))
        .tuning()
        .movement;
    assert_eq!(
        combatant,
        BodyMovementTuning::BASELINE,
        "a row without a `movement` patch resolves to the generic baseline",
    );
}
