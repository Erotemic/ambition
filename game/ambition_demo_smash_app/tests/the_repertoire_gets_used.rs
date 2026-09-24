//! Observe which moves CPUs actually start during a real CPU-versus-CPU match.
//!
//! The histogram is instrumentation; assertions enforce minimum repertoire coverage, not tuning
//! targets. Move classes are derived from each body's authored `ActorMoveset`, so generic coverage
//! does not depend on character-specific move names.

use std::collections::{BTreeMap, BTreeSet};

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
use ambition_platformer2d::entity_catalog::MovesetContract;
use bevy::prelude::*;

/// Which body threw what, counted by move id.
#[derive(Default)]
struct MoveLedger {
    /// Every move start observed, per seat.
    started: BTreeMap<usize, BTreeMap<String, usize>>,
    /// Seat → ticks spent outside the stage's own footprint.
    ///
    /// This is the premise of every recovery question: only a fighter off the
    /// stage wants a route home.
    offstage_ticks: BTreeMap<usize, usize>,
    /// The last `(move id, clock)` seen per entity, so a move that is still
    /// running is not counted again every tick.
    live: BTreeMap<Entity, (String, f32)>,
}

impl MoveLedger {
    fn sample(&mut self, app: &mut App) {
        let stage = ambition_demo_smash::smash_stage().world.size;
        let world = app.world_mut();
        // Position is a proxy for "knocked out toward the blast zone and trying to
        // get home". A fighter waiting to respawn (D192) also lies outside the
        // stage, but cannot press anything, so exclude it.
        let mut positions = world.query_filtered::<
            (&MatchSeat, &ambition_platformer2d::actor::BodyKinematics),
            bevy::prelude::Without<ambition_platformer2d::actor::PendingRespawn>,
        >();
        let offstage: Vec<usize> = positions
            .iter(world)
            .filter(|(_, kin)| kin.pos.x < 0.0 || kin.pos.x > stage.x || kin.pos.y > stage.y)
            .map(|(seat, _)| seat.0)
            .collect();
        for seat in offstage {
            *self.offstage_ticks.entry(seat).or_default() += 1;
        }
        let mut q = world.query::<(Entity, &MatchSeat, &MovePlayback)>();
        let rows: Vec<(Entity, usize, String, f32)> = q
            .iter(world)
            .map(|(e, seat, pb)| (e, seat.0, pb.spec.id.clone(), pb.t))
            .collect();
        for (entity, seat, id, t) in rows {
            let fresh = match self.live.get(&entity) {
                Some((last_id, last_t)) => last_id != &id || t < *last_t,
                None => true,
            };
            if fresh {
                *self
                    .started
                    .entry(seat)
                    .or_default()
                    .entry(id.clone())
                    .or_default() += 1;
            }
            self.live.insert(entity, (id, t));
        }
        // A body whose move finished drops out of the query; forget it so the
        // next press counts as a start.
        let alive: BTreeSet<Entity> = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<MovePlayback>>();
            q.iter(world).collect()
        };
        self.live.retain(|e, _| alive.contains(e));
    }

    fn distinct_for_the_busiest_seat(&self) -> usize {
        self.started
            .values()
            .map(|by_move| by_move.len())
            .max()
            .unwrap_or(0)
    }

    fn every_move_seen(&self) -> BTreeMap<&str, usize> {
        let mut all: BTreeMap<&str, usize> = BTreeMap::new();
        for by_move in self.started.values() {
            for (id, n) in by_move {
                *all.entry(id.as_str()).or_default() += *n;
            }
        }
        all
    }
}

/// Move ids some `special*` verb reaches.
fn specials(table: &MovesetContract) -> BTreeSet<&str> {
    table
        .verbs
        .iter()
        .filter(|(verb, _)| verb.starts_with("special"))
        .map(|(_, id)| id.as_str())
        .collect()
}

/// Move ids gated airborne-only — the aerials, as the table declares them.
fn aerials(table: &MovesetContract) -> BTreeSet<&str> {
    table
        .moves
        .iter()
        .filter(|m| m.gates.grounded == Some(false))
        .map(|m| m.id.as_str())
        .collect()
}

