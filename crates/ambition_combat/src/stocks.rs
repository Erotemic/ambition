//! **Stocks: the loop a KO'd fighter actually goes round.** (S4 part 1)
//!
//! [`FighterStocks`](crate::components::FighterStocks) has existed as vocabulary
//! with no consumer — no rule spent one, nothing respawned, nothing was ever
//! eliminated, and it was not rollback state. This is the loop that makes it a
//! count of something.
//!
//! ## Why a KO needs its own signal
//!
//! The obvious implementation reads health: a ruleset watches for `!alive()` and
//! calls that a KO. It cannot work here, and the reason is the whole shape of
//! S4. A stocks fighter is [`DeathPolicy::Unbounded`] — its meter never kills,
//! because its death is the world's — so its pool is FULL at the moment it is
//! knocked off the stage. A ruleset watching health would watch a healthy
//! fighter fall out of the world forever.
//!
//! So the death paths announce it. [`BodyKnockedOut`] is written exactly where
//! the two `RulesetOwnsDeath` arms already decided that a match, not the world,
//! owns this body's death — the same branch, now saying so out loud instead of
//! leaving a ruleset to infer it from a health value that no longer moves.
//!
//! ## The authority split
//!
//! This module owns the COUNT: spend one, decide whether that was the last, mark
//! the fighter eliminated, and clear the meter so a respawning body comes back
//! at 0%. It does not know where a body goes, what a round is, or when a match
//! is over — those need a stage, a seat and a scoreboard, and they belong to the
//! ruleset. [`FighterStockSpent`] is the handoff.
//!
//! Engine-shaped rather than Smash-shaped: a lives counter is the same object,
//! and a game with three lives and a checkpoint uses this without knowing what a
//! stock is.

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageReader, MessageWriter, Query, Without,
};

use crate::components::FighterStocks;

/// **A body whose death a RULESET owns was knocked out this tick.**
///
/// Written from the `RulesetOwnsDeath` arms of both death paths — the player's
/// in `damage_apply` and the actor's in `actor_hit` — which is where the engine
/// already stops and hands the consequence over. Carries the cause so a ruleset
/// can tell a ring-out from a meter death without re-deriving it.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct BodyKnockedOut {
    pub body: Entity,
    pub cause: crate::HitSource,
}

/// **Out of the match**, as a fact on the body rather than a number to compare.
///
/// A marker rather than `stocks.remaining == 0` at every call site: elimination
/// is checked by rules, HUDs and the match-end condition, and three readers
/// re-deriving the same comparison is how one of them ends up using `<= 0` on a
/// `u32`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterEliminated;

/// **A stock was spent** — the ruleset's cue to place a body or end a match.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct FighterStockSpent {
    pub body: Entity,
    /// Stocks left AFTER this one was spent.
    pub remaining: u32,
    /// `true` when that was the last: this fighter is out.
    pub eliminated: bool,
}

/// **The set [`spend_fighter_stocks`] runs in — this tick's stock spend lands.**
///
/// A match's own rules (respawn placement, elimination, the winner announcement)
/// run after the spend in the same phase, so a match decided before this tick's
/// elimination does not announce the previous frame's answer.
///
/// ⚠ ONE member. `decide_stocks_match` is chained after and CONSUMES the spend;
/// including it would make a game's rules wait on the engine's decision, which
/// is the thing those rules are meant to run alongside.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FighterStocksSpent;

/// Spend one stock per knockout, and clear the meter of anyone coming back.
///
/// ⚠ **a fighter already eliminated is skipped, not spent again.** Without the
/// `Without<FighterEliminated>` filter a body that is out but still standing —
/// which it is, until a ruleset removes it — would keep absorbing knockouts, and
/// `spend()` saturates at zero, so the elimination would be re-announced every
/// time. A rule that ends the match on "somebody was eliminated" would then fire
/// on every subsequent KO of a corpse.
pub fn spend_fighter_stocks(
    mut commands: Commands,
    mut knockouts: MessageReader<BodyKnockedOut>,
    mut spent: MessageWriter<FighterStockSpent>,
    mut fighters: Query<&mut FighterStocks, Without<FighterEliminated>>,
    mut meters: Query<&mut ambition_characters::actor::BodyHealth>,
) {
    // Message order is write order, which is deterministic; nothing here sorts
    // or iterates a query, so there is no hash-order hazard to guard against.
    for knockout in knockouts.read() {
        let Ok(mut stocks) = fighters.get_mut(knockout.body) else {
            continue;
        };
        let eliminated = stocks.spend();
        let remaining = stocks.remaining;
        if eliminated {
            commands.entity(knockout.body).try_insert(FighterEliminated);
        } else if let Ok(mut health) = meters.get_mut(knockout.body) {
            // A fighter coming back comes back FRESH. The meter is the reason
            // it was knocked off the stage; carrying it into the next stock
            // would make the second one shorter than the first and the third
            // shorter again, which is a difficulty ramp nobody authored.
            health.reset();
        }
        spent.write(FighterStockSpent {
            body: knockout.body,
            remaining,
            eliminated,
        });
    }
}

