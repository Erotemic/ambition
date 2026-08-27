//! Ruleset-owned lives/stocks accounting.
//!
//! A stocks fighter may use [`DeathPolicy::Unbounded`], so health cannot signal a
//! knockout. Death paths emit [`BodyKnockedOut`] when a ruleset owns the death.
//! This module spends the count, marks elimination, resets the meter for a
//! respawn, and emits [`FighterStockSpent`]. Stage placement, rounds, and match
//! completion remain ruleset responsibilities.

use bevy::prelude::{
    Commands, Component, Entity, Message, MessageReader, MessageWriter, On, Query, Res, With,
    Without,
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

/// A fighter that lost a stock and has not come back yet.
///
/// ⭐ THE MATCH LIFECYCLE STATE D192 WAS MISSING. A knocked-out fighter used to
/// go straight from "alive" to "standing on the respawn platform" on ONE tick,
/// so there was no state in which it was out of the match but still returning —
/// and every consumer that needed that beat invented its own answer. The KO cue
/// played over a fighter who was already back; the camera was required to frame a
/// live body that had appeared 500 units away with no travel; and the knockout
/// beat kept a previous-frame position cache because by the time presentation
/// read the entity, the body had already moved. ⭐ THAT CACHE IS GONE: a body
/// waiting out its window is not placed, so `spend_fighter_stocks` publishes the
/// beat from the position it can simply read.
///
/// ⛔⛔ ONE BIT, AND IT CARRIES ONLY WHAT NOTHING ELSE DOES: *which* consequence
/// this open window owes. D192 spelled the whole beat here — a countdown, its
/// own authored duration, and a hand-removed `ActiveCombatant` stand-in for
/// "the world's hands are off this body" — and every one of those already
/// existed as [`DeathInterlude`](crate::death_rules::DeathInterlude) /
/// [`OutOfPlay`](crate::death_rules::OutOfPlay) (ADR 0033). The window is now
/// the engine's; this says the window ends in a RESPAWN rather than in a level
/// replay, which is the one thing `DeathInterlude` deliberately does not know.
///
/// ⛔ it is NOT redundant with `DeathInterlude`: a Mary-O death opens one too,
/// and re-placing that body would be a stocks rule reaching into a game that
/// never asked for stocks.
///
/// ⛔⛔ [`spend_fighter_stocks`] OPENS an episode and
/// [`respawn_when_the_interlude_closes`] is the only thing that CLOSES one.
/// Register both or neither: a schedule with just the spend latches every
/// fighter out of play after its first knockout, and the symptom is silent — a
/// body that cannot spend a stock never loses another one.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PendingRespawn;

/// How long a returning fighter waits, authored by the RULESET.
///
/// Default zero, which is the behaviour every existing ruleset already had: the
/// body is placed on the tick the stock is spent. A mode that wants the beat
/// inserts its own. ⛔ this is CONFIG, set once when the mode is built — it is
/// not rollback state, and nothing in the sim writes it.
///
/// ⭐ SECONDS, and the same seconds
/// [`DeathRules::interlude`](crate::death_rules::DeathRules::interlude) counts.
/// D192 authored ticks and argued determinism for it; the engine's own window
/// has counted seconds against `WorldTime` since ADR 0033 and rewinds correctly,
/// so the tick spelling was a deviation defended by a premise its neighbour
/// already disproves. One clock, or the beat is a different length depending on
/// which half of it you ask.
///
/// ⛔ NOT folded into `DeathRules` itself, and the reason is a SCOPE difference
/// rather than a layering excuse: `DeathRules` is declared per ROOM and answers
/// "what does a participant's death cost the level", resolved through the
/// active room's mode — which this crate cannot see. `RespawnInterval` is
/// declared per MATCH RULESET. No room in the shipped composition carries both:
/// the Smash arena declares no death rules at all, so the two knobs have never
/// once had to agree. If a mode ever wants both, that is the moment to make the
/// resolved `DeathRules` reach this seam, not before.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, PartialEq, Default)]
pub struct RespawnInterval {
    /// Seconds the window stays open before the body is placed.
    pub seconds: f32,
}

/// The interval elapsed — the ruleset's cue to PLACE the body.
///
/// ⭐ THE SEAM. The engine owns *when* a fighter comes back; a ruleset owns
/// *where* and *how*. Placement used to read [`FighterStockSpent`] directly,
/// which is why the beat could not exist: the only cue available was the one
/// that fires on the knockout tick.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct FighterRespawnDue {
    pub body: Entity,
}

/// The set [`respawn_when_the_interlude_closes`] runs in — this tick's returns
/// have been decided, and any [`FighterRespawnDue`] for it is written.
///
/// A ruleset orders its PLACEMENT after this. Ordering only against
/// [`FighterStocksSpent`] is not enough: the spend and the return are now
/// different ticks, and a placement racing the tick-down would read an empty
/// message queue on the tick the fighter was actually due.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FighterRespawnsDue;

