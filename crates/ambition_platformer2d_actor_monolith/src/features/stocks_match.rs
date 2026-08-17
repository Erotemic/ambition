//! **The ruleset-facing half of the stocks loop: when is the match over?** (S4)
//!
//! The engine owns the COUNT (`ambition_combat::stocks`) and this owns the
//! QUESTION it makes answerable — which side is the last one with a fighter in
//! play. It lives in `ambition_platformer2d_actor_monolith` rather than beside the count because it
//! needs `MatchSeat`, which is this crate's, and the predicate it calls
//! (`last_side_standing`) is the same one the versus stage settles rounds with.
//! One predicate, two liveness inputs: a round asks "is this fighter's health
//! above zero", a stocks match asks "does this fighter have a stock left".

use bevy::prelude::{Has, MessageWriter, Query, Res, ResMut, Resource, World};

use crate::character_runtime::{ActiveMatch, MatchInstance};
use crate::time::time_control::{ClockRequester, ClockScaleRequest};
use ambition_time::ClockDomain;

use ambition_combat::stocks::{
    last_side_standing, FighterEliminated, SidesOutcome, StocksMatchDecided,
};

/// **THE STOCKS OUTCOME FOR ONE MATCH: which match has been settled.**
///
/// Set once [`decide_stocks_match`] has written a [`StocksMatchDecided`], so the
/// outcome is announced once rather than every tick after it becomes true.
///
/// ⛔⛔ **this was a bare `bool` about the PROCESS, and D140 is what that
/// costs.** A match that ended set it true; nothing on this stage set it back,
/// because the only retraction was `decide_stocks_match` observing NO active
/// match and there is no tick between two matches on which the receipt is
/// absent. Match two opened wearing match one's verdict and could never be
/// decided — Jon, 2026-08-16: *"the GO stays on the screen for the entire match,
/// and the match does not end."* The repair at the time was to retract it from
/// `activate_the_prepared_match`, which worked and which made the GENERIC match
/// activation road know that one particular ruleset keeps a private boolean
/// latch (D147).
///
/// ⭐ **it is not a timeless global. It is the outcome for match X**, and saying
/// so is the whole fix: a verdict stamped with the match it is about goes stale
/// BY CONSTRUCTION when a different match activates. Nobody retracts it, nothing
/// has to be ordered against activation, and a composition that never installed
/// this ruleset is not mentioned anywhere on the activation road.
///
/// ⚠ still a resource rather than a `Local`, and still rollback state: a `Local`
/// does not rewind, and this gates a message the ruleset acts on, so a rewind
/// across the deciding frame must be able to un-decide the match. It rewinds
/// alongside [`ActiveMatch`], which is what makes the comparison below correct
/// after a rewind rather than merely plausible.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StocksMatchSettled(Option<MatchInstance>);

impl StocksMatchSettled {
    /// **Has THIS match been decided?** A verdict for a different match is not
    /// this match's, which is the whole reason the stamp is here.
    pub fn settled(&self, active: &ActiveMatch) -> bool {
        self.0 == Some(active.instance())
    }

    /// Record that this match has been decided.
    pub fn settle(&mut self, active: &ActiveMatch) {
        self.0 = Some(active.instance());
    }

    /// Rebuild from a rollback snapshot. See `snapshot_impls`.
    #[doc(hidden)]
    pub fn from_snapshot(decided: Option<MatchInstance>) -> Self {
        Self(decided)
    }

    /// The match this verdict is about, for the wire format. ⚠ **not a
    /// "has anything been decided" predicate** — that question needs the live
    /// match to compare against, which is [`Self::settled`].
    #[doc(hidden)]
    pub fn decided_match(&self) -> Option<MatchInstance> {
        self.0
    }
}

