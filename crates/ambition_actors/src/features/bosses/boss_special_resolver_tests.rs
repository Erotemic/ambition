//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod boss_special_resolver_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;

/// Every special-flavored profile must map to a Some(spec) — otherwise
/// the boss tick will emit no Special message for that beat and the
/// schedule silently degrades. Pin the mapping so future schedule
/// edits can't introduce a profile without its consumer wiring.
#[test]
fn every_special_profile_resolves_to_a_spec_for_gradient_sentinel() {
    use ambition_characters::brain::BossAttackProfile;
    for key in [
        "overfit_volley",
        "minima_trap",
        "saddle_point",
        "gradient_cascade",
    ] {
        let profile = BossAttackProfile::Special(key.into());
        assert!(
            boss_special_for_profile(&profile).is_some(),
            "{profile:?} must resolve to a spec for Gradient Sentinel",
        );
    }
}

/// GNU-ton's apple rain still resolves through the open seam: the
/// `Special("apple_rain")` beat maps to a `Special` spec carrying the
/// verbatim key, which the content apple-rain Technique recognizes.
#[test]
fn gnu_apple_rain_profile_resolves_to_apple_rain_spec_for_gnu_ton() {
    use ambition_characters::brain::{BossAttackProfile, SpecialActionSpec};
    match boss_special_for_profile(&BossAttackProfile::Special("apple_rain".into())) {
        Some(SpecialActionSpec::Special(key)) => assert_eq!(key, "apple_rain"),
        other => panic!("expected Special(apple_rain) spec, got {other:?}"),
    }
}

/// Ordinary melee-style profiles return None — they don't go
/// through the Special path; their damage routes via
/// `boss_attack_damage` reading `BossAttackState` directly.
#[test]
fn ordinary_profiles_resolve_to_none() {
    use ambition_characters::brain::BossAttackProfile;
    for profile in [
        BossAttackProfile::Strike("floor_slam".to_string()),
        BossAttackProfile::Strike("side_sweep".to_string()),
        BossAttackProfile::Strike("full_body_pulse".to_string()),
        BossAttackProfile::Strike("hazard_column".to_string()),
    ] {
        assert!(
            boss_special_for_profile(&profile).is_none(),
            "{profile:?} should not have a Special spec",
        );
    }
}
