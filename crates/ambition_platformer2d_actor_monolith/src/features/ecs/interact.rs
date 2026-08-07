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
    mut next_mode: ResMut<NextState<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    mut banner: ResMut<GameplayBanner>,
    controlled: Option<Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
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
            Option<&ambition_characters::actor::BodyHealth>,
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
    // A gameplay body without an authored identity is not a valid dialogue
    // speaker. Do not silently substitute a process-global default: that
    // would make dialogue authority depend on whichever provider initialized
    // first in this process. A speaker-less body skips dialogue but still
    // works switches below.
    let speaker_id =
        dialogue_identity(interactions.get(subject).ok(), identities.get(subject).ok())
            .or_else(|| dialogue.worn.get(subject).ok().map(|w| w.id().to_string()));
    for (actor_entity, aabb, disposition, identity, interaction_payload, health) in &actors {
        let Some(speaker_id) = speaker_id.as_deref() else {
            break;
        };
        // A hostile actor gates dialogue off; a dead one is an intangible corpse
        // and cannot be talked to (Jon 2026-07-22 — one tangibility policy).
        if disposition.is_hostile() || crate::combat::util::body_is_corpse(health) {
            continue;
        }
        let interactable = &interaction_payload.interactable;
        if !aabb.aabb().strict_intersects(reach_aabb) {
            continue;
        }
        let request =
            super::super::npcs::npc_dialogue_request(interactable, &identity.name, &identity.id);
        let listener_id = character_id_of(interactable).unwrap_or(&identity.id);
        let context = ambition_dialog::DialogueContext::between(speaker_id, listener_id);

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
        // **THE AUTHORITY, in one call.** ⛔ this was `start()` then
        // `set_speaker_entity()` then `set_initiator_entity()` — three calls to
        // establish one fact, where a conversation existed and was missing a
        // participant in between. Both bodies and the seat that owns the
        // conversation are now settled at the moment it opens or not at all.
        //
        // ⭐ and the OTHER participant is here symmetrically: a conversation the
        // world keeps running through has to be able to ask about both bodies —
        // how far apart they are, whether either can hold station, whether
        // either was hit — and none of that can be asked of a character id.
        //
        // ⛔ **AND THE TEXT BOX IS NO LONGER OPENED FROM HERE** (GPT 5.6,
        // 2026-08-07). This system runs in the SIM schedule, so a rollback across
        // the tick somebody pressed Interact replays it — and
        // `DialogState::start` is not a harmless setter: it resets the line, the
        // options and the typewriter and enqueues a `runner.start_node`.
        // `DialogState` is left out of rollback so a rewind does not stutter the
        // box, and replaying this call stuttered it anyway. The box is a
        // PROJECTION of the authority now, opened by
        // `open_dialog_ui_when_the_conversation_starts` outside the sim
        // schedule — so everything it needs is stated here, at the tick the
        // decision is made.
        let input_owner = conversation_owner(&dialogue.driver, subject);
        dialogue
            .conversation
            .open(crate::conversation::LiveConversation {
                initiator: Some(subject),
                talker: Some(actor_entity),
                dialogue_id: entry_node.clone(),
                input_owner,
                // WHEN it opened, which is what tells a rewind-restored
                // conversation apart from the next one through the same node.
                opened_at: dialogue.tick.map_or(0, |tick| tick.0),
                speaker_name: request.npc_name.clone(),
                context,
            });
        next_mode.set(ambition_platformer2d_shared_tangle::schedule::GameMode::Dialogue);
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
pub struct DialogueDispatch<'w, 's> {
    /// What the SIMULATION believes about the live conversation. Rollback-owned.
    ///
    /// ⛔ **`DialogState` used to be here beside it, and its removal is the
    /// point.** The UI read-model is not rewound — deliberately, because
    /// rewinding a typewriter would stutter the text box — so a simulation
    /// system that TOUCHED it, in either direction, was reaching across the
    /// rollback boundary. Reading it would branch on another timeline's state;
    /// writing it (which this did, to open the runner) replays the write. The
    /// box follows this resource now and this system never names it.
    pub conversation: ResMut<'w, crate::conversation::ActiveConversation>,
    /// WHEN a conversation opened, which is part of what it IS — see
    /// `LiveConversation::opened_at`. `Option` for the same reason preparation's
    /// is: a composition with no timeline has no replay to disagree with.
    pub tick: Option<Res<'w, ambition_time::SimTick>>,
    /// Who drives a body, for attributing the conversation to a seat.
    ///
    /// ⭐ the brain is what actually answers "whose body is this" — possession is
    /// a brain transfer, so a possessed actor's conversation belongs to the seat
    /// that possessed it without this needing to know possession exists.
    pub driver: Query<'w, 's, &'static ambition_characters::brain::Brain>,
    /// Which Yarn nodes content compiled. Read to decide whether a
    /// self-conversation has a branch to enter; an unpopulated index never
    /// suppresses.
    pub nodes: Res<'w, ambition_dialog::DialogueNodeIndex>,
    /// The character a speaking body is WEARING — read from the ENTITY's canonical
    /// [`WornCharacter`] identity, not the app-local startup selection resource, so
    /// after a runtime re-wear or snapshot restore the home avatar speaks as the
    /// character it currently IS.
    pub worn: Query<'w, 's, &'static ambition_characters::actor::WornCharacter>,
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

/// **Which seat owns a conversation this body just started.**
///
/// ⛔ **there is no "nobody said, so capture everybody" arm, and that absence is
/// the fix** (GPT 5.6 review through `c32e690`, finding 2). Dialogue used to
/// claim every participant's input, so one person talking to an NPC took
/// gameplay away from everyone else at the couch — while the world kept running
/// around them.
///
/// ⭐ the question is answered by the initiator's BRAIN, not by an entity index
/// or a device slot that happens to share a number with a seat. Possession is a
/// brain transfer, so a seat that possessed an actor and walked it up to an NPC
/// owns that conversation without this function knowing possession exists.
///
/// ⚠ **the non-player arm is a DECISION, not a fallback.** A body with no player
/// brain cannot have pressed Interact under its own steam, so this is a
/// composition that drove the interaction some other way; the primary seat owns
/// the box, because somebody has to be able to advance it and capturing the
/// whole couch for a conversation nobody at it started is the behaviour being
/// removed.
fn conversation_owner(
    driver: &Query<&'static ambition_characters::brain::Brain>,
    initiator: Entity,
) -> crate::conversation::ConversationInputOwner {
    use crate::conversation::ConversationInputOwner;
    match driver.get(initiator) {
        Ok(ambition_characters::brain::Brain::Player(slot)) => {
            ConversationInputOwner::Participant(crate::participant_seat::participant_of(*slot))
        }
        _ => ConversationInputOwner::Primary,
    }
}

#[cfg(test)]
mod tests;
