use super::*;
use crate::features::events::GameplayEffect;
use bevy::prelude::{MessageWriter, Res, ResMut, Resource};

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

/// Cross-system bus for typed gameplay effects that need to fan out to
/// resources outside the local feature/runtime tick.
///
/// Producers should enqueue complete `GameplayEffect` values here rather than
/// adding one more bespoke Vec to `FeatureEvents`. The drain system is the
/// central routing table for progression/save/switch/boss/NPC/audio effects.
#[derive(Resource, Default)]
pub struct FeatureEventBus {
    pub effects: Vec<GameplayEffect>,
}

impl FeatureEventBus {
    pub fn emit(&mut self, effect: GameplayEffect) {
        self.effects.push(effect);
    }

    pub fn extend<I>(&mut self, effects: I)
    where
        I: IntoIterator<Item = GameplayEffect>,
    {
        self.effects.extend(effects);
    }

    /// Enqueue every typed gameplay effect from a feature tick.
    ///
    /// Filtering belongs in the drain/router, not at the ingest site, so new
    /// variants get one obvious place to wire their downstream consumers.
    pub fn ingest(&mut self, events: &FeatureEvents) {
        self.effects.extend(events.effects.iter().cloned());
    }

    pub fn drain(&mut self) -> Vec<GameplayEffect> {
        std::mem::take(&mut self.effects)
    }
}

/// Bevy system: drain `FeatureEventBus` into downstream systems.
///
/// This is the typed gameplay event bus. It is intentionally scheduled after
/// `sandbox_update` and `update_projectiles`, but before encounter/boss/quest
/// progression consumers, so events emitted by the frame's sim tick are visible
/// to those systems in the same Update frame.
pub fn drain_feature_event_bus(
    mut bus: ResMut<FeatureEventBus>,
    mut switch_activations: ResMut<crate::encounter::SwitchActivationQueue>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut quests: ResMut<crate::quest::QuestRegistry>,
    mut boss_registry: ResMut<crate::boss_encounter::BossEncounterRegistry>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut cutscene_queue: ResMut<crate::cutscene::CutsceneTriggerQueue>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    let effects = bus.drain();

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

    // Switch activations are gameplay events too. Parse the authored payload
    // once at the bus boundary and feed the encounter queue before the
    // encounter sync systems run.
    for effect in &effects {
        if let GameplayEffect::ActivateSwitch { payload, pos } = effect {
            if let Some(activation) = crate::encounter::SwitchActivation::parse_custom(payload) {
                switch_activations.0.push(activation);
            }
            sfx.write(crate::audio::SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_SWITCH_TOGGLE,
                pos: *pos,
            });
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
    // today, but retaining the typed event keeps a single path for
    // future trace / hostility systems.
    for effect in &effects {
        if let GameplayEffect::StrikeNpc { .. } = effect {
            // Intentionally no-op today.
        }
    }

    // Standalone audio-only gameplay events. Presentation-shaped cues like
    // pickups/chests/breakables still come through `FeatureEvents` because
    // they already include concrete render/audio facts.
    for effect in &effects {
        if let GameplayEffect::PlaySfx { id, pos } = effect {
            sfx.write(crate::audio::SfxMessage::Play { id: *id, pos: *pos });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_ingests_all_typed_effect_variants() {
        let mut events = FeatureEvents::default();
        events.set_flag("flag", true);
        events.advance_quest(ae::QuestAdvanceEvent::NpcTalked("guide".into()));
        events.activate_switch("switch:mob_lab", ae::Vec2::new(1.0, 2.0));
        events.damage_boss("clockwork_warden", 2);
        events.strike_npc("guide", ae::Vec2::new(3.0, 4.0));
        events.play_sfx(ambition_sfx::ids::PLAYER_DAMAGE, ae::Vec2::new(5.0, 6.0));

        let mut bus = FeatureEventBus::default();
        bus.ingest(&events);

        assert_eq!(bus.effects.len(), 6);
        assert!(matches!(bus.effects[0], GameplayEffect::SetFlag { .. }));
        assert!(matches!(bus.effects[1], GameplayEffect::AdvanceQuest(_)));
        assert!(matches!(
            bus.effects[2],
            GameplayEffect::ActivateSwitch { .. }
        ));
        assert!(matches!(bus.effects[3], GameplayEffect::DamageBoss { .. }));
        assert!(matches!(bus.effects[4], GameplayEffect::StrikeNpc { .. }));
        assert!(matches!(bus.effects[5], GameplayEffect::PlaySfx { .. }));
    }
}
