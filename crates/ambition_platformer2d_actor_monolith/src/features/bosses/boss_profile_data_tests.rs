//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod boss_profile_data_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;

/// `assets/data/boss_profiles.ron` must carry a row for every
/// boss the codebase has a constructor for. Without this, the
/// `from_data` lookup would panic at the first spawn of a
/// missing boss.
#[test]
fn ron_carries_every_known_boss() {
    for id in [
        "clockwork_warden",
        "mockingbird",
        "gnu_ton_rider",
        "smirking_behemoth_boss",
    ] {
        // `from_data` panics with a clear message when the row is
        // missing (the registry static is private to behavior.rs).
        let _ = BossBehaviorProfile::from_data(crate::boss_encounter::test_boss_catalog(), id);
    }
}

/// Spot-check the legacy pre-data values for a divergent
/// archetype: the Clockwork Warden's macro tuning and attack
/// damage. Catches accidental tuning drift on the row the
/// player notices first.
#[test]
fn legacy_baseline_pins() {
    let warden = BossBehaviorProfile::clockwork_warden();
    assert_eq!(warden.id, "clockwork_warden");
    assert_eq!(warden.attack_damage, 2);
    assert_eq!(warden.body_damage, 1);
    assert!((warden.strike_speed_scale - 0.20).abs() < f32::EPSILON);
    assert!((warden.macro_tuning.too_close_distance - 110.0).abs() < f32::EPSILON);
    assert!((warden.macro_tuning.engage_max_duration_s - 9.0).abs() < f32::EPSILON);
    let gnu = BossBehaviorProfile::gnu_ton_rider();
    assert_eq!(gnu.body_damage, 0);
    assert_eq!(gnu.attacks.len(), 5);
    let mocker = BossBehaviorProfile::mockingbird();
    assert!(matches!(mocker.attack_pattern, BossAttackPattern::Cycle));
}
