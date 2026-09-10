//! Prove two CPUs in the shipped Smash composition actually fight each other.
//!
//! Each fighter must accumulate substantial damage and enter hitstun, while both
//! seats remain present. Damage is expressed as a ratio (`1.0 == 100%`); hitstun
//! distinguishes opponent hits from environmental/self damage.

use ambition_demo_smash::select::SmashRoster;
use ambition_demo_smash::SMASH_DUELIST_BRAIN;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

/// An ordinary fighter a player can pick — asserted to be on the assembled grid
/// below, because a name this composition cannot seat proves nothing.
const FIGHTER_DEFAULT: &str = "npc_pirate_admiral";

/// The fighter this run duels, overridable by `AMBITION_DUEL_FIGHTER`.
///
/// ⭐⭐ **BECAUSE SWEEPING THE ROSTER SHOULD NOT COST NINETEEN REBUILDS.** The
/// const is the shipped subject and stays the default, so the gate is unchanged
/// and nobody has to pass anything. But answering *"is the gate's population one
/// fighter, or is the roster broadly inert?"* means running this cell once per
/// fighter, and editing a `const` in a test file recompiles `app_it` every time
/// — MEASURED at 3m55s wall for 27s of test. ⇒ The subject becomes an input, and
/// the sweep costs one build.
///
/// ⚠ IT IS STILL VALIDATED. An id passed here goes through the same
/// assembled-grid assertion below as the default, so a typo names itself instead
/// of measuring whatever the seating fell back to.
fn fighter() -> String {
    std::env::var("AMBITION_DUEL_FIGHTER").unwrap_or_else(|_| FIGHTER_DEFAULT.to_string())
}

/// The top authored rung. If any rung fights, this one does.
const RUNG_DEFAULT: u8 = 9;

/// The rung this run fights at, overridable by `AMBITION_DUEL_RUNG`.
///
/// ⛔ The default is the one that matters and the override exists to leave it:
/// rung 9 is the ONLY rung where `execution_noise * interval()` rounds to zero
/// (`0.4999999701976776` in f32), so the per-seat cognition seed is drawn and
/// discarded and a symmetric mirror bout is deterministically bit-identical.
/// Sweeping the lower rungs is how that claim is checked against the composed
/// app rather than against the arithmetic.
///
/// ⚠ An unparseable value is a PANIC, not a fallback to 9: silently fighting
/// at the default while a caller believes it asked for rung 3 is a whole sweep
/// of rows that all agree for the wrong reason.
///
/// ⛔⛔ AND THE VALID SET IS THE PUBLISHED POLICIES, NOT THE LADDER'S RANGE —
/// TWO VOCABULARIES, AND THE WIDER ONE IS THE WRONG ONE TO VALIDATE AGAINST.
/// `FighterBrainProfile::for_level` clamps to 1..=9, so every rung in that
/// range is a real ladder rung. But `smash_roster_at_levels` names a seat's
/// brain `"{SMASH_DUELIST_BRAIN}_l{level}"`, and the smash experience publishes
/// only FIVE of them. Measured 2026-09-10: a sweep asked for rung 8, every seat
/// was refused, and all three fighters came back `0 of 3600 ticks (decided on
/// None)` after a full 3600-tick run each — a silent, expensive nothing that
/// read as "the instrument could not measure it" rather than "you named a
/// policy that does not exist".
const PUBLISHED_RUNGS: [u8; 5] = [1, 3, 5, 6, 9];

fn rung() -> u8 {
    let Ok(raw) = std::env::var("AMBITION_DUEL_RUNG") else {
        return RUNG_DEFAULT;
    };
    raw.trim()
        .parse::<u8>()
        .ok()
        .filter(|r| PUBLISHED_RUNGS.contains(r))
        .unwrap_or_else(|| {
            panic!(
                "AMBITION_DUEL_RUNG={raw:?} names no published duelist policy. \
                 The smash experience publishes `{SMASH_DUELIST_BRAIN}_l<n>` for \
                 n in {PUBLISHED_RUNGS:?} only — the ladder has nine rungs and \
                 the roster can seat five of them. Asking for one of the other \
                 four seats nobody and spends a full duel finding out."
            )
        })
}

/// One minute at 60Hz — the same budget `ladder_rig` uses, so the two are
/// readable against each other.
const TICKS: usize = 3_600;

/// Half a pool PER MINUTE OF DUEL. deliberately far below the 1.69
/// measured: this guards *"a fight happened"*, not the tuning, and a test that
/// pinned the measured value would go red on every balance change.
///
/// ⭐ PER MINUTE OF DUEL, not per minute of wall clock, and the distinction only
/// appeared once the duel started FINISHING. A decided match stops accumulating
/// damage the moment a seat leaves the cast, so a fight good enough to end in
/// twenty seconds read as a third of the damage of one that never resolved —
/// and the winner, who by definition takes less, read lower still. Measured
/// 2026-08-23 after the capture fix: the match is decided on tick 1232 with four
/// stocks spent, and the two seats take 0.34 and 0.19 — which is 0.99 and 0.56
/// at this rate. The THRESHOLD did not move; what moved is the window it is
/// divided by.
const A_REAL_FIGHT: f32 = 0.5;

/// The shortest duel a rate may be read off.
///
/// ⛔ the rate is a division, and a short enough denominator makes any numerator
/// look like a fight. Ten seconds of two fighters on the stage is the floor
/// below which this test says nothing rather than something flattering.
const A_MEASURABLE_DUEL: usize = 600;

