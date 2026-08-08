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

use super::instance::ConversationInstanceId;

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
///
/// ⚠ **"deterministic" is the test, not "the simulation branches on it".** The
/// display name below is here because it is DECIDED by the simulation — read off
/// the two bodies at the tick somebody pressed Interact — and because the text
/// box has to be able to open from this and nothing else. The alternative is the
/// simulation reaching into the runner while it decides, which is what put a
/// presentation side effect inside a replayable system.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveConversation {
    /// **WHICH conversation this is** — see [`ConversationInstanceId`].
    ///
    /// ⛔ **WHEN and WHO are part of the fact, and leaving them out is the same
    /// determinism hole `PreparedMatch::effective_from` was written to close.**
    /// Two things need this. A narrative record crossing back from the
    /// non-rewound runner has to name the conversation it belongs to, or a
    /// finished conversation's ending closes a fresh one. And the presentation
    /// projection has to know whether the box it is showing is THIS conversation
    /// — a rewind restores the authority unchanged, and a projection that could
    /// not tell would restart the runner under a player who is mid-sentence.
    ///
    /// ⚠ **the Yarn node lives in here**, not beside it. It is one of the facts
    /// that make a conversation that conversation, and a second copy on this
    /// struct would be a second answer to keep in step.
    ///
    /// ⛔ **and so does the [`ambition_dialog::DialogueContext`], which used to
    /// be a SIBLING of this field** (GPT 5.6 review, D29). It is not decoration:
    /// the bridge publishes it as `$speaker_id`, `$listener_id` and
    /// `$speaker_is_self`, and the speaker resolves from the initiator's
    /// `WornCharacter` — rollback-owned and runtime-mutable. So two corrected
    /// timelines could differ in what Yarn observes and still mint the same
    /// instance id, which made the projection's attachment memo and every
    /// instance-gated ledger record wrong at once. Identity is this field's whole
    /// job; the fact belongs IN it, and [`Self::context`] reads it back out.
    pub instance: ConversationInstanceId,
    /// The body that walked up and started it. `None` for a scripted
    /// conversation with no in-world initiator.
    ///
    /// ⚠ **a handle, not an identity.** GGRS remaps it on `LoadWorld` (see the
    /// [`bevy::ecs::entity::MapEntities`] impl below); the identity that survives
    /// that is the `SimId` inside [`Self::instance`]. Both are here because the
    /// continuity rules need to ASK things of the live body — how far away is it,
    /// was it hit — and none of that can be asked of an id.
    pub initiator: Option<Entity>,
    /// The body being talked TO — the one a hold applies to. Same handle-vs-identity
    /// split as [`Self::initiator`].
    pub talker: Option<Entity>,
    /// **Who owns input while this is live** — see [`ConversationInputOwner`].
    ///
    /// ⚠ **deliberately NOT part of [`Self::instance`].** It publishes nothing
    /// into Yarn and selects no node; `declare_in_session_input_contexts`
    /// re-reads it off this rollback-owned authority every tick, so a correction
    /// repairs it without identity's help. Keying on it would make "somebody
    /// possessed the body mid-sentence" a different conversation and restart the
    /// runner from the top. The reasoning is in [`ConversationInstanceId`]'s
    /// module docs, with the presentation field below.
    pub input_owner: ConversationInputOwner,
    /// The display name the box shows when a line carries no speaker prefix.
    ///
    /// ⛔ **PRESENTATION, and the one field here that is.** A localization
    /// changing it is not a different conversation and Yarn never sees it, so it
    /// is out of the instance id and out of the desync fingerprint alike.
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

    /// **The identity context Yarn is entered with.**
    ///
    /// ⭐ read back out of [`Self::instance`], because it is part of what makes
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
    /// **Open a conversation in ONE call.**
    ///
    /// ⛔ the shape this replaces was `start()`, then `set_speaker_entity()`,
    /// then `set_initiator_entity()` — three calls to establish one fact, where
    /// the second and third are the ones a new call site forgets. An authority
    /// that needs a follow-up call has a window in which it is wrong, and here
    /// that window was a tick of the simulation schedule.
    /// ⚠ **the whole value, so the COMPILER enumerates what a conversation
    /// is.** It took four positional arguments and grew two more; a struct
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
