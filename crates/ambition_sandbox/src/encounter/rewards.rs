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

#[cfg(test)]
mod rewards_tests {
    use super::*;
    use crate::interaction::PickupKind;

    fn spec_with_trigger(min: [f32; 2], size: [f32; 2]) -> EncounterSpec {
        EncounterSpec {
            id: "test_enc".into(),
            waves: Vec::new(),
            trigger_min: min,
            trigger_size: size,
            camera_zoom: 1.0,
            lock_wall: None,
            intro_seconds: 0.0,
            music_track: String::new(),
            reward: PickupKind::Health { amount: 2 },
        }
    }

    #[test]
    fn looted_flag_is_namespaced_by_encounter_id() {
        assert_eq!(
            encounter_reward_looted_flag("goblin_encounter"),
            "encounter_goblin_encounter_reward_dropped"
        );
        // Distinct encounters get distinct save keys.
        assert_ne!(
            encounter_reward_looted_flag("a"),
            encounter_reward_looted_flag("b")
        );
    }

    #[test]
    fn chest_centers_on_the_trigger_and_rests_on_its_floor() {
        // Trigger spans (100,100)..(300,180); chest is 28x28.
        let spec = spec_with_trigger([100.0, 100.0], [200.0, 80.0]);
        let pos = encounter_reward_chest_pos(&spec, ae::Vec2::new(28.0, 28.0));
        assert_eq!(pos.x, 200.0, "centered on the trigger in x");
        // Chest center sits half its height above the trigger's bottom edge,
        // so its bottom rests on the floor (y-down world space).
        assert_eq!(pos.y, 180.0 - 14.0);
    }
}