/// The window closed on a fighter who still has stocks: announce the return.
///
/// ⛔ THE COUNTDOWN IS NOT HERE, deliberately. `tick_death_interlude` advances
/// every open window in the process on the sim clock, so a second ticker for
/// this one would be a second answer to "how long has this body been dead" —
/// and the two would disagree the first time a rewind landed between them. This
/// reads the window and owns only the CONSEQUENCE.
///
/// ⛔ THE ZERO CASE STILL RESOLVES ON THE SPEND'S OWN TICK. A window opened with
/// `remaining == 0.0` is closed the moment it exists, so a ruleset that
/// authored no beat sees exactly the same frame it always did. That is why this
/// is ordered after [`FighterStocksSpent`] rather than being a separate phase.
pub fn respawn_when_the_interlude_closes(
    mut commands: Commands,
    mut pending: Query<
        (
            Entity,
            &crate::death_rules::DeathInterlude,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            Option<&mut ambition_characters::control::ControlHolds>,
        ),
        With<PendingRespawn>,
    >,
    mut due: MessageWriter<FighterRespawnDue>,
) {
    // ⛔⛔ NOT SORTED, AND THE REASON IS THE INTERESTING PART. This carried a
    // comment saying *"the message order here decides placement order for two
    // fighters returning on one tick — which is a seat-dependent position"*, and
    // then sorted to make it true. THE PREMISE WAS FALSE: every
    // `FighterRespawnDue` consumer reads the SEAT off the body it is placing
    // (`place_respawning_fighters` takes `Option<&MatchSeat>` and asks
    // `respawn_placement` for that seat's point), so each returning fighter goes
    // to its own place and the operation is COMMUTATIVE. Nothing consumes the
    // order, here or anywhere.
    //
    // ⭐⭐ AND THE FIRST REPAIR WAS ALSO WRONG, WHICH IS THE LESSON WORTH
    // KEEPING. The sort was originally on `Entity` — allocator identity, a fake
    // canonicalization — and the repair sorted on `SimId` instead. That
    // replaced a fake canonicalization with a REAL one that nothing consumes:
    // anticipatory ordering infrastructure for a dependency that does not
    // exist, carrying its own fallback for bodies with no identity. Both
    // versions answered "what is the canonical key" when the question to ask
    // was "does anything need one".
    //
    // ⇒ a future ruleset that genuinely makes simultaneous returns
    // order-sensitive should name its own canonical key — probably `MatchSeat`,
    // which its placement already reads — rather than inheriting a guess made
    // here on its behalf.
    let ready: Vec<Entity> = pending
        .iter()
        .filter(|(_, window, _, _)| !window.open())
        .map(|(entity, _, _, _)| entity)
        .collect();
    for entity in ready {
        // ⭐ THE WHOLE RETURN, IN ONE PLACE. This says the fighter is back, so
        // it hands back every fact the spend took — including the ones the
        // ENGINE owns. Leaving `OutOfPlay` for `clear_out_of_play_on_restart`
        // to pick up off `BodyRestarted` looks tidier and is wrong: a ruleset
        // that hears the cue and does not place a body (a mode with no beat and
        // no respawn platform, and every rules-only harness) would then hold a
        // fighter out of play forever, and the symptom is the silent one — a
        // body that cannot spend a stock never loses another.
        //
        // ⛔ "back in the fight" and "the world's hands are off this body" are
        // contradictory states; whichever system asserts one must retract the
        // other in the same breath.
        let (_, _, _, holds) = pending.get_mut(entity).expect("just collected");
        let holds = holds.map(|holds| holds.into_inner());
        commands.entity(entity).remove::<(
            PendingRespawn,
            crate::death_rules::OutOfPlay,
            crate::death_rules::DeathInterlude,
        )>();
        // RELEASE the bit this beat claimed, never `clear_control_holds`: a
        // fighter can be knocked out while a conversation or an opening
        // ceremony also holds it, and taking the whole set would hand control
        // back to a body somebody else is still driving.
        ambition_characters::control::release_control_hold(
            &mut commands,
            entity,
            holds,
            ambition_characters::control::ControlHold::Sequence,
        );
        // Back IN the fight, which is the half that makes the body targetable
        // and gives it a place on the anti-clump board again.
        commands
            .entity(entity)
            .try_insert(crate::components::ActiveCombatant);
        due.write(FighterRespawnDue { body: entity });
    }
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
    // ⛔⛔ AND `Without<OutOfPlay>`. A waiting body is NOT PLACED — it is still
    // lying wherever it died, which for a ring-out is inside the blast zone.
    // Same-tick placement hid this by moving the body out on the tick it died;
    // with a beat, the zone would knock it out again every tick and spend every
    // remaining stock during the wait.
    //
    // ⭐ `OutOfPlay` and not `PendingRespawn`, though both are on the body: the
    // rule is "a body the world has its hands off cannot lose a stock", and
    // that is true of every open death window, not only the ones that end in a
    // respawn. Filtering on the narrower marker would have left the same hole
    // open for any other game whose death path reaches this spend.
    mut fighters: Query<
        &mut FighterStocks,
        (
            Without<FighterEliminated>,
            Without<crate::death_rules::OutOfPlay>,
        ),
    >,
    mut meters: Query<&mut ambition_characters::actor::BodyHealth>,
    // ⛔⛔ THE SWING THE STOCK COST, TORN DOWN WHERE THE STOCK IS SPENT. A move
    // clock is NOT the movement kernel: `advance_move_playback` reads neither
    // `OutOfPlay` nor `ActiveCombatant`, so a fighter KO'd mid-swing went on
    // advancing its move for the whole death beat — opening active hit volumes,
    // firing authored events, throwing projectiles and playing sfx while dead.
    //
    // ⭐ BY VALUE, not as a component to strip: cancelling a move means
    // despawning the strike boxes it derived, and only the playback knows which
    // entities those are. `cancel_move_playback` is the canonical teardown.
    //
    // ⛔ SMASH ALREADY DID THIS — inside `place_respawning_fighters`, which runs
    // when the fighter comes BACK. Its comment was right that the move did not
    // survive the stock it cost; it was one lifecycle boundary too late, so the
    // swing stayed live for the entire interlude and only got cleaned up a
    // second later. Owned here, by the seam that opens the episode.
    mut swings: Query<&mut crate::moveset::MovePlayback>,
    // ⭐ WHERE THE BODY IS, READ WHERE THE STOCK IS SPENT. The knockout beat is
    // drawn at the place the fighter left play, and until D201 that position was
    // destroyed on the same tick — the respawn teleported the body — so
    // presentation kept a previous-frame cache in a non-rollback `Local` to
    // recover it. A body waiting out its death beat is no longer placed until
    // the window closes, so the position is simply here, and the cache went with
    // the problem it existed for. An ELIMINATED body still gets despawned by its
    // ruleset, which is why the beat is published HERE and not later.
    positions: Query<&ambition_platformer2d_core::BodyKinematics>,
    mut beat: MessageWriter<ambition_vfx::vfx::KnockoutBeatRequested>,
    interval: Option<Res<RespawnInterval>>,
) {
    // Absent resource == zero == the same-tick placement every ruleset had
    // before D192. A missing knob must not change anybody's behaviour.
    let interval = interval.map(|i| *i).unwrap_or_default();
    // Message order is write order, which is deterministic; nothing here sorts
    // or iterates a query, so there is no hash-order hazard to guard against.
    for knockout in knockouts.read() {
        let Ok(mut stocks) = fighters.get_mut(knockout.body) else {
            continue;
        };
        let eliminated = stocks.spend();
        let remaining = stocks.remaining;
        // ⭐ THE SWING IS OVER, AND FOR AN ELIMINATED FIGHTER THIS IS THE ONLY
        // PLACE THAT CAN SAY SO. A body that enters `OutOfPlay` has its move
        // ended by `end_moves_for_bodies_out_of_play`, which is the generic
        // invariant — but elimination does NOT open a death window, so an
        // eliminated fighter never becomes out-of-play and would keep swinging
        // until its ruleset despawned it.
        if eliminated {
            if let Ok(mut playback) = swings.get_mut(knockout.body) {
                crate::moveset::cancel_move_playback(&mut commands, knockout.body, &mut playback);
            }
        }
        // The beat, at the place it happened. A presentation INTENT, so it rides
        // the confirmed-effect quarantine with the sfx and the shake beside it
        // rather than being sampled off a read-model on a different clock.
        if let Ok(kin) = positions.get(knockout.body) {
            beat.write(ambition_vfx::vfx::KnockoutBeatRequested {
                pos: kin.pos,
                eliminated,
                speed: kin.vel.length(),
            });
        }
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
        } else {
            if let Ok(mut health) = meters.get_mut(knockout.body) {
                // A fighter coming back comes back FRESH. The meter is the reason
                // it was knocked off the stage; carrying it into the next stock
                // would make the second one shorter than the first and the third
                // shorter again, which is a difficulty ramp nobody authored.
                health.reset();
            }
            // The body is OUT until its window closes. FOUR facts, and the
            // engine already owned three of them (ADR 0033):
            //
            // - `OutOfPlay` — the world's hands are off. `step_body` halts the
            //   body and steps it with `dt == 0`, `damage_apply` drops it from
            //   the victim query, and the room's exit sweep skips it. D192
            //   approximated this by removing `ActiveCombatant`, which stops a
            //   body being TARGETED but leaves it falling through the blast
            //   zone that killed it.
            // - `DeathInterlude` — the countdown, ticked by the engine's own
            //   `tick_death_interlude` on the sim clock, and rollback-registered
            //   since long before this beat existed. It also arms the death row
            //   (`BodyAnimFacts::death_anim_timer`), so a KO finally LOOKS like
            //   one without this ruleset knowing the animation exists.
            // - `ControlHold::Sequence` — ⭐ THE PIECE D192 SKIPPED, and its
            //   absence was a shipped bug: *"in smash when you are respawning,
            //   if I make the character jump they raise up on the platform"*.
            //   A dead body does not answer input, and the engine has said so
            //   in one word since Mary-O's death beat stopped reinventing it.
            //   Claimed as a bit rather than stamped, so the release that
            //   follows is arithmetic — see `open_death_interlude`.
            // - `ActiveCombatant` off, which elimination does too, for the same
            //   reason: a body that is not in the fight must not go on holding
            //   attack state and a place on the anti-clump board.
            //
            // `PendingRespawn` carries the fifth and only new one: this window
            // ends in a RESPAWN.
            commands.entity(knockout.body).try_insert((
                PendingRespawn,
                crate::death_rules::OutOfPlay,
                crate::death_rules::DeathInterlude {
                    remaining: interval.seconds,
                    // ⛔ the LEVEL's consequence, which for a stocks match is
                    // none. `close_death_interlude` spends this once the window
                    // shuts and then asks the room's `LevelReset`; a versus
                    // arena declares no death rules, so it answers `Never`. The
                    // respawn is NOT hung off this debt — two consumers of one
                    // flag is a race decided by whichever system the scheduler
                    // happens to order first.
                    consequence_pending: true,
                },
            ));
            commands
                .entity(knockout.body)
                .remove::<crate::components::ActiveCombatant>();
            ambition_characters::control::claim_control_hold(
                &mut commands,
                knockout.body,
                ambition_characters::control::ControlHold::Sequence,
            );
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

    /// ⛔⛔ ALL THREE SYSTEMS, ALWAYS. `spend_fighter_stocks` opens a window,
    /// `tick_death_interlude` is the only thing that advances one and
    /// `respawn_when_the_interlude_closes` is the only thing that spends one, so
    /// a harness wiring just the first latches every fighter out of play after
    /// its first knockout — silently, because a body that cannot spend a stock
    /// simply never loses another. This wired only the spend, and the "spends
    /// until eliminated" test is what caught it.
    fn stocks_app() -> App {
        respawn_app(0.0)
    }

    /// One sim step, so a beat authored in seconds has a tick length here.
    const TEST_DT: f32 = 1.0 / 60.0;

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

    /// A stocks app that also runs the return beat.
    ///
    /// ⭐ THE REAL `tick_death_interlude`, not a stand-in. The countdown is the
    /// part D201 handed back to the engine, so a harness that ticked the window
    /// itself would be testing its own arithmetic and would go on passing if the
    /// engine's ticker were removed entirely.
    /// ⛔ THE HARNESS RUNS THE INVARIANT TOO. Production ends an out-of-play
    /// body's move in `end_moves_for_bodies_out_of_play`, not in the spend, so a
    /// fixture wiring only the spend measures a rule that no longer lives there.
    fn respawn_app(interval_seconds: f32) -> App {
        let mut app = App::new();
        app.add_message::<BodyKnockedOut>();
        app.add_message::<FighterStockSpent>();
        app.add_message::<FighterRespawnDue>();
        // The knockout beat the spend publishes. A presentation intent, but the
        // channel has to exist or the spend cannot run at all.
        app.add_message::<ambition_vfx::vfx::KnockoutBeatRequested>();
        app.insert_resource(RespawnInterval {
            seconds: interval_seconds,
        });
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: TEST_DT,
            scaled_dt: TEST_DT,
        });
        app.add_systems(
            Update,
            (
                spend_fighter_stocks,
                crate::death_rules::end_moves_for_bodies_out_of_play,
                crate::death_rules::tick_death_interlude,
                respawn_when_the_interlude_closes,
            )
                .chain(),
        );
        app
    }

    /// ⛔ DRAIN, never a fresh cursor. Bevy keeps a message readable for a
    /// second frame, so a helper that made a new cursor each call counted one
    /// return twice and made "exactly once" unprovable.
    fn returned_this_tick(app: &mut App) -> Vec<Entity> {
        app.world_mut()
            .resource_mut::<Messages<FighterRespawnDue>>()
            .drain()
            .map(|due| due.body)
            .collect()
    }

    /// Spawn a fighter and let the harness settle.
    ///
    /// ⛔⛔ THE SETTLE IS LOAD-BEARING. A bare `App`'s FIRST `update()` runs the
    /// `Update` schedule TWICE, which silently spent two ticks of the interval
    /// and made every count here off by one. Measured, not assumed.
    fn settled_fighter(app: &mut App, stocks: u32) -> Entity {
        let body = combat_fighter(app, stocks);
        app.update();
        let _ = returned_this_tick(app);
        body
    }

    fn combat_fighter(app: &mut App, stocks: u32) -> Entity {
        app.world_mut()
            .spawn((
                FighterStocks::new(stocks),
                BodyHealth::new(Health::new(50)).with_policy(DeathPolicy::Unbounded),
                crate::components::ActiveCombatant,
            ))
            .id()
    }

    /// ⛔⛔ THE BEAT IS PUBLISHED WHERE THE BODY LEFT PLAY, AND IT NEEDS NO
    /// CACHE TO KNOW WHERE THAT WAS.
    ///
    /// Presentation used to read a rebuilt `KnockoutsView`, which kept a
    /// previous-frame `LastSeenBodies` record in a non-rollback `Local` — because
    /// under D192 the respawn teleported the body onto the platform on the same
    /// tick the stock was spent, so by the time anything looked, the position was
    /// already gone. A `Local` a rewind does not restore then answered "where did
    /// it leave play" from the abandoned branch.
    ///
    /// ⭐ D201 RETIRED THE PROBLEM RATHER THAN THE SYMPTOM: a body waiting out
    /// its death window is not placed until the window closes, so the position is
    /// simply readable here. This arm is what says so — the beat's position is
    /// the body's, not the respawn point's.
    ///
    /// ⭐ AND EXACTLY ONE beat per knockout, because a duplicate is what a
    /// read-model sampled on the wrong clock produces.
    #[test]
    fn the_knockout_beat_names_the_place_the_body_left_play() {
        let mut app = respawn_app(0.5);
        let body = combat_fighter(&mut app, 3);
        let died_at = ambition_platformer2d_core::Vec2::new(-420.0, 310.0);
        app.world_mut()
            .entity_mut(body)
            .insert(ambition_platformer2d_core::BodyKinematics {
                pos: died_at,
                vel: ambition_platformer2d_core::Vec2::new(0.0, -1261.0),
                size: ambition_platformer2d_core::Vec2::new(16.0, 32.0),
                facing: 1.0,
            });
        app.update();
        let _ = returned_this_tick(&mut app);
        knock_out(&mut app, body);

        let beats: Vec<ambition_vfx::vfx::KnockoutBeatRequested> = {
            let messages = app
                .world()
                .resource::<Messages<ambition_vfx::vfx::KnockoutBeatRequested>>();
            let mut cursor = messages.get_cursor();
            cursor.read(messages).copied().collect()
        };
        assert_eq!(
            beats.len(),
            1,
            "one knockout owes exactly one beat, and got {}",
            beats.len()
        );
        assert_eq!(
            beats[0].pos, died_at,
            "the beat was published somewhere other than where the body was when \
             it lost the stock"
        );
        assert!(
            !beats[0].eliminated,
            "a fighter with stocks left was reported as eliminated"
        );
        assert!(
            (beats[0].speed - 1261.0).abs() < 1e-3,
            "the beat did not carry the flight that ended: {}",
            beats[0].speed
        );
    }

    /// A five-second swing: long enough that nothing ends it but the rule under
    /// test.
    fn swing_spec() -> ambition_entity_catalog::MoveSpec {
        ambition_entity_catalog::MoveSpec {
                display_name: None,
                id: "swing".to_string(),
                clip: ambition_entity_catalog::ClipBinding {
                    clip: "swing".to_string(),
                    fallbacks: vec![],
                },
                duration_s: 5.0,
                windows: vec![],
                events: vec![],
                gates: Default::default(),
                start_impulse: None,
                smash_charge_mult: 1.0,
                smash_charge: None,
                charge_gesture: ambition_entity_catalog::ChargeGesture::Smash,
                repeat: None,
                landing_lag_s: None,
                autocancel_after_s: None,
                sprite_spin_hz: None,
                equips: None,
            }
    }

    /// ⛔⛔ THE SWING DOES NOT SURVIVE THE STOCK IT COST.
    ///
    /// A move clock is not the movement kernel. D201 froze the body — `OutOfPlay`
    /// halts `step_body`, the control hold silences input — but
    /// `advance_move_playback` reads neither `OutOfPlay` nor `ActiveCombatant`,
    /// so a fighter KO'd mid-swing went on advancing its move for the whole
    /// death beat: opening active hit volumes, firing authored events, throwing
    /// projectiles and playing sfx while dead.
    ///
    /// ⭐ SMASH ALREADY CANCELLED IT — inside `place_respawning_fighters`, which
    /// runs when the fighter comes BACK. That comment was right that the move
    /// did not survive the stock; it was one lifecycle boundary too late. Owned
    /// here now, by the seam that opens the episode.
    ///
    /// ⭐ THE SURVIVING-STOCK ARM IS THE PREMISE GUARD: a spend that cancelled
    /// nothing, or one that cancelled by eliminating the fighter outright, would
    /// satisfy the first assertion for the wrong reason.
    #[test]
    fn a_knocked_out_fighter_stops_swinging_the_move_that_cost_it_the_stock() {
        use crate::moveset::MovePlayback;

        fn swing_after_knockout(stocks: u32) -> bool {
            let mut app = respawn_app(0.5);
            let body = combat_fighter(&mut app, stocks);
            app.world_mut()
                .entity_mut(body)
                .insert(MovePlayback::new(swing_spec(), 1.0));
            app.update();
            let _ = returned_this_tick(&mut app);
            assert!(
                app.world().get::<MovePlayback>(body).is_some(),
                "the fixture lost its swing before the knockout, so nothing below \
                 measures the spend"
            );
            knock_out(&mut app, body);
            // ⭐ ONE MORE TICK, because the guarantee is per COMBAT PHASE and not
            // per instruction. Production runs
            // `end_moves_for_bodies_out_of_play` first in `CombatSet::Trigger`,
            // so a body put out of play by a spend has no move before anything
            // triggers or advances one — which is the tick after the spend, not
            // the instruction after it. An ELIMINATED fighter never becomes
            // out-of-play at all, so the spend itself still ends its move and
            // this extra tick changes nothing for that arm.
            app.update();
            app.world().get::<MovePlayback>(body).is_some()
        }

        assert!(
            !swing_after_knockout(3),
            "a fighter that lost a stock kept swinging: its move goes on opening \
             hit volumes and firing authored events for the whole death beat"
        );
        assert!(
            !swing_after_knockout(1),
            "an ELIMINATED fighter kept swinging — and elimination opens no death \
             window, so the generic out-of-play invariant never sees it and this \
             seam is the only thing that can end the move"
        );
    }

    /// ⛔⛔ AND IT IS AN INVARIANT, NOT A RULE EACH DEATH ROAD REMEMBERS.
    ///
    /// The first version of this lived inside `spend_fighter_stocks`, which
    /// fixed Smash and left the other real customer broken:
    /// `session::death::open_death_interlude` — how Mary-O and every non-stock
    /// ruleset die — inserts `OutOfPlay` and cancelled nothing. A body could die
    /// mid-swing and go on opening hit windows and firing authored events.
    ///
    /// ⭐ SO THIS ARM DOES NOT SPEND A STOCK AT ALL. It puts `OutOfPlay` on a
    /// body directly, which is what every death road has in common, and asks
    /// whether the move survives it.
    #[test]
    fn any_body_that_leaves_play_loses_its_move_however_it_left() {
        use crate::moveset::MovePlayback;

        let mut app = App::new();
        app.add_systems(Update, crate::death_rules::end_moves_for_bodies_out_of_play);
        let body = app
            .world_mut()
            .spawn(MovePlayback::new(swing_spec(), 1.0))
            .id();
        app.update();
        assert!(
            app.world().get::<MovePlayback>(body).is_some(),
            "a body still IN play lost its move, so the arm below would pass for \
             the wrong reason"
        );

        app.world_mut()
            .entity_mut(body)
            .insert(crate::death_rules::OutOfPlay);
        app.update();
        assert!(
            app.world().get::<MovePlayback>(body).is_none(),
            "a body that left play kept swinging. Whatever road put it out — a \
             stock, a pit, a hazard, a script — the world has its hands off it \
             and the move clock does not care"
        );
    }

    /// ⛔⛔ TWO FIGHTERS DUE ON ONE TICK BOTH COME BACK — AND THE ORDER IS NOT
    /// ASSERTED, DELIBERATELY.
    ///
    /// This arm used to demand a canonical ORDER, and that was a claim about a
    /// consumer that does not exist. Every `FighterRespawnDue` consumer reads the
    /// SEAT off the body it is placing, so two returning fighters go to two
    /// different authored points and the operation is commutative. Asserting an
    /// order here would pin infrastructure nothing needs, and — worse — would go
    /// on passing after somebody removed the sort, because with two bodies the
    /// query happens to come back in allocation order anyway. It did exactly
    /// that, which is how a test starts measuring luck.
    ///
    /// ⭐ SO IT MEASURES THE SET. Both fighters return, both on the SAME tick,
    /// and that answer does not move when the same two semantic fighters are
    /// SPAWNED in the opposite order — which is the property that would actually
    /// break if returns became allocation-sensitive.
    #[test]
    fn two_fighters_due_on_one_tick_both_return_whatever_order_they_were_spawned_in() {
        use ambition_platformer2d_shared_tangle::sim_id::SimId;

        /// Spawn `alpha` and `beta` in the given allocation order, knock both
        /// out on one tick, and report the SimIds they came back in.
        fn returned_order(alpha_first: bool) -> Vec<String> {
            let mut app = respawn_app(0.05);
            let mut spawn = |name: &str| -> Entity {
                let body = app
                    .world_mut()
                    .spawn((
                        FighterStocks::new(3),
                        BodyHealth::new(Health::new(50)).with_policy(DeathPolicy::Unbounded),
                        crate::components::ActiveCombatant,
                        SimId::placement(name),
                    ))
                    .id();
                body
            };
            let (alpha, beta) = if alpha_first {
                let a = spawn("alpha");
                let b = spawn("beta");
                (a, b)
            } else {
                let b = spawn("beta");
                let a = spawn("alpha");
                (a, b)
            };
            app.update();
            let _ = returned_this_tick(&mut app);

            for body in [alpha, beta] {
                app.world_mut()
                    .resource_mut::<Messages<BodyKnockedOut>>()
                    .write(BodyKnockedOut {
                        body,
                        cause: crate::HitSource::LeftTheWorld,
                    });
            }
            // The spend tick, then the interval, then the tick they are due.
            let mut due = Vec::new();
            for _ in 0..8 {
                app.update();
                due = returned_this_tick(&mut app);
                if !due.is_empty() {
                    break;
                }
            }
            due.into_iter()
                .map(|body| {
                    app.world()
                        .get::<SimId>(body)
                        .expect("every fighter here is identified")
                        .as_str()
                        .to_string()
                })
                .collect()
        }

        let mut forward = returned_order(true);
        let mut reversed = returned_order(false);
        // The premise: both fighters really do come back, on ONE tick. Without
        // this the equality below is satisfied by two empty lists.
        assert_eq!(
            forward.len(),
            2,
            "the two fighters did not both return on the same tick, so nothing \
             below is measuring what it says"
        );
        forward.sort();
        reversed.sort();
        assert_eq!(
            forward, reversed,
            "the same two fighters came back as a different SET when they were \
             spawned in the opposite order — returning is supposed to depend on \
             the window closing and nothing else"
        );
    }

    #[test]
    fn a_knocked_out_fighter_is_not_due_back_on_the_tick_it_lost_the_stock() {
        // ⛔⛔ D192 ITSELF. The body used to be placed inside the same tick the
        // stock was spent, so the KO cue played over a fighter who was already
        // standing on the platform and the camera had to frame a body that
        // teleported ~500 units with no travel.
        let mut app = respawn_app(1.0);
        let body = settled_fighter(&mut app, 3);
        knock_out(&mut app, body);
        app.update();

        assert!(
            returned_this_tick(&mut app).is_empty(),
            "the fighter must NOT be due back on the knockout tick"
        );
        // ⛔ the COUNT is deliberately not asserted here. This harness is a bare
        // `App`, whose schedule is not the game's — measured, its `Update` runs
        // twice on the first pass — so an exact remaining-ticks number would pin
        // the harness rather than the rule. The wall-clock length of the beat is
        // proven where the real schedule runs.
        assert!(
            app.world().get::<PendingRespawn>(body).is_some(),
            "it is waiting to come back"
        );
    }

    #[test]
    fn a_waiting_fighter_is_out_of_the_fight_and_is_counted_back_in_when_it_returns() {
        // The half that actually keeps the body from being targeted, captured or
        // crowding the anti-clump board while it waits. A state that said "a
        // return is coming" without this would leave a fighter fightable in a
        // place it is not standing.
        let mut app = respawn_app(0.05);
        let body = settled_fighter(&mut app, 3);
        knock_out(&mut app, body);
        app.update();
        assert!(
            app.world()
                .get::<crate::components::ActiveCombatant>(body)
                .is_none(),
            "a fighter awaiting respawn is NOT in the fight"
        );

        for _ in 0..3 {
            app.update();
        }
        assert!(
            app.world()
                .get::<crate::components::ActiveCombatant>(body)
                .is_some(),
            "and it is back in the fight on the tick it returns"
        );
        assert!(
            app.world().get::<PendingRespawn>(body).is_none(),
            "the episode is over, not left latched"
        );
    }

    #[test]
    fn a_fighter_waiting_to_come_back_does_not_answer_input() {
        // ⭐⭐ JON'S BUG, and the piece D192 skipped. *"in smash when you are
        // respawning, if I make the character jump they raise up on the
        // platform."* A body with no control hold is a body the input road
        // still drives, and it was lying in the blast zone with a jump left.
        //
        // The hold is checked as a CLAIM and not as the derived marker: the
        // marker means "`ControlHolds` is non-empty", so asserting it alone
        // would pass for a beat that claimed somebody else's bit.
        use ambition_characters::control::{ControlHold, ControlHolds, ScriptedControl};
        let mut app = respawn_app(1.0);
        let body = settled_fighter(&mut app, 3);
        knock_out(&mut app, body);
        app.update();

        assert!(
            app.world()
                .get::<ControlHolds>(body)
                .is_some_and(|holds| holds.holds(ControlHold::Sequence)),
            "a fighter awaiting respawn must be holding `Sequence` — normal input \
             does not reach a body that is waiting to come back"
        );
        assert!(
            app.world().get::<ScriptedControl>(body).is_some(),
            "and the derived marker agrees with the claim"
        );
        assert!(
            app.world()
                .get::<crate::death_rules::OutOfPlay>(body)
                .is_some(),
            "the world's hands are off it too (ADR 0033), so nothing steps or \
             damages it where it fell"
        );

        // ⛔ AND IT GETS CONTROL BACK. A hold that is claimed and never released
        // is the same bug pointed the other way, and it is the quieter one: a
        // fighter that returns and cannot be driven looks like a dead pad.
        for _ in 0..200 {
            app.update();
            if !returned_this_tick(&mut app).is_empty() {
                break;
            }
        }
        assert!(
            app.world()
                .get::<ControlHolds>(body)
                .is_none_or(|holds| !holds.holds(ControlHold::Sequence)),
            "the returning fighter is still holding `Sequence`"
        );
        assert!(
            app.world()
                .get::<crate::death_rules::OutOfPlay>(body)
                .is_none(),
            "and it is back in play"
        );
    }

    #[test]
    fn a_longer_authored_interval_waits_strictly_longer() {
        // ⭐ A COMPARATIVE, because it is the part that survives the harness.
        // The absolute tick count here would be measuring a bare `App`'s
        // schedule; that one interval outlasts a shorter one is a fact about the
        // rule, and it is what a knob being wired at all actually means. Two
        // arms that STRADDLE, so a countdown wired to a constant fails it.
        fn ticks_until_due(interval: f32) -> usize {
            let mut app = respawn_app(interval);
            let body = settled_fighter(&mut app, 3);
            knock_out(&mut app, body);
            for tick in 0..500 {
                app.update();
                if !returned_this_tick(&mut app).is_empty() {
                    return tick;
                }
            }
            panic!("a fighter with a {interval}s interval never came back");
        }

        let short = ticks_until_due(0.05);
        let long = ticks_until_due(0.7);
        assert!(
            long > short,
            "a 0.7s beat must outlast a 0.05s one (got {long} vs {short} ticks)"
        );
        assert!(
            short > 0,
            "and even a short beat is not the knockout tick itself"
        );
    }

    #[test]
    fn a_zero_interval_returns_on_the_knockout_tick() {
        // ⛔ THE COMPATIBILITY ARM, and it straddles the interesting boundary
        // with the test above. Every ruleset that never asked for a beat must
        // behave exactly as it did before D192 — placed on the spend tick.
        let mut app = respawn_app(0.0);
        let body = settled_fighter(&mut app, 3);
        knock_out(&mut app, body);
        app.update();

        assert_eq!(
            returned_this_tick(&mut app),
            vec![body],
            "with no authored beat the fighter is due back immediately"
        );
        assert!(app.world().get::<PendingRespawn>(body).is_none());
    }

    #[test]
    fn a_body_waiting_to_respawn_cannot_spend_another_stock() {
        // ⛔⛔ THE BEAT'S OWN HAZARD. The body is not placed while it waits, so a
        // ring-out leaves it lying in the blast zone that killed it. Without the
        // filter that zone knocks it out again every tick and the fighter loses
        // every stock it has during one respawn.
        let mut app = respawn_app(1.0);
        let body = settled_fighter(&mut app, 3);
        knock_out(&mut app, body);
        app.update();
        assert_eq!(app.world().get::<FighterStocks>(body).unwrap().remaining, 2);

        for _ in 0..10 {
            knock_out(&mut app, body);
            app.update();
        }
        assert_eq!(
            app.world().get::<FighterStocks>(body).unwrap().remaining,
            2,
            "a waiting fighter spends NO further stocks"
        );
    }

    #[test]
    fn an_eliminated_fighter_never_waits_to_come_back() {
        // It has no stock to return on. A pending episode here would be a body
        // scheduled to be placed for a knockout that ended its match.
        let mut app = respawn_app(1.0);
        let body = settled_fighter(&mut app, 1);
        knock_out(&mut app, body);
        app.update();

        assert!(app.world().get::<FighterEliminated>(body).is_some());
        assert!(
            app.world().get::<PendingRespawn>(body).is_none(),
            "an eliminated fighter is not coming back"
        );
        for _ in 0..70 {
            app.update();
            assert!(
                returned_this_tick(&mut app).is_empty(),
                "and it is never announced as due"
            );
        }
    }

    #[test]
    fn a_fighter_is_announced_due_exactly_once() {
        // A latch that re-fired would place the body every tick after the
        // interval, which reads as a fighter frozen on the respawn platform.
        let mut app = respawn_app(0.05);
        let body = settled_fighter(&mut app, 3);
        knock_out(&mut app, body);

        let mut announcements = 0;
        for _ in 0..12 {
            app.update();
            announcements += returned_this_tick(&mut app).len();
        }
        assert_eq!(announcements, 1, "one knockout, one return");
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
