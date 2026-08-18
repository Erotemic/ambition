//! **Which conversation this is, in a form a corrected timeline agrees with.**
//!
//! A narrative fact that crosses back into the simulation — the runner ran out
//! of lines, the player bought something, a choice provoked a fight — has to say
//! WHICH conversation produced it, or it applies to whatever happens to be live
//! when the simulation gets around to reading it. Naming the Yarn node is not
//! enough: talk to the same NPC twice and both conversations are `"chat"`.
//!
//! ## The contract, which is stronger than "a unique number"
//!
//! > **The id must be deterministically re-mintable by resimulation, from the
//! > conversation's own opening facts.**
//!
//! That is what rules out both of the obvious answers:
//!
//! * a **nonce** cannot be re-minted at all, so a resimulated tick would mint a
//!   different one and every record from the original run would stop matching;
//! * a **rollback-rewound counter** re-mints perfectly and is still wrong. It
//!   encodes HISTORY — how many conversations preceded this one — so a corrected
//!   branch that opened a *different* conversation mints the same number for it,
//!   and the abandoned branch's records apply to a conversation that never had
//!   them. The counter would rewind exactly as designed and hand the record to
//!   the wrong conversation anyway.
//!
//! So the id is a function of CONTENT: when it opened, which node, which two
//! bodies, and **what Yarn is entered with**.
//!
//! ## ⭐ The rule for what belongs in it
//!
//! > **If two authoritative openings can make Yarn observe different narrative
//! > semantics, they must not share an identity.**
//!
//! That is the test — not "is it a field on the conversation". It admits the
//! [`ambition_dialog::DialogueContext`] ids, which the bridge publishes as
//! `$speaker_id`, `$listener_id` and `$speaker_is_self` for content to branch on,
//! and it excludes two things that look eligible:
//!
//! * ⛔ **`speaker_name` is PRESENTATION.** It is the display string the box
//!   shows when a line carries no prefix; a localization changing it is not a
//!   different conversation, and Yarn never sees it.
//! * ⛔ **`ConversationInputOwner` is CONTROL ROUTING**, and putting it here
//!   would be actively harmful. It publishes nothing into Yarn and selects no
//!   node — it decides which seats the box captures, and
//!   `declare_in_session_input_contexts` re-reads it off the rollback-owned
//!   authority every tick, so a correction repairs it without identity's help.
//!   Meanwhile it is derived from the initiator's `Brain`, which possession
//!   transfers at runtime: keying on it would make "somebody else took over the
//!   body mid-sentence" a *different conversation*, so the in-flight narrative
//!   end would stop matching and the projection would restart the runner from the
//!   top under a player who is mid-sentence. That is the exact defect the
//!   projection's attachment memo exists to prevent.
//!
//! ⚠ **the desync probe in `crate::rollback_registration` hashes more than
//! this on purpose, and that is not a contradiction.** Its question is *"do two peers
//! agree about the live conversation"*, which covers every authoritative field;
//! this type's question is *"is this the same conversation"*. `input_owner` is a
//! yes to the first and a no to the second.
//!
//! ## Why `SimId` AND the character ids, rather than either alone
//!
//! * ⛔ **not `Entity`** — GGRS remaps entity handles on `LoadWorld`, so an id
//!   built from one names a different body after a restore. This is the same
//!   reason `SimId` exists at all.
//! * **the `SimId`s alone are not enough**, and this is what the GPT 5.6 review
//!   (D29) found. A body's dialogue identity is not fixed: for a body with no
//!   authored `ActorIdentity` it is the `WornCharacter` it currently wears, which
//!   is rollback-owned and runtime-mutable. Two corrected timelines can therefore
//!   agree on the tick, the node and both `SimId`s while entering Yarn with a
//!   different `$speaker_id` — one conversation by the old id, two by any honest
//!   reading of what the player is in.
//! * **the character ids alone are not enough either**, which is what the earlier
//!   revision of this paragraph was right about: they are CHARACTER ids, and two
//!   identical NPCs standing in one room share theirs. The review's own
//!   counterexample (talker A replaced by talker B, same node, same tick) is
//!   exactly what a character id cannot separate.
//!
//! Neither pair subsumes the other, so the id carries both.
//!
//! ⭐ **and the context is stored HERE rather than beside this** — see
//! [`Self::context`]. `LiveConversation` used to carry a `DialogueContext` as a
//! sibling of its instance id, which was one question with two answers to keep in
//! step; the answer that mattered was in the field whose whole job is identity,
//! and it was the one that did not have it.
//!
//! ## A struct, not a flattened string
//!
//! `SimId` is a string because a desync report has to read as a sentence, and it
//! pays for that with an escape function so that `placement("giant/0")` and
//! `spawned(placement("giant"), 0)` cannot collide. Nothing here needs to pay
//! that: separate fields are injective for free, and [`Display`] still prints the
//! readable form for a report.
//!
//! ## ⚠ Still open: a composition with no clock
//!
//! A composition that never advances `SimTick` maps every opening to tick zero,
//! so two visits to one NPC through one node, with the same identities on both
//! sides, are one conversation. The fix above narrows that — a re-wear between
//! the two visits now separates them — and does not close it. It is a degenerate
//! clock rather than a hole in this type (every shipped composition gets
//! `SimTick` from `ambition_platformer2d_runtime`'s sim core and advances it once
//! per step), and calling it that is not the same as answering it. See
//! [`Self::opened_at`].

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
    /// **Who Yarn is told is speaking** — published as `$speaker_id`. Empty for
    /// a scripted conversation with no in-world speaker.
    ///
    /// ⚠ **not the same question as [`Self::initiator`].** That names a BODY;
    /// this names the character that body is being at the opening tick, which for
    /// a body with no authored identity is its `WornCharacter` — rollback-owned
    /// and runtime-mutable.
    speaker: String,
    /// **Who Yarn is told is being spoken to** — published as `$listener_id`.
    listener: String,
}