#[test]
fn two_cpus_in_the_shipped_composition_damage_each_other() {
    let fighter = fighter();
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // the frame is load-bearing: `PreparedCharacterRegistry` is filled by a
    // `Startup` system, so a build that has never updated has a catalog and no
    // registry.
    app.update();

    {
        let registry = app
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .expect("the composed host has a prepared-character registry");
        let grid = SmashRoster::assemble(registry);
        let ids: Vec<&str> = grid.ids().collect();
        assert!(
            ids.contains(&fighter.as_str()),
            "`{fighter}` is not on the assembled smash grid in this composition, so \
             seating it proves nothing about what a player can pick. Grid: {ids:?}"
        );
    }

    let roster = ambition_demo_smash::smash_roster_at_levels(
        [fighter.as_str(), fighter.as_str()],
        &[rung(), rung()],
    );
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut taken = [0.0f32; 2];
    let mut last = [0.0f32; 2];
    let mut hitstun_ticks = [0usize; 2];
    let mut both_seated_ticks = 0usize;
    let mut running_ticks = [0usize; 2];
    let mut move_counts: [std::collections::BTreeMap<String, usize>; 2] = Default::default();
    let mut last_instance: [Option<(String, u32)>; 2] = [None, None];
    // ⭐ DAMAGE BY MOVE, joined on the occurrence the verdict now carries.
    // `ResolvedBodyHit::attacker_move_instance` is `Some(n)` exactly when a move
    // claimed the strike, so `(attacker seat, n) -> move id` attributes the
    // RESOLVED amount to the use that earned it rather than to whatever the body
    // happens to be playing when the verdict lands a frame later.
    let mut instance_move: [std::collections::BTreeMap<u32, String>; 2] = Default::default();
    let mut damage_by_move: [std::collections::BTreeMap<String, i32>; 2] = Default::default();
    let mut unclaimed_damage = [0i32; 2];
    let mut seat_of: std::collections::HashMap<Entity, usize> = Default::default();
    // ⭐ DO THEY EVER GET CLOSE? Separates "the CPUs never approach" from "they
    // stand next to each other and decline to press" — two different bugs with
    // the same reading of zero damage.
    let mut min_gap = f32::INFINITY;
    // The two seats start as mirror images about the stage centre. If the CPUs
    // read a SYMMETRIC STAGE they stay mirrored, and their damage/hitstun
    // totals cannot part company. So the tick symmetry BREAKS is the tick the
    // duel stops being a reflection — and a bout that never breaks it is a
    // bout whose lockstep needs no explanation beyond the stage.
    //
    // Mirror about an unknown vertical axis is `x0 + x1 == const` with
    // `y0 == y1`; both are read off the first tick both seats exist.
    //
    // ⚠ THE FIRST VERSION OF THIS PROBE REPORTED A BREAK FOR THREE OF FIVE
    // BOUTS AT 0.0001px, WHICH IS ~3 ULP OF AN f32 NEAR A FEW HUNDRED PIXELS.
    // "When did it first differ" is not a question about the duel; every bout
    // differs in the last bits almost immediately. The question is whether the
    // difference AMPLIFIES, so the magnitude is what is reported, and the first
    // tick is only counted once the split is a whole pixel.
    // ⭐ THE BRAIN EACH SEAT WAS BUILT WITH, read on the first tick it exists —
    // because the reading at the END of the duel is a reading AFTER any
    // knockout, and a respawned body is a body that was built a second time.
    let mut first_brain: [Option<String>; 2] = [None, None];
    // ⛔⛔ AND THE STREAM POSITION AT BIRTH, WHICH IS THE ONLY TICK IT IS THE
    // SEED. `FighterState::new(cfg, seed)` stores the seed IN `state.noise` and
    // then ADVANCES it on every draw, so the value read at the end of a bout is
    // a stream position, not a seed. Two seats that differ there may have
    // started equal and simply drawn a different number of samples — so an
    // end-of-bout reading can witness "these two share a stream" (equal) but
    // never "these two have distinct seeds" (different). The birth reading can.
    let mut first_noise: [Option<u64>; 2] = [None, None];
    let mut mirror_axis: Option<f32> = None;
    let mut mirror_worst = (0.0f32, 0.0f32);
    let mut mirror_broke_at: Option<(usize, f32, f32)> = None;
    // PROBE: the ABSOLUTE tick each seat is first seen, and each body's
    // half-extent.
    //
    // ⛔⛔ **THE FIGHT REPRODUCES AND THE WINDOW DOES NOT, AND THOSE HAVE TWO
    // CAUSES.** Two runs of `perfect_cellular_automaton` gave byte-identical
    // damage, hitstun and in-reach ticks over windows of 3580 and 3602. Either
    // seating waits on wall-clock asset IO and the SIM after it is deterministic
    // — harmless, harness-only — or the sim itself differs at the seating
    // transaction, which in a rollback game is a desync class. ⇒ The
    // discriminator is ABSOLUTE ticks: if the prologue length moves and
    // everything after is identical offset by it, the sim is clean.
    //
    // ⚠ AND THE WIDTH IS HERE BECAUSE `walks_off` SCALES WITH IT.
    // `ahead < half_extent.x * 2.0` is the only fighter-varying term in movement
    // scoring, so a wider body reads "approach walks me off" from further back.
    // Printing the width per fighter is what turns that into a claim or kills it.
    let mut first_seen: [Option<usize>; 2] = [None, None];
    let mut half_extent: [Option<f32>; 2] = [None, None];
    let mut close_ticks = 0usize;
    let mut hits = app
        .world()
        .resource::<Messages<ambition_platformer2d::combat::hitbox::ResolvedBodyHit>>()
        .get_cursor();
    let mut grounded_ticks = [0usize; 2];
    // ⭐ THE DUEL ENDS WHEN SOMEBODY WINS, and everything after that is not a
    // measurement of a fight. A decided match despawns the loser, so the loop
    // below would otherwise go on dividing a finished fight by a full minute.
    let mut duel_began = false;
    let mut decided_on: Option<usize> = None;
    // ⭐ THE STOCK ECONOMY IS PART OF THE STRUCTURE THIS THRESHOLD IS
    // CALIBRATED AGAINST. A knockout resets a meter, spends a stock and — once a
    // ruleset declares a respawn beat — takes a fighter off the stage for it. A
    // reading that does not say how many happened cannot be compared with one
    // taken under a different economy.
    let mut knockouts = 0usize;
    let mut respawned_this_tick: Vec<bevy::prelude::Entity> = Vec::new();
    let mut spent_cursor = None;
    let mut due_cursor = None;
    // ⭐⭐ D194'S FAILURE MODE, NAMED. Two grabs on one tick made both bodies
    // captor AND captive, and a body in that state can neither act nor be
    // released — which is what put the capture policy out of reach and cost the
    // mirror 28% of the match. `CapturedBy` is the sole authority and the
    // inverse is derived, so the shape is checkable directly: nobody may be
    // held while also holding somebody.
    //
    // Here because D192's interval is the arm that was never run against the
    // repaired regime: the old hold could not test `D194 fix + interval` at all.
    let mut mutual_capture_ticks = 0usize;
    for tick in 0..(countdown + TICKS) {
        app.update();
        {
            let world = app.world_mut();
            let mut held = world.query::<(
                bevy::prelude::Entity,
                &ambition_platformer2d::combat::capture::CapturedBy,
            )>();
            let pairs: Vec<(bevy::prelude::Entity, bevy::prelude::Entity)> = held
                .iter(world)
                .map(|(body, by)| (body, by.captor))
                .collect();
            let captives: Vec<bevy::prelude::Entity> = pairs.iter().map(|(b, _)| *b).collect();
            if pairs.iter().any(|(_, captor)| captives.contains(captor)) {
                mutual_capture_ticks += 1;
            }
        }
        let world = app.world_mut();
        {
            let messages = world
                .resource::<bevy::ecs::message::Messages<
                    ambition_platformer2d::actor::FighterStockSpent,
                >>();
            let cursor = spent_cursor.get_or_insert_with(|| messages.get_cursor());
            knockouts += cursor.read(messages).count();
        }
        // ⛔⛔ THE RETURN, NOT THE SPEND. D192 put a beat between them, so the
        // spend tick is no longer the tick the body is placed — reading the
        // recovery off `FighterStockSpent` would now sample a fighter that is
        // still lying where it died, ~60 ticks before the reset this asserts on.
        {
            let messages = world
                .resource::<bevy::ecs::message::Messages<
                    ambition_platformer2d::actor::FighterRespawnDue,
                >>();
            let cursor = due_cursor.get_or_insert_with(|| messages.get_cursor());
            for due in cursor.read(messages) {
                respawned_this_tick.push(due.body);
            }
        }
        // ⛔⛔ A RETURNING FIGHTER COMES BACK WITH ITS RECOVERY, checked on the
        // tick it returns — the only tick where the answer is unambiguous.
        //
        // `place_respawning_fighters` resets the body IN THE AIR and runs no
        // landing-class refresh after it, so whatever the reset leaves is what
        // the fighter fights the next stock with. Both fresh-construction paths
        // spelled the jump cluster `..Default::default()`, and Default is the
        // SPENT state, so a returning fighter could not use the special meant to
        // save it. Every test that touched the floor first was immediately
        // correct, which is why this needs the RESPAWN tick specifically.
        //
        // ⛔ NOT "any airborne fighter has a recovery": a fighter that has
        // legitimately spent one is airborne with zero, and a check that could
        // not tell the two apart would fire on correct play.
        for body in respawned_this_tick.drain(..) {
            let jump = world
                .get::<ambition_platformer2d::engine_core::BodyJumpState>(body)
                .expect("a respawned fighter is still a body");
            assert!(
                jump.recovery_charges > 0,
                "a fighter came back from a lost stock, in the air, with no \
                 recovery charge (tick {tick})"
            );
        }
        let mut seated = 0usize;
        for (seat, health, combat) in world
            .query::<(&MatchSeat, &BodyHealth, Option<&BodyCombat>)>()
            .iter(world)
        {
            if seat.0 < 2 {
                seated += 1;
                if decided_on.is_some() {
                    continue;
                }
                // ACCUMULATED, not peaked. A KO resets a body's percent to
                // zero, so the highest reading a seat ever shows is capped by
                // how long it survives — and the faster the fight, the LOWER
                // that number goes. Measured 2026-08-23: a scorer fix that
                // raised damage, tumbling and teching across every stream pushed
                // this seat's peak from 1.69 to 0.49, because it was dying
                // before it could accumulate. Summing the rises is immune to
                // that; it is also what "each fighter must accumulate
                // substantial damage" says.
                let now = health.damage_percent();
                if now > last[seat.0] {
                    taken[seat.0] += now - last[seat.0];
                }
                last[seat.0] = now;
                if combat.is_some_and(|c| c.hitstun_timer > 0.0) {
                    hitstun_ticks[seat.0] += 1;
                }
            }
        }
        {
            // Absolute, and BEFORE the `seated == 2` gate, so a seat that
            // arrives alone is still dated.
            let w = app.world_mut();
            let mut sq = w.query::<&MatchSeat>();
            for seat in sq.iter(w) {
                if seat.0 < 2 && first_seen[seat.0].is_none() {
                    first_seen[seat.0] = Some(tick);
                }
            }
        }
        if seated == 2 {
            duel_began = true;
            if decided_on.is_none() {
                both_seated_ticks += 1;
                // PROBE: how much of the duel is spent RUNNING while grounded —
                // the stance in which the press road collapses the attack menu
                // to the dash attack.
                let w = app.world_mut();
                let mut q = w.query::<(
                    &MatchSeat,
                    &ambition_platformer2d::engine_core::BodyMotionFacts,
                    &ambition_platformer2d::engine_core::BodyGroundState,
                )>();
                let rows: Vec<(usize, bool, bool)> = q
                    .iter(w)
                    .map(|(seat, f, g)| (seat.0, f.running, g.on_ground))
                    .collect();
                // PROBE: which MOVES each seat actually starts. Keyed on
                // (id, instance) so a self-cancel into the same move counts
                // twice — `MovePlayback::instance` exists for exactly that.
                let mut mq = w.query::<(
                    Entity,
                    &MatchSeat,
                    &ambition_platformer2d::combat::moveset::MovePlayback,
                )>();
                let starts: Vec<(Entity, usize, String, u32)> = mq
                    .iter(w)
                    .map(|(e, seat, pb)| (e, seat.0, pb.spec.id.clone(), pb.instance))
                    .collect();
                for (entity, slot, id, instance) in starts {
                    if slot < 2 {
                        seat_of.insert(entity, slot);
                        instance_move.get_mut(slot).unwrap().insert(instance, id.clone());
                        if last_instance[slot].as_ref() != Some(&(id.clone(), instance)) {
                            *move_counts[slot].entry(id.clone()).or_default() += 1;
                            last_instance[slot] = Some((id, instance));
                        }
                    }
                }
                // ⛔ ONE CURSOR, KEPT. A fresh `get_cursor()` inside the tick loop
                // starts at the OLDEST buffered message and bevy holds messages
                // two frames, so every hit would be counted twice.
                {
                    let msgs = w
                        .resource::<Messages<ambition_platformer2d::combat::hitbox::ResolvedBodyHit>>(
                        );
                    for hit in hits.read(msgs) {
                        let Some(attacker) = hit.attacker else { continue };
                        let Some(&slot) = seat_of.get(&attacker) else {
                            continue;
                        };
                        match hit
                            .attacker_move_instance
                            .and_then(|n| instance_move[slot].get(&n).cloned())
                        {
                            Some(id) => *damage_by_move[slot].entry(id).or_default() += hit.damage,
                            None => unclaimed_damage[slot] += hit.damage,
                        }
                    }
                }
                {
                    let mut bq = w.query::<(
                        &MatchSeat,
                        &ambition_platformer2d::characters::brain::Brain,
                    )>();
                    for (seat, brain) in bq.iter(w) {
                        if seat.0 < 2 && first_brain[seat.0].is_none() {
                            first_brain[seat.0] = Some(brain.label().to_string());
                            if let ambition_platformer2d::characters::brain::Brain::StateMachine(
                                ambition_platformer2d::characters::brain::StateMachineCfg::Fighter {
                                    state,
                                    ..
                                },
                            ) = brain
                            {
                                first_noise[seat.0] = Some(state.noise);
                            }
                        }
                    }
                }
                {
                    let mut pq = w.query::<(
                        &MatchSeat,
                        &ambition_platformer2d::engine_core::BodyKinematics,
                    )>();
                    for (seat, kin) in pq.iter(w) {
                        if seat.0 < 2 {
                            half_extent[seat.0] = Some(kin.size.x * 0.5);
                        }
                    }
                    let mut pos: [Option<ambition_platformer2d::engine_core::Vec2>; 2] =
                        [None, None];
                    for (seat, kin) in pq.iter(w) {
                        if seat.0 < 2 {
                            pos[seat.0] = Some(kin.pos);
                        }
                    }
                    if let (Some(x), Some(y)) = (pos[0], pos[1]) {
                        let gap = (x - y).length();
                        let axis = x.x + y.x;
                        let dy = x.y - y.y;
                        match mirror_axis {
                            None => mirror_axis = Some(axis),
                            Some(a0) => {
                                let daxis = axis - a0;
                                mirror_worst.0 = mirror_worst.0.max(daxis.abs());
                                mirror_worst.1 = mirror_worst.1.max(dy.abs());
                                if mirror_broke_at.is_none()
                                    && (daxis.abs() >= 1.0 || dy.abs() >= 1.0)
                                {
                                    mirror_broke_at = Some((tick, daxis, dy));
                                }
                            }
                        }
                        if gap < min_gap {
                            min_gap = gap;
                        }
                        // Roughly a body-and-a-half: inside this an ordinary
                        // grounded attack can reach.
                        if gap <= 60.0 {
                            close_ticks += 1;
                        }
                    }
                }
                for (slot, running, on_ground) in rows {
                    if slot < 2 {
                        if on_ground {
                            grounded_ticks[slot] += 1;
                            if running {
                                running_ticks[slot] += 1;
                            }
                        }
                    }
                }
            }
        } else if duel_began && decided_on.is_none() {
            decided_on = Some(tick);
        }
    }

    // ⛔ "SEATING FAILED" AND "SOMEBODY WON" ARE NOT THE SAME READING, and the
    // predecessor could not tell them apart: it required two seats for half the
    // budget, which a decisive match fails BY WINNING. What makes the numbers
    // below meaningless is a duel that never happened; what makes them better is
    // a duel that ended.
    assert!(
        both_seated_ticks >= A_MEASURABLE_DUEL,
        "two fighters shared the stage for only {both_seated_ticks} of {TICKS} \
         ticks (decided on {decided_on:?}), which is under the {A_MEASURABLE_DUEL}-tick \
         floor a rate can be read off — so nothing below is a measurement of a duel"
    );

    assert_eq!(
        mutual_capture_ticks, 0,
        "a body was BOTH captor and captive on {mutual_capture_ticks} tick(s) — \
         D194's lock is back, and with a respawn interval in play it would hold \
         two fighters through the beat as well as through the fight"
    );

    // ⭐ REPORTED ON SUCCESS TOO. The threshold is calibrated against a match
    // STRUCTURE — how much of a duel is spent fighting — and a respawn interval,
    // a countdown or a grab lock each change that without changing the tuning. A
    // guard that only speaks when it fails cannot say which of the two moved.
    let per_minute = |seat: usize| taken[seat] / both_seated_ticks as f32 * TICKS as f32;
    let rung = rung();
    println!(
        "[duel] {fighter} rung {rung}: duel ran {both_seated_ticks} ticks (decided \
         {decided_on:?}), took {:.2} / {:.2} of pool = {:.2} / {:.2} per minute of \
         duel, hitstun {hitstun_ticks:?} ticks, {knockouts} knockouts",
        taken[0],
        taken[1],
        per_minute(0),
        per_minute(1),
    );

    for seat in 0..2 {
        println!(
            "[stance] seat {seat}: grounded {} ticks, running {} of them ({:.0}%)",
            grounded_ticks[seat],
            running_ticks[seat],
            100.0 * running_ticks[seat] as f32 / grounded_ticks[seat].max(1) as f32
        );
    }
    println!(
        "[body] half-extent x: {:?} / {:?}; first seen at absolute tick {:?} / {:?}",
        half_extent[0], half_extent[1], first_seen[0], first_seen[1]
    );
    // ⭐ THE SEED EACH SEAT IS THINKING ON, and the authored breadth it is
    // choosing from — the two facts that separate "these fighters share a
    // cognitive stream" from "they have distinct streams and behave identically
    // anyway", which is the open question on D-CPU-INERT.
    {
        let w = app.world_mut();
        let mut q = w.query::<(
            &MatchSeat,
            &ambition_platformer2d::characters::brain::Brain,
            Option<&ambition_platformer2d::combat::moveset::ActorMoveset>,
            &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        )>();
        let mut rows: Vec<(usize, String, usize)> = q
            .iter(w)
            .filter(|(seat, _, _, _)| seat.0 < 2)
            .map(|(seat, brain, moveset, config)| {
                use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
                let seed = match brain {
                    Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) => {
                        format!("{:#018x}", state.noise)
                    }
                    other => format!("<none: {} brain>", other.label()),
                };
                (
                    seat.0,
                    format!(
                        "{seed} {} cfg={:?}",
                        brain.label(),
                        config.brain_profile.template
                    ),
                    moveset.map_or(0, |m| m.0.moves.len()),
                )
            })
            .collect();
        rows.sort_by_key(|r| r.0);
        // ⛔⛔ ITERATE THE SEATS, NOT THE QUERY. The query above reads the world
        // at the END of the bout, and a seat that was knocked out is not in it
        // — measured at rung 3, where medic scores 5 knockouts and seat 0
        // printed NO `[brain]` line at all. The birth facts (`first_brain`,
        // `first_noise`) exist for both seats regardless, and they are the ones
        // the seed question is asked of, so a missing END row must degrade the
        // `now=` field alone rather than delete the row.
        //
        // ⚠ This mattered because a consumer that requires two `seed=` values
        // reads a one-line block as an unparseable log, not as a dead seat, and
        // reports UNMEASURABLE for every rung where anybody dies.
        for seat in 0..2usize {
            let (seed, authored) = rows
                .iter()
                .find(|r| r.0 == seat)
                .map_or(("<not alive at the end>".to_string(), 0), |r| {
                    (r.1.clone(), r.2)
                });
            let born = first_brain[seat].as_deref().unwrap_or("<never seated>");
            let seed_at_birth = first_noise[seat]
                .map_or_else(|| "<no fighter brain at birth>".to_string(), |n| format!("{n:#018x}"));
            println!(
                "[brain] seat {seat}: born={born} seed={seed_at_birth} now={seed} \
                 authored_moves={authored}"
            );
        }
    }
    println!(
        "[gap] closest the seats ever came: {min_gap:.0}px; ticks within 60px: \
         {close_ticks} of {both_seated_ticks}"
    );
    let (worst_axis, worst_dy) = mirror_worst;
    match mirror_broke_at {
        None => println!(
            "[sym] never split by a whole pixel in {both_seated_ticks} ticks; \
             worst axis drift {worst_axis:.4}px, worst height split {worst_dy:.4}px"
        ),
        Some((tick, daxis, dy)) => println!(
            "[sym] split by a pixel at tick {tick} ({daxis:+.2}, {dy:+.2}); \
             worst axis drift {worst_axis:.2}px, worst height split {worst_dy:.2}px"
        ),
    }
    for seat in 0..2 {
        let mut rows: Vec<(&String, &usize)> = move_counts[seat].iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1));
        let total: usize = move_counts[seat].values().sum();
        let mut dmg: Vec<(&String, &i32)> = damage_by_move[seat].iter().collect();
        dmg.sort_by(|a, b| b.1.cmp(a.1));
        let dealt: i32 = damage_by_move[seat].values().sum();
        println!(
            "[dealt] seat {seat}: {dealt} damage across {} moves (+{} unclaimed) -> {:?}",
            damage_by_move[seat].len(),
            unclaimed_damage[seat],
            dmg.iter().take(30).collect::<Vec<_>>()
        );
        println!(
            "[moves] seat {seat}: {total} starts across {} distinct -> {:?}",
            move_counts[seat].len(),
            rows.iter().take(30).collect::<Vec<_>>()
        );
    }
    for seat in 0..2 {
        assert!(
            per_minute(seat) >= A_REAL_FIGHT,
            "seat {seat} took {:.0}% of its pool per minute of duel ({:.0}% over \
             the {both_seated_ticks} ticks the duel actually ran) — the CPUs are \
             not fighting. ⚠ read the UNITS before believing this: the value is a \
             RATIO, so {:.2} means {:.0}%, and a rig that printed it under a \
             literal `%` is what turned a 169% duel into a documented finding \
             that they never hit each other.",
            per_minute(seat) * 100.0,
            taken[seat] * 100.0,
            per_minute(seat),
            per_minute(seat) * 100.0,
        );
        assert!(
            hitstun_ticks[seat] > 0,
            "seat {seat} never entered hitstun, so whatever moved its damage meter \
             by {:.0}% was not the other fighter",
            taken[seat] * 100.0,
        );
    }
}

