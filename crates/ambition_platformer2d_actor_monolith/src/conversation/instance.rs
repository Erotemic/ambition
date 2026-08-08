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
//! So the id is a function of CONTENT: when it opened, which node, and which two
//! bodies. Two conversations that agree on all of those are the same
//! conversation — there is only one Yarn runner, running in real time, and it is
//! showing that one.
//!
//! ## Why `SimId` and not the things that were already there
//!
//! * ⛔ **not `Entity`** — GGRS remaps entity handles on `LoadWorld`, so an id
//!   built from one names a different body after a restore. This is the same
//!   reason `SimId` exists at all.
//! * ⛔ **not [`ambition_dialog::DialogueContext`]'s speaker/listener ids** —
//!   those are CHARACTER ids, and two identical NPCs standing in one room share
//!   theirs. The review's own counterexample (talker A replaced by talker B, same
//!   node, same tick) is exactly what a character id cannot separate.
//!
//! ## A struct, not a flattened string
//!
//! `SimId` is a string because a desync report has to read as a sentence, and it
//! pays for that with an escape function so that `placement("giant/0")` and
//! `spawned(placement("giant"), 0)` cannot collide. Nothing here needs to pay
//! that: separate fields are injective for free, and [`Display`] still prints the
//! readable form for a report.

use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// One logical conversation, identified by what the simulation decided when it
/// opened. See the module docs for why every ingredient is a content fact.
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
}

impl ConversationInstanceId {
    /// Mint the id for a conversation opening now.
    ///
    /// ⚠ **every argument is read off the world at the opening tick**, which is
    /// what makes a resimulation of that tick mint an equal id. A caller that
    /// reaches for anything else — a counter, a frame number, a wall clock — has
    /// broken the contract in the module docs.
    pub fn mint(
        opened_at: u64,
        node: impl Into<String>,
        initiator: Option<SimId>,
        talker: Option<SimId>,
    ) -> Self {
        Self {
            opened_at,
            node: node.into(),
            initiator,
            talker,
        }
    }

    /// The `SimTick` this conversation opened on.
    ///
    /// ⚠ **a composition whose clock never advances cannot tell two visits
    /// apart**, and that is a degenerate clock rather than a hole in this type:
    /// every shipped composition gets `SimTick` from
    /// `ambition_platformer2d_runtime`'s sim core and advances it once per step.
    /// A unit fixture that opens two conversations between the same two bodies
    /// through the same node, at a standing tick, is asking a question with one
    /// answer.
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(id: &str) -> Option<SimId> {
        Some(SimId::placement(id))
    }

    /// **The review's counterexample**: a corrected branch replaces the body
    /// being talked to, and everything else about the opening is identical.
    ///
    /// ⛔ `(node, opened_at)` reports these as one conversation, so the abandoned
    /// branch's narrative end would close the corrected branch's conversation —
    /// a different conversation, with a different body, that nobody has finished.
    #[test]
    fn two_talkers_at_one_tick_through_one_node_are_two_conversations() {
        let to_a = ConversationInstanceId::mint(100, "chat", placement("player"), placement("a"));
        let to_b = ConversationInstanceId::mint(100, "chat", placement("player"), placement("b"));
        assert_ne!(to_a, to_b);

        // ⭐ and the INITIATOR separates them too: two seats at the couch can
        // reach the same NPC on the same tick, and whose conversation it is is
        // part of what it is.
        let by_other =
            ConversationInstanceId::mint(100, "chat", placement("other"), placement("a"));
        assert_ne!(to_a, by_other);
    }

    /// **A resimulated opening mints the SAME id**, which is the half that makes
    /// a record from the original run still apply to its own conversation.
    #[test]
    fn re_minting_the_same_opening_is_equal() {
        let original =
            ConversationInstanceId::mint(100, "chat", placement("player"), placement("a"));
        let replayed =
            ConversationInstanceId::mint(100, "chat", placement("player"), placement("a"));
        assert_eq!(original, replayed);

        // The next visit to the same NPC is a different conversation.
        let next_visit =
            ConversationInstanceId::mint(140, "chat", placement("player"), placement("a"));
        assert_ne!(original, next_visit);
    }

    /// **A body with no `SimId` degrades to a weaker id rather than to a wrong
    /// one.** `None` is a distinct value, so an anonymous talker is separated
    /// from a named one — what it cannot do is separate two anonymous talkers,
    /// and the honest place for that to be visible is here.
    #[test]
    fn an_anonymous_body_is_not_the_same_as_a_named_one() {
        let anonymous = ConversationInstanceId::mint(100, "chat", None, None);
        let named = ConversationInstanceId::mint(100, "chat", placement("player"), placement("a"));
        assert_ne!(anonymous, named);
    }
}