/// Move ids whose authored frame data commands a rise — the ways home,
/// recognised by geometry exactly the way `lifting_candidates` recognises them
/// for the brain, so the two layers cannot disagree about what a route is.
fn routes(table: &MovesetContract) -> BTreeSet<&str> {
    table
        .moves
        .iter()
        .filter(|m| m.frame_data().lift_speed > 0.0)
        .map(|m| m.id.as_str())
        .collect()
}

fn count_within(started: &BTreeMap<String, usize>, ids: &BTreeSet<&str>) -> usize {
    started
        .iter()
        .filter(|(id, _)| ids.contains(id.as_str()))
        .map(|(_, n)| *n)
        .sum()
}

/// One CPU-versus-CPU match, and everything read off it.
struct MatchReport {
    ledger: MoveLedger,
    /// Seat → the table preparation gave that body. Read from the world,
    /// because what a seat wears can differ from what was authored.
    tables: BTreeMap<usize, MovesetContract>,
    /// Seat → the character id that seat was asked to wear.
    characters: BTreeMap<usize, String>,
}

impl MatchReport {
    fn started(&self, seat: usize) -> BTreeMap<String, usize> {
        self.ledger.started.get(&seat).cloned().unwrap_or_default()
    }

    /// The whole measurement, as a human reads it. Printed every run.
    fn render(&self) -> String {
        let mut out = String::new();
        for (seat, table) in &self.tables {
            let started = self.started(*seat);
            let total: usize = started.values().sum();
            let (specials, aerials, routes) = (specials(table), aerials(table), routes(table));
            // Printed on success too. Offstage time is the premise of every route
            // claim, so a green run must show whether the affordance was observed.
            let offstage = self.ledger.offstage_ticks.get(seat).copied().unwrap_or(0);
            out.push_str(&format!(
                "  seat {seat} wearing {:<22} starts={total} distinct={} \
                 specials={}/{} aerials={}/{} routes={}/{} offstage={offstage}\n    threw={started:?}\n",
                self.characters
                    .get(seat)
                    .map_or("?", std::string::String::as_str),
                started.len(),
                count_within(&started, &specials),
                specials.len(),
                count_within(&started, &aerials),
                aerials.len(),
                count_within(&started, &routes),
                routes.len(),
            ));
        }
        out
    }
}

/// Enough stocks that neither CPU is eliminated inside a patience budget.
///
/// The mirror spends about ten knockouts per seat in a 3600-tick window.
/// Twenty-five leaves room for a lopsided run. If a match ends anyway,
/// `run_a_match_at` fails instead of measuring a finished match.
const STOCKS_THAT_OUTLAST_THE_WINDOW: u32 = 25;

fn run_a_match(characters: [&str; 2], ticks: usize) -> MatchReport {
    run_a_match_at(characters, ticks, &[5, 5])
}