/// SWEEP THE WHOLE GRID, because every CPU number this project has is one
/// matchup's.
///
/// The demo shell's catalog carries three fighters; this composition carries the
/// whole select grid (D189). So every measurement taken through
/// `ambition_demo_smash_app`'s rigs — the decision histogram, the move census,
/// the weight override, the launch distributions the trail is fitted against —
/// describes George against George or the two stand-in duelists, and nothing
/// establishes that any of it generalises.
///
/// This is the instrument that can say. It is `#[ignore]`d because it is a
/// MEASUREMENT rather than a guard: it asserts only the thing that would make
/// its own numbers meaningless, and printing is the deliverable.
///
/// ```text
/// cargo test -p ambition_app --test app_it -- --ignored --nocapture every_fighter_on_the_grid
/// ```
#[test]
#[ignore = "a measurement, not a guard: minutes per fighter, run it when a scoring change needs validating"]
fn every_fighter_on_the_grid_can_fight_its_mirror() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let ids: Vec<String> = {
        let registry = app
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .expect("the composed host has a prepared-character registry");
        SmashRoster::assemble(registry)
            .ids()
            .map(str::to_string)
            .collect()
    };
    assert!(
        ids.len() > 3,
        "this composition assembled {} fighters, which is the demo shell's count — \
         so this sweep would measure exactly what the cheaper rigs already do",
        ids.len()
    );
    drop(app);

    println!(
        "[grid-sweep] {} fighters, mirror matches, {TICKS} ticks each",
        ids.len()
    );
    println!(
        "[grid-sweep] {:<30} {:>9} {:>9} {:>9} {:>7} {:>10}  {}",
        "fighter", "took0%", "took1%", "hitstun", "moves", "used/seen/kit", "most thrown"
    );
    let mut silent: Vec<String> = Vec::new();
    for id in &ids {
        let (taken, hitstun, moves, top, kit, distinct, reachable, asked) = mirror_bout(id);
        println!(
            "[grid-sweep] {id:<30} {:>8.0}% {:>8.0}% {:>9} {:>7} {:>4}/{:>3}/{:<4}  {top:<46} {asked}",
            taken[0] * 100.0,
            taken[1] * 100.0,
            hitstun[0] + hitstun[1],
            moves,
            distinct,
            reachable,
            kit,
        );
        if taken[0] + taken[1] < 0.1 {
            silent.push(id.clone());
        }
    }
    // The ONE assertion, and it guards the reading rather than the game: a
    // fighter that cannot be seated at all prints zeros indistinguishable from a
    // fighter that stands still, and a sweep where that goes unsaid is a table
    // of numbers with holes in it nobody can see.
    assert!(
        silent.len() * 2 < ids.len(),
        "{} of {} fighters took no damage at all in their own mirror — that is a \
         seating failure wearing a balance number: {silent:?}",
        silent.len(),
        ids.len()
    );
}

