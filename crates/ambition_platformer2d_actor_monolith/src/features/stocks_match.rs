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
            team.map(|team| team.as_str().to_string())
                .unwrap_or_else(|| format!("seat {}", seat.0 + 1)),
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
