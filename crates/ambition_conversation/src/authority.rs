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

impl LiveConversation {
    /// The seat whose dialogue history this conversation is (Q134): its
    /// input owner. A conversation every seat drives, or none, is the
    /// primary seat's.
    pub fn history_seat(&self) -> u8 {
        match self.input_owner {
            InputOwner::Participant(id) => id.slot(),
            _ => 0,
        }
    }

    /// The bodies it holds: the talker, then the joined. The initiator is not
    /// held; its seat's input drives the text box.
    pub fn held(&self) -> impl Iterator<Item = Entity> + '_ {
        self.talker.into_iter().chain(self.joined.iter().copied())
    }

    /// Whether `body` is in it, in any part.
    pub fn includes(&self, body: Entity) -> bool {
        self.bodies().any(|member| member == body)
    }

    /// Whether two conversations would claim one seat's input: an owner
    /// that accepts every seat overlaps every other.
    fn shares_a_seat_with(&self, other: &LiveConversation) -> bool {
        match (self.input_owner, other.input_owner) {
            (InputOwner::Participant(a), InputOwner::Participant(b)) => a == b,
            _ => true,
        }
    }
}

/// The conversations the simulation is having: each seat talks in its own
/// (SIMULATION-ISLANDS N1), so a player in another room does not wait on a
/// talk he is not in. Two conversations never share a body or a seat.
///
/// Kept in the order they opened. A question about "the" conversation names
/// whose: a body's ([`Self::of_body`]), a seat's ([`Self::of_owner`]).
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct ActiveConversation {
    live: Vec<LiveConversation>,
}

impl ActiveConversation {
    /// Open a conversation in ONE call.
    ///
    /// The whole value, so the COMPILER enumerates what a conversation is: a
    /// struct literal makes a new fact break every call site. A conversation
    /// that shares a body or a seat with this one is replaced by it, as the
    /// one conversation used to be.
    pub fn open(&mut self, live: LiveConversation) {
        self.live.retain(|other| !other.shares_a_seat_with(&live) && !live.bodies().any(|body| other.includes(body)));
        self.live.push(live);
    }

    /// End the conversation `body` is in. `false` when it is in none.
    pub fn close_of(&mut self, body: Entity) -> bool {
        let before = self.live.len();
        self.live.retain(|live| !live.includes(body));
        before != self.live.len()
    }

    /// End the conversation `owner`'s input drives. `false` when none.
    pub fn close_owned_by(&mut self, owner: InputOwner) -> bool {
        let before = self.live.len();
        self.live.retain(|live| live.input_owner != owner);
        before != self.live.len()
    }

    /// End the conversation with this identity. `false` when none is live.
    pub fn close_instance(&mut self, instance: &ConversationInstanceId) -> bool {
        let before = self.live.len();
        self.live.retain(|live| &live.instance != instance);
        before != self.live.len()
    }

    /// End every conversation (a session teardown, a scene that takes the world).
    pub fn close_all(&mut self) {
        self.live.clear();
    }

    /// Whether any conversation is live.
    pub fn is_live(&self) -> bool {
        !self.live.is_empty()
    }

    /// Every live conversation, in the order they opened.
    pub fn conversations(&self) -> impl Iterator<Item = &LiveConversation> + '_ {
        self.live.iter()
    }

    /// Every live conversation, for a restore to remap or a rule to edit.
    pub fn conversations_mut(&mut self) -> impl Iterator<Item = &mut LiveConversation> + '_ {
        self.live.iter_mut()
    }

    /// The conversation `body` is in, in any part.
    pub fn of_body(&self, body: Entity) -> Option<&LiveConversation> {
        self.live.iter().find(|live| live.includes(body))
    }

    /// The conversation `owner`'s input drives.
    pub fn of_owner(&self, owner: InputOwner) -> Option<&LiveConversation> {
        self.live.iter().find(|live| live.input_owner == owner)
    }

    /// The conversation a seat's input answers to: one it owns, or one every
    /// seat drives.
    pub fn of_seat(&self, seat: ambition_input::ParticipantId) -> Option<&LiveConversation> {
        self.live.iter().find(|live| live.input_owner.accepts(seat))
    }

    /// The conversation with this identity.
    pub fn of_instance(&self, instance: &ConversationInstanceId) -> Option<&LiveConversation> {
        self.live.iter().find(|live| &live.instance == instance)
    }

    /// Every in-world body in any conversation.
    pub fn participants(&self) -> impl Iterator<Item = Entity> + '_ {
        self.live.iter().flat_map(LiveConversation::bodies)
    }

    /// Whether a conversation holds `body`: it is a talker or it joined.
    pub fn holds(&self, body: Entity) -> bool {
        self.held().any(|held| held == body)
    }

    /// The bodies every conversation holds.
    pub fn held(&self) -> impl Iterator<Item = Entity> + '_ {
        self.live.iter().flat_map(LiveConversation::held)
    }

    /// Bring `body` into the conversation `member` is in. `false` when
    /// `member` is in none, or `body` is in one already.
    pub fn join(&mut self, member: Entity, body: Entity) -> bool {
        if self.of_body(body).is_some() {
            return false;
        }
        let Some(live) = self.live.iter_mut().find(|live| live.includes(member)) else {
            return false;
        };
        live.joined.push(body);
        true
    }

    /// Take `body` out of the conversation it is in.
    ///
    /// The conversation ends when the initiator leaves, or when fewer than
    /// two bodies are left: nobody is talking to anybody. When the talker
    /// leaves and others stay, the conversation goes on with no talker: the
    /// body it was opened with is gone, and the ones who joined are not
    /// promoted into its place (a verb that acts on the talker acts on nobody).
    pub fn leave(&mut self, body: Entity) -> Leaving {
        let Some(at) = self.live.iter().position(|live| live.includes(body)) else {
            return Leaving::NotAMember;
        };
        let live = &mut self.live[at];
        if live.initiator == Some(body) {
            self.live.remove(at);
            return Leaving::Closed;
        }
        if live.talker == Some(body) {
            live.talker = None;
        } else if let Some(slot) = live.joined.iter().position(|member| *member == body) {
            live.joined.remove(slot);
        }
        if live.bodies().count() < 2 {
            self.live.remove(at);
            return Leaving::Closed;
        }
        Leaving::Left
    }

    /// `by` breaks into the conversation `member` is in: it joins, and the
    /// narrative goes on from `node` as of `tick` (a new instance, so the text
    /// box starts that node). `false` when `member` is in none.
    pub fn interrupt(&mut self, member: Entity, by: Entity, node: impl Into<String>, tick: u64) -> bool {
        self.join(member, by);
        let Some(live) = self.live.iter_mut().find(|live| live.includes(member)) else {
            return false;
        };
        live.instance = live.instance.redirected(tick, node);
        true
    }

    /// Every entity this resource names — the probe the rollback registration
    /// localizes through, so a desync here reports as the bodies' stable
    /// identities rather than as "a resource differs".
    pub fn referenced_entities(&self) -> Vec<Entity> {
        self.participants().collect()
    }
}

impl bevy::ecs::entity::MapEntities for ActiveConversation {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for live in &mut self.live {
            for entity in live.bodies_mut() {
                *entity = mapper.get_mapped(*entity);
            }
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