/// One mirror match in the shipped composition: damage each seat ACCUMULATED
/// (a KO resets the meter, so a peak measures survival rather than violence) and
/// ticks each spent in hitstun.
fn mirror_bout(
    fighter: &str,
) -> (
    [f32; 2],
    [usize; 2],
    usize,
    String,
    usize,
    usize,
    usize,
    String,
) {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let roster = ambition_demo_smash::smash_roster_at_levels([fighter, fighter], &[rung(), rung()]);
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut taken = [0.0f32; 2];
    let mut last = [0.0f32; 2];
    let mut hitstun = [0usize; 2];
    let mut grounded_hitstun = [0usize; 2];
    let mut peak_stun = [0.0f32; 2];
    // WHAT THEY THREW, because 0% has two completely different causes and this
    // is what tells them apart: a fighter that starts no moves is missing a
    // repertoire or a brain, and one that starts plenty and deals nothing is
    // missing reach, hit volumes, or a victim it can legally strike.
    let mut started = std::collections::BTreeMap::<String, usize>::new();
    let mut situations = std::collections::BTreeMap::<String, usize>::new();
    let mut gaps: Vec<f32> = Vec::new();
    let mut live = std::collections::BTreeMap::<bevy::prelude::Entity, (String, f32)>::new();
    for _ in 0..(countdown + TICKS) {
        app.update();
        let world = app.world_mut();
        for (seat, health, combat, ground) in world
            .query::<(
                &MatchSeat,
                &BodyHealth,
                Option<&BodyCombat>,
                Option<&ambition_platformer2d::engine_core::BodyGroundState>,
            )>()
            .iter(world)
        {
            if seat.0 < 2 {
                let now = health.damage_percent();
                if now > last[seat.0] {
                    taken[seat.0] += now - last[seat.0];
                }
                last[seat.0] = now;
                // ⭐ THE LONGEST HITSTUN THIS BODY WAS EVER PUT IN, which is the
                // number the "hitstun must be shorter than the attacker's move
                // cycle" invariant is about. Hitstun scales with the LAUNCH, so
                // it grows through a match while the attacker's frame data does
                // not - and the tick a hit's stun exceeds the move's own total
                // is the tick that move becomes an infinite for anybody, human
                // or CPU.
                if let Some(c) = combat {
                    peak_stun[seat.0] = peak_stun[seat.0].max(c.hitstun_timer);
                }
                let stunned = combat.is_some_and(|c| c.hitstun_timer > 0.0);
                if stunned {
                    hitstun[seat.0] += 1;
                    // ⭐⭐ IS THE VICTIM ON THE FLOOR WHILE IT IS BEING HIT?
                    // This is the column that decides D191, because a GROUNDED
                    // body in hitstun has no agency at all: `survival_stick`
                    // refuses it deliberately (holding a stick on the floor is
                    // walking out of hitstun) and `apply_post_hit_input_gates`
                    // exempts the Burst edge only while TUMBLING. An AIRBORNE
                    // juggle is a fight the victim is losing; a GROUNDED one is
                    // a fight it is not allowed to play.
                    if ground.is_some_and(|g| g.on_ground) {
                        grounded_hitstun[seat.0] += 1;
                    }
                }
            }
        }
        {
            let mut bodies = world.query::<(
                &MatchSeat,
                &ambition_platformer2d::engine_core::BodyKinematics,
            )>();
            let xs: Vec<f32> = bodies
                .iter(world)
                .filter(|(seat, _)| seat.0 < 2)
                .map(|(_, kin)| kin.pos.x)
                .collect();
            if xs.len() == 2 {
                gaps.push((xs[0] - xs[1]).abs());
            }
        }
        // WHICH QUESTION IS THE BRAIN ANSWERING? `situation_of` is the classifier
        // itself, asked of the live state — not a re-derivation. A fighter that
        // throws one move three times a second is answering the SAME question
        // every tick, and this is the column that says which one.
        {
            let mut brains = world.query::<&ambition_platformer2d::characters::brain::Brain>();
            for brain in brains.iter(world) {
                if let ambition_platformer2d::characters::brain::Brain::StateMachine(
                    ambition_platformer2d::characters::brain::StateMachineCfg::Fighter {
                        state,
                        ..
                    },
                ) = brain
                {
                    if let Some(situation) =
                        ambition_platformer2d::combat::brain::fighter::decision::situation_of(state)
                    {
                        *situations.entry(format!("{situation:?}")).or_default() += 1;
                    }
                }
            }
        }
        let rows: Vec<(bevy::prelude::Entity, String, f32)> = world
            .query::<(
                bevy::prelude::Entity,
                &MatchSeat,
                &ambition_platformer2d::combat::moveset::MovePlayback,
            )>()
            .iter(world)
            .map(|(entity, _, pb)| (entity, pb.spec.id.clone(), pb.t))
            .collect();
        for (entity, id, t) in rows {
            let fresh = match live.get(&entity) {
                Some((last_id, last_t)) => last_id != &id || t < *last_t,
                None => true,
            };
            if fresh {
                *started.entry(id.clone()).or_default() += 1;
            }
            live.insert(entity, (id, t));
        }
    }
    // ⛔⛔ EVERY "ASK ONE SEATED BODY" QUERY BELOW TAKES THE LOWEST SEAT, NOT THE
    // FIRST ROW. `.iter().next()` hands back whichever body archetype iteration
    // happens to reach first, and that is not stable across runs: the same
    // fighter reported `kit 26` on one sweep and `kit 0` on the next, and a
    // `sight` column read `no-perception-component` for a body that plainly had
    // one. A column that answers a different body on each run is worse than a
    // missing column, because it looks like a finding.
    //
    // ⚠ these columns describe SEAT 0 and this is a mirror match, so seat 0 and
    // seat 1 carry the same character. They are not a claim about both seats.
    // HOW BIG IS THE KIT? "Threw six moves in a minute" is a content finding if
    // the body only has two, and a selection finding if it has sixteen. The
    // authored table is on the body; asking it costs one query.
    let kit = {
        let world = app.world_mut();
        world
            .query::<(
                &MatchSeat,
                &ambition_platformer2d::combat::moveset::ActorMoveset,
            )>()
            .iter(world)
            .min_by_key(|(seat, _)| seat.0)
            .map(|(_, moveset)| moveset.0.moves.len())
            .unwrap_or(0)
    };
    // ⭐ HOW MANY MOVES CAN THE BRAIN EVEN SEE? The kit is not the moveset. It is
    // built by asking `move_for_directional_verb` for three verbs across five
    // directions plus a grab — at most sixteen entries — so a character's
    // authored breadth is visible to the scorer only where it is BOUND to one of
    // those presses. A fighter with thirty-three moves and two bound slots has
    // thirty-one the brain will never offer, and "it only ever throws two" is
    // then a wiring fact rather than a scoring one.
    let reachable = {
        let world = app.world_mut();
        world
            .query::<(
                &MatchSeat,
                &ambition_platformer2d::combat::moveset::ActorMoveset,
            )>()
            .iter(world)
            .min_by_key(|(seat, _)| seat.0)
            .map(|(_, moveset)| {
                let mut ids = std::collections::BTreeSet::new();
                for verb in [
                    ambition_platformer2d::combat::moveset::ATTACK_VERB,
                    ambition_platformer2d::combat::moveset::SMASH_VERB,
                    ambition_platformer2d::combat::moveset::SPECIAL_VERB,
                ] {
                    for dir in [
                        ambition_platformer2d::entity_catalog::AttackDir::Neutral,
                        ambition_platformer2d::entity_catalog::AttackDir::Forward,
                        ambition_platformer2d::entity_catalog::AttackDir::Back,
                        ambition_platformer2d::entity_catalog::AttackDir::Up,
                        ambition_platformer2d::entity_catalog::AttackDir::Down,
                    ] {
                        for grounded in [true, false] {
                            if let Some(spec) =
                                moveset.0.move_for_directional_verb(verb, dir, grounded)
                            {
                                ids.insert(spec.id.clone());
                            }
                        }
                    }
                }
                ids.len()
            })
            .unwrap_or(0)
    };
    // ⭐⭐ CAN THE BRAIN SEE THE FOE AT ALL? `Perception` has two modes and the
    // difference is a hard cliff, not a falloff: `Omniscient` knows the nearest
    // hostile ANYWHERE, `Sighted { viewport_half }` is BLIND past the box (plus
    // decaying memory pursuit). `DEFAULT_VIEWPORT_HALF.x` is 480 world px, and
    // the platform is 480 wide — so whether a pair at gap 500 is inside or
    // outside its own senses is the question the gap column raises and cannot
    // answer. This column answers it.
    let sight = {
        let world = app.world_mut();
        world
            .query::<(
                &MatchSeat,
                &ambition_platformer2d::actors::features::ecs::perception::Perception,
            )>()
            .iter(world)
            .min_by_key(|(seat, _)| seat.0)
            .map(|(_, perception)| match perception {
                ambition_platformer2d::actors::features::ecs::perception::Perception::Omniscient => {
                    "omniscient".to_string()
                }
                ambition_platformer2d::actors::features::ecs::perception::Perception::Sighted {
                    viewport_half,
                } => format!("sees±{:.0}", viewport_half.x),
            })
            // ⭐ ABSENT IS NOT UNKNOWN. A body with no `Perception` reads as
            // `Omniscient` by documented policy, and for a seated fighter that
            // is the EXPECTED state — `ensure_perception` skips a body carrying
            // a `MatchSeat`, so no component is exactly what the fix produces.
            // Printed distinctly from an explicit `Omniscient` so the column can
            // still tell "the grant was skipped" from "somebody declared it".
            .unwrap_or_else(|| "omniscient(default)".to_string())
    };
    let moves: usize = started.values().sum();
    let top = started
        .iter()
        .max_by_key(|(id, count)| (**count, std::cmp::Reverse((*id).clone())))
        .map(|(id, count)| format!("{id}×{count}"))
        .unwrap_or_else(|| "—".to_string());
    // DID THEY EVER MEET? Two fighters that never close the distance are in
    // `Neutral` forever by construction — nobody has anything — and 98% Neutral
    // with four presses a minute has two completely different explanations: they
    // met and did nothing, or they never met. The gap between the bodies is what
    // separates those, and it costs one subtraction a tick.
    //
    // ⚠ measured only while BOTH are present, so a KO's absence does not read as
    // infinite distance.
    let median_gap = {
        let mut sorted = gaps.clone();
        sorted.sort_by(f32::total_cmp);
        sorted.get(sorted.len() / 2).copied().unwrap_or(-1.0)
    };
    // HOW LONG DOES THE MOVE THEY KEEP THROWING LAST? Four presses in sixty
    // seconds is what a body looks like when one move owns it for fifteen
    // seconds at a time — the offers exist, and the body is never free to take
    // one. `total_s` is the move's own authored length, asked of the same table
    // the brain is scored out of.
    let top_id = started
        .iter()
        .max_by_key(|(id, count)| (**count, std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone());
    let top_secs = {
        let world = app.world_mut();
        top_id
            .as_ref()
            .and_then(|wanted| {
                world
                    .query::<(
                        &MatchSeat,
                        &ambition_platformer2d::combat::moveset::ActorMoveset,
                    )>()
                    .iter(world)
                    .min_by_key(|(seat, _)| seat.0)
                    .and_then(|(_, moveset)| {
                        moveset
                            .0
                            .moves
                            .iter()
                            .find(|spec| &spec.id == wanted)
                            .map(|spec| spec.frame_data().total_s)
                    })
            })
            .unwrap_or(0.0)
    };
    let asked = {
        let total: usize = situations.values().sum::<usize>().max(1);
        let mut rows: Vec<_> = situations.iter().collect();
        rows.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        rows.iter()
            .take(2)
            .map(|(name, count)| format!("{name} {:.0}%", 100.0 * **count as f32 / total as f32))
            .collect::<Vec<_>>()
            .join(" ")
    };
    (
        taken,
        hitstun,
        moves,
        format!(
            "{top} {top_secs:.2}s gap{median_gap:.0} grounded-stun {}% peak-stun {:.2}s {sight}",
            if hitstun[0] + hitstun[1] == 0 {
                0
            } else {
                100 * (grounded_hitstun[0] + grounded_hitstun[1]) / (hitstun[0] + hitstun[1])
            },
            peak_stun[0].max(peak_stun[1])
        ),
        kit,
        started.len(),
        reachable,
        asked,
    )
}

/// ⭐ HOW MANY SOUNDS DOES THE GOBLIN / PCA FIGHT ASK FOR?
///
/// Jon, 2026-08-25: *"there is a bad sfx problem with goblin and pca"*, and he
/// wants to know whether the volume of triggers indicates a deeper bug rather
/// than a mix problem.
///
/// ⛔⛔ IT COUNTS THE ASK, NOT WHAT A LISTENER HEARS. `OwnedSfxMessage` is what
/// mechanics emit, before any volume, ducking or voice limiting decides what
/// reaches a speaker. If the count is wrong here no mix change can fix it — and
/// if the count is fine, the problem IS the mix and this says so.
///
/// ⛔ IN THE SHIPPED COMPOSITION, not the demo shell. D189: the demo shell's
/// catalog carries George and two stand-ins, so neither of these two can be
/// seated there — a rig that tried would measure an empty stage.
///
/// The assertion is deliberately structural. A tuned ceiling on total density
/// would go red on any balance change and teach nobody anything; many of ONE
/// sound on ONE tick, with two fighters on the stage, cannot be anything but a
/// duplicate emission.
#[test]
fn the_goblin_and_the_pca_do_not_ask_for_the_same_sound_many_times_on_one_tick() {
    use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxMessage};
    use bevy::ecs::message::Messages;
    use std::collections::BTreeMap;

    fn variant(request: &SfxMessage) -> &'static str {
        match request {
            SfxMessage::Jump { .. } => "jump",
            SfxMessage::DoubleJump { .. } => "double_jump",
            SfxMessage::Dash { .. } => "dash",
            SfxMessage::Blink { .. } => "blink",
            SfxMessage::Pogo { .. } => "pogo",
            SfxMessage::Land { .. } => "land",
            SfxMessage::Slash { .. } => "slash",
            SfxMessage::Hit { .. } => "hit",
            SfxMessage::Death { .. } => "death",
            SfxMessage::Reset { .. } => "reset",
            SfxMessage::Play { .. } => "play",
        }
    }

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let roster = ambition_demo_smash::smash_roster_at_levels(
        ["goblin", "perfect_cellular_automaton"],
        &[rung(), rung()],
    );
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut total: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut worst: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut busiest = (0usize, 0usize);
    let mut by_id: BTreeMap<String, usize> = BTreeMap::new();
    let mut worst_by_id: BTreeMap<String, usize> = BTreeMap::new();
    let mut seated_ticks = 0usize;
    let mut cursor = None;

    for tick in 0..(countdown + TICKS) {
        app.update();
        let world = app.world_mut();
        let seats = world.query::<&MatchSeat>().iter(world).count();
        if seats >= 2 {
            seated_ticks += 1;
        }
        let messages = world.resource::<Messages<OwnedSfxMessage>>();
        let cursor = cursor.get_or_insert_with(|| messages.get_cursor());
        let mut this_tick: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut tick_by_id: BTreeMap<String, usize> = BTreeMap::new();
        for owned in cursor.read(messages) {
            let name = variant(&owned.request);
            *this_tick.entry(name).or_default() += 1;
            *total.entry(name).or_default() += 1;
            // ⭐ WHICH authored sound, not just "an authored sound". Nearly every
            // request in this fight is `Play`, so the variant histogram alone
            // cannot say whether one emitter is stuck or the fight is simply loud.
            // ⛔ KEYED BY THE SOUND, NOT BY "an authored sound". A per-tick
            // ceiling on the `play` AGGREGATE is wrong and was measured wrong:
            // a george mirror legitimately asks for six DIFFERENT authored
            // sounds on one tick. Many of ONE id on one tick is the duplicate.
            let key = match owned.request {
                SfxMessage::Play { id, .. } => format!("{id:?}"),
                _ => name.to_string(),
            };
            *by_id.entry(key.clone()).or_default() += 1;
            *tick_by_id.entry(key).or_default() += 1;
        }
        let n: usize = this_tick.values().sum();
        if n > busiest.1 {
            busiest = (tick, n);
        }
        for (name, count) in this_tick {
            let slot = worst.entry(name).or_default();
            *slot = (*slot).max(count);
        }
        for (key, count) in tick_by_id {
            let slot = worst_by_id.entry(key).or_default();
            *slot = (*slot).max(count);
        }
    }

    // ⛔ THE PREMISE. A fight that never seated two fighters asks for few sounds
    // for a reason that has nothing to do with sfx.
    assert!(
        seated_ticks > TICKS / 4,
        "goblin vs PCA shared the stage for only {seated_ticks} ticks, so nothing \
         below is a measurement of a fight — check this composition can seat both"
    );

    let asked: usize = total.values().sum();
    eprintln!(
        "[sfx-census] goblin vs perfect_cellular_automaton, {seated_ticks} seated ticks: \
         {asked} requests = {:.1}/s\n  by variant (total, worst single tick): {}\n  \
         busiest tick: {} with {} requests",
        asked as f32 / (seated_ticks.max(1) as f32 / 60.0),
        total
            .iter()
            .map(|(k, v)| format!("{k}={v}/{}", worst.get(k).copied().unwrap_or(0)))
            .collect::<Vec<_>>()
            .join(" "),
        busiest.0,
        busiest.1,
    );
    let mut ranked: Vec<(&String, &usize)> = by_id.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!(
        "  loudest authored ids: {}",
        ranked
            .iter()
            .take(8)
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ")
    );

    // ⭐⭐ THE RATE GUARD, WHICH IS WHAT ACTUALLY CAUGHT D206 — and it is here
    // now because the fix is (see the seat gate in `apply_actor_contact_damage`).
    // The per-tick ceiling below could never see it: goblin vs PCA asked for
    // `player.hit` 6,997 times in 3,776 ticks, which is ~2 per tick, sustained
    // rather than bursty, and passed every burst check while being 31x a george
    // mirror's 223.
    //
    // ⛔ A SUSTAINED RATE IS A DIFFERENT SHAPE FROM A BURST, and neither
    // subsumes the other. A duplicate emission on one tick is a stuck emitter;
    // one sound at 111/s for a whole match is an emitter running on the wrong
    // clock — here, a hit event written EVERY TICK two bodies overlapped.
    //
    // The ceiling is set well above what a loud honest fight asks for and well
    // below the defect. Measured with the fix in: this fight asks for 751
    // requests over 3,776 seated ticks — **11.9/s total**, down from 114.2/s —
    // and its loudest id is `land` at 6.7/s, with `player.hit` down from 6,997
    // to 27. Anything sustaining 20/s for a whole match is one emitter, not a
    // busy stage.
    //
    // PROBED RED: with the seat gate reverted, this fails naming
    // `SfxId(1147272914855045707)` — `player.hit` — at **111.2/s**.
    const ONE_SOUND_PER_SECOND_CEILING: f32 = 20.0;
    let seconds = seated_ticks.max(1) as f32 / 60.0;
    let too_often: Vec<String> = by_id
        .iter()
        .filter(|(_, count)| **count as f32 / seconds > ONE_SOUND_PER_SECOND_CEILING)
        .map(|(name, count)| format!("{name} at {:.1}/s", *count as f32 / seconds))
        .collect();
    assert!(
        too_often.is_empty(),
        "one sound was asked for more than {ONE_SOUND_PER_SECOND_CEILING}/s across          the whole match: {} — that is an emitter on the wrong clock, not a busy          stage, and no mix change can fix it",
        too_often.join(", ")
    );

    // What this still catches is a genuine burst: many of ONE sound on ONE tick.
    // ⚠ keyed by ID, not by the `play` aggregate — a george mirror legitimately
    // asks for six DIFFERENT authored sounds on one tick, which the aggregate
    // version called a duplicate.
    const SAME_SOUND_ONE_TICK_CEILING: usize = 4;
    let offenders: Vec<String> = worst_by_id
        .iter()
        .filter(|(_, w)| **w > SAME_SOUND_ONE_TICK_CEILING)
        .map(|(name, w)| format!("{name} x{w}"))
        .collect();
    assert!(
        offenders.is_empty(),
        "one tick asked for the same sound more than {SAME_SOUND_ONE_TICK_CEILING} \
         times: {} — with two fighters on the stage that is a duplicate emission, \
         not density, and no mix change can fix it",
        offenders.join(", ")
    );
}