/// The same match at a named pair of rungs. Two CPUs through the demo
/// shell, watched for `ticks` frames after the countdown.
fn run_a_match_at(characters: [&str; 2], ticks: usize, levels: &[u8]) -> MatchReport {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // Both seats are CPUs at the same rung. `smash_roster_at_levels` seats
    // every slot as a CPU; the other helper makes seat 0 a human with no
    // controller.
    let mut roster = ambition_demo_smash::smash_roster_at_levels(characters, levels);
    // The window must be a window of fighting. These tests are a patience
    // budget for a CPU to go offstage and throw its route home. A match that
    // ends inside the budget fails for want of opportunity. With D192's
    // respawn beat, this mirror decides at about half the budget at the
    // shipped stock count. So raise the stocks for the instrument only: the
    // test is about which move a CPU throws, not how long three stocks last.
    roster.rules.stocks = Some(STOCKS_THAT_OUTLAST_THE_WINDOW);
    // Seat → character from the roster handed in. That two different fighters
    // were seated is proved from the world, by the two tables differing.
    let seat_characters: BTreeMap<usize, String> = roster
        .participants
        .iter()
        .enumerate()
        .map(|(index, p)| (index, p.character.to_string()))
        .collect();
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // The warm-up must outlast the countdown: fighters are held for the whole
    // 3-2-1-GO. Read it from the ruleset.
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    let seated: Vec<(usize, Option<MovesetContract>)> = {
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, Option<&ActorMoveset>)>();
        q.iter(world)
            .map(|(seat, moveset)| (seat.0, moveset.map(|m| m.0.clone())))
            .collect()
    };
    let tables: BTreeMap<usize, MovesetContract> = seated
        .iter()
        .filter_map(|(seat, table)| table.clone().map(|t| (*seat, t)))
        .collect();
    assert_eq!(
        tables.len(),
        2,
        "the match did not put two bodies with movesets on the stage: {} seated, \
         {} of them without an `ActorMoveset`. Asked for {seat_characters:?}. \
         Seated 0 means preparation never ran; seated 2 with no moveset means the \
         bodies arrived with no repertoire, which is a product defect rather than \
         a fixture one.",
        seated.len(),
        seated.iter().filter(|(_, t)| t.is_none()).count(),
    );

    let mut ledger = MoveLedger::default();
    for _ in 0..ticks {
        app.update();
        ledger.sample(&mut app);
    }

    // Check the premise: every claim below needs a fight to throw moves in.
    // A match that ended inside the budget makes "it never threw its route"
    // true for want of opportunity.
    //
    // Count the remaining seats; do not query `FighterEliminated`.
    // `take_eliminated_fighters_out_of_play` despawns the loser, so the marker
    // query finds nothing in both cases.
    {
        let world = app.world_mut();
        let mut seats = world.query::<&MatchSeat>();
        let still_seated = seats.iter(world).count();
        assert_eq!(
            still_seated, 2,
            "only {still_seated} seat(s) were still on the stage after {ticks} \
             ticks, so the match ENDED inside the window and part of it was a \
             finished match — nothing below is a measurement of a duel. Raise \
             STOCKS_THAT_OUTLAST_THE_WINDOW"
        );
    }

    let report = MatchReport {
        ledger,
        tables,
        characters: seat_characters,
    };
    // Every run, not only a failing one.
    eprintln!(
        "[repertoire] {characters:?} over {ticks} ticks\n{}",
        report.render()
    );
    report
}

/// A mirror match: the same character in both seats.
fn watch_a_match(character: &str, ticks: usize) -> MoveLedger {
    run_a_match([character, character], ticks).ledger
}

/// A fighter with sixteen moves throws more than one of them.
///
/// The poison is the shared table: `smash_duelist_a` has eleven moves and
/// no specials. If that fighter also reached this floor, the number would
/// measure the brain's appetite for variety, not George's repertoire.
#[test]
fn the_cpu_reaches_for_more_than_one_move() {
    const WINDOW: usize = 900;
    const FLOOR: usize = 5;

    let george = watch_a_match(ambition_demo_smash::SMASH_GEORGE_BOOUL, WINDOW);
    let distinct = george.distinct_for_the_busiest_seat();
    assert!(
        distinct >= FLOOR,
        "over {WINDOW} ticks the busiest George seat started only {distinct} \
         distinct moves (floor {FLOOR}). What it actually threw: {:?}",
        george.every_move_seen()
    );
}

/// The recovery is a move the CPU throws, not only a move it owns.
///
/// Weak on when, strict on whether. Pinning the recovery to an offstage
/// position would pin the demo's tuning.
///
/// This test names a move because it is about George's own way home. The
/// generic form is `every_authored_route_gets_pressed`.
#[test]
fn the_cpu_throws_its_authored_recovery_during_a_match() {
    // 3600 ticks is a patience budget, not a claim that a CPU recovers within
    // N ticks. Measured: 2100 fails, 2400 passes. Behaviour changes elsewhere
    // (for example, shield-release lag) move when George first goes offstage.
    const WINDOW: usize = 3600;

    let ledger = watch_a_match(ambition_demo_smash::SMASH_GEORGE_BOOUL, WINDOW);
    let seen = ledger.every_move_seen();
    let ascents = seen.get("excluded_middle").copied().unwrap_or(0);
    assert!(
        ascents > 0,
        "over {WINDOW} ticks nobody threw the authored recovery. Everything the \
         two Georges did throw: {seen:?}"
    );
}

/// The repertoire causes it, not the brain's appetite for variety.
#[test]
fn a_fighter_that_authored_no_special_throws_none() {
    let ledger = watch_a_match(ambition_demo_smash::SMASH_CHARACTER_ID, 600);
    let seen = ledger.every_move_seen();
    assert!(
        !seen.contains_key("excluded_middle"),
        "the stand-in table has no specials, but a seat threw one: {seen:?}"
    );
    // It is not standing still: an empty ledger would make the assertion
    // above meaningless.
    assert!(
        !seen.is_empty(),
        "the stand-in fighters threw nothing at all, so this poison proves nothing"
    );
}

