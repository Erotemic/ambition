//! Player → ECS feature interaction (peaceful NPC dialogue, switches).
//!
//! Chests stay in `open_ecs_chests` because they have their own
//! reward/persistence path; this system covers the conversational
//! and switch-activation interactions that share the
//! `PlayerInteractionState` buffered-press contract.

use super::*;

/// Handle interactions with ECS switches and peaceful NPCs. Chests stay in
/// `open_ecs_chests` because they have their own reward/persistence path.
pub fn interact_ecs_actors_and_switches(
    mut dialogue: ResMut<crate::dialog::DialogState>,
    mut next_mode: ResMut<NextState<crate::GameMode>>,
    mut banner: ResMut<GameplayBanner>,
    mut player: Query<
        (
            &crate::player::BodyKinematics,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerAnimState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    // Talkable actors carry the shared `ActorInteraction` payload (dialogue
    // is an actor capability, not an NPC type). Dialogue is offered only to a
    // PEACEFUL talkable actor — a provoked one keeps its `ActorInteraction`
    // but its `ActorDisposition::Hostile` gates dialogue off.
    actors: Query<
        (
            &CenteredAabb,
            &ActorDisposition,
            &ActorIdentity,
            &ActorInteraction,
        ),
        With<FeatureSimEntity>,
    >,
    mut switches: Query<
        (
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            &SwitchFeature,
            &mut SwitchOn,
        ),
        With<FeatureSimEntity>,
    >,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut quest_advance: MessageWriter<QuestAdvanceRequested>,
    mut switch_activated: MessageWriter<SwitchActivated>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    // Iterate every player's buffered-interact state so each player
    // can talk to an NPC or activate a switch independently. The NPC
    // dialogue gate is global (one DialogState resource), so when one
    // player engages dialogue the loop short-circuits — a future
    // per-player dialogue surface (OVERNIGHT-TODO #17) would let
    // simultaneous NPC interactions land per-player. Single-player
    // behavior preserved because the iterator has one entity today.
    // How long the player's `Interact` pose holds after the interaction
    // commits. Short enough that the gesture clears before dialogue UI
    // or the room transition takes camera focus.
    const INTERACT_ANIM_HOLD_SECS: f32 = 0.28;
    for (player_kin, mut interaction, mut anim) in &mut player {
        if !interaction.buffered() {
            continue;
        }
        let player_aabb = player_kin.aabb();
        let mut consumed = false;
        for (aabb, disposition, identity, interaction_payload) in &actors {
            if disposition.is_hostile() {
                continue;
            }
            let interactable = &interaction_payload.interactable;
            if !aabb.aabb().strict_intersects(player_aabb) {
                continue;
            }
            interaction.clear();
            anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
            banner.show(
                super::super::npcs::npc_message(interactable, &identity.name, false),
                2.6,
            );
            let request = super::super::npcs::npc_dialogue_request(
                interactable,
                &identity.name,
                &identity.id,
            );
            dialogue.start(&request.dialogue_id, &request.npc_name);
            next_mode.set(crate::GameMode::Dialogue);
            quest_advance.write(QuestAdvanceRequested(
                crate::quest::QuestAdvanceEvent::NpcTalked(identity.id.clone()),
            ));
            set_flag.write(SetFlagRequested {
                id: "met_any_hub_npc".into(),
                on: true,
            });
            set_flag.write(SetFlagRequested {
                id: format!("npc_{}_talked", request.dialogue_id),
                on: true,
            });
            vfx.write(VfxMessage::Burst {
                pos: aabb.center,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            // Dialogue is a global mode flip; once one player engages
            // it the loop short-circuits — no other player can also
            // start a dialogue this frame. Return entirely (skip the
            // switch loop too) to match the prior single() semantic.
            return;
        }
        for (_id, name, aabb, switch, mut on) in &mut switches {
            if !aabb.aabb().strict_intersects(player_aabb) {
                continue;
            }
            interaction.clear();
            anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
            banner.show(format!("activated {}", name.0.as_str()), 2.6);
            on.0 = true;
            switch_activated.write(SwitchActivated {
                activation: switch.activation.clone(),
                pos: aabb.center,
            });
            vfx.write(VfxMessage::Burst {
                pos: aabb.center,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            consumed = true;
            break;
        }
        if consumed {
            // Switch activation is per-target; once this player
            // flipped a switch we don't keep checking actors / other
            // switches for them this tick (matches the prior
            // single-`return`-after-switch semantic).
            continue;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;
    use crate::features::{
        CenteredAabb, FeatureId, FeatureName, FeatureSimEntity, SwitchFeature, SwitchOn,
    };
    use bevy::prelude::{App, NextState, Update};

    fn spawn_interaction_player(app: &mut App, pos: ae::Vec2) {
        let mut scratch = crate::player::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
        scratch.ground.on_ground = true;
        let mut bundle = crate::player::PlayerSimulationBundle::from_scratch(
            scratch,
            ambition_characters::actor::Health::new(10),
        );
        bundle.interaction.interact_buffer_timer = 0.15;
        app.world_mut().spawn(bundle);
    }

    #[test]
    fn buffered_interact_toggles_an_adjacent_switch() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(crate::dialog::DialogState::default());
        app.insert_resource(NextState::<crate::GameMode>::default());
        app.add_message::<SetFlagRequested>();
        app.add_message::<QuestAdvanceRequested>();
        app.add_message::<SwitchActivated>();
        app.add_message::<VfxMessage>();
        spawn_interaction_player(&mut app, center);

        let switch = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("gate_switch"),
                FeatureName::new("Gate Switch"),
                CenteredAabb::from_center_size(center, ae::Vec2::new(24.0, 24.0)),
                SwitchFeature::new(crate::encounter::SwitchActivation {
                    id: "gate_switch".into(),
                    action: "open".into(),
                    target_encounter: String::new(),
                }),
                SwitchOn(false),
            ))
            .id();

        app.add_systems(Update, interact_ecs_actors_and_switches);
        app.update();

        assert!(
            app.world().get::<SwitchOn>(switch).unwrap().0,
            "a buffered interact on an adjacent switch should toggle it on"
        );
    }
}
