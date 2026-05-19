use super::*;
use crate::features::events::GameplayEffect;
use bevy::prelude::{MessageReader, MessageWriter, ResMut};

/// Save writes first so quest conditions that read flags see them this same
/// frame. `on == true` also mirrors into `QuestAdvanceEvent::FlagSet`, matching
/// the former monolithic router behavior.
pub fn apply_flag_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut quests: ResMut<crate::content::quest::QuestRegistry>,
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
    mut quests: ResMut<crate::content::quest::QuestRegistry>,
) {
    for effect in effects.read() {
        if let GameplayEffect::AdvanceQuest(event) = effect {
            quests.push_event(event.clone());
        }
    }
}

/// Switch activations are gameplay events too. The activation is already
/// typed by the LDtk-to-ECS spawn path (see [`crate::features::SwitchFeature`]),
/// so this consumer just forwards it to the encounter queue and emits the
/// click SFX.
pub fn apply_switch_effects(
    mut effects: MessageReader<GameplayEffect>,
    mut switch_activations: ResMut<crate::encounter::SwitchActivationQueue>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    for effect in effects.read() {
        if let GameplayEffect::ActivateSwitch { activation, pos } = effect {
            switch_activations.0.push(activation.clone());
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
    mut cutscene_queue: ResMut<crate::presentation::cutscene::CutsceneTriggerQueue>,
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
/// inside feature damage resolution. No additional action today, but retaining
/// the typed reader keeps a single scheduled hook for future trace / hostility
/// systems without rebuilding a god-router.
pub fn apply_npc_strike_effects(mut effects: MessageReader<GameplayEffect>) {
    for effect in effects.read() {
        if let GameplayEffect::StrikeNpc { .. } = effect {
            // Intentionally no-op today.
        }
    }
}

/// Standalone audio-only gameplay events. Presentation-shaped cues with richer
/// semantics should use their own typed messages; this reader only handles
/// bare SFX requests.
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
    fn gameplay_effect_variants_remain_typed_and_orderable() {
        let effects = [
            GameplayEffect::SetFlag {
                id: "flag".into(),
                on: true,
            },
            GameplayEffect::AdvanceQuest(ae::QuestAdvanceEvent::NpcTalked("guide".into())),
            GameplayEffect::ActivateSwitch {
                activation: crate::encounter::SwitchActivation {
                    id: "mob_lab".into(),
                    action: "ResetEncounter".into(),
                    target_encounter: "mob_lab".into(),
                },
                pos: ae::Vec2::new(1.0, 2.0),
            },
            GameplayEffect::DamageBoss {
                boss_id: "clockwork_warden".into(),
                amount: 2,
            },
            GameplayEffect::StrikeNpc {
                npc_id: "guide".into(),
                pos: ae::Vec2::new(3.0, 4.0),
            },
            GameplayEffect::PlaySfx {
                id: ambition_sfx::ids::PLAYER_DAMAGE,
                pos: ae::Vec2::new(5.0, 6.0),
            },
        ];

        assert!(matches!(effects[0], GameplayEffect::SetFlag { .. }));
        assert!(matches!(effects[1], GameplayEffect::AdvanceQuest(_)));
        assert!(matches!(effects[2], GameplayEffect::ActivateSwitch { .. }));
        assert!(matches!(effects[3], GameplayEffect::DamageBoss { .. }));
        assert!(matches!(effects[4], GameplayEffect::StrikeNpc { .. }));
        assert!(matches!(effects[5], GameplayEffect::PlaySfx { .. }));
        assert_eq!(effects.len(), 6);
    }
}
