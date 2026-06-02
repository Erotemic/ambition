use crate::features::events::{
    GameplaySfxRequested, QuestAdvanceRequested, SetFlagRequested, SwitchActivated,
};
use bevy::prelude::{
    App, IntoScheduleConfigs, MessageReader, MessageWriter, Plugin, ResMut, Update,
};

/// Save writes first so quest conditions that read flags see them this same
/// frame. `on == true` also mirrors into `QuestAdvanceEvent::FlagSet`, matching
/// the former monolithic router behavior.
pub fn apply_flag_effects(
    mut effects: MessageReader<SetFlagRequested>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut quests: ResMut<crate::content::quest::QuestRegistry>,
) {
    for effect in effects.read() {
        if effect.on {
            quests.push_event(crate::quest::QuestAdvanceEvent::FlagSet(effect.id.clone()));
        }
        save.data_mut().set_flag(effect.id.clone(), effect.on);
    }
}

/// Structured quest events from gameplay (NPC talked, item collected, etc.).
pub fn apply_quest_effects(
    mut effects: MessageReader<QuestAdvanceRequested>,
    mut quests: ResMut<crate::content::quest::QuestRegistry>,
) {
    for effect in effects.read() {
        quests.push_event(effect.0.clone());
    }
}

/// Switch activations are gameplay events too. The activation is already
/// typed by the LDtk-to-ECS spawn path (see [`crate::features::SwitchFeature`]),
/// so this consumer just forwards it to the encounter queue and emits the
/// click SFX.
pub fn apply_switch_effects(
    mut effects: MessageReader<SwitchActivated>,
    mut switch_activations: ResMut<crate::encounter::SwitchActivationQueue>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    for effect in effects.read() {
        switch_activations.0.push(effect.activation.clone());
        sfx.write(crate::audio::SfxMessage::Play {
            id: ambition_sfx::ids::WORLD_SWITCH_TOGGLE,
            pos: effect.pos,
        });
    }
}

/// Standalone audio-only gameplay events. Presentation-shaped cues with richer
/// semantics should use their own typed messages; this reader only handles
/// bare SFX requests.
pub fn apply_gameplay_sfx_effects(
    mut effects: MessageReader<GameplaySfxRequested>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    for effect in effects.read() {
        sfx.write(crate::audio::SfxMessage::Play {
            id: effect.id,
            pos: effect.pos,
        });
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
                super::ecs::apply_npc_stimuli,
                super::ecs::apply_actor_stimuli,
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
    fn gameplay_effect_messages_remain_typed() {
        let flag = SetFlagRequested {
            id: "flag".into(),
            on: true,
        };
        let quest = QuestAdvanceRequested(crate::quest::QuestAdvanceEvent::NpcTalked("guide".into()));
        let switch = SwitchActivated {
            activation: crate::encounter::SwitchActivation {
                id: "goblin_encounter".into(),
                action: "ResetEncounter".into(),
                target_encounter: "goblin_encounter".into(),
            },
            pos: ae::Vec2::new(1.0, 2.0),
        };
        let sfx = GameplaySfxRequested {
            id: ambition_sfx::ids::PLAYER_DAMAGE,
            pos: ae::Vec2::new(5.0, 6.0),
        };

        assert!(flag.on);
        assert!(matches!(quest.0, crate::quest::QuestAdvanceEvent::NpcTalked(_)));
        assert_eq!(switch.pos, ae::Vec2::new(1.0, 2.0));
        assert_eq!(sfx.pos, ae::Vec2::new(5.0, 6.0));
    }
}
