//! Intro NPC dialogue ids.
//!
//! This module survives as the canonical list of intro-owned dialogue ids so the LDtk content
//! validator can approve `NpcSpawn.dialogue_id` fields without forcing intro content to escape
//! its submodule.

/// Dialogue identifiers consumed by the LDtk `NpcSpawn.dialogue_id`
/// field for intro-room NPCs. Returned to the validator via
/// [`intro_dialogue_ids`]. Used by `intro/tests.rs` as the canonical "intro
/// module owns these ids" list, which the validator's known ids must contain.
///
/// An id here is one an NPC can START. `oiler_post_stabilizer` and
/// `alice_after_bob_survey` are not here: they exist only as `__1`/`__2`
/// nodes that another node reaches by `<<jump>>`.
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
    "intro_lab_raider",
    "intro_salvage_guard",
    "news_board_lab_incident",
    "manifest_kiosk_wrong_list",
    "alice_intro_stub",
    "bob_intro_stub",
];

#[allow(dead_code, reason = "test-only accessor for the ownership list")]
pub fn intro_dialogue_ids() -> &'static [&'static str] {
    INTRO_DIALOGUE_IDS
}
