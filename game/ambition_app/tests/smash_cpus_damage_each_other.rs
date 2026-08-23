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
    let countdown = roster.opening_countdown_ticks as usize;
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
    let mut spent_cursor = None;
    for tick in 0..(countdown + TICKS) {
        app.update();
        let world = app.world_mut();
        {
            let messages = world
                .resource::<bevy::ecs::message::Messages<
                    ambition_platformer2d::actor::FighterStockSpent,
                >>();
            let cursor = spent_cursor.get_or_insert_with(|| messages.get_cursor());
            knockouts += cursor.read(messages).count();
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
    let countdown = roster.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut taken = [0.0f32; 2];
    let mut last = [0.0f32; 2];
    let mut hitstun = [0usize; 2];
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
        for (seat, health, combat) in world
            .query::<(&MatchSeat, &BodyHealth, Option<&BodyCombat>)>()
            .iter(world)
        {
            if seat.0 < 2 {
                let now = health.damage_percent();
                if now > last[seat.0] {
                    taken[seat.0] += now - last[seat.0];
                }
                last[seat.0] = now;
                if combat.is_some_and(|c| c.hitstun_timer > 0.0) {
                    hitstun[seat.0] += 1;
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
        format!("{top} {top_secs:.2}s gap{median_gap:.0} {sight}"),
        kit,
        started.len(),
        reachable,
        asked,
    )
}
