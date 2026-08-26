//! Ruleset-owned lives/stocks accounting.
//!
//! A stocks fighter may use [`DeathPolicy::Unbounded`], so health cannot signal a
//! knockout. Death paths emit [`BodyKnockedOut`] when a ruleset owns the death.
//! This module spends the count, marks elimination, resets the meter for a
//! respawn, and emits [`FighterStockSpent`]. Stage placement, rounds, and match
//! completion remain ruleset responsibilities.

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageReader, MessageWriter, On, Query, Res, Without,
};

use crate::components::FighterStocks;

/// A body whose death a RULESET owns was knocked out this tick.
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

/// Out of the match, as a fact on the body rather than a number to compare.
///
/// A marker rather than `stocks.remaining == 0` at every call site: elimination
/// is checked by rules, HUDs and the match-end condition, and three readers
/// re-deriving the same comparison is how one of them ends up using `<= 0` on a
/// `u32`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FighterEliminated;

/// A returning fighter's protection, as a FACT ON THE BODY rather than a shape
/// its grant happens to have.
///
/// ⛔⛔ AND THAT IS THE WHOLE REASON IT EXISTS. Respawn protection is an
/// `Empowered` holding `UNTOUCHABLE`, and so is a Sanic super state and so is
/// Mary-O's star — the traits are a CAPABILITY, not a claim about who granted
/// it. A rule that ended "the grant whose traits equal `UNTOUCHABLE`" would be
/// releasing by value equality, which is not ownership: it would strip a
/// power-up somebody else gave the same body, and it would go wrong silently the
/// first time a third granter used the same trait.
///
/// ⇒ the ruleset marks what IT gave, and only removes that.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct RespawnGrace {
    /// Seconds of protection left. Owned HERE rather than borrowed from an
    /// `Empowered`, which is the whole correction: `Empowered` is one component,
    /// so a respawn that granted through it overwrote whatever power-up the body
    /// was already carrying, and ending the beat removed that component and
    /// every semantic in it. The invulnerability itself is
    /// `Invulnerability::RESPAWN`, a reason bit, which retracts alone.
    pub remaining: f32,
}

/// Advance every returning fighter's protection, and publish it as a reason.
///
/// ⭐ ONE AUTHORITY FOR ONE BEAT: the component's clock decides, the reason bit
/// is derived from it every tick, and the component leaving is what retracts the
/// bit. Nothing else writes `Invulnerability::RESPAWN`, so a body that also holds
/// a power-up, a transformation or an authored invuln window keeps every one of
/// them through a respawn and past the end of it.
pub fn tick_respawn_grace(
    mut commands: Commands,
    time: Res<ambition_time::WorldTime>,
    mut bodies: Query<(
        Entity,
        &mut RespawnGrace,
        &mut ambition_characters::actor::BodyHealth,
    )>,
) {
    let dt = time.sim_dt();
    for (entity, mut grace, mut health) in &mut bodies {
        grace.remaining -= dt;
        if grace.remaining <= 0.0 {
            health
                .health
                .invulnerable
                .set(ambition_characters::actor::Invulnerability::RESPAWN, false);
            commands.entity(entity).remove::<RespawnGrace>();
            continue;
        }
        health
            .health
            .invulnerable
            .set(ambition_characters::actor::Invulnerability::RESPAWN, true);
    }
}

/// Retract the reason when the grace is taken away by something OTHER than its
/// clock — a swing spending it, or a body being rebuilt.
///
/// ⛔ a reason cleared only where the component is removed by hand is a latch
/// waiting for the second removal site nobody remembers. This is that clearing,
/// once, keyed on the removal itself.
pub fn retract_respawn_grace_on_removal(
    removed: On<bevy::ecs::lifecycle::Remove, RespawnGrace>,
    mut bodies: Query<&mut ambition_characters::actor::BodyHealth>,
) {
    if let Ok(mut health) = bodies.get_mut(removed.entity) {
        health
            .health
            .invulnerable
            .set(ambition_characters::actor::Invulnerability::RESPAWN, false);
    }
}

/// A stock was spent — the ruleset's cue to place a body or end a match.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct FighterStockSpent {
    pub body: Entity,
    /// Stocks left AFTER this one was spent.
    pub remaining: u32,
    /// `true` when that was the last: this fighter is out.
    pub eliminated: bool,
}

/// The set [`spend_fighter_stocks`] runs in — this tick's stock spend lands.
///
/// ONE member. `decide_stocks_match` is chained after and CONSUMES the spend;
/// including it would make a game's rules wait on the engine's decision, which
/// is the thing those rules are meant to run alongside.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FighterStocksSpent;

