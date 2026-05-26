//! Intro NPC dialogue ids.
//!
//! The actual dialogue content (speakers, lines, choices) moved to
//! `assets/data/dialogue/registry.ron` in the data-driven migration.
//! This module survives as the canonical list of intro-owned
//! dialogue ids so the LDtk content validator can approve
//! `NpcSpawn.dialogue_id` fields without forcing intro content to
//! escape its submodule.

/// Dialogue identifiers consumed by the LDtk `NpcSpawn.dialogue_id`
/// field for intro-room NPCs. Returned to the validator via
/// [`intro_dialogue_ids`]; matches the keys in
/// `assets/data/dialogue/registry.ron`. Used by `intro/tests.rs` as
/// the canonical "intro module owns these ids" list against which
/// the data registry is validated.
#[allow(
    dead_code,
    reason = "test-only ownership list; production code reads ids from the data registry"
)]
pub const INTRO_DIALOGUE_IDS: &[&str] = &[
    "creator_intro",
    "creator_final_normal",
    "creator_final_fast",
    "creator_final_impossible",
    "oiler_intro",
    "gate_janitor_ripple",
    "framebreaker_hardliner",
    "nazi_salvage_guard",
    "news_board_lab_incident",
    "manifest_kiosk_wrong_list",
    "alice_intro_stub",
    "bob_intro_stub",
    "oiler_post_stabilizer",
    "alice_after_bob_survey",
    "bob_after_report",
];

#[allow(dead_code, reason = "test-only accessor for the ownership list")]
pub fn intro_dialogue_ids() -> &'static [&'static str] {
    INTRO_DIALOGUE_IDS
}
