use super::*;

/// Apply save-derived state (NPC hostility, boss defeats) onto the
/// live `FeatureRuntime`. Public free function so room-load paths
/// that already hold the save can apply it inline; a Bevy system
/// (`sync_features_with_save`) calls it each frame as a safety net.
pub fn apply_save_to_features(features: &mut FeatureRuntime, save: &ae::SandboxSaveData) {
    features.apply_save(save);
}

/// Bevy system: keep the feature runtime in sync with the save
/// resource. Runs each frame; cheap idempotent linear pass.
pub fn sync_features_with_save(
    mut runtime: ResMut<crate::SandboxRuntime>,
    save: Res<crate::save::SandboxSave>,
) {
    apply_save_to_features(&mut runtime.features, save.data());
}

/// Cross-system bus for feature events that need to fan out to
/// resources `sandbox_update` doesn't hold. Refilled each frame in
/// `feature_runtime_phase`; drained by `drain_feature_event_bus`.
#[derive(Resource, Default)]
pub struct FeatureEventBus {
    pub boss_damage: Vec<(String, i32)>,
    pub npc_struck: Vec<(String, ae::Vec2)>,
    pub quest_advance: Vec<ae::QuestAdvanceEvent>,
    pub flag_writes: Vec<(String, bool)>,
}

impl FeatureEventBus {
    pub fn ingest(&mut self, events: &FeatureEvents) {
        self.boss_damage.extend(events.boss_damage.iter().cloned());
        self.npc_struck.extend(events.npc_struck.iter().cloned());
        self.quest_advance
            .extend(events.quest_advance.iter().cloned());
        self.flag_writes.extend(events.flag_writes.iter().cloned());
    }
}

/// Bevy system: drain `FeatureEventBus` into the right downstream
/// systems. Splits the work across resources so `sandbox_update`
/// stays under the system-param limit.
pub fn drain_feature_event_bus(
    mut bus: ResMut<FeatureEventBus>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut quests: ResMut<crate::quest::QuestRegistry>,
    mut boss_registry: ResMut<crate::boss_encounter::BossEncounterRegistry>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut cutscene_queue: ResMut<crate::cutscene::CutsceneTriggerQueue>,
) {
    // Flag writes first so quest conditions that read flags see them
    // this same frame.
    let flags = std::mem::take(&mut bus.flag_writes);
    for (id, on) in flags {
        // Mirror flag write into a quest advance event so any quest
        // step keyed on this flag can react.
        if on {
            quests.push_event(ae::QuestAdvanceEvent::FlagSet(id.clone()));
        }
        save.data_mut().set_flag(id, on);
    }
    // Quest advance events from gameplay (NPC talked, etc.).
    let advances = std::mem::take(&mut bus.quest_advance);
    for ev in advances {
        quests.push_event(ev);
    }
    // Boss damage routes through the boss encounter machine.
    let boss_damage = std::mem::take(&mut bus.boss_damage);
    for (boss_id, amount) in boss_damage {
        crate::boss_encounter::record_boss_damage(
            &mut boss_registry,
            &mut music_request,
            &mut cutscene_queue,
            &mut runtime.features,
            &boss_id,
            amount,
        );
    }
    // NPC strikes are reportable for the trace; the actual hostility
    // flip happens inside `apply_player_attack`. Drain to avoid
    // accumulation.
    bus.npc_struck.clear();
}