/// Two different tables on one stage play different games.
///
/// George authors sixteen moves with four specials and one commanded rise;
/// the stand-in duelist authors eleven with no special and nothing that
/// lifts.
///
/// This asserts difference, not quality: each fighter threw something the
/// other did not. The printed histogram shows the distributions.
#[test]
fn two_different_tables_produce_two_different_fights() {
    const WINDOW: usize = 1200;

    let m = run_a_match(
        [
            ambition_demo_smash::SMASH_GEORGE_BOOUL,
            ambition_demo_smash::SMASH_CHARACTER_ID,
        ],
        WINDOW,
    );
    let report = m.render();

    // Poison: an unknown character id is seated as a stand-in on the shared
    // table, and both seats would be the same fighter.
    let tables: Vec<&MovesetContract> = m.tables.values().collect();
    assert_ne!(
        tables[0], tables[1],
        "both seats are wearing the same authored table, so this is one fighter \
         twice and the comparison below is vacuous.\n{report}"
    );

    for (seat, table) in &m.tables {
        let theirs: BTreeSet<&str> = m
            .tables
            .iter()
            .filter(|(other, _)| *other != seat)
            .flat_map(|(_, t)| t.moves.iter().map(|mv| mv.id.as_str()))
            .collect();
        let started = m.started(*seat);
        // Non-vacuity first: a guard after the assertion it protects never runs.
        assert!(
            !started.is_empty(),
            "seat {seat} threw nothing at all, so it cannot be compared to \
             anything and the claim below would be about an empty set.\n{report}"
        );

        // Compare with what the opponent threw, not with its whole table. George's
        // 16 moves contain the duelist's 11, so the duelist could never throw
        // something George's table lacks. A viewer sees what was done.
        let mut theirs_thrown: BTreeSet<String> = BTreeSet::new();
        for other in m.tables.keys().filter(|other| *other != seat) {
            theirs_thrown.extend(m.started(*other).keys().cloned());
        }
        let unique: Vec<&str> = started
            .keys()
            .map(String::as_str)
            .filter(|id| !theirs_thrown.contains(*id))
            .collect();
        assert!(
            !unique.is_empty(),
            "seat {seat} ({}) threw nothing its opponent did not also throw, so \
             the two bodies are indistinguishable to a viewer.\n{report}",
            m.characters.get(seat).map_or("?", String::as_str),
        );
        // The tables must still differ in what they offer; otherwise a
        // difference in throws is chance, not character.
        assert!(
            table
                .moves
                .iter()
                .any(|mv| !theirs.contains(mv.id.as_str()))
                || theirs.iter().count() != table.moves.len(),
            "seat {seat}'s table is identical to its opponent's, so any \
             difference above is noise rather than character.\n{report}"
        );
    }
}

