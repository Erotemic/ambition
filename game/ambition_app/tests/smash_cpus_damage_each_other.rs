//! Prove two CPUs in the shipped Smash composition actually fight each other.
//!
//! Each fighter must accumulate substantial damage and enter hitstun, while both
//! seats remain present. Damage is expressed as a ratio (`1.0 == 100%`); hitstun
//! distinguishes opponent hits from environmental/self damage.

use ambition_demo_smash::select::SmashRoster;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

/// An ordinary fighter a player can pick — asserted to be on the assembled grid
/// below, because a name this composition cannot seat proves nothing.
const FIGHTER: &str = "npc_pirate_admiral";

/// The top authored rung. If any rung fights, this one does.
const RUNG: u8 = 9;

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
            ids.contains(&FIGHTER),
            "`{FIGHTER}` is not on the assembled smash grid in this composition, so \
             seating it proves nothing about what a player can pick. Grid: {ids:?}"
        );
    }

    let roster = ambition_demo_smash::smash_roster_at_levels([FIGHTER, FIGHTER], &[RUNG, RUNG]);
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
        if seated == 2 {
            duel_began = true;
            if decided_on.is_none() {
                both_seated_ticks += 1;
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
    println!(
        "[duel] {FIGHTER} rung {RUNG}: duel ran {both_seated_ticks} ticks (decided \
         {decided_on:?}), took {:.2} / {:.2} of pool = {:.2} / {:.2} per minute of \
         duel, hitstun {hitstun_ticks:?} ticks, {knockouts} knockouts",
        taken[0],
        taken[1],
        per_minute(0),
        per_minute(1),
    );

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
    let roster = ambition_demo_smash::smash_roster_at_levels([fighter, fighter], &[RUNG, RUNG]);
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
                Option<
                    &ambition_platformer2d::actors::avatar::movement_components::BodyGroundState,
                >,
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
                &ambition_platformer2d::actors::actor::BodyKinematics,
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
                        ambition_platformer2d::characters::brain::fighter::decision::situation_of(
                            state,
                        )
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
        &[RUNG, RUNG],
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
/// (`ambition_combat::util`), so every hit event on either of these two bodies
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
        &[RUNG, RUNG],
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