/// PROBE, print-only: WHERE do the goblin/PCA fight's 111 `player.hit`
/// requests per second come from?
///
/// The census above establishes the RATE and the id; it cannot say which
/// emitter. `player.hit` is the unauthored default for an ENEMY-profile victim
/// (`ambition_platformer2d::combat::util`), so every hit event on either of these two bodies
/// that carries no authored strike sound lands on the same id — which means the
/// sound is downstream of however many HIT EVENTS there are. This counts the
/// events by `HitSource`, which is the fork: `Melee` is a swing landing (paced
/// by a move's active window and deduplicated by `HitboxHits`), `Contact` is
/// `apply_actor_contact_damage`, which writes an event EVERY TICK two bodies
/// overlap and is gated only by the victim's i-frames.
#[test]
#[ignore = "PROBE, print-only: attributes the goblin/PCA hit rate by source"]
fn probe_where_the_goblin_pca_hit_events_come_from() {
    use ambition_platformer2d::combat::events::HitEvent;
    use bevy::ecs::message::Messages;
    use std::collections::BTreeMap;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let roster = ambition_demo_smash::smash_roster_at_levels(
        ["goblin", "perfect_cellular_automaton"],
        &[rung(), rung()],
    );
    let countdown = roster.rules.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
    let mut seated_ticks = 0usize;
    let mut cursor = None;
    for _ in 0..(countdown + TICKS) {
        app.update();
        let world = app.world_mut();
        let seats = world.query::<&MatchSeat>().iter(world).count();
        if seats >= 2 {
            seated_ticks += 1;
        }
        let messages = world.resource::<Messages<HitEvent>>();
        let cursor = cursor.get_or_insert_with(|| messages.get_cursor());
        for event in cursor.read(messages) {
            *by_source.entry(format!("{:?}", event.source)).or_default() += 1;
        }
    }
    let total: usize = by_source.values().sum();
    eprintln!("PROBE seated_ticks={seated_ticks} hit_events={total}");
    for (source, count) in &by_source {
        eprintln!(
            "PROBE   {source:<14} {count:>6}  ({:.1}/s)",
            *count as f32 * 60.0 / seated_ticks.max(1) as f32
        );
    }
}