/// Every fighter that authors a way home presses it: the generic form of
/// `the_cpu_throws_its_authored_recovery_during_a_match`, with no move id.
///
/// The route set uses the same `lift_speed > 0` predicate as the brain's
/// `lifting_candidates`. A fighter that authors none is skipped, and a guard
/// stops the whole test from becoming a skip.
///
/// A throw is not a decision. Any rising move satisfies this, including one
/// authored as a juggle (the Pirate Admiral's `air_up`). So green means "the
/// fighter pressed something that displaces it", not "the fighter recovered
/// with its recovery". `the_decision_log` asks what the brain selected in
/// `Situation::Recovery`.
#[test]
fn every_authored_route_gets_pressed() {
    // The window is patience, not the measurement: it only needs a match that
    // puts a fighter far enough out. Measured: 2100 fails, 2400 passes.
    const WINDOW: usize = 3600;

    // A mirror match: only George authors a route home, and which fighter is
    // knocked out first is chaotic. With both seats carrying the route,
    // whoever goes offstage can be checked.
    let m = run_a_match(
        [
            ambition_demo_smash::SMASH_GEORGE_BOOUL,
            ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ],
        WINDOW,
    );
    let report = m.render();

    let mut fighters_with_a_route = 0;
    let mut fighters_offstage = 0;
    for (seat, table) in &m.tables {
        let routes = routes(table);
        if routes.is_empty() {
            continue;
        }
        fighters_with_a_route += 1;
        // The premise is per match, not per seat. Which fighter goes offstage
        // first is chaotic, so check only seats that went offstage, and require
        // at least one so the test cannot become vacuous.
        let offstage = m.ledger.offstage_ticks.get(seat).copied().unwrap_or(0);
        if offstage == 0 {
            continue;
        }
        fighters_offstage += 1;
        assert!(
            count_within(&m.started(*seat), &routes) > 0,
            "seat {seat} carries {} authored route(s) home ({routes:?}), spent \
             {offstage} ticks off the stage, and pressed none across {WINDOW} \
             ticks — the shape of a CPU that recovers on legacy drift-and-jump \
             while holding a real recovery.\n{report}",
            routes.len(),
        );
    }
    assert!(
        fighters_offstage > 0,
        "no fighter carrying a route home ever left the stage in {WINDOW} ticks, \
         so this match cannot say anything about whether one is used. That is a \
         statement about the MATCH, not about any fighter.\n{report}"
    );
    assert!(
        fighters_with_a_route > 0,
        "neither seat authors a move that commands a rise, so this test skipped \
         every fighter and asserted nothing.\n{report}"
    );
}

/// `Situation::Recovery`: which action was selected, read from the brain's
/// own decision facts.
///
/// A `MovePlayback` histogram cannot tell "pressed the juggle aerial because
/// the search came back empty" from "pressed the recovery the kernel
/// found". `fighter_decision` carries `situation`, `attack` (the selected
/// move id), `recovery_routes` (what the repertoire proposed), and
/// `recovery_move` (what the kernel endorsed).
///
/// Gated on `causal`, which is not a default feature, because recording
/// costs work per tick:
/// `cargo test -p ambition_demo_smash_app --features causal --test smash_it -- the_repertoire_gets_used --nocapture`
#[cfg(feature = "causal")]
mod the_decision_log {
    use super::*;
    use ambition_platformer2d::causal::{
        domains, CausalFact, CausalPlugin, CausalRecording, FactValue, RecordingPolicy,
    };

