//! Player → ECS feature interaction (peaceful NPC dialogue, switches).
//!
//! Chests stay in `open_ecs_chests` because they have their own
//! reward/persistence path; this system covers the conversational
//! and switch-activation interactions that share the
//! `PlayerInteractionState` buffered-press contract.

use super::*;

/// Handle interactions with ECS switches and peaceful NPCs. Chests stay in
/// `open_ecs_chests` because they have their own reward/persistence path.
///
/// The interaction is resolved for the **controlled subject** — the body the
/// local player is driving (the home avatar during normal play, a possessed
/// actor while possessing). Intent (the buffered `Interact` press) comes from
/// slot-0's input surface, the primary player's `PlayerInteractionState`, which
/// the device writes every frame regardless of which body is possessed; the
/// GEOMETRY (whose AABB decides what's in reach) comes from the driven body. So
/// possessing an actor and pressing Interact activates whatever THAT body is
/// standing next to, not whatever the vacated home avatar is next to. In normal
/// play the two are the same entity, so single-player behavior is unchanged.
pub fn interact_ecs_actors_and_switches(
    mut dialogue: ResMut<crate::dialog::DialogState>,
    mut next_mode: ResMut<NextState<ambition_platformer_primitives::schedule::GameMode>>,
    mut banner: ResMut<GameplayBanner>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    // The local controller's buffered interact lives on its SLOT, published from the
    // device even while the home avatar is vacated — the right source for "the local
    // player wants to interact" independent of which body is being driven.
    mut slot_gestures: ResMut<crate::player::SlotInteractionState>,
    // Interact-gesture pose on the primary player's presentation anim (+ the
    // startup-frame fallback subject).
    mut input_surface: Query<
        (Entity, &mut crate::player::BodyAnimFacts),
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    // The driven body's kinematics — body-generic so the reach test uses the
    // controlled subject's position whether it's the player or a possessed actor.
    bodies: Query<&crate::actor::BodyKinematics>,
    // Talkable actors carry the shared `ActorInteraction` payload (dialogue
    // is an actor capability, not an NPC type). Dialogue is offered only to a
    // PEACEFUL talkable actor — a provoked one keeps its `ActorInteraction`
    // but its `ActorDisposition::Hostile` gates dialogue off.
    actors: Query<
        (
            Entity,
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
    // How long the player's `Interact` pose holds after the interaction
    // commits. Short enough that the gesture clears before dialogue UI
    // or the room transition takes camera focus.
    const INTERACT_ANIM_HOLD_SECS: f32 = 0.28;
    let Ok((primary_entity, mut anim)) = input_surface.single_mut() else {
        return;
    };
    if !slot_gestures.primary().buffered() {
        return;
    }
    // The body actually doing the interacting: the controlled subject (the body
    // carrying `Brain::Player`), falling back to the input surface itself for
    // the startup frame before the subject resolver has run.
    let subject = controlled
        .and_then(|subject| subject.0)
        .unwrap_or(primary_entity);
    let Ok(subject_kin) = bodies.get(subject) else {
        return;
    };
    let reach_aabb = subject_kin.aabb();
    for (actor_entity, aabb, disposition, identity, interaction_payload) in &actors {
        if disposition.is_hostile() {
            continue;
        }
        let interactable = &interaction_payload.interactable;
        if !aabb.aabb().strict_intersects(reach_aabb) {
            continue;
        }
        slot_gestures.primary_mut().clear();
        anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
        banner.show(
            super::super::npcs::npc_message(interactable, &identity.name, false),
            2.6,
        );
        let request =
            super::super::npcs::npc_dialogue_request(interactable, &identity.name, &identity.id);
        dialogue.start(&request.dialogue_id, &request.npc_name);
        // Record which actor we're talking to so dialogue commands like
        // `<<challenge>>` can provoke THIS NPC into a fight.
        dialogue.set_speaker_entity(actor_entity);
        next_mode.set(ambition_platformer_primitives::schedule::GameMode::Dialogue);
        quest_advance.write(QuestAdvanceRequested(
            ambition_persistence::quest::QuestAdvanceEvent::NpcTalked(identity.id.clone()),
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
        // Dialogue is a global mode flip; a talk consumes the interact and skips
        // the switch loop this tick.
        return;
    }
    for (_id, name, aabb, switch, mut on) in &mut switches {
        if !aabb.aabb().strict_intersects(reach_aabb) {
            continue;
        }
        slot_gestures.primary_mut().clear();
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
        // Switch activation is per-target; once we flip one we stop.
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::{
        CenteredAabb, FeatureId, FeatureName, FeatureSimEntity, SwitchFeature, SwitchOn,
    };
    use ambition_engine_core as ae;
    use bevy::prelude::{App, NextState, Update};

    fn spawn_interaction_player(app: &mut App, pos: ae::Vec2) {
        let scratch = crate::player::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
        let bundle = crate::player::PlayerSimulationBundle::from_scratch(
            scratch,
            ambition_characters::actor::Health::new(10),
        );
        app.world_mut().spawn(bundle);
        // The interact buffer is SLOT state now (published from the device); prime
        // the primary controller's slot so the system sees a live buffered interact.
        app.world_mut()
            .get_resource_or_insert_with(crate::player::SlotInteractionState::default)
            .primary_mut()
            .interact_buffer_timer = 0.15;
    }

    #[test]
    fn buffered_interact_toggles_an_adjacent_switch() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(crate::dialog::DialogState::default());
        app.insert_resource(NextState::<ambition_platformer_primitives::schedule::GameMode>::default());
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

    #[test]
    fn interact_lands_on_the_controlled_subject_not_the_vacated_home_avatar() {
        use ambition_platformer_primitives::markers::ControlledSubject;
        use crate::actor::BodyKinematics;

        let home_pos = ae::Vec2::new(0.0, 0.0);
        let subject_pos = ae::Vec2::new(600.0, 0.0);

        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(crate::dialog::DialogState::default());
        app.insert_resource(NextState::<ambition_platformer_primitives::schedule::GameMode>::default());
        app.add_message::<SetFlagRequested>();
        app.add_message::<QuestAdvanceRequested>();
        app.add_message::<SwitchActivated>();
        app.add_message::<VfxMessage>();

        // Slot-0 input surface: the home avatar, far from the switch, with a
        // buffered interact press from the device.
        spawn_interaction_player(&mut app, home_pos);

        // The possessed body the player is DRIVING, standing on the switch.
        let subject = app
            .world_mut()
            .spawn(BodyKinematics {
                pos: subject_pos,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 40.0),
                facing: 1.0,
            })
            .id();
        app.insert_resource(ControlledSubject(Some(subject)));

        // A switch next to the DRIVEN body...
        let near_subject = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("subject_switch"),
                FeatureName::new("Subject Switch"),
                CenteredAabb::from_center_size(subject_pos, ae::Vec2::new(24.0, 24.0)),
                SwitchFeature::new(crate::encounter::SwitchActivation {
                    id: "subject_switch".into(),
                    action: "open".into(),
                    target_encounter: String::new(),
                }),
                SwitchOn(false),
            ))
            .id();

        // ...and a decoy next to the vacated home avatar, which must NOT fire.
        let near_home = app
            .world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new("home_switch"),
                FeatureName::new("Home Switch"),
                CenteredAabb::from_center_size(home_pos, ae::Vec2::new(24.0, 24.0)),
                SwitchFeature::new(crate::encounter::SwitchActivation {
                    id: "home_switch".into(),
                    action: "open".into(),
                    target_encounter: String::new(),
                }),
                SwitchOn(false),
            ))
            .id();

        app.add_systems(Update, interact_ecs_actors_and_switches);
        app.update();

        assert!(
            app.world().get::<SwitchOn>(near_subject).unwrap().0,
            "interact should activate the switch next to the CONTROLLED body"
        );
        assert!(
            !app.world().get::<SwitchOn>(near_home).unwrap().0,
            "interact must NOT reach the switch next to the vacated home avatar"
        );
    }
}
