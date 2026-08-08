//! Player → ECS feature interaction (peaceful NPC dialogue, switches).
//!
//! Chests stay in `open_ecs_chests` because they have their own
//! reward/persistence path; this system covers the conversational
//! and switch-activation interactions that share the
//! `PlayerInteractionState` buffered-press contract.
//!
//! ⚠ **the DIALOGUE half moved to `crate::conversation::opening`.** This module
//! owns the interaction facts — a body, a reach box, a buffered press — and the
//! world-side consequences of a conversation starting (the banner, the flags,
//! the quest pump, the mode flip). Deciding whether the pair has anything to say
//! and what the conversation IS belongs to the dialogue domain, and keeping it
//! here was the last thing pinning `ambition_dialog` into `features`.

use super::*;

// ⭐ the ONLY dialogue name this module has left, and it is a port rather than a
// type from `ambition_dialog`. See `crate::conversation::opening`.
use crate::conversation::DialogueDispatch;

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
        let listener_id =
            crate::conversation::character_id_of(interactable).unwrap_or(&identity.id);

        // **THE DIALOGUE DECISION IS THE DIALOGUE DOMAIN'S.** Whether this pair
        // has anything to say — a self-conversation needs an authored `__self`
        // branch — and what the conversation IS are both answered there; this
        // system owns the INTERACTION facts (a body, a reach box, a buffered
        // press) and the world-side consequences below.
        //
        // ⚠ `continue`, not `return`: another body in reach may still be
        // talkable, and the buffered press has not been consumed. An
        // interaction that does not happen must leave no trace — no banner, no
        // flags, no quest pump, no mode flip.
        if !dialogue.open_between(
            subject,
            actor_entity,
            &request.dialogue_id,
            &request.npc_name,
            speaker_id,
            listener_id,
        ) {
            continue;
        }

        slot_gestures.primary_mut().clear();
        anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
        banner.show(
            super::super::npcs::npc_message(interactable, &identity.name, false),
            2.6,
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
mod tests;