    fn text<'a>(fact: &'a CausalFact, key: &str) -> Option<&'a str> {
        match fact.get(key) {
            Some(FactValue::Text(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    #[test]
    fn the_recovery_decisions_name_the_action_they_selected() {
        const WINDOW: usize = 1800;

        let mut app = build_demo_app();
        // The feature compiles the publishers in; only `CausalPlugin` creates the
        // recording they write to.
        app.add_plugins(CausalPlugin);
        // Brain only: `CausalLog` holds 4096 facts and drops the oldest, so
        // recording every domain over this match would keep only its last second.
        // `dropped()` is reported below.
        ambition_platformer2d::causal::record_domains(
            &mut app,
            RecordingPolicy::only([domains::BRAIN]),
        );
        for _ in 0..30 {
            app.update();
        }
        let characters = [
            ambition_demo_smash::SMASH_GEORGE_BOOUL,
            ambition_demo_smash::SMASH_CHARACTER_ID,
        ];
        app.world_mut()
            .insert_resource(ambition_demo_smash::smash_roster_at_levels(
                characters,
                &[5, 5],
            ));
        app.world_mut()
            .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
                ambition_platformer2d::game_shell::ShellRouteId::new(
                    ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
                ),
            ));
        let countdown = ambition_demo_smash::smash_roster(characters)
            .rules
            .opening_countdown_ticks;
        for _ in 0..(countdown as usize + 30 + WINDOW) {
            app.update();
        }

        let recording = app.world().resource::<CausalRecording>();
        let dropped = recording.dropped();

        // subject → (situation, selected action) → count.
        let mut by_subject: BTreeMap<String, BTreeMap<(String, String), usize>> = BTreeMap::new();
        // What the recovery search said, separately.
        let mut recovery: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        let mut decisions = 0usize;
        for fact in recording
            .facts()
            .filter(|fact| fact.detail.kind == "fighter_decision")
        {
            decisions += 1;
            let subject = fact
                .subject
                .as_ref()
                .map_or_else(|| "unattributed".to_string(), ToString::to_string);
            let situation = text(fact, "situation").unwrap_or("?").to_string();
            let action = text(fact, "attack").unwrap_or("?").to_string();
            *by_subject
                .entry(subject.clone())
                .or_default()
                .entry((situation.clone(), action))
                .or_default() += 1;
            if situation == "Recovery" {
                // Record the proposals with the outcome: `no-route` can mean the
                // repertoire offered nothing or the kernel declined everything, and
                // only the second is a tuning question. `pressed` is what the
                // decision armed.
                let proposed = text(fact, "recovery_routes").unwrap_or("[]");
                let pressed = text(fact, "attack").unwrap_or("?");
                let outcome = match (
                    fact.get("recovery_regained"),
                    text(fact, "recovery_move"),
                    text(fact, "recovery_bounded_by"),
                ) {
                    (Some(FactValue::Bool(true)), Some("none"), _) => {
                        "home-already (pressed nothing)".to_string()
                    }
                    (Some(FactValue::Bool(true)), Some(id), _) => format!("route:{id}"),
                    (_, _, Some(bound)) => {
                        format!("no-route from {proposed} (searched {bound}) -> pressed {pressed}")
                    }
                    _ => "no-search".to_string(),
                };
                *recovery
                    .entry(subject)
                    .or_default()
                    .entry(outcome)
                    .or_default() += 1;
            }
        }

        eprintln!(
            "[fighter decisions] {decisions} facts, {dropped} dropped by the ring\n{by_subject:#?}"
        );
        eprintln!("[recovery routes]\n{recovery:#?}");

        assert!(
            decisions > 0,
            "a whole CPU match published no `fighter_decision` fact — either no \
             fighter brain is seated, or the `causal` feature stopped reaching \
             `ambition_characters`"
        );
        // The subject lets the histogram tell the two fighters apart.
        assert!(
            by_subject.len() >= 2,
            "both seats are CPUs and the decisions came back under {} subject(s), \
             so the histogram cannot tell the two fighters apart: {by_subject:#?}",
            by_subject.len()
        );
        // Some tick was a recovery decision. None means the fighters never left
        // the stage, so every recovery claim here is untested.
        assert!(
            !recovery.is_empty(),
            "no fighter was ever classified `Situation::Recovery` in {WINDOW} \
             ticks, so nothing here measured a recovery decision at all"
        );
    }
}

/// The defensive vocabulary is something the CPU uses, not only owns.
///
/// A smash's charge multiplier is paid against how long Attack stays down,
/// and a tech is armed by the evade press while tumbling. Both are read from
/// the body, not from the frame the brain emits.
///
/// It runs several noise streams and asks for one. The measured per-stream
/// charge rate is about 0.3, so `STREAMS = 10` decides the test about 97
/// times in 100. If charges become rarer, re-measure the rate before raising
/// `STREAMS`: a larger sample can hide a falling rate.
///
/// Strict on whether, weak on when: pinning a charge percentage or a tech
/// position would pin demo tuning.
#[test]
fn the_cpu_charges_a_smash_and_techs_a_landing_in_some_match() {
    // Ninety seconds: a charge needs an opening (the brain holds fully only
    // when the opponent is committed or offstage). At 1800 ticks the measured
    // best charge was 0.00 in all streams tried.
    const WINDOW: usize = 5400;
    const STREAMS: u64 = 10;

    let mut charged_in = 0usize;
    let mut teched_in = 0usize;
    let mut tumbled_in = 0usize;
    let mut best_charge = 0.0f32;
    for stream in 0..STREAMS {
        let (charge, techs, tumbles) = watch_the_vocabulary(WINDOW, 0x5F37_7A11 * (stream + 1));
        best_charge = best_charge.max(charge);
        if charge > 0.0 {
            charged_in += 1;
        }
        if techs > 0 {
            teched_in += 1;
        }
        if tumbles > 0 {
            tumbled_in += 1;
        }
    }

    assert!(
        charged_in > 0,
        "no CPU held a smash in any of {STREAMS} matches of {WINDOW} ticks — the \
         charge multiplier is authored on every fighter and nobody paid for any \
         of it (best fraction seen {best_charge:.2}). This sample decides a \
         per-stream rate of 0.3 about 97 times in 100, so zero here is a \
         finding rather than a draw."
    );
    // Non-vacuity for the tech half: with no tumble there is no landing to
    // tech.
    assert!(
        tumbled_in > 0,
        "nobody tumbled in any of {STREAMS} matches, so this cannot say anything \
         about teching"
    );
    assert!(
        teched_in > 0,
        "bodies tumbled in {tumbled_in} of {STREAMS} matches and no CPU ever \
         armed a tech"
    );
}

