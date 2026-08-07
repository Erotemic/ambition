//! **What the simulation believes about the live conversation.**
//!
//! Rollback-owned, and it reads nothing — every other trace of a conversation in
//! the world is derived from this.
//!
//! ⚠ **`DialogState` keeps everything else, and keeps owning it.** Nothing here
//! is a second copy of the typewriter reveal, the option list or the pointer arm:
//! those are presentation facts, they are deliberately NOT rewound (rewinding
//! them would stutter the text box under a rollback), and no simulation system
//! may branch on them.

use bevy::prelude::*;

/// **Who owns input while a conversation is live.**
///
/// ⛔ **no `Default`, deliberately.** "Nobody said whose conversation this is, so
/// capture everybody" is the behaviour this replaces — one seat talking to an
/// NPC took gameplay away from every other seat at the couch. A conversation
/// that cannot say whose it is still has to answer the question, and answering
/// it in a `Default` impl is how it stops being answered at the call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationInputOwner {
    /// The seat driving the initiator — somebody walked up and pressed Interact.
    ///
    /// ⭐ derived from that body's `Brain::Player(slot)`, because the brain is
    /// what actually answers "who drives this body". ⛔ not an entity index, and
    /// not a device slot that happens to share a number with a seat.
    Participant(ambition_input::ParticipantId),
    /// The primary seat only. A scripted conversation nobody in the world
    /// started, where the shell's owner advances the box.
    Primary,
    /// Every seat. For a conversation that genuinely should stop the couch —
    /// chosen at the call site, never inferred from an absence.
    AllParticipants,
}

/// The live conversation's deterministic facts. Nothing presentational.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveConversation {
    /// The body that walked up and started it. `None` for a scripted
    /// conversation with no in-world initiator.
    pub initiator: Option<Entity>,
    /// The body being talked TO — the one a hold applies to.
    pub talker: Option<Entity>,
    /// Which Yarn node is live, so the UI projection can follow the authority
    /// rather than keeping its own idea of what is running.
    pub dialogue_id: String,
    pub input_owner: ConversationInputOwner,
}

/// The conversation the simulation is having, if any.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveConversation {
    live: Option<LiveConversation>,
}

impl ActiveConversation {
    /// **Open a conversation in ONE call.**
    ///
    /// ⛔ the shape this replaces was `start()`, then `set_speaker_entity()`,
    /// then `set_initiator_entity()` — three calls to establish one fact, where
    /// the second and third are the ones a new call site forgets. An authority
    /// that needs a follow-up call has a window in which it is wrong, and here
    /// that window was a tick of the simulation schedule.
    pub fn open(
        &mut self,
        initiator: Option<Entity>,
        talker: Option<Entity>,
        dialogue_id: impl Into<String>,
        input_owner: ConversationInputOwner,
    ) {
        self.live = Some(LiveConversation {
            initiator,
            talker,
            dialogue_id: dialogue_id.into(),
            input_owner,
        });
    }

    pub fn close(&mut self) {
        self.live = None;
    }

    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    pub fn live(&self) -> Option<&LiveConversation> {
        self.live.as_ref()
    }

    /// The body being talked TO. The hold applies to this one.
    pub fn talker(&self) -> Option<Entity> {
        self.live.as_ref().and_then(|live| live.talker)
    }

    /// The body that started it — whose seat owns the conversation's input.
    pub fn initiator(&self) -> Option<Entity> {
        self.live.as_ref().and_then(|live| live.initiator)
    }

    pub fn input_owner(&self) -> Option<ConversationInputOwner> {
        self.live.as_ref().map(|live| live.input_owner)
    }

    pub fn dialogue_id(&self) -> Option<&str> {
        self.live.as_ref().map(|live| live.dialogue_id.as_str())
    }

    /// Both in-world bodies.
    ///
    /// ⭐ **the continuity questions are about the PAIR** — how far apart are
    /// they, was either one hit — and a caller that reaches for [`Self::talker`]
    /// alone has already made the rule player-centric in the way Jon's design
    /// says it must not be (*"both characters should hover"*).
    pub fn participants(&self) -> impl Iterator<Item = Entity> + '_ {
        self.live
            .iter()
            .flat_map(|live| [live.initiator, live.talker])
            .flatten()
    }

    /// Every entity this resource names — the probe the rollback registration
    /// localizes through, so a desync here reports as the two bodies' stable
    /// identities rather than as "a resource differs".
    pub fn referenced_entities(&self) -> Vec<Entity> {
        self.participants().collect()
    }
}

impl bevy::ecs::entity::MapEntities for ActiveConversation {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        if let Some(entity) = live.initiator.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
        if let Some(entity) = live.talker.as_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}
