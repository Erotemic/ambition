//! Rollback-owned authority for the live conversation. Presentation state such
//! as reveal progress, option layout, and pointer interaction stays in
//! `DialogState` and is never a simulation branch input.

use bevy::prelude::*;

use super::instance::ConversationInstanceId;

use ambition_input::InputOwner;

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
    /// The bodies that joined after it opened, in the order they joined: a
    /// third character brought in by authored content (`conversation.join`)
    /// or a threat that interrupted it ([`ActiveConversation::interrupt`]).
    /// Each is held as the talker is, and each leaves by the same rules
    /// (`break_dialogue_on_hit_or_separation`). Not part of the instance
    /// identity: a body joining does not restart the narrative.
    pub joined: Vec<Entity>,
    /// Input ownership. This is rollback state but not conversation identity;
    /// changing the driver does not restart the narrative instance.
    pub input_owner: InputOwner,
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
        input_owner: InputOwner,
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
            joined: Vec::new(),
            input_owner,
            speaker_name: String::new(),
        }
    }

    /// Every body in it: the initiator, the talker, then the joined in the
    /// order they joined.
    pub fn bodies(&self) -> impl Iterator<Item = Entity> + '_ {
        [self.initiator, self.talker].into_iter().flatten().chain(self.joined.iter().copied())
    }

    /// The handles of every body in it, for a rollback restore to remap.
    pub fn bodies_mut(&mut self) -> impl Iterator<Item = &mut Entity> + '_ {
        [self.initiator.as_mut(), self.talker.as_mut()].into_iter().flatten().chain(self.joined.iter_mut())
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

    pub fn input_owner(&self) -> Option<InputOwner> {
        self.live.as_ref().map(|live| live.input_owner)
    }

    /// The seat whose dialogue history this conversation is (Q134): its
    /// input owner. A conversation every seat drives, or none, is the
    /// primary seat's.
    pub fn history_seat(&self) -> u8 {
        match self.input_owner() {
            Some(InputOwner::Participant(id)) => id.slot(),
            _ => 0,
        }
    }

    pub fn dialogue_id(&self) -> Option<&str> {
        self.live.as_ref().map(LiveConversation::dialogue_id)
    }

    /// Which conversation is live, for a narrative record that has to name one.
    pub fn instance(&self) -> Option<&ConversationInstanceId> {
        self.live.as_ref().map(|live| &live.instance)
    }

    /// Every in-world body in it: the initiator, the talker, then the joined.
    ///
    ///  the continuity questions are about all of them — how far apart are they, was
    /// one hit — and a caller that reaches for [`Self::talker`] alone has already made the rule
    /// player-centric in the way the rule says it must not be (*"both characters should
    /// hover"*).
    pub fn participants(&self) -> impl Iterator<Item = Entity> + '_ {
        self.live.iter().flat_map(LiveConversation::bodies)
    }

    /// Whether the conversation holds `body`: it is the talker or it joined.
    /// The initiator is not held; its seat's input drives the text box.
    pub fn holds(&self, body: Entity) -> bool {
        self.held().any(|held| held == body)
    }

    /// The bodies the conversation holds: the talker, then the joined.
    pub fn held(&self) -> impl Iterator<Item = Entity> + '_ {
        self.live
            .iter()
            .flat_map(|live| live.talker.into_iter().chain(live.joined.iter().copied()))
    }

    /// Bring `body` into the live conversation. `false` when nothing is live
    /// or the body is in it already.
    pub fn join(&mut self, body: Entity) -> bool {
        let Some(live) = self.live.as_mut() else {
            return false;
        };
        if live.bodies().any(|member| member == body) {
            return false;
        }
        live.joined.push(body);
        true
    }

    /// Take `body` out of the live conversation.
    ///
    /// The conversation ends when the initiator leaves, or when fewer than
    /// two bodies are left: nobody is talking to anybody. When the talker
    /// leaves and others stay, the conversation goes on with no talker: the
    /// body it was opened with is gone, and the ones who joined are not
    /// promoted into its place (a verb that acts on the talker acts on nobody).
    pub fn leave(&mut self, body: Entity) -> Leaving {
        let Some(live) = self.live.as_mut() else {
            return Leaving::NotAMember;
        };
        if live.initiator == Some(body) {
            self.close();
            return Leaving::Closed;
        }
        if live.talker == Some(body) {
            live.talker = None;
        } else if let Some(at) = live.joined.iter().position(|member| *member == body) {
            live.joined.remove(at);
        } else {
            return Leaving::NotAMember;
        }
        if live.bodies().count() < 2 {
            self.close();
            return Leaving::Closed;
        }
        Leaving::Left
    }

    /// `by` breaks into the live conversation: it joins, and the narrative
    /// goes on from `node` as of `tick` (a new instance, so the text box
    /// starts that node). `false` when nothing is live.
    pub fn interrupt(&mut self, by: Entity, node: impl Into<String>, tick: u64) -> bool {
        self.join(by);
        let Some(live) = self.live.as_mut() else {
            return false;
        };
        live.instance = live.instance.redirected(tick, node);
        true
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
        for entity in live.bodies_mut() {
            *entity = mapper.get_mapped(*entity);
        }
    }
}

/// What [`ActiveConversation::leave`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leaving {
    /// The body was not in a live conversation.
    NotAMember,
    /// It left, and the others talk on.
    Left,
    /// Its leaving ended the conversation.
    Closed,
}