/// The set the match-end decision runs in — after this, the outcome for this
/// tick is settled.
///
/// the twin of [`FighterStocksSpent`], and it exists because "run alongside
/// the decision" is safe for most of a ruleset and fatal for one kind of rule.
/// The note above is right that a game's HUD, its respawn placement and its
/// countdown are meant to run beside the engine's answer rather than behind it.
/// But a rule that REMOVES A PARTICIPANT is not running alongside the
/// question — it is destroying the question's input.
///
/// Smash's `take_eliminated_fighters_out_of_play` DESPAWNS an eliminated body, and
/// `decide_stocks_match` reads the sides off the bodies that still exist. Both sat in
/// `CombatSet::Settle` with no ordering between them, and the ruleset's own `.chain()` inserts
/// an `ApplyDeferred`, so the despawn becomes visible part-way through the set. Lose the last
/// loser's row and [`last_side_standing`] sees ONE side — and one side is not a match, so it
/// answers `None`, forever. Whether a match ends therefore depended on how the scheduler
/// happened to break a tie, which differs between the standalone demo and the hosted app:
/// *"several cases"* is what an ambiguity looks like from the couch.
///
/// so a ruleset orders only its participant-removing rules against this.
/// Ordering a whole rules chain behind it would take away the concurrency the
/// note above is protecting.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MatchOutcomeDecided;

/// Spend one stock per knockout, and clear the meter of anyone coming back.
///
/// a fighter already eliminated is skipped, not spent again. Without the
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
            // and it stops being IN the fight, which is the other half of
            // being out of it. The body stays standing until a ruleset removes
            // it, so without this it goes on holding attack state and a place on
            // the anti-clump board — a corpse crowding the fighters who are still
            // playing. See `ActiveCombatant`.
            commands
                .entity(knockout.body)
                .remove::<crate::components::ActiveCombatant>();
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

/// Which side, if any, is the only one left.
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
/// the caller names the sides. An earlier draft resolved a teamless seat to
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

/// HOW A MATCH ENDED — the three answers, as three answers.
///
/// ⭐⭐ `NoContest` IS NOT A WINNER VALUE, and that is the whole reason this
/// type exists. Jon, W8 playtest, asking for an `Exit Match` command: *"It
/// should not award an ordinary winner/loser result... add it at the semantic
/// match-outcome layer rather than encoding it as some special winner value."*
/// The message used to carry `winner: Option<String>` with `None` meaning DRAW,
/// so an abandoned match had nowhere to go but to impersonate one — and a draw
/// is a thing the fighters achieved together, which an abandonment is not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchVerdict {
    /// This side was the last one standing, or ahead when the clock ran out.
    Winner(String),
    /// Every side went out together — a two-fighter simultaneous ring-out
    /// reaches this easily — or the tiebreak found them genuinely level.
    Draw,
    /// The match was stopped from outside the fight. Nobody won and nobody
    /// drew: the question was withdrawn.
    NoContest,
}

impl From<SidesOutcome> for MatchVerdict {
    fn from(outcome: SidesOutcome) -> Self {
        match outcome {
            SidesOutcome::Winner(side) => Self::Winner(side),
            SidesOutcome::Draw => Self::Draw,
        }
    }
}

impl MatchVerdict {
    /// The winning side, if a side won. `None` for a draw AND for a no contest,
    /// which is the collapse this type exists to make the caller opt into.
    pub fn winner(&self) -> Option<&str> {
        match self {
            Self::Winner(side) => Some(side),
            Self::Draw | Self::NoContest => None,
        }
    }
}

/// The match is over, and this is how it ended.
///
/// Written once, by the ruleset-facing half of the loop, when
/// [`last_side_standing`] first answers, the clock expires, or somebody stops
/// the match (`MatchAbandonRequest`, which the RULESET owns because only it
/// knows which match an ask is about).
#[derive(Message, Clone, Debug, PartialEq)]
pub struct StocksMatchDecided {
    pub outcome: MatchVerdict,
}

// It is the latch that stops `decide_stocks_match` announcing twice — and `decide_stocks_match` is
// not in this crate, deliberately: this module owns the COUNT, and the QUESTION needs a seat and a
// live match, which are the ruleset's. The latch is that question's private state and now lives
// beside it, keyed to the match it is about rather than to the process. See
// `ambition_platformer2d_actor_monolith::features::stocks_match::StocksMatchSettled`.

/// WHICH SIDE A SEATED FIGHTER FIGHTS FOR — its declared team, or its own
/// seat when the match declared none and every fighter is a side of one.
///
/// Naming it here makes "the same side" a call rather than a convention.
pub fn side_label(seat: usize, team: Option<&crate::targeting::MatchTeam>) -> String {
    team.map(|team| team.as_str().to_string())
        .unwrap_or_else(|| format!("seat {}", seat + 1))
}

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

    /// A respawning fighter comes back at 0%, or every stock after the first
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
