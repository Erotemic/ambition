//! Rollback-owned authority for the live conversation. Presentation state such
//! as reveal progress, option layout, and pointer interaction stays in
//! `DialogState` and is never a simulation branch input.

use bevy::prelude::*;

use super::instance::ConversationInstanceId;

/// Who owns input while a conversation is live. There is no default; callers
/// must choose ownership explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationInputOwner {
    /// Participant driving the initiator, resolved from `DrivingParticipant`.
    Participant(ambition_input::ParticipantId),
    /// The primary seat only. A scripted conversation nobody in the world
    /// started, where the shell's owner advances the box.
    Primary,
    /// Every seat. For a conversation that genuinely should stop the couch —
    /// chosen at the call site, never inferred from an absence.
    AllParticipants,
}

/// The live conversation's deterministic facts. Nothing presentational.
///
///  "deterministic" is the test, not "the simulation branches on it". The
/// display name below is here because it is DECIDED by the simulation — read off
/// the two bodies at the tick somebody pressed Interact — and because the text
/// box has to be able to open from this and nothing else. The alternative is the
/// simulation reaching into the runner while it decides, which is what put a
/// presentation side effect inside a replayable system.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveConversation {
    /// Deterministic conversation identity, including the opening tick, Yarn
    /// node, participants, and dialogue context used by the runner.
    pub instance: ConversationInstanceId,
    /// The body that walked up and started it. `None` for a scripted
    /// conversation with no in-world initiator.
    ///
    ///  a handle, not an identity. GGRS remaps it on `LoadWorld` (see the
    /// [`bevy::ecs::entity::MapEntities`] impl below); the identity that survives
    /// that is the `SimId` inside [`Self::instance`]. Both are here because the
    /// continuity rules need to ASK things of the live body — how far away is it,
    /// was it hit — and none of that can be asked of an id.
    pub initiator: Option<Entity>,
    /// The body being talked TO — the one a hold applies to. Same handle-vs-identity
    /// split as [`Self::initiator`].
    pub talker: Option<Entity>,
    /// Input ownership. This is rollback state but not conversation identity;
    /// changing the driver does not restart the narrative instance.
    pub input_owner: ConversationInputOwner,
    /// Presentation-only fallback speaker name; excluded from instance identity
    /// and desync fingerprints.
    pub speaker_name: String,
}

impl LiveConversation {
    /// A conversation with only its SIMULATION facts, for a test about the
    /// hold, the break rule or input ownership.
    ///
    /// The fields stay public and production constructs the whole value, so a
    /// new presentation fact breaks every real call site; this is the hatch, and
    /// it is named for what it is. A conversation built here opens at tick zero
    /// and carries no display name — which is exactly what a test that never
    /// looks at the text box means.
    #[doc(hidden)]
    pub fn for_test(
        initiator: Option<Entity>,
        talker: Option<Entity>,
        dialogue_id: impl Into<String>,
        input_owner: ConversationInputOwner,
    ) -> Self {
        Self {
            instance: ConversationInstanceId::mint(
                0,
                dialogue_id,
                None,
                None,
                &ambition_dialog::DialogueContext::scripted(),
            ),
            initiator,
            talker,
            input_owner,
            speaker_name: String::new(),
        }
    }

    /// Which Yarn node is live, so the UI projection can follow the authority
    /// rather than keeping its own idea of what is running.
    pub fn dialogue_id(&self) -> &str {
        self.instance.node()
    }

    /// The identity context Yarn is entered with.
    ///
    ///  read back out of [`Self::instance`], because it is part of what makes
    /// this conversation this conversation — see that type's docs.
    pub fn context(&self) -> ambition_dialog::DialogueContext {
        self.instance.context()
    }

    /// The `SimTick` this conversation opened on.
    pub const fn opened_at(&self) -> u64 {
        self.instance.opened_at()
    }
}

/// The conversation the simulation is having, if any.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveConversation {
    live: Option<LiveConversation>,
}

impl ActiveConversation {
    /// Open a conversation in ONE call.
    ///
    ///  the shape this replaces was `start()`, then `set_speaker_entity()`,
    /// then `set_initiator_entity()` — three calls to establish one fact, where
    /// the second and third are the ones a new call site forgets. An authority
    /// that needs a follow-up call has a window in which it is wrong, and here
    /// that window was a tick of the simulation schedule.
    ///  the whole value, so the COMPILER enumerates what a conversation
    /// is. It took four positional arguments and grew two more; a struct
    /// literal makes a new fact break every call site, which is the only way a
    /// conversation opened somewhere else keeps saying everything the projection
    /// needs.
    pub fn open(&mut self, live: LiveConversation) {
        self.live = Some(live);
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
        self.live.as_ref().map(LiveConversation::dialogue_id)
    }

    /// Which conversation is live, for a narrative record that has to name one.
    pub fn instance(&self) -> Option<&ConversationInstanceId> {
        self.live.as_ref().map(|live| &live.instance)
    }

    /// Both in-world bodies.
    ///
    ///  the continuity questions are about the PAIR — how far apart are they, was either
    /// one hit — and a caller that reaches for [`Self:talker`] alone has already made the rule
    /// player-centric in the way the rule says it must not be (*"both characters should
    /// hover"*).
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
