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

use ambition_match::{ActiveMatch, MatchInstance};
use ambition_time::time_control::{ClockRequester, ClockScaleRequest};
use ambition_time::ClockDomain;

use ambition_combat::stocks::{
    last_side_standing, FighterEliminated, MatchVerdict, SidesOutcome, StocksMatchDecided,
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

/// A level timeout became sudden death. The RULESET does the rest — putting the
/// contenders on the authored damage is a stage's business, not the count's.
#[derive(bevy::prelude::Message, Clone, Debug, PartialEq, Eq)]
pub struct SuddenDeathBegan {
    /// The damage every contender starts on.
    pub starting_damage: i32,
    /// ⭐⭐ THE TIED SIDES, AND ONLY THOSE. Sudden death breaks a TIE, so the
    /// round is populated by the sides that were tied — with three or more
    /// sides alive at the timeout, a side the clock had already put behind is
    /// not one of them, and carrying it in would hand a losing side an even
    /// restart.
    ///
    /// Side labels in `BTreeMap` order, so a replay names them the same way.
    pub contenders: Vec<String>,
}

/// Decide a stocks match, once.
///
/// no sort, and that is deliberate rather than an oversight.
/// `last_side_standing` folds each row into a `BTreeMap` with `|=`, so the
/// result does not depend on the order the query yields entities — which is the
/// one thing a Bevy query does not promise. A version that collected and sorted
/// would be adding ceremony to buy a property the fold already has.
/// SOMEBODY ASKED TO STOP A MATCH — the third way one ends.
///
/// ⭐ A MATCH-LEVEL COMMAND, not a body's action. Jon: *"This should also work
/// in CPU-vs-CPU and other roster configurations; it is a match-level command,
/// not a player-body action."* So it carries no seat and no reason — only WHICH
/// MATCH is being stopped.
///
/// ⛔⛔ AND IT NAMES ITS MATCH BECAUSE IT CANNOT REWIND. ⛔ A `MatchAbandoned`
/// MESSAGE registered with `clear_message_on_rollback` CANNOT carry it: the
/// backend `.clear()`s the buffer rather than restoring the channel with its
/// cursor, so an Exit Match consumed on a speculative frame is simply GONE after
/// a rewind — the player presses it and the match keeps going.
///
/// ⛔ AND SNAPSHOTTING IT WOULD NOT HELP EITHER, which is the part that decides
/// the shape. The ask is made OUTSIDE the simulation, so a resimulation cannot
/// re-make it; rewinding a resource that holds it throws it away exactly as the
/// clear did. What survives both is a latch that does NOT rewind and is scoped by
/// the match it is about: a rewind leaves the ask standing, the resim reaches the
/// same verdict, and the next match ignores it because the instance differs.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub struct MatchAbandonRequest {
    of: Option<MatchInstance>,
}

impl MatchAbandonRequest {
    /// Ask for `active` to stop.
    pub fn stop(active: &ActiveMatch) -> Self {
        Self {
            of: Some(active.instance()),
        }
    }

    /// Is THIS match the one somebody asked to stop?
    pub fn asks_to_stop(&self, active: &ActiveMatch) -> bool {
        self.of.as_ref() == Some(&active.instance())
    }
}

