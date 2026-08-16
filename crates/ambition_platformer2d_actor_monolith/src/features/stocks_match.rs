//! **The ruleset-facing half of the stocks loop: when is the match over?** (S4)
//!
//! The engine owns the COUNT (`ambition_combat::stocks`) and this owns the
//! QUESTION it makes answerable — which side is the last one with a fighter in
//! play. It lives in `ambition_platformer2d_actor_monolith` rather than beside the count because it
//! needs `MatchSeat`, which is this crate's, and the predicate it calls
//! (`last_side_standing`) is the same one the versus stage settles rounds with.
//! One predicate, two liveness inputs: a round asks "is this fighter's health
//! above zero", a stocks match asks "does this fighter have a stock left".

use bevy::prelude::{Has, MessageWriter, Query, Res, ResMut};

use crate::time::time_control::{ClockRequester, ClockScaleRequest};
use ambition_time::ClockDomain;

use ambition_combat::stocks::{
    last_side_standing, FighterEliminated, SidesOutcome, StocksMatchDecided, StocksMatchSettled,
};

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
    active: Option<Res<crate::character_runtime::ActiveMatch>>,
    fighters: Query<(
        &crate::character_runtime::MatchSeat,
        Option<&ambition_combat::targeting::MatchTeam>,
        &ambition_combat::components::FighterStocks,
        Has<FighterEliminated>,
    )>,
) {
    // Only a LIVE match can end. Without this the sweep runs over a stage that
    // is still seating and decides a match against a cast that is half built.
    if active.is_none() {
        // A match that went away un-decides itself, so the next one can be
        // decided. Re-entering the stage is an ordinary thing to do.
        settled.0 = false;
        return;
    }
    if settled.0 {
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
    settled.0 = true;
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
/// every tick makes it self-healing: the tick a new match is activated is the
/// tick `StocksMatchSettled` is retracted (see `activate_the_prepared_match`),
/// and this puts the clock back by itself.
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
    active: Option<Res<crate::character_runtime::ActiveMatch>>,
    mut pace: MessageWriter<ClockScaleRequest>,
) {
    // No match, no opinion. ⚠ `absence of an opinion is not an opinion` is the
    // sink's own rule, so leaving the stage leaves the clock wherever the last
    // match put it — which is exactly why the live arm below re-states full
    // speed rather than trusting a reset somewhere else.
    if active.is_none() {
        return;
    }
    pace.write(if settled.0 {
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
