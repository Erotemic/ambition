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
    mut dialogue: DialogueDispatch,
    mut next_mode: ResMut<NextState<ambition_platformer_primitives::schedule::GameMode>>,
    mut banner: ResMut<GameplayBanner>,
    controlled: Option<Res<ambition_platformer_primitives::markers::ControlledSubject>>,
    // The local controller's buffered interact lives on its SLOT, published from the
    // device even while the home avatar is vacated — the right source for "the local
    // player wants to interact" independent of which body is being driven.
    mut slot_gestures: ResMut<crate::control::SlotInteractionState>,
    // Interact-gesture pose on the primary player's presentation anim (+ the
    // startup-frame fallback subject).
    mut input_surface: Query<
        (Entity, &mut crate::actor::BodyAnimFacts),
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    // The driven body's kinematics — body-generic so the reach test uses the
    // controlled subject's position whether it's the player or a possessed actor.
    bodies: Query<&crate::actor::BodyKinematics>,
    // The driven body's identity + interaction payload, when it has them (a
    // possessed actor). The home avatar has neither and speaks as its worn
    // character instead.
    identities: Query<&ActorIdentity>,
    interactions: Query<&ActorInteraction>,
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
    // WHO is doing the talking. A possessed body speaks as the character it IS;
    // the home avatar speaks as the character it WEARS; a body that is neither
    // speaks as its placement. Ids, never display names — a name is a
    // localization artifact and two characters can share one.
    let speaker_id =
        dialogue_identity(interactions.get(subject).ok(), identities.get(subject).ok())
            .unwrap_or_else(|| dialogue.worn_character.effective_id().to_string());
    for (actor_entity, aabb, disposition, identity, interaction_payload) in &actors {
        if disposition.is_hostile() {
            continue;
        }
        let interactable = &interaction_payload.interactable;
        if !aabb.aabb().strict_intersects(reach_aabb) {
            continue;
        }
        let request =
            super::super::npcs::npc_dialogue_request(interactable, &identity.name, &identity.id);
        let listener_id = character_id_of(interactable).unwrap_or(&identity.id);
        let context = ambition_dialog::DialogueContext::between(&speaker_id, listener_id);

        // SELF-TALK. The speaker IS the listener — the player possessed this body,
        // or wears the character it is. By default a body has nothing to say to
        // itself, and the interaction is SUPPRESSED here, BEFORE the banner, the
        // flags, the quest pump, and the mode flip: an interaction that does not
        // happen must leave no trace. Content opts in by authoring a
        // `<dialogue_id>__self` node, which becomes the node we enter.
        //
        // `continue`, not `return`: another body in reach may still be talkable,
        // and the buffered press has not been consumed.
        let Some(entry_node) = dialogue
            .nodes
            .entry_node(&request.dialogue_id, context.speaker_is_self)
        else {
            continue;
        };

        slot_gestures.primary_mut().clear();
        anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
        banner.show(
            super::super::npcs::npc_message(interactable, &identity.name, false),
            2.6,
        );
        dialogue
            .state
            .start(&entry_node, &request.npc_name, context);
        // Record which actor we're talking to so dialogue commands like
        // `<<challenge>>` can provoke THIS NPC into a fight.
        dialogue.state.set_speaker_entity(actor_entity);
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

/// The dialogue-dispatch seam: everything `interact_*` needs to decide WHETHER a
/// conversation happens and WHO it is between.
///
/// Grouped into one `SystemParam` because they are one concern, and because the
/// interact system is already at Bevy's parameter ceiling — a signal that a
/// system reaching for this many worlds should name its sub-worlds.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DialogueDispatch<'w> {
    /// The conversation read-model the UI polls.
    pub state: ResMut<'w, ambition_dialog::DialogState>,
    /// Which Yarn nodes content compiled. Read to decide whether a
    /// self-conversation has a branch to enter; an unpopulated index never
    /// suppresses.
    pub nodes: Res<'w, ambition_dialog::DialogueNodeIndex>,
    /// The character the home avatar is WEARING — its identity when it speaks.
    pub worn_character: Res<'w, crate::avatar::StartingCharacter>,
}

/// The catalog character this interactable IS, if it is a character at all.
///
/// A Hall pedestal, a hub NPC, a possessed body — each authors a `character_id`.
/// A switch, a chest, a nameless prop does not.
fn character_id_of(interactable: &ambition_interaction::Interactable) -> Option<&str> {
    match &interactable.kind {
        ambition_interaction::InteractionKind::Npc { character_id, .. } => character_id.as_deref(),
        _ => None,
    }
}