pub fn decide_stocks_match(
    mut settled: ResMut<StocksMatchSettled>,
    mut decided: MessageWriter<StocksMatchDecided>,
    // SUDDEN DEATH's latch and its announcement. Both live here rather than in a
    // system of their own for the reason the abandon reader does: "the match is
    // over" and "the match refuses to be over" are one decision, and splitting
    // them would need an ordering between two systems reading one clock.
    mut sudden_death: ResMut<SuddenDeathEntered>,
    mut began: MessageWriter<SuddenDeathBegan>,
    // The stage's rules, for the one question this system asks of them.
    // `Option` for the reason every other reader of the projection is: a bare
    // fixture never installs it, and there the honest answer is no sudden death.
    combat_rules: Option<Res<ambition_combat::rules::ResolvedCombatTuning>>,
    // THE THIRD WAY A MATCH ENDS: somebody stopped it. Read here rather than in
    // a system of its own so that "the match is over" has exactly one author and
    // the once-only latch below covers all three roads.
    //
    // `Option`, because a composition with no shell offers no way to ask.
    abandoned: Option<bevy::prelude::Res<MatchAbandonRequest>>,
    active: Option<Res<ActiveMatch>>,
    // The CLOCK's half — the RULE (how long is this match). Optional for the
    // reason every other reader of the projection is: a bare fixture has no
    // prepared plan, and the honest answer there is a match with no time limit.
    // The COUNT is below it, and the pair is what a timeout is.
    prepared: Option<Res<ambition_match::PreparedMatch>>,
    // HOW LONG THIS MATCH HAS BEEN FOUGHT. Not `Option`: the clock is installed
    // by the same plugin this system is, so a composition that can decide a
    // match can count one.
    live: Res<crate::character_runtime::live_match_clock::LiveMatchTicks>,
    fighters: Query<(
        &ambition_match::MatchSeat,
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
        // A stop request arriving after the fight settled itself is answered by
        // the match already being over. Nothing to drain: the ask NAMES its
        // match, so it cannot leak into the next one.
        return;
    }
    // ⭐ ASKED FIRST, AND IT IS THE ONLY ONE OF THE THREE THAT NEEDS NO CAST.
    // The two roads below both read the fighters, so both answer `return` on a
    // stage that is still seating — and a player who opens the menu during the
    // opening ceremony and picks Exit Match is entitled to an answer.
    if abandoned.is_some_and(|ask| ask.asks_to_stop(&active)) {
        settled.settle(&active, MatchVerdict::NoContest);
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
            // ⭐ THE LIVE CLOCK, not ticks since the cast was built. The
            // ceremony and every pause are excluded there, once, by the one
            // system that owns the question — see `live_match_clock`.
            let expired = prepared
                .as_deref()
                .is_some_and(|prepared| prepared.rules().time_expired(live.of(&active)));
            if !expired {
                return;
            }
            // ⛔⛔ A MATCH ALREADY IN SUDDEN DEATH HAS NO CLOCK LEFT TO CONSULT,
            // and forgetting that made the mechanic "first damage wins".
            //
            // `expired` stays true for every tick after the timeout — clocks do
            // not un-expire — so this arm re-ran the tiebreak every frame. Both
            // fighters start level, which is a Draw and re-enters (the latch
            // absorbs it); then one takes a single point of damage without
            // dying, the tiebreak's damage rung answers WINNER, and the match
            // settled on a hit that killed nobody.
            //
            // ⇒ once the match is in sudden death the clock is spent. It ends
            // the ordinary way, by last side standing, which the arm above this
            // one already answers — so returning here is the whole fix.
            if sudden_death.entered(&active) {
                return;
            }
            // ⭐⭐ SUDDEN DEATH IS ENTERED INSTEAD OF DECIDING, which is what
            // keeps this out of the trap the mechanic is usually built into:
            // nothing mutates a finished match back into a running one, because
            // the match was never finished. It simply does not settle, the
            // survivors go to the authored damage, and the fight ends it the
            // ordinary way — last side standing.
            //
            // ⛔ ONLY ON A GENUINE TIE. A timeout with a leader is a WIN, and
            // sending a fighter who was ahead into a coin flip would take away
            // the thing the clock was measuring.
            // ONE FOLD, both readings. The verdict and the sudden-death field
            // are two questions about the same standing, and computing them
            // from separate passes is how they come to disagree about who was
            // tied with whom.
            let sides = fold_the_sides_on_the_clock(&fighters);
            let outcome = clock_outcome(&sides);
            if let Some(damage) = timeout_continues_as_sudden_death(
                &outcome,
                combat_rules
                    .as_deref()
                    .and_then(|rules| rules.sudden_death_damage),
            ) {
                if !sudden_death.entered(&active) {
                    sudden_death.enter(&active);
                    began.write(SuddenDeathBegan {
                        starting_damage: damage,
                        contenders: leading_sides(&sides),
                    });
                }
                // ⛔ AND IT RETURNS UNSETTLED. Every later tick is caught by the
                // guard above, which is what stops the spent clock from deciding
                // a match it no longer measures.
                return;
            }
            outcome
        }
    };
    let verdict: MatchVerdict = outcome.into();
    settled.settle(&active, verdict.clone());
    decided.write(StocksMatchDecided { outcome: verdict });
}

