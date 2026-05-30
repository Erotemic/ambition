use crate::features::events::GameplayEffect;
use bevy::prelude::{
    App, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, ResMut, Update,
};

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
                quests.push_event(crate::quest::QuestAdvanceEvent::FlagSet(id.clone()));
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

/// Boss damage events — drain the message queue.
///
/// After OVERNIGHT-TODO #8, the actual damage application happens
/// inline inside [`apply_feature_hit_events`] (see
/// `content/features/ecs/damage.rs`), which calls `record_boss_damage`
/// directly so the engine `BossEncounterState` is the source of truth
/// for HP. This reader stays as a typed seam: future tracing /
/// quest / replay hooks can subscribe to `GameplayEffect::DamageBoss`
/// without re-routing through the boss encounter registry. Today the
/// body is a no-op apart from draining the queue so the message
/// buffer doesn't grow unbounded.
pub fn apply_boss_damage_effects(mut effects: MessageReader<GameplayEffect>) {
    for effect in effects.read() {
        if let GameplayEffect::DamageBoss { .. } = effect {
            // Engine state already updated; nothing to do here.
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

/// Module-local Bevy plugin: schedules the gameplay-effect bus chain
/// (`apply_flag_effects` → `apply_quest_effects` → … →
/// `apply_gameplay_sfx_effects`) into
/// [`crate::app::SandboxSet::GameplayEffects`].
///
/// Carved out of `app/plugins.rs::register_gameplay_effects_systems`
/// per OVERNIGHT-TODO #6. Every reader system in this chain lives in
/// this file (`bus.rs`), so this is the right place to own the
/// schedule registration.
pub struct GameplayEffectsSchedulePlugin;

impl Plugin for GameplayEffectsSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_flag_effects,
                apply_quest_effects,
                apply_switch_effects,
                apply_boss_damage_effects,
                apply_npc_strike_effects,
                apply_gameplay_sfx_effects,
            )
                .chain()
                .in_set(crate::app::SandboxSet::GameplayEffects),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_core as ae;

    #[test]
    fn gameplay_effect_variants_remain_typed_and_orderable() {
        let effects = [
            GameplayEffect::SetFlag {
                id: "flag".into(),
                on: true,
            },
            GameplayEffect::AdvanceQuest(crate::quest::QuestAdvanceEvent::NpcTalked(
                "guide".into(),
            )),
            GameplayEffect::ActivateSwitch {
                activation: crate::encounter::SwitchActivation {
                    id: "goblin_encounter".into(),
                    action: "ResetEncounter".into(),
                    target_encounter: "goblin_encounter".into(),
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
