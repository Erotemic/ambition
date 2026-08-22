//! Deterministic identity for one logical conversation.
//!
//! The id must be re-mintable during resimulation from the conversation's opening
//! semantics, not from process-local identity or prior history. It includes the
//! opening tick, Yarn node, both bodies' `SimId`s, and the speaker/listener ids
//! visible to Yarn.
//!
//! Presentation (`speaker_name`) and control routing (`ConversationInputOwner`) do
//! not affect identity. Entity handles are also excluded because rollback may remap
//! them.
//!
//! A host must advance `SimTick`: repeated identical openings at a permanently
//! zero tick cannot be distinguished.

use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// One logical conversation, identified by what the simulation decided when it
/// opened. See the module docs for why every ingredient is a content fact, and
/// for the rule that decides what is an ingredient.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationInstanceId {
    /// The `SimTick` it opened on.
    opened_at: u64,
    /// The Yarn node that is running.
    node: String,
    /// The body that walked up and started it. `None` for a scripted
    /// conversation with no in-world initiator, or for a body with no `SimId`.
    initiator: Option<SimId>,
    /// The body being talked TO.
    talker: Option<SimId>,
    /// Who Yarn is told is speaking — published as `$speaker_id`. Empty for
    /// a scripted conversation with no in-world speaker.
    ///
    ///  not the same question as [`Self::initiator`]. That names a BODY;
    /// this names the character that body is being at the opening tick, which for
    /// a body with no authored identity is its `WornCharacter` — rollback-owned
    /// and runtime-mutable.
    speaker: String,
    /// Who Yarn is told is being spoken to — published as `$listener_id`.
    listener: String,
}

impl ConversationInstanceId {
    /// Mint the id for a conversation opening now.
    ///
    ///  every argument is read off the world at the opening tick, which is
    /// what makes a resimulation of that tick mint an equal id. A caller that
    /// reaches for anything else — a counter, a frame number, a wall clock — has
    /// broken the contract in the module docs.
    ///
    ///  it takes the whole [`ambition_dialog::DialogueContext`], not two
    /// strings, so the identity is minted from the very value that will be
    /// published to Yarn. A caller cannot mint an id for one context and enter
    /// the runner with another.
    pub fn mint(
        opened_at: u64,
        node: impl Into<String>,
        initiator: Option<SimId>,
        talker: Option<SimId>,
        context: &ambition_dialog::DialogueContext,
    ) -> Self {
        Self {
            opened_at,
            node: node.into(),
            initiator,
            talker,
            speaker: context.speaker_id.clone(),
            listener: context.listener_id.clone(),
        }
    }

    /// The identity context Yarn is entered with, rebuilt from the identity
    /// that decided it.
    ///
    ///  rebuilt, not stored a second time. `speaker_is_self` is a function
    /// of the two ids — [`ambition_dialog::DialogueContext::between`] is the one
    /// place that comparison is made — so keeping the built value alongside these
    /// fields would be a second answer that could drift from them.
    pub fn context(&self) -> ambition_dialog::DialogueContext {
        ambition_dialog::DialogueContext::between(&self.speaker, &self.listener)
    }

    /// The `SimTick` this conversation opened on.
    ///
    ///  a composition whose clock never advances cannot tell two visits
    /// apart, and that is a degenerate clock rather than a hole in this type:
    /// every shipped composition gets `SimTick` from
    /// `ambition_platformer2d_runtime`'s sim core and advances it once per step.
    /// A unit fixture that opens two conversations between the same two bodies,
    /// through the same node, under the same identities, at a standing tick, is
    /// asking a question with one answer.
    pub const fn opened_at(&self) -> u64 {
        self.opened_at
    }

    /// Which Yarn node is running.
    pub fn node(&self) -> &str {
        &self.node
    }
}