/// **Has the LIVE match been decided?** — both halves of the question, for a
/// caller holding a world rather than a system's parameters.
///
/// ⚠ it exists so that a test or a tool asking "is the match over" cannot ask
/// the half of it that used to be the whole thing. A latch with a verdict in it
/// says nothing on its own; it has to be the verdict for the match that is
/// running.
pub fn the_live_match_is_settled(world: &World) -> bool {
    match (
        world.get_resource::<ActiveMatch>(),
        world.get_resource::<StocksMatchSettled>(),
    ) {
        (Some(active), Some(settled)) => settled.settled(active),
        _ => false,
    }
}

/// Decide a stocks match, once.
///
/// ⚠ **no sort, and that is deliberate rather than an oversight.**
/// `last_side_standing` folds each row into a `BTreeMap` with `|=`, so the
/// result does not depend on the order the query yields entities — which is the
/// one thing a Bevy query does not promise. A version that collected and sorted
/// would be adding ceremony to buy a property the fold already has.
pub fn decide_stocks_match(
    mut settled: ResMut<StocksMatchSettled>,
    mut decided: MessageWriter<StocksMatchDecided>,
    active: Option<Res<ActiveMatch>>,
    fighters: Query<(
        &crate::character_runtime::MatchSeat,
        Option<&ambition_combat::targeting::MatchTeam>,
        &ambition_combat::components::FighterStocks,
        Has<FighterEliminated>,
    )>,
) {
    // Only a LIVE match can end. Without this the sweep runs over a stage that
    // is still seating and decides a match against a cast that is half built.
    //
    // ⛔ this used to RETRACT the latch here — *"a match that went away
    // un-decides itself"* — and it was the whole retraction, which is why the
    // second match on a stage that never removes its receipt inherited the
    // first's verdict (D140). Nothing is written on this path now: the verdict
    // names its match, so the next one is undecided without anybody saying so.
    let Some(active) = active else {
        return;
    };
    if settled.settled(&active) {
        return;
    }
    let mut any = false;
    let outcome = last_side_standing(fighters.iter().map(|(seat, team, _, eliminated)| {
        any = true;
        (
            // The same naming rule the stage's scoreboard uses: a declared team,
            // or the seat when a match declared none and every seat is a side.
            // ⚠ it is `stocks::side_label` now rather than a copy of the rule —
            // a winner card that names a side has to name the SAME side.
            ambition_combat::stocks::side_label(seat.0, team),
            !eliminated,
        )
    }));
    if !any {
        return;
    }
    let Some(outcome) = outcome else {
        return;
    };
    settled.settle(&active);
    decided.write(StocksMatchDecided {
        winner: match outcome {
            SidesOutcome::Winner(side) => Some(side),
            SidesOutcome::Draw => None,
        },
    });
}