/// **Which side, if any, is the only one left.**
///
/// The predicate a match-end condition is, lifted out of the versus stage so a
/// stocks match and a rounds match cannot drift apart on it. `in_play` is the
/// only thing that differs between them: a round asks "is this fighter's health
/// above zero", a stocks match asks "does this fighter have a stock left", and
/// everything after that question is identical.
///
/// `None` means the match continues — INCLUDING the three-side case where one
/// side has been wiped out and two are still fighting, which is the case a
/// `survivors.len() == 1` test written the obvious way gets wrong in the other
/// direction.
/// ⚠ **the caller names the sides.** An earlier draft resolved a teamless seat to
/// a label here and immediately baked a display convention into the engine — the
/// versus stage numbers its seats from ONE, and the engine numbered from zero, so
/// the two disagreed about who won. What a side is CALLED is the game's business;
/// this only answers whether one of them is the last.
pub fn last_side_standing(rows: impl Iterator<Item = (String, bool)>) -> Option<SidesOutcome> {
    let mut standing: std::collections::BTreeMap<String, bool> = std::collections::BTreeMap::new();
    for (side, in_play) in rows {
        let entry = standing.entry(side).or_insert(false);
        *entry |= in_play;
    }
    // ONE side is not a match. Without this the sole side is "wiped out" the
    // instant it falls and the stage awards a win against nobody.
    if standing.len() < 2 {
        return None;
    }
    // BTreeMap, so the survivor list is in a stable order rather than a hash
    // one — a tie broken by iteration order is a different winner on a replay.
    let survivors: Vec<&String> = standing
        .iter()
        .filter(|(_, in_play)| **in_play)
        .map(|(side, _)| side)
        .collect();
    match survivors.len() {
        0 => Some(SidesOutcome::Draw),
        1 => Some(SidesOutcome::Winner(survivors[0].clone())),
        _ => None,
    }
}

/// **The match is over, and this is who took it.**
///
/// Written once, by the ruleset-facing half of the loop, when
/// [`last_side_standing`] first answers. `None` is a draw — every side going out
/// together, which a two-fighter simultaneous ring-out reaches easily and which a
/// `winner: String` shape would have had to invent a sentinel for.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct StocksMatchDecided {
    pub winner: Option<String>,
}

/// Set once a [`StocksMatchDecided`] has been written, so the outcome is
/// announced once rather than every tick after it becomes true.
///
/// A resource rather than a `Local`, because a `Local` is not rollback state and
/// this gates a message the ruleset acts on: a rewind across the deciding frame
/// must be able to un-decide the match.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StocksMatchSettled(pub bool);