impl std::fmt::Display for ConversationInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "conversation:{}@{}", self.node, self.opened_at)?;
        if let Some(initiator) = &self.initiator {
            write!(f, " by {initiator}")?;
        }
        if let Some(talker) = &self.talker {
            write!(f, " to {talker}")?;
        }
        //  the identities Yarn sees, printed only when there are any: a scripted
        // conversation has none, and an empty `as ->` in a desync report is
        // noise a reader has to learn to skip.
        if !self.speaker.is_empty() || !self.listener.is_empty() {
            write!(f, " as {} -> {}", self.speaker, self.listener)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(id: &str) -> Option<SimId> {
        Some(SimId::placement(id))
    }

    fn between(speaker: &str, listener: &str) -> ambition_dialog::DialogueContext {
        ambition_dialog::DialogueContext::between(speaker, listener)
    }

    /// Different talker identities at the same tick/node must mint different conversation instances,
    /// even when both bodies wear the same character/dialogue identity.
    #[test]
    fn two_talkers_at_one_tick_through_one_node_are_two_conversations() {
        let twins = between("player", "npc_guard");
        let to_a =
            ConversationInstanceId::mint(100, "chat", placement("player"), placement("a"), &twins);
        let to_b =
            ConversationInstanceId::mint(100, "chat", placement("player"), placement("b"), &twins);
        assert_ne!(to_a, to_b);

        // Initiator identity is also part of the conversation instance.
        let by_other =
            ConversationInstanceId::mint(100, "chat", placement("other"), placement("a"), &twins);
        assert_ne!(to_a, by_other);
    }

    /// The mirror: the same two BODIES, and a different `$speaker_id`.
    ///
    /// A body with no authored identity speaks as the `WornCharacter` it currently wears —
    /// rollback-owned and runtime-mutable — so a correction can leave the tick, the node and
    /// both `SimId`s untouched while Yarn is entered as somebody else. The behavioural half of
    /// this, driven through the real opening, is
    /// `conversation:tests:two_worn_characters_are_two_conversations`.
    #[test]
    fn two_speakers_at_one_tick_through_one_node_are_two_conversations() {
        let as_mary = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &between("mary_o", "npc_admiral"),
        );
        let as_sanic = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &between("sanic", "npc_admiral"),
        );
        assert_ne!(as_mary, as_sanic);

        // And the LISTENER: the same body can be re-worn too, and content
        // branches on `$listener_id`.
        let to_someone_else = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &between("mary_o", "npc_captain"),
        );
        assert_ne!(as_mary, to_someone_else);
    }

    /// The context comes back out of the id, so nothing has to keep a second
    /// copy of it in step — including the derived self flag.
    #[test]
    fn the_context_round_trips_through_the_id() {
        let talking_to_itself = between("mary_o", "mary_o");
        assert!(talking_to_itself.speaker_is_self, "precondition");
        let id = ConversationInstanceId::mint(
            100,
            "chat__self",
            placement("player"),
            placement("player"),
            &talking_to_itself,
        );
        assert_eq!(id.context(), talking_to_itself);

        let scripted = ambition_dialog::DialogueContext::scripted();
        let id = ConversationInstanceId::mint(100, "intro", None, None, &scripted);
        assert_eq!(id.context(), scripted);
    }

    /// A resimulated opening mints the SAME id, which is the half that makes
    /// a record from the original run still apply to its own conversation.
    #[test]
    fn re_minting_the_same_opening_is_equal() {
        let context = between("mary_o", "npc_admiral");
        let original = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &context,
        );
        let replayed = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &context,
        );
        assert_eq!(original, replayed);

        // The next visit to the same NPC is a different conversation.
        let next_visit = ConversationInstanceId::mint(
            140,
            "chat",
            placement("player"),
            placement("a"),
            &context,
        );
        assert_ne!(original, next_visit);
    }

    /// A body with no `SimId` degrades to a weaker id rather than to a wrong
    /// one. `None` is a distinct value, so an anonymous talker is separated
    /// from a named one — what it cannot do is separate two anonymous talkers,
    /// and the honest place for that to be visible is here.
    #[test]
    fn an_anonymous_body_is_not_the_same_as_a_named_one() {
        let anonymous = ConversationInstanceId::mint(
            100,
            "chat",
            None,
            None,
            &between("mary_o", "npc_admiral"),
        );
        let named = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &between("mary_o", "npc_admiral"),
        );
        assert_ne!(anonymous, named);
    }

    /// A desync report still reads as a sentence, which is the whole reason
    /// this is a struct of readable parts rather than a hash.
    #[test]
    fn it_prints_who_is_speaking() {
        let id = ConversationInstanceId::mint(
            100,
            "chat",
            placement("player"),
            placement("a"),
            &between("mary_o", "npc_admiral"),
        );
        assert_eq!(
            id.to_string(),
            "conversation:chat@100 by placement:player to placement:a as mary_o -> npc_admiral"
        );

        // A scripted conversation has no identities, and says nothing about them.
        let scripted = ConversationInstanceId::mint(
            100,
            "intro",
            None,
            None,
            &ambition_dialog::DialogueContext::scripted(),
        );
        assert_eq!(scripted.to_string(), "conversation:intro@100");
    }
}
