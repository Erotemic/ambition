//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod canonical_boss_id_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;
use ambition_engine_core as ae;

/// PhaseScript brain wins over display name. The user-reported
/// bug: BossSpawn named "System Boss" in `first_system_boss`
/// derived encounter_id "system_boss" (no profile, no music).
/// With `canonical_boss_id_from` reading the brain's
/// `PhaseScript:clockwork_warden` it resolves to the
/// authored profile and the boss fight gets its violin music.
#[test]
fn phase_script_brain_wins_over_display_name() {
    let id = canonical_boss_id_from(
        "System Boss",
        &ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    assert_eq!(id, "clockwork_warden");
}

/// Empty PhaseScript falls back to the display name.
#[test]
fn empty_phase_script_falls_back_to_name() {
    let id = canonical_boss_id_from(
        "System Boss",
        &ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: String::new(),
        },
    );
    assert_eq!(id, "system_boss");
}

/// Custom brain with a non-empty label is treated like a name
/// (gets normalized to an encounter_id slug).
#[test]
fn custom_brain_label_becomes_encounter_id_slug() {
    let id = canonical_boss_id_from(
        "Display",
        &ambition_entity_catalog::placements::BossBrain::Custom("Clockwork Warden".to_string()),
    );
    assert_eq!(id, "clockwork_warden");
}

/// Dormant brain falls back to the display name.
#[test]
fn dormant_brain_falls_back_to_name() {
    let id = canonical_boss_id_from(
        "Clockwork Warden",
        &ambition_entity_catalog::placements::BossBrain::Dormant,
    );
    assert_eq!(id, "clockwork_warden");
}

/// BossRuntime constructed with a "System Boss" name + PhaseScript
/// brain ends up with the clockwork_warden behavior — the runtime
/// resolves the canonical id before reading
/// `BossBehaviorProfile::for_authored_boss`. Without this fix the
/// runtime would carry a generic placeholder behavior.
#[test]
fn boss_runtime_uses_phase_script_for_behavior_lookup() {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0));
    let boss = super::super::ecs::boss_clusters::BossClusterScratch::new(
        crate::boss_encounter::test_boss_catalog(),
        "boss_under_test",
        "System Boss",
        aabb,
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    assert_eq!(boss.config.behavior.id, "clockwork_warden");
    // Sanity: the Gradient Sentinel macro tuning is non-trivial
    // (chase/retreat thresholds non-zero), which the generic
    // boss profile doesn't set.
    assert!(
        boss.config.behavior.macro_tuning.is_enabled(),
        "clockwork_warden behavior should carry macro tuning",
    );
}
