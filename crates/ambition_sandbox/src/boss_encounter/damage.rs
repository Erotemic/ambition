use crate::cutscene::CutsceneTriggerQueue;

use super::{events::publish_events, BossEncounterRegistry};

/// Helper: feed a damage delta into the encounter machine. Called by
/// `apply_player_attack` after damage hits the BossRuntime.
pub fn record_boss_damage(
    registry: &mut BossEncounterRegistry,
    music_request: &mut crate::encounter::EncounterMusicRequest,
    cutscene_queue: &mut CutsceneTriggerQueue,
    features: &mut crate::features::FeatureRuntime,
    boss_runtime_id: &str,
    damage: i32,
) {
    let Some((id, _)) = registry
        .runtime_ids
        .iter()
        .find(|(_id, runtime_id)| runtime_id.as_str() == boss_runtime_id)
        .map(|(id, runtime_id)| (id.clone(), runtime_id.clone()))
    else {
        return;
    };
    let Some(state) = registry.encounters.get_mut(&id) else {
        return;
    };
    let evs = state.apply_player_damage(damage);
    publish_events(&id, &evs, music_request, cutscene_queue, features);
}