/// Who took it, if anyone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidesOutcome {
    Winner(String),
    Draw,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::{BodyHealth, DeathPolicy, Health};
    use bevy::prelude::*;

    fn stocks_app() -> App {
        let mut app = App::new();
        app.add_message::<BodyKnockedOut>();
        app.add_message::<FighterStockSpent>();
        app.add_systems(Update, spend_fighter_stocks);
        app
    }

    fn fighter(app: &mut App, stocks: u32) -> Entity {
        app.world_mut()
            .spawn((
                FighterStocks::new(stocks),
                BodyHealth::new(Health::new(50)).with_policy(DeathPolicy::Unbounded),
            ))
            .id()
    }

    fn knock_out(app: &mut App, body: Entity) {
        app.world_mut()
            .resource_mut::<Messages<BodyKnockedOut>>()
            .write(BodyKnockedOut {
                body,
                cause: crate::HitSource::LeftTheWorld,
            });
        app.update();
    }

    fn last_spend(app: &mut App) -> Option<FighterStockSpent> {
        let messages = app.world().resource::<Messages<FighterStockSpent>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).last().copied()
    }

    /// The whole loop, in the order a match runs it.
    #[test]
    fn a_fighter_spends_stocks_until_it_is_eliminated() {
        let mut app = stocks_app();
        let body = fighter(&mut app, 3);

        knock_out(&mut app, body);
        assert_eq!(
            last_spend(&mut app),
            Some(FighterStockSpent {
                body,
                remaining: 2,
                eliminated: false
            })
        );
        knock_out(&mut app, body);
        assert_eq!(last_spend(&mut app).map(|s| s.remaining), Some(1));
        assert!(
            app.world().get::<FighterEliminated>(body).is_none(),
            "a fighter with a stock left was marked out of the match"
        );

        knock_out(&mut app, body);
        let spent = last_spend(&mut app).expect("the last stock was spent");
        assert_eq!(spent.remaining, 0);
        assert!(
            spent.eliminated,
            "spending the last stock did not eliminate"
        );
        assert!(app.world().get::<FighterEliminated>(body).is_some());
    }

    /// **A respawning fighter comes back at 0%**, or every stock after the first
    /// is shorter than the one before it.
    #[test]
    fn spending_a_stock_clears_the_meter_that_caused_it() {
        let mut app = stocks_app();
        let body = fighter(&mut app, 2);
        app.world_mut()
            .get_mut::<BodyHealth>(body)
            .unwrap()
            .damage(140);
        assert_eq!(
            app.world().get::<BodyHealth>(body).unwrap().damage_taken(),
            140
        );

        knock_out(&mut app, body);
        assert_eq!(
            app.world().get::<BodyHealth>(body).unwrap().damage_taken(),
            0,
            "the fighter respawned still carrying the damage that killed it"
        );
    }

    /// An eliminated body is still standing until a ruleset removes it, so it
    /// can be knocked out again. That must not re-announce the elimination.
    #[test]
    fn an_eliminated_fighter_is_not_spent_again() {
        let mut app = stocks_app();
        let body = fighter(&mut app, 1);
        knock_out(&mut app, body);
        assert!(last_spend(&mut app).is_some_and(|s| s.eliminated));

        app.world_mut()
            .resource_mut::<Messages<FighterStockSpent>>()
            .clear();
        knock_out(&mut app, body);
        assert_eq!(
            last_spend(&mut app),
            None,
            "a fighter that was already out spent another stock, so 'somebody \
             was eliminated' fires again on every later KO of the same corpse"
        );
    }

    /// One side is not a match: without the guard the sole side is "wiped out"
    /// the instant it falls and the stage awards a win against nobody.
    #[test]
    fn a_single_side_never_wins() {
        assert_eq!(
            last_side_standing(
                [("blue".to_string(), false), ("blue".to_string(), false)].into_iter()
            ),
            None
        );
    }

    /// A side is out only when EVERY member is out — one partner left standing
    /// keeps it in.
    #[test]
    fn a_side_survives_while_one_member_is_in_play() {
        assert_eq!(
            last_side_standing(
                [
                    ("blue".to_string(), false),
                    ("blue".to_string(), true),
                    ("red".to_string(), false),
                ]
                .into_iter()
            ),
            Some(SidesOutcome::Winner("blue".to_string()))
        );
    }

    /// Three sides, one wiped out: the match CONTINUES. The obvious
    /// `survivors.len() == 1` reading gets this wrong in the other direction.
    #[test]
    fn a_third_side_falling_does_not_end_a_match_two_are_still_fighting() {
        assert_eq!(
            last_side_standing(
                [
                    ("blue".to_string(), true),
                    ("red".to_string(), true),
                    ("green".to_string(), false),
                ]
                .into_iter()
            ),
            None
        );
    }

    #[test]
    fn both_sides_going_out_together_is_a_draw() {
        assert_eq!(
            last_side_standing(
                [("blue".to_string(), false), ("red".to_string(), false)].into_iter()
            ),
            Some(SidesOutcome::Draw)
        );
    }

    /// A body with no stocks at all is not a stocks fighter and is left alone —
    /// an exploration enemy dying in a room with a match running elsewhere.
    #[test]
    fn a_body_without_stocks_is_not_a_fighter() {
        let mut app = stocks_app();
        let body = app.world_mut().spawn(BodyHealth::new(Health::new(10))).id();
        knock_out(&mut app, body);
        assert_eq!(last_spend(&mut app), None);
    }
}
