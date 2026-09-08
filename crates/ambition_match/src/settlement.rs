//! Rollback-safe match settlement state.
//!
//! These facts describe one `MatchInstance`, not a particular stocks ruleset.
//! Rules decide and enter sudden death from above; match identity owns the stamped
//! state so clocks, presentation, and other consumers do not depend on the actor
//! monolith's rules module.

use bevy::prelude::{Resource, World};
use ambition_combat::stocks::MatchVerdict;
use crate::{ActiveMatch, MatchInstance};

/// THE STOCKS OUTCOME FOR ONE MATCH: which match has been settled.
///
/// Set once a stocks ruleset has decided the live match, so the
/// outcome is announced once rather than every tick after it becomes true.
///
/// this was a bare `bool` about the PROCESS, and is what that costs. A match that ended set
/// it true; nothing on this stage set it back, because the only retraction was
/// `decide_stocks_match` observing NO active match and there is no tick between two matches on
/// which the receipt is absent.
///
/// it is not a timeless global. It is the outcome for match X, and saying
/// so is the whole fix: a verdict stamped with the match it is about goes stale
/// BY CONSTRUCTION when a different match activates. Nobody retracts it, nothing
/// has to be ordered against activation, and a composition that never installed
/// this ruleset is not mentioned anywhere on the activation road.
///
/// still a resource rather than a `Local`, and still rollback state: a `Local`
/// does not rewind, and this gates a message the ruleset acts on, so a rewind
/// across the deciding frame must be able to un-decide the match. It rewinds
/// alongside [`ActiveMatch`], which is what makes the comparison below correct
/// after a rewind rather than merely plausible.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct StocksMatchSettled(Option<(MatchInstance, MatchVerdict)>);

impl StocksMatchSettled {
    /// Has THIS match been decided? A verdict for a different match is not
    /// this match's, which is the whole reason the stamp is here.
    pub fn settled(&self, active: &ActiveMatch) -> bool {
        self.decided_match() == Some(active.instance())
    }

    /// Record that this match has been decided, and HOW.
    ///
    /// ⭐⭐ THE VERDICT LIVES HERE BECAUSE PRESENTATION MAY NOT READ A MESSAGE.
    /// The winner card and the return countdown both reacted to
    /// `StocksMatchDecided`, which a SPECULATIVE frame can write — and neither
    /// is retractable. The countdown was fixed by reading this latch, which
    /// rewinds; the CARD could not follow because the latch said only WHETHER,
    /// and the outcome it needs was in the message.
    ///
    /// ⛔ AND WAITING FOR CONFIRMATION IS NOT ENOUGH ON A MESSAGE. A reader that
    /// declines to consume until the frame is confirmed keeps its cursor, and a
    /// message channel is two frames deep — so a confirmation arriving later
    /// than that loses the announcement rather than delaying it. State has no
    /// cursor.
    pub fn settle(&mut self, active: &ActiveMatch, verdict: MatchVerdict) {
        self.0 = Some((active.instance(), verdict));
    }

    /// How THIS match ended, or `None` for a match that has not been decided.
    pub fn verdict(&self, active: &ActiveMatch) -> Option<&MatchVerdict> {
        self.0
            .as_ref()
            .filter(|(instance, _)| *instance == active.instance())
            .map(|(_, verdict)| verdict)
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(decided: Option<(MatchInstance, MatchVerdict)>) -> Self {
        Self(decided)
    }

    /// The match this verdict is about, for the wire format. not a
    /// "has anything been decided" predicate — that question needs the live
    /// match to compare against, which is [`Self::settled`].
    #[doc(hidden)]
    pub fn decided_match(&self) -> Option<MatchInstance> {
        self.0.as_ref().map(|(instance, _)| instance.clone())
    }

    /// The verdict this latch holds, for the wire format.
    #[doc(hidden)]
    pub fn decided_verdict(&self) -> Option<&MatchVerdict> {
        self.0.as_ref().map(|(_, verdict)| verdict)
    }
}

/// Has the LIVE match been decided? — both halves of the question, for a
/// caller holding a world rather than a system's parameters.
///
/// A latch with a verdict in it says nothing on its own; it has to be the verdict for the match
/// that is running.
pub fn the_live_match_is_settled(world: &World) -> bool {
    match (
        world.get_resource::<ActiveMatch>(),
        world.get_resource::<StocksMatchSettled>(),
    ) {
        (Some(active), Some(settled)) => settled.settled(active),
        _ => false,
    }
}

/// THE MATCH ENTERED SUDDEN DEATH, and WHICH match it is about.
///
/// ⭐ THE SAME STAMPED SHAPE AS [`StocksMatchSettled`], for the same reason and
/// with the same payoff: a fact about match X goes stale BY CONSTRUCTION when
/// match Y activates, so nobody has to retract it and nothing has to be ordered
/// against activation.
///
/// ⛔⛔ AND IT IS WHAT KEEPS THE CLOCK FROM RE-FIRING. Sudden death is entered by
/// NOT settling the match, so `time_expired` stays true for every tick that
/// follows — without this latch the tie would be re-entered sixty times a second
/// and every fighter would be reset to the starting damage forever.
///
/// Rollback state for the reason its sibling is: this gates a message the
/// ruleset acts on, so a rewind across the entering frame must be able to
/// un-enter it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuddenDeathEntered(Option<MatchInstance>);

impl SuddenDeathEntered {
    /// Is THIS match in sudden death?
    pub fn entered(&self, active: &ActiveMatch) -> bool {
        self.0 == Some(active.instance())
    }

    /// Record that this match has entered it.
    pub fn enter(&mut self, active: &ActiveMatch) {
        self.0 = Some(active.instance());
    }

    /// Which match, for the wire format. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn entered_match(&self) -> Option<MatchInstance> {
        self.0
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(entered: Option<MatchInstance>) -> Self {
        Self(entered)
    }
}