/// **THE PACE A MATCH RUNS AT — full speed while it is undecided, STOPPED once
/// it is over.** (D140)
///
/// Jon, 2026-08-16: *"When there is only 1 player alive or 1 team alive for team
/// matches the time in the game should freeze with 'WINNER: <name>' to show the
/// match is over, and not let players continue to play after the match ends."*
///
/// ⭐ **one statement, both halves.** A system that only spoke when the match was
/// over would leave the freeze standing: nothing else says "back to full speed"
/// on a CPU-vs-CPU stage, because the only other producer of a neutral request
/// is [`emit_player_time_intent_system`](crate::time::time_control::emit_player_time_intent_system),
/// which reads the PRIMARY PLAYER and has nothing to say in a match with none.
/// So the first frozen match would freeze every match after it. Saying the pace
/// every tick makes it self-healing: a new activation is a new
/// [`MatchInstance`], so the previous match's verdict stops applying on the tick
/// the cast is built, and this puts the clock back by itself.
///
/// ⚠ **it composes with hitstop rather than fighting it**, and that is a
/// property of the sink rather than of this: `apply_clock_scale_requests`
/// reduces a frame's granted requests by `min`, so a `1.0` from here and a `0.0`
/// from a landed hit still yield the hitstop. The reduction is why saying "full
/// speed" every tick is safe at all — under a last-wins sink it would have
/// erased every slow-motion effect in the game.
///
/// ⛔ **not a control hold.** `ScriptedControl` (the opening ceremony's
/// instrument) stops a body from ACTING while the world keeps moving, which is
/// what a countdown wants and not what an ending wants — a winner launched off
/// the top of the screen would go on travelling under it. The clock is the thing
/// that means "the game stopped".
pub fn state_the_matchs_pace(
    settled: Res<StocksMatchSettled>,
    active: Option<Res<ActiveMatch>>,
    mut pace: MessageWriter<ClockScaleRequest>,
) {
    // No match, no opinion. ⚠ `absence of an opinion is not an opinion` is the
    // sink's own rule, so leaving the stage leaves the clock wherever the last
    // match put it — which is exactly why the live arm below re-states full
    // speed rather than trusting a reset somewhere else.
    let Some(active) = active else {
        return;
    };
    pace.write(if settled.settled(&active) {
        ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 0.0,
            requester: ClockRequester::Engine,
            reason: "match_over",
        }
    } else {
        ClockScaleRequest {
            domain: ClockDomain::SimClock,
            scale: 1.0,
            requester: ClockRequester::Engine,
            reason: "match_live",
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

    fn match_activated_on(tick: u64) -> ActiveMatch {
        ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(tick))
    }

    /// **THE PROPERTY D147 BOUGHT, AND THE BUG D140 WAS.** (D147)
    ///
    /// A verdict is *the outcome for match X*. The second match on a stage that
    /// never removes its receipt used to open wearing the first match's verdict,
    /// because the verdict was a process-global `bool` and the only thing that
    /// retracted it was a tick with no match on it — which two matches in a row
    /// never produce. Jon watched that: *"the GO stays on the screen for the
    /// entire match, and the match does not end."*
    ///
    /// ⚠ **nothing here retracts anything**, and that is the assertion. The
    /// second match is undecided because it is a DIFFERENT match, not because
    /// somebody remembered to clear a latch — so there is no ordering to get
    /// wrong and nothing for generic match activation to know about this
    /// ruleset.
    #[test]
    fn the_previous_matchs_verdict_does_not_settle_this_one() {
        let first = match_activated_on(100);
        let second = match_activated_on(900);

        let mut settled = StocksMatchSettled::default();
        assert!(
            !settled.settled(&first),
            "a fresh latch already considered a match decided"
        );
        settled.settle(&first);
        assert!(
            settled.settled(&first),
            "the match that was just decided does not read as decided"
        );
        assert!(
            !settled.settled(&second),
            "the NEXT match on this stage opened wearing the previous one's \
             verdict — the second match can never be decided and the ceremony \
             never hands the card over, which is exactly what Jon reported"
        );
    }

    /// A verdict for a match in ANOTHER session is not this session's either —
    /// the quit-to-title road, which Jon also walked.
    #[test]
    fn a_verdict_from_another_session_does_not_settle_this_match() {
        let mut settled = StocksMatchSettled::default();
        settled.settle(&ActiveMatch::activated(
            2,
            None,
            Some(SessionScopeId(0)),
            Some(100),
        ));
        assert!(
            !settled.settled(&ActiveMatch::activated(
                2,
                None,
                Some(SessionScopeId(1)),
                Some(100),
            )),
            "a new session's match inherited the previous session's verdict"
        );
    }

    /// **A live activation may adopt a seat topology mid-match**
    /// ([`ActiveMatch::adopt_seat_topology`]), and that must not un-decide a
    /// match that has already been announced.
    ///
    /// ⚠ this is the reason the identity is the ACTIVATION's two facts rather
    /// than the whole receipt: keying on a value with a mutable field in it
    /// would put the winner card back on a live clock the moment anything
    /// touched it.
    #[test]
    fn adopting_a_seat_topology_does_not_un_decide_the_match() {
        let mut active = match_activated_on(100);
        let mut settled = StocksMatchSettled::default();
        settled.settle(&active);
        active.adopt_seat_topology(7);
        assert!(
            settled.settled(&active),
            "recording which topology decided this seating un-decided the match \
             it had already announced"
        );
    }
}
