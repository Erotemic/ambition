//! Player → ECS feature interaction (peaceful NPC dialogue, switches).
//!
//! Chests stay in `open_ecs_chests` because they have their own
//! reward/persistence path; this system covers the conversational
//! and switch-activation interactions that share the
//! `PlayerInteractionState` buffered-press contract.

use super::*;

use ambition_characters::brain::ScriptedControl;

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
        dialogue
            .state
            .start(&entry_node, &request.npc_name, context);
        // Record which actor we're talking to so dialogue commands like
        // `<<challenge>>` can provoke THIS NPC into a fight.
        dialogue.state.set_speaker_entity(actor_entity);
        // ⭐ and the OTHER participant, symmetrically. A conversation that the
        // world keeps running through has to be able to ask about both bodies —
        // how far apart they are, whether either can hold station, whether
        // either was hit — and none of that can be asked of a character id.
        dialogue.state.set_initiator_entity(subject);
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
    /// The conversation read-model the UI polls.
    pub state: ResMut<'w, ambition_dialog::DialogState>,
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

#[cfg(test)]
mod tests;

/// **Break a conversation the world has carried its participants out of.**
///
/// The consumer of Jon's continuity design
/// (`docs/planning/engine/dialogue-continuity.md`). Since `GameMode::Dialogue`
/// left the suspend set, a conversation is a SUSTAINED condition rather than a
/// modal state — bodies keep moving, hits keep landing, and a text box that
/// survives either is a text box floating over two people who are no longer
/// talking.
///
/// ⭐ **symmetric, and that is load-bearing.** It reads
/// [`ambition_dialog::DialogState::participants`], which yields BOTH bodies, and
/// folds them into one `any_struck` before asking. There is deliberately no
/// place in this system for "was the player hit" — an NPC knocked off a ledge
/// mid-sentence has ended the conversation just as surely as the player being
/// knocked across the room. Jon: *"both characters should hover"*.
///
/// ⚠ **the reach test is the interaction's own**, not a second authored range:
/// the same `strict_intersects` of the two bodies' AABBs that decided the
/// conversation could START decides it can continue. Two ranges would drift, and
/// the symptom — a conversation you can begin but not sustain, or one that
/// follows you across a room — is the kind nobody reports as a range bug.
///
/// A conversation with fewer than two in-world participants (scripted dialogue,
/// a system-started box) cannot be walked away from, and is left alone.
pub fn break_dialogue_on_hit_or_separation(
    mut commands: Commands,
    mut dialogue: ResMut<ambition_dialog::DialogState>,
    bodies: Query<(&CenteredAabb, Option<&BodyCombat>)>,
    held: Query<(), With<HeldByConversation>>,
) {
    use ambition_dialog::DialogueBreak;

    if !dialogue.active() {
        return;
    }
    let participants: Vec<_> = dialogue.participants().collect();
    let [a, b] = participants.as_slice() else {
        // Scripted dialogue with no two in-world bodies. Nothing here can walk
        // away from anything.
        return;
    };
    let (Ok((a_aabb, a_combat)), Ok((b_aabb, b_combat))) = (bodies.get(*a), bodies.get(*b)) else {
        // A participant stopped existing — despawned, or the room swapped under
        // the conversation. That is a separation of the most literal kind.
        dialogue.close();
        return;
    };

    // ⚠ KNOCKBACK, not damage. The reason a hit ends a conversation is that it
    // MOVES you, so the signal is the recoil/hitstun control lock rather than
    // any health change: a poison tick or a chip of environmental damage leaves
    // both bodies standing where they were and leaves them talking.
    let struck = |combat: Option<&BodyCombat>| {
        combat.is_some_and(|c| c.recoil_lock_timer > 0.0 || c.hitstun_timer > 0.0)
    };
    let any_struck = struck(a_combat) || struck(b_combat);
    let in_reach = a_aabb.aabb().strict_intersects(b_aabb.aabb());

    if let Some(_break_reason) = DialogueBreak::evaluate(any_struck, in_reach) {
        // ▢ the BARK is owed: `DialogueBreak::bark_pool` names the pool, and the
        // bark ticker takes pools from the actor RON's `suggested_barks`. Wiring
        // it needs a break-pool authored on at least one character to be worth
        // anything, so the reason is computed and dropped here rather than
        // pretending a silent break is the finished behaviour.
        dialogue.close();
        return;
    }

    // **THE HOLD.** A conversation asks its participants to stay where they are,
    // and the rule is one line: their movement INTENT goes to zero. Everything
    // Jon described falls out of that with no case analysis — a grounded body
    // stands, a flying body hovers (`integrate_flight_clusters` drives toward
    // `local_stick * terminal_speed`, so neutral input decays to rest), and a
    // falling body with no flight has no intent to zero, keeps falling, leaves
    // reach, and breaks the conversation on the branch above. That is the parrot
    // case, correct by omission rather than by a rule about parrots.
    //
    // ⚠ **two mechanisms, one effect, and that is not player-centrism.** The
    // TALKER's intent is already neutral — `DIALOGUE_CONTEXT` captured their
    // input, so their `ControlFrame` is default. The other participant takes its
    // intent from a BRAIN, which nothing has captured, so it needs
    // `ScriptedControl`. The rule is symmetric ("every participant's intent is
    // neutral"); the two halves differ only because the two bodies are driven
    // from different places.
    //
    // ⭐ `ScriptedControl`'s own doc says a blanked frame "is not a frozen body,
    // and gravity will happily walk an undriven one out from under its pose",
    // and names that as the inserter's problem. Here it is the DESIGN: a body
    // gravity walks away is a body that could not hold station, and the break
    // above is what happens next.
    if let Some(brained) = dialogue.speaker_entity() {
        // ⚠ **`HeldByConversation` is the CLAIMANT `ScriptedControl`'s doc asks
        // for.** It warns that consumers remove the marker without checking who
        // put it there, and that a second concurrent sequence needs a claimant
        // rather than two owners racing. All five existing owners mark the
        // PLAYER's driven body (death, flagpole, act clear, versus, seating)
        // while this marks the NPC, so they do not collide today — but the
        // discipline is cheap and this system removes only what it added.
        if held.get(brained).is_err() {
            commands
                .entity(brained)
                .try_insert((ScriptedControl, HeldByConversation));
        }
    }
}

/// This system's claim on a body it blanked for a conversation.
///
/// See the hold in [`break_dialogue_on_hit_or_separation`]: it exists so the
/// release removes only markers this system placed, rather than stomping a
/// death beat's.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HeldByConversation;

/// Release every body a conversation was holding, once it is no longer holding
/// one.
///
/// ⛔ **a stranded `ScriptedControl` is a permanently frozen NPC**, and the ways
/// a conversation ends are not one place: the break rule, the player walking the
/// Yarn runner to its end, a room swap, a session teardown. So the release does
/// not try to be a mirror of the insert — it asks the only question that is
/// always true, "is this body still being held by a live conversation", and
/// answers from the dialogue state rather than from remembering.
pub fn release_conversation_hold(
    mut commands: Commands,
    dialogue: Res<ambition_dialog::DialogState>,
    held: Query<Entity, With<HeldByConversation>>,
) {
    let talking = dialogue.active();
    let still_talking = talking.then(|| dialogue.speaker_entity()).flatten();
    for entity in &held {
        if Some(entity) == still_talking {
            continue;
        }
        commands
            .entity(entity)
            .try_remove::<(ScriptedControl, HeldByConversation)>();
    }
}
