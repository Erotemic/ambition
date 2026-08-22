//! What ends a conversation that the world keeps running through.
//!
//! > "if you get hit in dialog, dialog needs to be interrupted I think. Or say
//! > you are falling and you talk to a the flying parrot, if you fall away from
//! > them dialog should also break … A broken dialog can have some bark to
//! > indicate that it was broken."
//!
//! A modal dialogue needs none of this: the world is frozen, so nothing can
//! disturb it. Once the world keeps running (`GameMode::Dialogue` left the
//! suspend set), a conversation is a SUSTAINED condition that has to keep being
//! possible — and the two ways it stops being possible are the two arms below.
//!
//! ## Why the rule is here and the facts are not
//!
//! This module is pure and names no gameplay type. Whether a body was knocked
//! about lives in `BodyCombat`; whether two bodies are in talking range is an
//! AABB test in the actor crate. Gathering those is a system's job. Deciding
//! what they MEAN is this one's, so the meaning can be tested without a World
//! and cannot quietly differ between the interaction that starts a conversation
//! and the rule that ends it.

/// Why a conversation stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogueBreak {
    /// A participant was knocked about.
    ///
    /// knockback, not damage. The reason a hit ends a conversation is
    /// that it MOVES you — so a poison tick, a chip of environmental damage, or
    /// anything else that leaves both bodies standing where they were does not.
    /// The signal is the recoil/hitstun lock, which is exactly "something took
    /// your body away from you for a moment".
    Struck,
    /// The participants stopped being close enough to talk.
    ///
    /// The falling-past-a-parrot case. note what does NOT need to be checked
    /// here: whether either body can hold station. A participant that CAN hold
    /// station does not drift out of range, so the hold keeps this arm from
    /// firing without this arm knowing the hold exists. The two rules compose
    /// without either naming the other.
    Separated,
}

impl DialogueBreak {
    /// Whether this break deserves a bark of its own.
    ///
    /// `Struck` does NOT, and that is the finding rather than an
    /// omission: a body knocked about already barks through
    /// `npc_hit_bark_line`, which fires on every strike and falls back to a
    /// generic line when a character authored none. A second bubble for one
    /// event is worse than no bubble.
    ///
    /// Walking away is the genuinely new occasion — nothing in the game
    /// currently says anything when a conversation is simply left.
    pub fn wants_its_own_bark(self) -> bool {
        matches!(self, Self::Separated)
    }

    /// Which break, if any, ends a conversation in this state.
    ///
    /// `in_reach` is the same proximity that STARTED the conversation, not a
    /// second authored range. You stay in talking range or you stop talking, and
    /// borrowing the interaction's own test means the two cannot drift apart.
    ///
    /// Being struck wins, because it is the more specific thing that happened: a
    /// hit that also knocks you out of range should say you were hit.
    pub fn evaluate(any_struck: bool, in_reach: bool) -> Option<Self> {
        if any_struck {
            Some(Self::Struck)
        } else if !in_reach {
            Some(Self::Separated)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_undisturbed_conversation_does_not_break() {
        assert_eq!(DialogueBreak::evaluate(false, true), None);
    }

    #[test]
    fn either_participant_being_struck_ends_it() {
        // symmetric by construction: the caller folds both bodies into one
        // flag, so there is no place for "was the PLAYER hit" to creep in.
        assert_eq!(
            DialogueBreak::evaluate(true, true),
            Some(DialogueBreak::Struck)
        );
    }

    #[test]
    fn falling_out_of_range_ends_it() {
        assert_eq!(
            DialogueBreak::evaluate(false, false),
            Some(DialogueBreak::Separated)
        );
    }

    #[test]
    fn a_hit_that_also_separates_reports_the_hit() {
        // The more specific thing that happened. A player knocked across the
        // room should be told they were hit, not that they wandered off.
        assert_eq!(
            DialogueBreak::evaluate(true, false),
            Some(DialogueBreak::Struck)
        );
    }

    #[test]
    fn only_walking_away_asks_for_a_line_of_its_own() {
        // Being struck already barks through the combat path, which always
        // produces a line. Asking for a second would double the bubble.
        assert!(!DialogueBreak::Struck.wants_its_own_bark());
        assert!(DialogueBreak::Separated.wants_its_own_bark());
    }
}
