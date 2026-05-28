use crate::engine_core as ae;

use super::EncounterSpec;

/// Save-flag id used to remember whether the player has already opened
/// (looted) a given encounter's reward chest. Persists across
/// save/load so a re-spawned chest correctly reads as opened.
pub fn encounter_reward_looted_flag(encounter_id: &str) -> String {
    format!("encounter_{encounter_id}_reward_dropped")
}

/// Position the reward chest is spawned at, given an encounter spec.
/// Bottom edge of the chest snaps to the trigger AABB's `max.y` (the
/// lower edge in y-down world space, which the LDtk authoring puts
/// on the arena floor).
pub fn encounter_reward_chest_pos(spec: &EncounterSpec, chest_size: ae::Vec2) -> ae::Vec2 {
    use crate::engine_core::AabbExt;
    let trigger = spec.trigger_aabb();
    ae::Vec2::new(trigger.center().x, trigger.max.y - chest_size.y * 0.5)
}
