use super::*;
use crate::features::events::GameplayEffect;
use bevy::prelude::{MessageReader, MessageWriter, ResMut};

/// Forward a legacy `FeatureEvents` batch into Bevy's typed message stream.
///
/// Phase-1 strangler rule: this helper is the compatibility bridge between the
/// old phase-helper world and the new ECS message world. New Bevy-native
/// feature systems should write `GameplayEffect` (or a more specific domain
/// message) directly instead of routing through `FeatureEvents` or rebuilding a
/// custom bus resource.
pub fn write_feature_effects(
    writer: &mut MessageWriter<GameplayEffect>,
    events: &FeatureEvents,
) {
    writer.write_batch(events.effects.iter().cloned());
}

/// Save writes first so quest conditions that read flags see them this same
/// frame. `on == true` also mirrors into `QuestAdvanceEvent::FlagSet`, matching
/// the former monolithic router behavior.
pub fn apply_flag_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut save: ResMut<crate::save::SandboxSave>,
    mut quests: ResMut<crate::quest::QuestRegistry>,
) {
    for effect in effects.read() {
        if let GameplayEffect::SetFlag { id, on } = effect {
            if *on {
                quests.push_event(ae::QuestAdvanceEvent::FlagSet(id.clone()));
            }
            save.data_mut().set_flag(id.clone(), *on);
        }
    }
}

/// Structured quest events from gameplay (NPC talked, item collected, etc.).
pub fn apply_quest_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut quests: ResMut<crate::quest::QuestRegistry>,
) {
    for effect in effects.read() {
        if let GameplayEffect::AdvanceQuest(event) = effect {
            quests.push_event(event.clone());
        }
    }
}

/// Switch activations are gameplay events too. Parse the authored payload once
/// at the message boundary and feed the encounter queue before the encounter
/// sync systems run.
pub fn apply_switch_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut switch_activations: ResMut<crate::encounter::SwitchActivationQueue>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    for effect in effects.read() {
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
}

/// Boss damage routes through the boss encounter machine.
pub fn apply_boss_damage_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut boss_registry: ResMut<crate::boss_encounter::BossEncounterRegistry>,
    mut music_request: ResMut<crate::encounter::EncounterMusicRequest>,
    mut cutscene_queue: ResMut<crate::cutscene::CutsceneTriggerQueue>,
    mut banner: ResMut<crate::features::GameplayBanner>,
) {
    for effect in effects.read() {
        if let GameplayEffect::DamageBoss { boss_id, amount } = effect {
            crate::boss_encounter::record_boss_damage(
                &mut boss_registry,
                &mut music_request,
                &mut cutscene_queue,
                &mut banner,
                boss_id,
                *amount,
            );
        }
    }
}

/// NPC strikes are reportable for the trace; the actual hostility flip happens
/// inside `apply_player_attack`. No additional action today, but retaining the
/// typed reader keeps a single scheduled hook for future trace / hostility
/// systems without rebuilding a god-router.
pub fn apply_npc_strike_effects(mut effects: MessageReader<GameplayEffect>) {
    for effect in effects.read() {
        if let GameplayEffect::StrikeNpc { .. } = effect {
            // Intentionally no-op today.
        }
    }
}

/// Standalone audio-only gameplay events. Presentation-shaped cues like
/// pickups/chests/breakables still come through `FeatureEvents` because they
/// already include concrete render/audio facts.
pub fn apply_gameplay_sfx_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    for effect in effects.read() {
        if let GameplayEffect::PlaySfx { id, pos } = effect {
            sfx.write(crate::audio::SfxMessage::Play { id: *id, pos: *pos });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_event_batches_forward_all_typed_effect_variants() {
        let mut events = FeatureEvents::default();
        events.set_flag("flag", true);
        events.advance_quest(ae::QuestAdvanceEvent::NpcTalked("guide".into()));
        events.activate_switch("switch:mob_lab", ae::Vec2::new(1.0, 2.0));
        events.damage_boss("clockwork_warden", 2);
        events.strike_npc("guide", ae::Vec2::new(3.0, 4.0));
        events.play_sfx(ambition_sfx::ids::PLAYER_DAMAGE, ae::Vec2::new(5.0, 6.0));

        assert_eq!(events.effects.len(), 6);
        assert!(matches!(events.effects[0], GameplayEffect::SetFlag { .. }));
        assert!(matches!(events.effects[1], GameplayEffect::AdvanceQuest(_)));
        assert!(matches!(
            events.effects[2],
            GameplayEffect::ActivateSwitch { .. }
        ));
        assert!(matches!(events.effects[3], GameplayEffect::DamageBoss { .. }));
        assert!(matches!(events.effects[4], GameplayEffect::StrikeNpc { .. }));
        assert!(matches!(events.effects[5], GameplayEffect::PlaySfx { .. }));
    }
}