/// Does this timeout CONTINUE as sudden death instead of deciding?
///
/// Split from the query for the reason [`clock_outcome`] is: the RULE can then
/// be asked without a `World`, and the fold cannot hide inside the answer.
///
/// ⛔ ONLY ON A GENUINE TIE. A timeout with a leader is a WIN — sending a
/// fighter who was ahead into a coin flip would take away the thing the clock
/// was measuring. And a ruleset that declared no sudden-death damage does not
/// have the mechanic, so a level timeout there is simply a draw.
fn timeout_continues_as_sudden_death(
    outcome: &SidesOutcome,
    declared_damage: Option<i32>,
) -> Option<i32> {
    matches!(outcome, SidesOutcome::Draw)
        .then_some(declared_damage)
        .flatten()
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
fn fold_the_sides_on_the_clock(
    fighters: &Query<(
        &ambition_match::MatchSeat,
        Option<&ambition_combat::targeting::MatchTeam>,
        &ambition_combat::components::FighterStocks,
        Option<&ambition_characters::actor::BodyHealth>,
        Has<FighterEliminated>,
    )>,
) -> std::collections::BTreeMap<String, (u32, i32)> {
    // BTreeMap, so a tie is broken the same way on a replay rather than by hash
    // order — the same reason `last_side_standing` uses one.
    let mut sides: std::collections::BTreeMap<String, (u32, i32)> =
        std::collections::BTreeMap::new();
    for (seat, team, stocks, health, eliminated) in fighters.iter() {
        // ⛔⛔ A FIGHTER THAT IS OUT DOES NOT SCORE, and this column was QUERIED
        // AND DISCARDED (`_`) until 2026-08-25 — which made the tiebreak depend
        // on CLEANUP TIMING rather than on the match. An eliminated body stays
        // resident until a ruleset despawns it, so a teammate knocked out one
        // tick before the clock contributed nothing while one knocked out ON the
        // clock tick still contributed its damage. Two identical histories,
        // different standings.
        //
        // ⭐ THE STATISTIC IS "WHO IS STILL STANDING", which is the only reading
        // that does not depend on residency. A side whose fighters are all out
        // has already lost by last-side-standing and is not a timeout contender.
        //
        // ⚠ THE OTHER READING — whole-team match HISTORY — is defensible and is
        // a product rule, not a bug fix: it would need side-level scoring that
        // outlives a body, because components cannot store what a despawned
        // fighter did.
        if eliminated {
            continue;
        }
        let entry = sides
            .entry(ambition_combat::stocks::side_label(seat.0, team))
            .or_insert((0, 0));
        entry.0 += stocks.remaining;
        entry.1 += health.map_or(0, |h| h.damage_taken());
    }
    sides
}

/// WHO IS AHEAD when the clock runs out — every side that reaches the best
/// standing, which is one side when the clock decided it and several when it
/// did not.
///
/// ⭐ ONE DEFINITION, because two consumers ask this question and a match whose
/// WINNER and whose SUDDEN-DEATH FIELD were computed by separate comparisons
/// could disagree about who was tied with whom.
///
/// Empty only when there are no sides at all. `BTreeMap` order, so a replay
/// lists them the same way.
fn leading_sides(sides: &std::collections::BTreeMap<String, (u32, i32)>) -> Vec<String> {
    // one comparison on `(stocks, -damage)`, so the two rungs cannot disagree
    // about which is the tiebreak.
    let Some(best) = sides
        .values()
        .map(|(stocks, damage)| (*stocks, -*damage))
        .max()
    else {
        return Vec::new();
    };
    sides
        .iter()
        .filter(|(_, (stocks, damage))| (*stocks, -*damage) == best)
        .map(|(side, _)| side.clone())
        .collect()
}

/// The tiebreak itself, over folded sides — split from the query so the RULE can
/// be asked without a `World` and the fold cannot hide inside the answer.
fn clock_outcome(sides: &std::collections::BTreeMap<String, (u32, i32)>) -> SidesOutcome {
    match leading_sides(sides).as_slice() {
        [alone] => SidesOutcome::Winner(alone.clone()),
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

    /// ⭐⭐ A LEVEL TIMEOUT CONTINUES; A TIMEOUT WITH A LEADER DECIDES.
    ///
    /// The rule sudden death turns on, and both halves matter. Sending a fighter
    /// who was AHEAD on the tiebreak into a coin flip would throw away the thing
    /// the clock spent eight minutes measuring — so the mechanic is reachable
    /// only from the one outcome that measured nothing.
    #[test]
    fn only_a_genuinely_level_timeout_becomes_sudden_death() {
        let level = SidesOutcome::Draw;
        let won = SidesOutcome::Winner("a".to_string());

        assert_eq!(
            super::timeout_continues_as_sudden_death(&level, Some(150)),
            Some(150),
            "a level timeout decided the match instead of continuing it"
        );
        assert_eq!(
            super::timeout_continues_as_sudden_death(&won, Some(150)),
            None,
            "a side that WON on the tiebreak was sent to sudden death anyway"
        );
        // ⛔ and a ruleset that never declared it does not have the mechanic:
        // a level timeout there is a draw, which is what every stage did before
        // this field existed.
        assert_eq!(
            super::timeout_continues_as_sudden_death(&level, None),
            None,
            "a stage that declared no sudden death got one"
        );
    }

    /// ⭐⭐ A MATCH IN SUDDEN DEATH IGNORES THE SPENT CLOCK.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS, found by review 2026-08-24: `expired` stays
    /// true for every tick after the timeout, so the tiebreak re-ran every
    /// frame. Both fighters start level (a Draw, absorbed by the latch); then one
    /// takes a single point of damage without dying, the tiebreak's DAMAGE rung
    /// answers Winner, and the match settled on a hit that killed nobody. Sudden
    /// death was "first damage wins".
    ///
    /// ⭐ THE TEST IS THE GUARD ITSELF, not the whole system: the fix is one
    /// early return keyed on the latch, and the tiebreak it skips is already
    /// covered by `the_clock_decides_by_stocks_then_damage_then_calls_it_level`.
    /// A full-system fixture here would need a PreparedMatch, a SimTick and a
    /// seated cast to assert a branch that is one comparison.
    #[test]
    fn a_match_in_sudden_death_no_longer_consults_the_clock() {
        let live = match_activated_on(100);
        let mut entered = SuddenDeathEntered::default();

        // Before entering: the clock still decides, which is what makes the
        // timeout a timeout.
        assert!(
            !entered.entered(&live),
            "a match that never tied is in sudden death"
        );

        entered.enter(&live);
        assert!(
            entered.entered(&live),
            "the guard cannot see the match it was just entered for, so the \
             spent clock keeps deciding and one point of damage ends the match"
        );

        // ⛔ AND IT IS PER MATCH. A later match must reach the clock again — a
        // guard that latched globally would make every subsequent timeout
        // undecidable.
        let next = match_activated_on(900);
        assert!(
            !entered.entered(&next),
            "the NEXT match inherited sudden death, so its clock can never \
             decide anything"
        );
    }

    /// ⭐⭐ THE LATCH IS WHAT STANDS BETWEEN SUDDEN DEATH AND A LOOP.
    ///
    /// ⛔⛔ Sudden death is entered by NOT SETTLING the match, so `time_expired`
    /// stays true for every tick that follows — the clock does not un-expire.
    /// Without a latch the tie is re-entered sixty times a second and both
    /// fighters are pinned at the starting damage forever, which is a match that
    /// can never end.
    ///
    /// ⭐ AND IT IS STAMPED WITH THE MATCH, so the NEXT match starts un-entered
    /// without anybody retracting anything — the same property that makes the
    /// verdict beside it safe.
    #[test]
    fn the_sudden_death_latch_fires_once_and_does_not_carry_to_the_next_match() {
        let first = match_activated_on(100);
        let mut entered = SuddenDeathEntered::default();
        assert!(
            !entered.entered(&first),
            "a fresh match started already in sudden death"
        );
        entered.enter(&first);
        assert!(
            entered.entered(&first),
            "entering sudden death did not record it, so the tie re-enters every \
             tick the clock stays expired"
        );

        let second = match_activated_on(900);
        assert!(
            !entered.entered(&second),
            "the NEXT match inherited a sudden death that belonged to the last \
             one — nobody retracts this, which is the whole reason it is stamped"
        );
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

    /// ⭐⭐ SUDDEN DEATH IS FOUGHT BY THE TIED SIDES, NOT BY EVERYONE ALIVE.
    ///
    /// ⛔⛔ THE DEFECT THIS PINS, found by review 2026-08-24: with three or more
    /// sides alive at a timeout, the round carried every SURVIVOR. A side the
    /// clock had already put behind on stocks got an even restart against the
    /// two it was losing to — the clock measured eight minutes and then handed
    /// its result back.
    ///
    /// ⭐ AND THE FIELD COMES FROM THE SAME COMPARISON AS THE VERDICT. Two
    /// passes over one standing is how "who won" and "who was tied" come to
    /// disagree, so `clock_outcome` is built on this function rather than
    /// beside it — a Winner is exactly the one-element case.
    #[test]
    fn sudden_death_is_fought_by_the_tied_leaders_and_not_by_every_survivor() {
        let leaders = |rows: Vec<(String, u32, i32)>| {
            let mut sides: std::collections::BTreeMap<String, (u32, i32)> =
                std::collections::BTreeMap::new();
            for (label, stocks, damage) in rows {
                let entry = sides.entry(label).or_insert((0, 0));
                entry.0 += stocks;
                entry.1 += damage;
            }
            super::leading_sides(&sides)
        };

        // THE CASE. Three sides alive, two of them level at the top: the third
        // is a stock behind and is not part of the tie the round exists to
        // break.
        assert_eq!(
            leaders(vec![side("a", 2, 80), side("b", 2, 80), side("c", 1, 0)]),
            vec!["a".to_string(), "b".to_string()],
            "a side the clock had already put behind was carried into sudden death"
        );
        // ⛔ AND THE DAMAGE RUNG SEPARATES TOO, or a side level on stocks but
        // visibly closer to losing one would join a tie it is not in.
        assert_eq!(
            leaders(vec![side("a", 2, 80), side("b", 2, 80), side("c", 2, 300)]),
            vec!["a".to_string(), "b".to_string()],
        );
        // A DECIDED timeout has exactly one leader — which is what makes
        // `clock_outcome` a special case of this and not a second opinion.
        assert_eq!(
            leaders(vec![side("a", 3, 0), side("b", 2, 0), side("c", 1, 0)]),
            vec!["a".to_string()],
        );
        // Every side level is every side tied: nobody is dropped from a round
        // that nobody is behind in.
        assert_eq!(
            leaders(vec![side("a", 2, 80), side("b", 2, 80), side("c", 2, 80)]),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
        );
    }

    /// A FIGHTER THAT IS OUT DOES NOT SCORE FOR ITS SIDE.
    ///
    /// ⛔⛔ THE COLUMN WAS QUERIED AND DISCARDED. `Has<FighterEliminated>` was
    /// bound to `_`, so an eliminated body folded its damage into its team's
    /// total for as long as it remained RESIDENT — and a body stays standing
    /// until a ruleset despawns it. Two identical histories then rank
    /// differently depending on whether the last stock was lost one tick before
    /// the clock or ON it. That is cleanup timing deciding a match.
    ///
    /// ⭐ AND THIS IS THE FIRST TEST TO RUN THE FOLD AT ALL — every existing arm
    /// builds the side map by hand and asserts the RANKING, so none of them
    /// could see what the fold puts in it.
    #[test]
    fn an_eliminated_teammate_does_not_fold_into_its_sides_total() {
        use ambition_characters::actor::{BodyHealth, Health};
        use ambition_combat::components::FighterStocks;
        use ambition_combat::targeting::MatchTeam;

        #[derive(bevy::prelude::Resource, Default)]
        struct Folded(std::collections::BTreeMap<String, (u32, i32)>);

        fn run_the_fold(
            fighters: Query<(
                &ambition_match::MatchSeat,
                Option<&MatchTeam>,
                &FighterStocks,
                Option<&BodyHealth>,
                Has<FighterEliminated>,
            )>,
            mut out: bevy::prelude::ResMut<Folded>,
        ) {
            out.0 = fold_the_sides_on_the_clock(&fighters);
        }

        let seat = |app: &mut bevy::prelude::App, index: usize, damage: i32, out: bool| {
            let mut health = BodyHealth::new(Health {
                current: 100,
                max: 100,
                invulnerable: Default::default(),
            });
            health.set_damage_taken(damage);
            let mut body = app.world_mut().spawn((
                ambition_match::MatchSeat(index),
                // ONE SIDE, two members — the composition the defect needs.
                MatchTeam("blue".to_string()),
                FighterStocks::new(if out { 0 } else { 2 }),
                health,
            ));
            if out {
                body.insert(FighterEliminated);
            }
        };

        let mut app = bevy::prelude::App::new();
        app.init_resource::<Folded>();
        app.add_systems(bevy::prelude::Update, run_the_fold);
        // A living teammate on 40, and one already OUT carrying 300.
        seat(&mut app, 0, 40, false);
        seat(&mut app, 1, 300, true);
        app.update();

        let folded = app.world().resource::<Folded>().0.clone();
        assert_eq!(
            folded.get("blue").copied(),
            Some((2, 40)),
            "the side folded an ELIMINATED teammate's stocks/damage into its \
             total, so its standing depends on whether that body has been \
             despawned yet rather than on the match"
        );
    }

    /// AN UNTIMED MATCH NEVER EXPIRES, AND A TIMED ONE DOES EXACTLY ONCE.
    ///
    /// the floor: `time_limit_ticks == 0` is every roster that existed before
    /// a clock did, and a clock that read zero as "already over" would decide
    /// every one of them on their first tick.
    #[test]
    fn a_match_with_no_declared_clock_has_no_clock() {
        let untimed = ambition_match::MatchRules::default();
        assert_eq!(untimed.time_remaining(0), None);
        assert!(!untimed.time_expired(0));
        assert!(!untimed.time_expired(u64::MAX));

        let timed = ambition_match::MatchRules {
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
        settled.settle(&first, MatchVerdict::Draw);
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
        settled.settle(
            &ActiveMatch::activated(2, None, Some(SessionScopeId(0)), Some(100)),
            MatchVerdict::Draw,
        );
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
        settled.settle(&active, MatchVerdict::Draw);
        active.adopt_seat_topology(7);
        assert!(
            settled.settled(&active),
            "recording which topology decided this seating un-decided the match \
             it had already announced"
        );
    }
}
