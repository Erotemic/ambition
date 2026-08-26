//! Player → ECS feature interaction (peaceful NPC dialogue, switches).
//!
//! Chests stay in `open_ecs_chests` because they have their own
//! reward/persistence path; this system covers the conversational
//! and switch-activation interactions that share the
//! `PlayerInteractionState` buffered-press contract.
//!
//! Deciding whether the pair has anything to say and what the conversation IS belongs to the
//! dialogue domain, and keeping it here was the last thing pinning `ambition_dialog` into
//! `features`.

use super::*;

// the ONLY dialogue name this module has left, and it is a port rather than a
// type from `ambition_dialog`. See `ambition_conversation::opening`.
use ambition_conversation::DialogueDispatch;

/// Handle interactions with ECS switches and peaceful NPCs. Chests stay in
/// `open_ecs_chests` because they have their own reward/persistence path.
///
/// The interaction is resolved for the controlled subject — the body the
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
    // the buffered interact belongs to the SEAT DRIVING THE ACTING BODY,
    // not to slot 0. Under possession those are different controllers, and
    // reading slot 0 meant a possessed body's interaction spent — and was gated
    // by — the home seat's press.
    mut acting: crate::control::ActingParticipant,
    // The startup-frame fallback subject, and nothing else: the interact POSE is
    // applied to the body that acted, below.
    primary: Query<
        Entity,
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    // Presentation anim for whichever body acted. Body-generic on purpose: a
    // possessed actor plays its own reach-and-open, and the vacated home avatar
    // plays nothing.
    mut anims: Query<&mut crate::actor::BodyAnimFacts>,
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
    // The body actually doing the interacting: the controlled subject (the body
    // holding the primary seat), falling back to the primary player itself for
    // the startup frame before the subject resolver has run.
    let Some(subject) = controlled
        .and_then(|subject| subject.0)
        .or_else(|| primary.iter().next())
    else {
        return;
    };
    if !acting.buffered_interact(subject) {
        return;
    }
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
    let speaker_id = dialogue.speaker_id(
        subject,
        interactions.get(subject).ok(),
        identities.get(subject).ok(),
    );
    for (actor_entity, aabb, disposition, identity, interaction_payload, health) in &actors {
        let Some(speaker_id) = speaker_id.as_deref() else {
            break;
        };
        // A hostile actor gates dialogue off; a dead one is an intangible corpse
        // and cannot be talked to.
        if disposition.is_hostile() || ambition_combat::util::body_is_corpse(health) {
            continue;
        }
        let interactable = &interaction_payload.interactable;
        if !aabb.aabb().strict_intersects(reach_aabb) {
            continue;
        }
        let request =
            super::super::npcs::npc_dialogue_request(interactable, &identity.name, &identity.id);
        let listener_id =
            ambition_conversation::character_id_of(interactable).unwrap_or(&identity.id);

        // THE DIALOGUE DECISION IS THE DIALOGUE DOMAIN'S. Whether this pair
        // has anything to say — a self-conversation needs an authored `__self`
        // branch — and what the conversation IS are both answered there; this
        // system owns the INTERACTION facts (a body, a reach box, a buffered
        // press) and the world-side consequences below.
        //
        // `continue`, not `return`: another body in reach may still be talkable, and the
        // buffered press has not been consumed. An interaction that does not happen must leave
        // no trace — no banner, no flags, no quest pump, no mode flip. WHOSE conversation
        // this is, decided here because the `ParticipantId` ↔ `PlayerSlot` correspondence
        // lives in exactly one place (`crate::participant_seat`) and that place is this crate.
        let input_owner = dialogue.driving_slot(subject).map_or(
            ambition_conversation::ConversationInputOwner::Primary,
            |slot| {
                ambition_conversation::ConversationInputOwner::Participant(
                    crate::participant_seat::participant_of(slot),
                )
            },
        );
        if !dialogue.open_between(
            subject,
            actor_entity,
            &request.dialogue_id,
            &request.npc_name,
            speaker_id,
            listener_id,
            input_owner,
        ) {
            continue;
        }

        acting.consume_interact(subject);
        pose_interact(&mut anims, subject, INTERACT_ANIM_HOLD_SECS);
        banner.show(
            super::super::npcs::npc_message(interactable, &identity.name, false),
            2.6,
        );
        ambition_platformer2d_shared_tangle::world_log::note_game_mode_request(
            ambition_platformer2d_shared_tangle::schedule::GameMode::Dialogue,
            "npc_interact",
        );
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
        acting.consume_interact(subject);
        pose_interact(&mut anims, subject, INTERACT_ANIM_HOLD_SECS);
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
mod tests;

/// Play the interact gesture on the body that ACTED.
pub(crate) fn pose_interact(
    anims: &mut Query<&mut crate::actor::BodyAnimFacts>,
    body: Entity,
    hold_secs: f32,
) {
    if let Ok(mut anim) = anims.get_mut(body) {
        anim.interact_anim_timer = hold_secs;
    }
}
