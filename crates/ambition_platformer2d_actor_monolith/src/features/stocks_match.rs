//! The ruleset-facing half of the stocks loop: when is the match over? (S4)
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
use ambition_time::time_control::{ClockRequester, ClockScaleRequest};
use ambition_time::ClockDomain;

use ambition_combat::stocks::{
    last_side_standing, FighterEliminated, MatchAbandoned, MatchVerdict, SidesOutcome,
    StocksMatchDecided,
};

/// THE STOCKS OUTCOME FOR ONE MATCH: which match has been settled.
///
/// Set once [`decide_stocks_match`] has written a [`StocksMatchDecided`], so the
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
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StocksMatchSettled(Option<MatchInstance>);

impl StocksMatchSettled {
    /// Has THIS match been decided? A verdict for a different match is not
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

    /// The match this verdict is about, for the wire format. not a
    /// "has anything been decided" predicate — that question needs the live
    /// match to compare against, which is [`Self::settled`].
    #[doc(hidden)]
    pub fn decided_match(&self) -> Option<MatchInstance> {
        self.0
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

/// Decide a stocks match, once.
///
/// no sort, and that is deliberate rather than an oversight.
/// `last_side_standing` folds each row into a `BTreeMap` with `|=`, so the
/// result does not depend on the order the query yields entities — which is the
/// one thing a Bevy query does not promise. A version that collected and sorted
/// would be adding ceremony to buy a property the fold already has.
pub fn decide_stocks_match(
    mut settled: ResMut<StocksMatchSettled>,
    mut decided: MessageWriter<StocksMatchDecided>,
    // THE THIRD WAY A MATCH ENDS: somebody stopped it. Read here rather than in
    // a system of its own so that "the match is over" has exactly one author and
    // the once-only latch below covers all three roads.
    mut abandoned: bevy::prelude::MessageReader<MatchAbandoned>,
    active: Option<Res<ActiveMatch>>,
    // The CLOCK's half, and both are optional for the same reason every
    // other pair here is: a bare fixture has no sim clock and no prepared plan,
    // and the honest answer there is a match with no time limit.
    prepared: Option<Res<crate::character_runtime::PreparedMatch>>,
    tick: Option<Res<ambition_time::SimTick>>,
    fighters: Query<(
        &crate::character_runtime::MatchSeat,
        Option<&ambition_combat::targeting::MatchTeam>,
        &ambition_combat::components::FighterStocks,
        Option<&ambition_characters::actor::BodyHealth>,
        Has<FighterEliminated>,
    )>,
) {
    // Only a LIVE match can end. Without this the sweep runs over a stage that
    // is still seating and decides a match against a cast that is half built.
    //
    // Nothing is written on this path now: the verdict names its match, so the next one is
    // undecided without anybody saying so.
    let Some(active) = active else {
        return;
    };
    if settled.settled(&active) {
        // Drain anyway: a stop request arriving after the fight settled itself
        // is answered by the match already being over, and leaving it in the
        // channel would end the NEXT match the moment it activates.
        abandoned.clear();
        return;
    }
    // ⭐ ASKED FIRST, AND IT IS THE ONLY ONE OF THE THREE THAT NEEDS NO CAST.
    // The two roads below both read the fighters, so both answer `return` on a
    // stage that is still seating — and a player who opens the menu during the
    // opening ceremony and picks Exit Match is entitled to an answer.
    if abandoned.read().count() > 0 {
        settled.settle(&active);
        decided.write(StocksMatchDecided {
            outcome: MatchVerdict::NoContest,
        });
        return;
    }
    let mut any = false;
    let outcome = last_side_standing(fighters.iter().map(|(seat, team, _, _, eliminated)| {
        any = true;
        (
            // The same naming rule the stage's scoreboard uses: a declared team,
            // or the seat when a match declared none and every seat is a side.
            // it is `stocks::side_label` now rather than a copy of the rule —
            // a winner card that names a side has to name the SAME side.
            ambition_combat::stocks::side_label(seat.0, team),
            !eliminated,
        )
    }));
    if !any {
        return;
    }
    // THE CLOCK IS THE SECOND WAY A MATCH ENDS, and it is asked SECOND.
    // A last-side-standing verdict on the tick the clock runs out is still a
    // knockout — the fighters settled it themselves — and reading the clock
    // first would relabel it as a timeout with the same winner and a worse
    // story. So the timeout only speaks when the fight has not.
    let outcome = match outcome {
        Some(outcome) => outcome,
        None => {
            let expired = prepared
                .as_ref()
                .zip(tick.as_ref())
                .and_then(|(prepared, tick)| {
                    active
                        .ticks_since_activation(tick.get())
                        .map(|elapsed| prepared.rules().time_expired(elapsed))
                })
                .unwrap_or(false);
            if !expired {
                return;
            }
            decide_on_the_clock(&fighters)
        }
    };
    settled.settle(&active);
    decided.write(StocksMatchDecided {
        outcome: outcome.into(),
    });
}

/// Who is ahead when the clock runs out, by the genre's tiebreak order.
///
/// ```text
/// 1. most STOCKS left        the fight's own currency
/// 2. least DAMAGE taken      how close each side came to losing one
/// 3. a DRAW                  genuinely level
/// ```
///
/// sides, not fighters, and the fold is what makes teams work: a team's
/// stocks are its members' summed, so a 2v2 where one side has three stocks
/// spread over two bodies beats a side with two on one.
///
/// PARTIAL against the genre, and named rather than implied: Ultimate
/// sends a level match to SUDDEN DEATH — both fighters at 300%, one stock, first
/// hit decides — where this calls it a draw. Sudden death is a second match
/// staged from the first's result, which is a lifecycle question rather than a
/// counting one, and belongs with whoever owns the stage transition.
fn decide_on_the_clock(
    fighters: &Query<(
        &crate::character_runtime::MatchSeat,
        Option<&ambition_combat::targeting::MatchTeam>,
        &ambition_combat::components::FighterStocks,
        Option<&ambition_characters::actor::BodyHealth>,
        Has<FighterEliminated>,
    )>,
) -> SidesOutcome {
    // BTreeMap, so a tie is broken the same way on a replay rather than by hash
    // order — the same reason `last_side_standing` uses one.
    let mut sides: std::collections::BTreeMap<String, (u32, i32)> =
        std::collections::BTreeMap::new();
    for (seat, team, stocks, health, _) in fighters.iter() {
        let entry = sides
            .entry(ambition_combat::stocks::side_label(seat.0, team))
            .or_insert((0, 0));
        entry.0 += stocks.remaining;
        entry.1 += health.map_or(0, |h| h.damage_taken());
    }
    clock_outcome(&sides)
}

/// The tiebreak itself, over folded sides — split from the query so the RULE can
/// be asked without a `World` and the fold cannot hide inside the answer.
fn clock_outcome(sides: &std::collections::BTreeMap<String, (u32, i32)>) -> SidesOutcome {
    // one comparison on `(stocks, -damage)`, so the two rungs cannot disagree
    // about which is the tiebreak.
    let Some(best) = sides
        .values()
        .map(|(stocks, damage)| (*stocks, -*damage))
        .max()
    else {
        return SidesOutcome::Draw;
    };
    let mut leaders = sides
        .iter()
        .filter(|(_, (stocks, damage))| (*stocks, -*damage) == best);
    match (leaders.next(), leaders.next()) {
        (Some((side, _)), None) => SidesOutcome::Winner(side.clone()),
        _ => SidesOutcome::Draw,
    }
}

/// THE PACE A MATCH RUNS AT — full speed while it is undecided, STOPPED once it is over.
///
/// matches the time in the game should freeze with 'WINNER: <name>' to show the
/// match is over, and not let players continue to play after the match ends."*
///
/// one statement, both halves. A system that only spoke when the match was
/// over would leave the freeze standing: nothing else says "back to full speed"
/// on a CPU-vs-CPU stage, because the only other producer of a neutral request
/// is [`emit_player_time_intent_system`](crate::time::time_control::emit_player_time_intent_system),
/// which reads the PRIMARY PLAYER and has nothing to say in a match with none.
/// So the first frozen match would freeze every match after it. Saying the pace
/// every tick makes it self-healing: a new activation is a new
/// [`MatchInstance`], so the previous match's verdict stops applying on the tick
/// the cast is built, and this puts the clock back by itself.
///
/// not a control hold. `ScriptedControl` (the opening ceremony's
/// instrument) stops a body from ACTING while the world keeps moving, which is
/// what a countdown wants and not what an ending wants — a winner launched off
/// the top of the screen would go on travelling under it. The clock is the thing
/// that means "the game stopped".
pub fn state_the_matchs_pace(
    settled: Res<StocksMatchSettled>,
    active: Option<Res<ActiveMatch>>,
    mut pace: MessageWriter<ClockScaleRequest>,
) {
    // No match, no opinion. `absence of an opinion is not an opinion` is the
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

    fn side(label: &str, stocks: u32, damage: i32) -> (String, u32, i32) {
        (label.to_string(), stocks, damage)
    }

    /// WHEN THE CLOCK RUNS OUT, THE SIDE WITH MOST STOCKS TAKES IT.
    ///
    /// three rungs and all three asserted, because a tiebreak that never
    /// reaches its second rung is a tiebreak with one rung: stocks decide, then
    /// damage, then it is genuinely level. A version that stopped at stocks
    /// would call every equal-stock timeout a draw, which on a stock match is
    /// nearly all of them.
    #[test]
    fn the_clock_decides_by_stocks_then_damage_then_calls_it_level() {
        let decide = |rows: Vec<(String, u32, i32)>| {
            let mut sides: std::collections::BTreeMap<String, (u32, i32)> =
                std::collections::BTreeMap::new();
            for (label, stocks, damage) in rows {
                let entry = sides.entry(label).or_insert((0, 0));
                entry.0 += stocks;
                entry.1 += damage;
            }
            super::clock_outcome(&sides)
        };

        assert_eq!(
            decide(vec![side("a", 2, 300), side("b", 1, 0)]),
            SidesOutcome::Winner("a".to_string()),
            "a stock lead lost to a damage lead"
        );
        assert_eq!(
            decide(vec![side("a", 2, 300), side("b", 2, 80)]),
            SidesOutcome::Winner("b".to_string()),
            "level on stocks, and the tiebreak never reached damage"
        );
        assert_eq!(
            decide(vec![side("a", 2, 80), side("b", 2, 80)]),
            SidesOutcome::Draw,
            "a genuinely level match invented a winner"
        );
        // a team's stocks are its members' SUMMED — three across two bodies
        // beats two on one.
        assert_eq!(
            decide(vec![
                side("red", 2, 0),
                side("red", 1, 0),
                side("blue", 2, 0)
            ]),
            SidesOutcome::Winner("red".to_string()),
        );
    }

    /// AN UNTIMED MATCH NEVER EXPIRES, AND A TIMED ONE DOES EXACTLY ONCE.
    ///
    /// the floor: `time_limit_ticks == 0` is every roster that existed before
    /// a clock did, and a clock that read zero as "already over" would decide
    /// every one of them on their first tick.
    #[test]
    fn a_match_with_no_declared_clock_has_no_clock() {
        let untimed = crate::character_runtime::MatchRules::default();
        assert_eq!(untimed.time_remaining(0), None);
        assert!(!untimed.time_expired(0));
        assert!(!untimed.time_expired(u64::MAX));

        let timed = crate::character_runtime::MatchRules {
            time_limit_ticks: 120,
            ..Default::default()
        };
        assert_eq!(timed.time_remaining(0), Some(120));
        assert_eq!(timed.time_remaining(119), Some(1));
        assert!(!timed.time_expired(119));
        assert!(timed.time_expired(120));
        assert!(timed.time_expired(10_000), "the clock un-expired");
    }

    /// A verdict is *the outcome for match X*.
    ///
    /// nothing here retracts anything, and that is the assertion. The
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

    /// A live activation may adopt a seat topology mid-match
    /// ([`ActiveMatch::adopt_seat_topology`]), and that must not un-decide a
    /// match that has already been announced.
    ///
    /// this is the reason the identity is the ACTIVATION's two facts rather
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
