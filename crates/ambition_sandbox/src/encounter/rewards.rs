use ambition_engine as ae;
use ambition_engine::AabbExt;

use super::{EncounterPhase, EncounterRegistry, EncounterSpec};

/// Drop the encounter's reward chest (if any) and clear the persisted
/// "looted" flag so the next clear pays out a fresh chest. Called by
/// the switch-reset re-arming branch; pure helper so unit tests can
/// drive the cycle without a Bevy app.
pub fn clear_encounter_reward(
    features: &mut crate::features::FeatureRuntime,
    save: &mut ae::SandboxSaveData,
    encounter_id: &str,
) {
    features.despawn_encounter_chest(encounter_id);
    let reward_flag = format!("encounter_{encounter_id}_reward_dropped");
    save.set_flag(reward_flag, false);
}

/// Save-flag id used to remember whether the player has already opened
/// (looted) a given encounter's reward chest. Persists across
/// save/load so a re-spawned chest correctly reads as opened.
pub fn encounter_reward_looted_flag(encounter_id: &str) -> String {
    format!("encounter_{encounter_id}_reward_dropped")
}

/// Position the reward chest is spawned at, given an encounter spec.
/// Bottom edge of the chest snaps to the trigger AABB's `max.y` (the
/// lower edge in y-down world space, which the LDtk authoring puts
/// on the arena floor). Pulled out as a helper so the placement
/// formula has one home and tests can pin it.
pub fn encounter_reward_chest_pos(spec: &EncounterSpec, chest_size: ae::Vec2) -> ae::Vec2 {
    let trigger = spec.trigger_aabb();
    ae::Vec2::new(trigger.center().x, trigger.max.y - chest_size.y * 0.5)
}

/// Idempotent reward-chest sync. For every encounter currently in
/// `Cleared` state with a loaded spec, ensure a chest with the
/// canonical `encounter_chest_<id>` id is in `features.chests` at
/// the on-floor position, with `chest.opened` mirroring the
/// persisted "looted" flag.
///
/// Runs each tick; cheap because:
///   - `spawn_chest` short-circuits on duplicate id;
///   - the registry usually has at most a few encounters loaded.
///
/// Called from `update_encounters_from_world` so it runs in the
/// same frame as the `Cleared` event AND on every subsequent
/// frame including the first one after save+reload.
pub fn sync_encounter_reward_chests(
    features: &mut crate::features::FeatureRuntime,
    save: &ae::SandboxSaveData,
    registry: &EncounterRegistry,
) {
    let chest_size = ae::Vec2::new(28.0, 28.0);
    for (encounter_id, state) in registry.encounters.iter() {
        if !matches!(state.phase, EncounterPhase::Cleared) {
            continue;
        }
        let Some(spec) = state.spec.as_ref() else {
            continue;
        };
        let chest_id = format!("encounter_chest_{encounter_id}");
        let chest_pos = encounter_reward_chest_pos(spec, chest_size);
        // `spawn_chest` is idempotent on the id, so re-running per
        // frame is a hash-set check after the first spawn.
        features.spawn_chest(
            chest_id.clone(),
            Some(ae::PickupKind::Health { amount: 2 }),
            chest_pos,
            chest_size,
        );
        // Mirror the persisted "looted" flag onto the live chest.
        // Without this, save+reload would re-spawn the chest as
        // closed even after the player already looted it.
        let looted = save.flag(&encounter_reward_looted_flag(encounter_id));
        if let Some(chest) = features.chests.iter_mut().find(|c| c.id == chest_id) {
            chest.opened = looted;
        }
    }
}
