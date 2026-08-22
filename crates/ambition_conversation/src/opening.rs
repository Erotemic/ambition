//! Deciding that a conversation happens, and opening it.
//!
//! The half of "somebody pressed Interact next to an NPC" that is about
//! DIALOGUE rather than about interaction: who is speaking, who is being spoken
//! to, whether the pair has anything to say, and which seat owns the box.
//!
//! this lived in `features/ecs/interact.rs`, and that placement was the
//! last thing pinning `ambition_dialog` into `features`. The decomposition
//! plan's step 5 is *put integration above the domains it joins*: pressing
//! Interact is an INTERACTION fact (a body, a reach box, a buffered press) and
//! entering a Yarn node is a DIALOGUE fact. `interact` owns the first and asks
//! this for the second, so `features` names no dialogue type at all.
//! (`docs/planning/engine/actor-monolith-decomposition.md`,
//! the whole coupling was TWO production lines.)
//!
//! the port takes `&str` and `Entity` and nothing else. It would have been
//! easy to hand it the monolith's `NpcDialogueRequest` and add a third inward
//! edge to this module; it takes the two strings out of that request instead, so
//! the carve accounting in [`super`] does not grow.

use bevy::prelude::*;

use ambition_combat::components::{ActorIdentity, ActorInteraction};

use super::authority::{ActiveConversation, ConversationInputOwner, LiveConversation};
use super::instance::ConversationInstanceId;

/// The dialogue-dispatch seam: everything the interaction system needs to decide
/// WHETHER a conversation happens and WHO it is between.
///
/// Grouped into one `SystemParam` because they are one concern, and because the
/// interact system is already at Bevy's parameter ceiling — a signal that a
/// system reaching for this many worlds should name its sub-worlds.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DialogueDispatch<'w, 's> {
    /// What the SIMULATION believes about the live conversation. Rollback-owned.
    ///
    /// Reading it would branch on another timeline's state; writing it (which this did, to open the
    /// runner) replays the write. The box follows this resource now and nothing here names it.
    pub conversation: ResMut<'w, ActiveConversation>,
    /// WHEN a conversation opened, which is part of what it IS — see
    /// [`LiveConversation::instance`]. `Option` for the same reason preparation's
    /// is: a composition with no timeline has no replay to disagree with.
    pub tick: Option<Res<'w, ambition_time::SimTick>>,
    /// Who drives a body, for attributing the conversation to a seat.
    ///
    /// `DrivingParticipant` is what answers "whose body is this" — possession is
    /// a SEAT REDIRECT, so a possessed actor's conversation belongs to the seat
    /// that possessed it without this needing to know possession exists.
    pub driver: Query<'w, 's, &'static ambition_characters::control::DrivingParticipant>,
    /// Which Yarn nodes content compiled. Read to decide whether a
    /// self-conversation has a branch to enter; an unpopulated index never
    /// suppresses.
    pub nodes: Res<'w, ambition_dialog::DialogueNodeIndex>,
    /// The two bodies' STABLE identities, for the conversation's instance id.
    ///
    /// not the entities beside them: GGRS remaps entity handles on
    /// `LoadWorld`, so an id built from one names a different body after a
    /// restore. A body with no `SimId` yields `None`, which is a weaker id
    /// rather than a wrong one — see [`ConversationInstanceId`].
    pub sim_ids: Query<'w, 's, &'static ambition_platformer2d_shared_tangle::sim_id::SimId>,
    /// The character a speaking body is WEARING — read from the ENTITY's
    /// canonical `WornCharacter` identity, not the app-local startup selection
    /// resource, so after a runtime re-wear or snapshot restore the home avatar
    /// speaks as the character it currently IS.
    pub worn: Query<'w, 's, &'static ambition_characters::actor::WornCharacter>,
}

impl DialogueDispatch<'_, '_> {
    /// The id that answers "who is this body?" for dialogue purposes, for a body
    /// that is about to speak.
    ///
    /// A gameplay body without an authored identity is not a valid dialogue
    /// speaker, and the answer is `None` rather than a process-global default:
    /// substituting one would make dialogue authority depend on whichever
    /// provider initialized first in this process.
    pub fn speaker_id(
        &self,
        body: Entity,
        interaction: Option<&ActorInteraction>,
        identity: Option<&ActorIdentity>,
    ) -> Option<String> {
        dialogue_identity(interaction, identity)
            .or_else(|| self.worn.get(body).ok().map(|worn| worn.id().to_string()))
    }

