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

/// Cross-system bus for typed feature effects that need to fan out to
/// resources `sandbox_update` doesn't hold. Refilled each frame in
/// `feature_runtime_phase`; drained by `drain_feature_event_bus`.
#[derive(Resource, Default)]
pub struct FeatureEventBus {
    pub effects: Vec<GameplayEffect>,
}

impl FeatureEventBus {
    pub fn ingest(&mut self, events: &FeatureEvents) {
        self.effects.extend(
            events
                .effects
                .iter()
                .filter(|effect| {
                    matches!(
                        effect,
                        GameplayEffect::SetFlag { .. }
                            | GameplayEffect::AdvanceQuest(_)
                            | GameplayEffect::DamageBoss { .. }
                            | GameplayEffect::StrikeNpc { .. }
                    )
                })
                .cloned(),
        );
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
    let effects = std::mem::take(&mut bus.effects);

    // Flag writes first so quest conditions that read flags see them
    // this same frame.
    for effect in &effects {
        if let GameplayEffect::SetFlag { id, on } = effect {
            if *on {
                quests.push_event(ae::QuestAdvanceEvent::FlagSet(id.clone()));
            }
            save.data_mut().set_flag(id.clone(), *on);
        }
    }

    // Quest advance events from gameplay (NPC talked, item collected, etc.).
    for effect in &effects {
        if let GameplayEffect::AdvanceQuest(event) = effect {
            quests.push_event(event.clone());
        }
    }

    // Boss damage routes through the boss encounter machine.
    for effect in &effects {
        if let GameplayEffect::DamageBoss { boss_id, amount } = effect {
            crate::boss_encounter::record_boss_damage(
                &mut boss_registry,
                &mut music_request,
                &mut cutscene_queue,
                &mut runtime.features,
                boss_id,
                *amount,
            );
        }
    }

    // NPC strikes are reportable for the trace; the actual hostility
    // flip happens inside `apply_player_attack`. No additional action
    // today, but retaining the typed effect keeps a single path for
    // future trace / hostility systems.
}