impl ConversationInstanceId {
    /// Mint the id for a conversation opening now.
    ///
    /// ⚠ **every argument is read off the world at the opening tick**, which is
    /// what makes a resimulation of that tick mint an equal id. A caller that
    /// reaches for anything else — a counter, a frame number, a wall clock — has
    /// broken the contract in the module docs.
    ///
    /// ⭐ **it takes the whole [`ambition_dialog::DialogueContext`], not two
    /// strings**, so the identity is minted from the very value that will be
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

    /// **The identity context Yarn is entered with**, rebuilt from the identity
    /// that decided it.
    ///
    /// ⭐ **rebuilt, not stored a second time.** `speaker_is_self` is a function
    /// of the two ids — [`ambition_dialog::DialogueContext::between`] is the one
    /// place that comparison is made — so keeping the built value alongside these
    /// fields would be a second answer that could drift from them.
    pub fn context(&self) -> ambition_dialog::DialogueContext {
        ambition_dialog::DialogueContext::between(&self.speaker, &self.listener)
    }

    /// The `SimTick` this conversation opened on.
    ///
    /// ⚠ **a composition whose clock never advances cannot tell two visits
    /// apart**, and that is a degenerate clock rather than a hole in this type:
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
        // ⚠ the identities Yarn sees, printed only when there are any: a scripted
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

    /// **The review's counterexample**: a corrected branch replaces the body
    /// being talked to, and everything else about the opening is identical.
    ///
    /// ⛔ `(node, opened_at)` reports these as one conversation, so the abandoned
    /// branch's narrative end would close the corrected branch's conversation —
    /// a different conversation, with a different body, that nobody has finished.
    ///
    /// ⚠ **the two talkers wear the same character**, which is the point: this is
    /// the case the dialogue ids cannot separate, so it is what the `SimId`s are
    /// for. The mirror case is
    /// [`two_speakers_at_one_tick_through_one_node_are_two_conversations`].
    #[test]
    fn two_talkers_at_one_tick_through_one_node_are_two_conversations() {
        let twins = between("player", "npc_guard");
        let to_a =
            ConversationInstanceId::mint(100, "chat", placement("player"), placement("a"), &twins);
        let to_b =
            ConversationInstanceId::mint(100, "chat", placement("player"), placement("b"), &twins);
        assert_ne!(to_a, to_b);

        // ⭐ and the INITIATOR separates them too: two seats at the couch can
        // reach the same NPC on the same tick, and whose conversation it is is
        // part of what it is.
        let by_other =
            ConversationInstanceId::mint(100, "chat", placement("other"), placement("a"), &twins);
        assert_ne!(to_a, by_other);
    }

    /// **The mirror**: the same two BODIES, and a different `$speaker_id`.
    ///
    /// ⛔ the D29 defect. A body with no authored identity speaks as the
    /// `WornCharacter` it currently wears — rollback-owned and runtime-mutable —
    /// so a correction can leave the tick, the node and both `SimId`s untouched
    /// while Yarn is entered as somebody else. The behavioural half of this,
    /// driven through the real opening, is
    /// `conversation::tests::two_worn_characters_are_two_conversations`.
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

    /// **The context comes back out of the id**, so nothing has to keep a second
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

    /// **A resimulated opening mints the SAME id**, which is the half that makes
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

    /// **A body with no `SimId` degrades to a weaker id rather than to a wrong
    /// one.** `None` is a distinct value, so an anonymous talker is separated
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

    /// **A desync report still reads as a sentence**, which is the whole reason
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