    /// Decide whether this pair has a conversation, and open it if they do.
    ///
    /// Returns `false` when there is nothing to say — a self-conversation with
    /// no authored `__self` branch — so the caller can leave the interaction
    /// with no trace: no banner, no flags, no quest pump, no mode flip.
    ///
    /// AND THE TEXT BOX IS NOT OPENED FROM HERE. The caller runs in the
    /// SIM schedule, so a rollback across the tick somebody pressed Interact
    /// replays it — and `DialogState::start` is not a harmless setter: it resets
    /// the line, the options and the typewriter and enqueues a
    /// `runner.start_node`. `DialogState` is left out of rollback so a rewind
    /// does not stutter the box, and replaying that call stuttered it anyway.
    /// The box is a PROJECTION of the authority now, so everything it needs is
    /// stated here, at the tick the decision is made.
    ///
    /// the whole value in one call, and both bodies symmetrically: a
    /// conversation the world keeps running through has to be able to ask about
    /// both — how far apart they are, whether either can hold station, whether
    /// either was hit — and none of that can be asked of a character id.
    pub fn open_between(
        &mut self,
        initiator: Entity,
        talker: Entity,
        dialogue_id: &str,
        speaker_name: &str,
        speaker_id: &str,
        listener_id: &str,
        input_owner: ConversationInputOwner,
    ) -> bool {
        let context = ambition_dialog::DialogueContext::between(speaker_id, listener_id);
        // SELF-TALK. The speaker IS the listener — the player possessed this
        // body, or wears the character it is. By default a body has nothing to
        // say to itself; content opts in by authoring a `<dialogue_id>__self`
        // node, which becomes the node entered.
        let Some(entry_node) = self.nodes.entry_node(dialogue_id, context.speaker_is_self) else {
            return false;
        };
        self.conversation.open(LiveConversation {
            // WHICH conversation this is: when it opened, which node, which two
            // bodies, and what Yarn is entered with — every ingredient read off
            // the world at this tick, so a resimulation of it mints an equal id
            // and a narrative record from the original run still finds its own
            // conversation.
            // `SimId`, never these entities: `LoadWorld` remaps handles.
            // and the CONTEXT, not just the bodies. `speaker_id` above
            // falls back to the initiator's `WornCharacter`, which is
            // rollback-owned: two corrected timelines can agree on the tick, the
            // node and both `SimId`s while entering Yarn as different characters.
            instance: ConversationInstanceId::mint(
                self.tick.as_ref().map_or(0, |tick| tick.0),
                entry_node,
                self.sim_ids.get(initiator).ok().cloned(),
                self.sim_ids.get(talker).ok().cloned(),
                &context,
            ),
            initiator: Some(initiator),
            talker: Some(talker),
            input_owner,
            speaker_name: speaker_name.to_owned(),
        });
        true
    }

    /// Which seat is DRIVING this body, for the caller to attribute the
    /// conversation with.
    ///
    /// `DrivingParticipant` is what actually answers "whose body is this" —
    /// a seat that possessed an actor and walked it up to an NPC is the answer
    /// here without this knowing possession exists.
    ///
    /// What moved is the dependency: attributing a conversation never required knowing how "who
    /// drives" is spelled. The component lives in `ambition_characters::brain`, so this costs no
    /// new crate edge.
    ///
    /// it returns the SLOT, and the conversion to a participant is the
    /// caller's. `ParticipantId` and `PlayerSlot` are two concepts sharing one
    /// number, and `crate::participant_seat` is the ONE place that correspondence
    /// lives — precisely because `ambition_input` and `ambition_characters` are
    /// siblings that cannot see each other. Converting here would make this
    /// module a second owner of it, and would put a `participant_seat` edge into
    /// a module whose whole carve accounting is two edges to the BARK.
    pub fn driving_slot(&self, body: Entity) -> Option<ambition_characters::control::PlayerSlot> {
        self.driver.get(body).ok().map(|driver| driver.0)
    }
}

/// The catalog character this interactable IS, if it is a character at all.
///
/// A Hall pedestal, a hub NPC, a possessed body — each authors a `character_id`.
/// A switch, a chest, a nameless prop does not.
pub fn character_id_of(interactable: &ambition_interaction::Interactable) -> Option<&str> {
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
    use ambition_platformer2d_core as ae;

    /// CHARACTER identity beats PLACEMENT identity, and `$speaker_is_self`
    /// is why: it must fire when you walk up to the Hall pedestal of the
    /// character you are wearing, not merely when a body interacts with its own
    /// placement.
    #[test]
    fn character_identity_beats_placement_identity() {
        let interactable = ambition_interaction::Interactable::new(
            "some_ldtk_placement_iid",
            "Talk",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(24.0, 40.0)),
            ambition_interaction::InteractionKind::Npc {
                character_id: Some("player_robot_v3".into()),
                dialogue_id: Some("hall_player".into()),
                patrol_radius: 0.0,
                patrol_path_id: None,
                brain_override: None,
            },
        );
        let interaction = ActorInteraction {
            interactable,
            talk_radius: 40.0,
        };
        let identity = ActorIdentity::new("some_ldtk_placement_iid", "Player");
        assert_eq!(
            dialogue_identity(Some(&interaction), Some(&identity)).as_deref(),
            Some("player_robot_v3"),
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