/// One match under one execution-noise stream: the best charge fraction any seat
/// reached, ticks with a tech armed, and ticks spent tumbling.
fn watch_the_vocabulary(window: usize, noise_seed: u64) -> (f32, usize, usize) {
    use ambition_platformer2d::actor::MotionModel;
    use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    let characters = [
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
        ambition_demo_smash::SMASH_GEORGE_BOOUL,
    ];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[5, 5]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    // Supply the stream here: a live fighter's is `participant ⊕ level`, and
    // this sweeps it (as `bin/ladder_probe` does).
    {
        let world = app.world_mut();
        let mut q = world.query::<&mut Brain>();
        for (index, mut brain) in q.iter_mut(world).enumerate() {
            if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
                state.noise = noise_seed.wrapping_mul(index as u64 + 1).wrapping_add(1);
            }
        }
    }

    let mut best_charge = 0.0f32;
    let mut techs = 0usize;
    let mut tumbles = 0usize;
    for _ in 0..window {
        app.update();
        let world = app.world_mut();
        let mut playbacks = world.query::<(&MatchSeat, &MovePlayback)>();
        let seen: Vec<f32> = playbacks
            .iter(world)
            .filter_map(|(_, pb)| pb.smash_charge_fraction())
            .collect();
        for fraction in seen {
            best_charge = best_charge.max(fraction);
        }
        let mut motion = world.query::<(&MatchSeat, &MotionModel)>();
        techs += motion
            .iter(world)
            .filter(|(_, model)| {
                matches!(
                    model,
                    ambition_platformer2d::engine_core::MotionModel::AxisSwept(axis)
                        if axis.state.tech_press_timer > 0.0
                )
            })
            .count();
        let mut facts = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::engine_core::BodyMotionFacts,
        )>();
        tumbles += facts.iter(world).filter(|(_, f)| f.tumbling).count();
    }
    (best_charge, techs, tumbles)
}

