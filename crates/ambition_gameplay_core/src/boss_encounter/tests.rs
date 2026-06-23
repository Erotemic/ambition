//! Tests for `encounter_id_from_name` slugging and registry profile/runtime linking.

use super::ids::MOCKINGBIRD_ENCOUNTER_ID;
use super::*;

#[test]
fn encounter_id_from_name_normalizes_capitalization_and_spaces() {
    assert_eq!(
        encounter_id_from_name("Clockwork Warden"),
        "clockwork_warden"
    );
    assert_eq!(
        encounter_id_from_name("Gradient Sentinel"),
        "gradient_sentinel"
    );
    assert_eq!(
        encounter_id_from_name("BOSS-of-the-Year!"),
        "boss_of_the_year"
    );
    assert_eq!(encounter_id_from_name("   "), "boss");
}

#[test]
fn encounter_id_from_name_handles_empty_input() {
    assert_eq!(encounter_id_from_name(""), "boss");
}

#[test]
fn encounter_id_from_name_collapses_consecutive_separators() {
    assert_eq!(encounter_id_from_name("a   b"), "a_b");
    assert_eq!(encounter_id_from_name("a---b"), "a_b");
    assert_eq!(encounter_id_from_name("a -+= b"), "a_b");
}

#[test]
fn encounter_id_from_name_strips_trailing_underscores() {
    assert_eq!(encounter_id_from_name("Boss!"), "boss");
    assert_eq!(encounter_id_from_name("Boss   "), "boss");
    assert_eq!(encounter_id_from_name("Boss--"), "boss");
    assert_eq!(encounter_id_from_name("Boss_"), "boss");
}

#[test]
fn encounter_id_from_name_preserves_alphanumeric_runs() {
    assert_eq!(encounter_id_from_name("R2D2"), "r2d2");
    assert_eq!(encounter_id_from_name("phase4-monster"), "phase4_monster");
}

#[test]
fn encounter_id_from_name_drops_non_ascii() {
    assert_eq!(encounter_id_from_name("日本語 Boss"), "boss");
    assert_eq!(encounter_id_from_name("Ω-omega"), "omega");
}

// Verify the mockingbird reward profile registers into the read-only data
// catalog — purely checking the registry shape (R3 deleted the live map +
// runtime-id links; the boss's live state is entity-local now).
#[test]
fn mockingbird_profile_registers_in_the_catalog() {
    let mut registry = BossEncounterRegistry::default();
    registry.ensure_profile(BossProfile::from_id("mockingbird").expect("mockingbird is authored"));
    assert!(registry.profiles.contains_key(MOCKINGBIRD_ENCOUNTER_ID));
    assert_eq!(
        registry.profile(MOCKINGBIRD_ENCOUNTER_ID).map(|p| p.id.as_str()),
        Some(MOCKINGBIRD_ENCOUNTER_ID)
    );
}