/// The id that answers "who is this body?" for dialogue purposes.
///
/// CHARACTER identity wins over PLACEMENT identity. A character id names a
/// person; a placement id names a spot on a map. `$speaker_is_self` is only a
/// useful signal under the first reading: it must fire when you walk up to the
/// Hall pedestal of the character you are wearing, not merely when a body
/// somehow interacts with its own placement.
///
/// Returns `None` for a body with neither — the home avatar, whose identity is
/// the character it wears.
fn dialogue_identity(
    interaction: Option<&ActorInteraction>,
    identity: Option<&ActorIdentity>,
) -> Option<String> {
    if let Some(character_id) = interaction.and_then(|i| character_id_of(&i.interactable)) {
        return Some(character_id.to_string());
    }
    identity.map(|identity| identity.id.clone())
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
        let scratch = crate::avatar::primary_player_scratch(pos, ae::AbilitySet::sandbox_all());
        let bundle = crate::avatar::PlayerSimulationBundle::from_scratch(
            scratch,
            ambition_characters::actor::Health::new(10),
        );
        app.world_mut().spawn(bundle);
        // The interact buffer is SLOT state now (published from the device); prime
        // the primary controller's slot so the system sees a live buffered interact.
        app.world_mut()
            .get_resource_or_insert_with(crate::control::SlotInteractionState::default)
            .primary_mut()
            .interact_buffer_timer = 0.15;
    }

    #[test]
    fn buffered_interact_toggles_an_adjacent_switch() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(ambition_dialog::DialogState::default());
        app.init_resource::<ambition_dialog::DialogueNodeIndex>();
        app.init_resource::<crate::avatar::StartingCharacter>();
        app.insert_resource(NextState::<
            ambition_platformer_primitives::schedule::GameMode,
        >::default());
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
        use crate::actor::BodyKinematics;
        use ambition_platformer_primitives::markers::ControlledSubject;

        let home_pos = ae::Vec2::new(0.0, 0.0);
        let subject_pos = ae::Vec2::new(600.0, 0.0);

        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(ambition_dialog::DialogState::default());
        app.init_resource::<ambition_dialog::DialogueNodeIndex>();
        app.init_resource::<crate::avatar::StartingCharacter>();
        app.insert_resource(NextState::<
            ambition_platformer_primitives::schedule::GameMode,
        >::default());
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

    /// Spawn a talkable Hall-style pedestal: a peaceful actor that IS a catalog
    /// character (`character_id`) and offers a dialogue node.
    fn spawn_pedestal(
        app: &mut App,
        pos: ae::Vec2,
        character_id: &str,
        dialogue_id: &str,
    ) -> Entity {
        let interactable = ambition_interaction::Interactable::new(
            "hall_pedestal_placement",
            "Talk",
            ae::Aabb::new(pos, ae::Vec2::new(24.0, 40.0)),
            ambition_interaction::InteractionKind::Npc {
                character_id: Some(character_id.to_string()),
                dialogue_id: Some(dialogue_id.to_string()),
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        );
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 40.0)),
                ActorDisposition::Peaceful,
                ActorIdentity::new("hall_pedestal_placement", "Player"),
                ActorInteraction {
                    interactable,
                    talk_radius: 40.0,
                },
            ))
            .id()
    }

    fn dialogue_app(worn: &str, nodes: &[&str]) -> App {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.insert_resource(ambition_dialog::DialogState::default());
        let mut index = ambition_dialog::DialogueNodeIndex::default();
        index.populate(nodes.iter().map(|n| (*n).to_string()));
        app.insert_resource(index);
        app.insert_resource(crate::avatar::StartingCharacter::new(worn));
        app.insert_resource(NextState::<
            ambition_platformer_primitives::schedule::GameMode,
        >::default());
        app.add_message::<SetFlagRequested>();
        app.add_message::<QuestAdvanceRequested>();
        app.add_message::<SwitchActivated>();
        app.add_message::<VfxMessage>();
        app
    }

    /// Wearing a DIFFERENT character: an ordinary conversation, on the node the
    /// pedestal authored.
    #[test]
    fn a_visitor_gets_the_pedestals_ordinary_node() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = dialogue_app("goblin", &["hall_player", "hall_player__self"]);
        spawn_interaction_player(&mut app, center);
        spawn_pedestal(&mut app, center, "player", "hall_player");

        app.add_systems(Update, interact_ecs_actors_and_switches);
        app.update();

        let state = app.world().resource::<ambition_dialog::DialogState>();
        assert!(state.active());
        assert_eq!(state.dialogue_id(), "hall_player");
    }

    /// Wearing the pedestal's OWN character, with a self branch authored: the
    /// engine enters that branch instead.
    #[test]
    fn wearing_the_pedestals_character_enters_the_self_branch() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = dialogue_app("player", &["hall_player", "hall_player__self"]);
        spawn_interaction_player(&mut app, center);
        spawn_pedestal(&mut app, center, "player", "hall_player");

        app.add_systems(Update, interact_ecs_actors_and_switches);
        app.update();

        let state = app.world().resource::<ambition_dialog::DialogState>();
        assert!(state.active());
        assert_eq!(
            state.dialogue_id(),
            "hall_player__self",
            "the speaker IS the listener, and content authored a self branch"
        );
    }

    /// The engine default. Wearing the pedestal's character with NO self branch
    /// authored: the interaction never happens — and leaves no trace. Not a
    /// dialogue that opens and closes, not a consumed press, not a quest event.
    #[test]
    fn self_talk_without_a_self_branch_is_suppressed_without_a_trace() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = dialogue_app("player", &["hall_player"]);
        spawn_interaction_player(&mut app, center);
        spawn_pedestal(&mut app, center, "player", "hall_player");

        // Pre-poison: if the system returns early for the WRONG reason, these
        // stay as-set and the assertions below would pass vacuously.
        app.world_mut()
            .resource_mut::<GameplayBanner>()
            .show("sentinel", 9.0);

        app.add_systems(Update, interact_ecs_actors_and_switches);
        app.update();

        let world = app.world();
        assert!(
            !world.resource::<ambition_dialog::DialogState>().active(),
            "no conversation may open"
        );
        assert_eq!(
            world.resource::<GameplayBanner>().text.as_str(),
            "sentinel",
            "no banner may be shown — the interaction did not happen"
        );
        assert!(
            world
                .resource::<crate::control::SlotInteractionState>()
                .primary()
                .buffered(),
            "the buffered press is NOT consumed: the player may still interact \
             with something else"
        );
        let quests = world.resource::<bevy::ecs::message::Messages<QuestAdvanceRequested>>();
        assert_eq!(quests.len(), 0, "no `NpcTalked` may fire");
        let flags = world.resource::<bevy::ecs::message::Messages<SetFlagRequested>>();
        assert_eq!(flags.len(), 0, "no `..._talked` flag may be set");
    }

    /// An index that never saw a compiled Yarn project (headless, RL, the frames
    /// before the runner spawns) must not swallow interactions.
    #[test]
    fn an_unpopulated_node_index_never_suppresses() {
        let center = ae::Vec2::new(100.0, 100.0);
        let mut app = dialogue_app("player", &[]);
        app.insert_resource(ambition_dialog::DialogueNodeIndex::default());
        spawn_interaction_player(&mut app, center);
        spawn_pedestal(&mut app, center, "player", "hall_player");

        app.add_systems(Update, interact_ecs_actors_and_switches);
        app.update();

        let state = app.world().resource::<ambition_dialog::DialogState>();
        assert!(state.active(), "not knowing is not grounds for suppressing");
        assert_eq!(state.dialogue_id(), "hall_player");
    }

    /// A body with a character identity speaks as that character, not as its
    /// placement. This is what makes `$speaker_is_self` fire at the Hall.
    #[test]
    fn character_identity_beats_placement_identity() {
        let interactable = ambition_interaction::Interactable::new(
            "some_ldtk_placement_iid",
            "Talk",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)),
            ambition_interaction::InteractionKind::Npc {
                character_id: Some("player".into()),
                dialogue_id: Some("hall_player".into()),
                patrol_radius: 0.0,
                patrol_path_id: None,
            },
        );
        let interaction = ActorInteraction {
            interactable,
            talk_radius: 40.0,
        };
        let identity = ActorIdentity::new("some_ldtk_placement_iid", "Player");
        assert_eq!(
            dialogue_identity(Some(&interaction), Some(&identity)).as_deref(),
            Some("player"),
        );
        // A body with no character identity falls back to its placement.
        assert_eq!(
            dialogue_identity(None, Some(&identity)).as_deref(),
            Some("some_ldtk_placement_iid"),
        );
        // The home avatar has neither; the caller supplies its worn character.
        assert_eq!(dialogue_identity(None, None), None);
    }
}