/// The jab string and the rapid jab, driven by a real button in the real
/// game.
///
/// Not a unit test on a timeline: several mechanics passed their unit tests
/// and did not work in the game. This presses a button on a pad, seats a
/// human on the shipped roster, and reads what the body played.
///
/// Holding Attack must walk `jab` → `jab2` → `jab3`, and `jab3` must loop:
/// the rapid jab is an authored `MoveLoop`.
#[test]
fn holding_attack_walks_the_jab_string_into_the_rapid_jab() {
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let character = ambition_demo_smash::SMASH_GEORGE_BOOUL;
    let mut app = build_demo_app();
    // Under the `input` feature, `populate_seat_control_frames` derives each
    // seat's raw frame from the device layer every frame, so a row written
    // into `SeatRawFrames` is overwritten (`nextest --workspace` turns
    // `visible` → `input` on). The roster seats its human on pad 0, so spawn
    // a real `Gamepad` before seating and hold its West button (Attack in the
    // default preset) through raw gamepad events. The headless build has no
    // device layer and keeps the raw row. Both roads give the same press.
    let pad = app
        .world_mut()
        .spawn(bevy::input::gamepad::Gamepad::default())
        .id();
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([character, character]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    let countdown = ambition_demo_smash::smash_roster([character, character])
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    // The human's body: the one seat the roster binds to a pad.
    let human = {
        let world = app.world_mut();
        let mut q = world.query::<(
            Entity,
            &ambition_platformer2d::characters::control::DrivingParticipant,
        )>();
        q.iter(world)
            .min_by_key(|(_, driver)| driver.0 .0)
            .map(|(entity, _)| entity)
            .expect("the shipped roster seats a human on the first pad")
    };

    // Attack down and never released. `SeatRawFrames` is the pre-latch table
    // one hop below `SlotControls`, so the press still crosses seat latching,
    // `ActorControl`, the gesture resolver, and the buffer. `versus_stage`
    // guards the pad-to-seat hop.
    let mut played: Vec<String> = Vec::new();
    let mut laps = 0.0f32;
    // Press only when standing: the human is still falling from spawn, and
    // `jab` is the grounded verb. A fixed tick would resolve `air_neutral`
    // whenever the warm-up's sim time changes.
    let mut pressed_yet = false;
    for _ in 0..240 {
        let standing = app
            .world()
            .get::<ambition_platformer2d::engine_core::BodyGroundState>(human)
            .is_some_and(|g| g.on_ground);
        {
            use ambition_platformer2d::characters::control::{PlayerSlot, SeatRawFrames};
            let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
            let mut frame = raw.get(PlayerSlot(0));
            frame.attack_pressed = standing && !pressed_yet;
            frame.attack_held = standing || pressed_yet;
            raw.set(PlayerSlot(0), frame);
        }
        if standing && !pressed_yet {
            app.world_mut()
                .write_message(bevy::input::gamepad::RawGamepadEvent::Button(
                    bevy::input::gamepad::RawGamepadButtonChangedEvent::new(
                        pad,
                        bevy::input::gamepad::GamepadButton::West,
                        1.0,
                    ),
                ));
        }
        pressed_yet |= standing;
        app.update();
        if let Some(pb) = app.world().get::<MovePlayback>(human) {
            let id = pb.spec.id.clone();
            if played.last() != Some(&id) {
                played.push(id);
            }
            laps = laps.max(pb.looped_s);
        }
    }

    for step in ["jab", "jab2", "jab3"] {
        assert!(
            played.iter().any(|id| id == step),
            "holding Attack never reached `{step}`. What the human actually \
             played: {played:?}. The chain reads an UNDIRECTED follow-up \
             inside a cancel window; if `jab` is missing the press never \
             resolved a move, and if only `jab` is there the follow-up never \
             reached the successor its window names."
        );
    }
    // A hold must not buy a route. George's jab window also names `smash` and
    // `special`, which are verb names, not move ids. A hold may only take a
    // successor named by move id, so the route still needs a directed press.
    assert!(
        !played.iter().any(|id| id.starts_with("smash")),
        "a HELD button bought George's smash route: {played:?}"
    );
    eprintln!("[jab string] a held Attack played {played:?}, {laps:.3}s of looped time");
    assert!(
        laps > 0.0,
        "the string reached `jab3` and the rapid jab never took a second lap \
         (looped_s stayed {laps}). The flurry is an authored `MoveLoop` that \
         repeats while the button is down — zero laps means the loop is a \
         decoration on a move that plays once."
    );
}

/// Probe: does a higher rung reach for more of its kit? Print-only; run with
/// `--ignored`.
///
/// D244's definition-of-done instrument. The decision rig
/// (`brain::fighter::evaluation`) never steps a world. This one does: same
/// match, same character, one rung moved, counting the distinct moves a
/// seat started. Only the registered rungs (1, 3, 5, 6, 9) are valid.
///
/// Keep the window at 2700. At 900 the result reverses (same seed):
///
/// ```text
///          900 ticks        2700 ticks
/// L1        9 distinct      11 distinct
/// L9        6 distinct      17 distinct   ← including grabs, pummels, throws
/// ```
///
/// A fast rung needs longer to show its kit, because it spends more of a
/// short window committed. Do not shorten the budget.
///
/// One character and one run: read the shape, not the steps. This harness
/// cannot seat a second character (`build_demo_app` composes only the
/// demo's cast); sweeping characters needs `game/ambition_app`'s harness.
#[test]
#[ignore = "PROBE, print-only: distinct moves started per ladder rung"]
fn probe_repertoire_by_rung() {
    const WINDOW: usize = 2700;
    for level in [1u8, 3, 5, 6, 9] {
        let ledger = run_a_match_at(
            [
                ambition_demo_smash::SMASH_GEORGE_BOOUL,
                ambition_demo_smash::SMASH_GEORGE_BOOUL,
            ],
            WINDOW,
            &[level, level],
        )
        .ledger;
        let seen = ledger.every_move_seen();
        println!(
            "L{level}: busiest seat {} distinct, {} across both seats — {:?}",
            ledger.distinct_for_the_busiest_seat(),
            seen.len(),
            seen.keys().collect::<Vec<_>>()
        );
    }
}
