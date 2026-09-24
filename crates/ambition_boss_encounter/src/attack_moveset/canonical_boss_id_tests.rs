use super::*;
use ambition_platformer2d_core as ae;

/// The PhaseScript brain wins over the display name: `canonical_boss_id_from`
/// reads the brain's `PhaseScript:clockwork_warden`, so the boss gets the
/// authored profile (and its music).
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

/// A boss built with a "System Boss" name and a PhaseScript brain gets the
/// clockwork_warden behavior: the canonical id is resolved before
/// `BossBehaviorProfile::for_authored_boss` is read. Otherwise the boss would
/// carry a generic placeholder behavior.
#[test]
fn boss_runtime_uses_phase_script_for_behavior_lookup() {
    let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0));
    let boss = crate::BossClusterScratch::new(
        crate::test_boss_catalog(),
        "boss_under_test",
        "System Boss",
        aabb,
        ambition_entity_catalog::placements::BossBrain::PhaseScript {
            script_id: "clockwork_warden".to_string(),
        },
    );
    assert_eq!(boss.config.behavior.id, "clockwork_warden");
    // The Gradient Sentinel macro tuning is non-trivial (chase/retreat
    // thresholds non-zero), which the generic profile does not set.
    assert!(
        boss.config.behavior.macro_tuning.is_enabled(),
        "clockwork_warden behavior should carry macro tuning",
    );
}
