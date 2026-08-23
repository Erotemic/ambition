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
    /// Every move START observed, per seat.
    started: BTreeMap<usize, BTreeMap<String, usize>>,
    /// Seat → ticks spent OUTSIDE the stage's own footprint.
    ///
    /// ⛔⛔ THE PREMISE OF EVERY "DID IT USE ITS RECOVERY" QUESTION, and it was
    /// missing. A route home is only wanted by a fighter that is off the stage,
    /// so a match that never puts one there cannot observe the affordance —
    /// and the guard below spent two behaviour changes accusing the CPU of
    /// recovering on legacy drift when the match had simply never launched it.
    offstage_ticks: BTreeMap<usize, usize>,
    /// The last `(move id, clock)` seen per entity, so a move that is still
    /// running is not counted again every tick.
    live: BTreeMap<Entity, (String, f32)>,
}

impl MoveLedger {
    fn sample(&mut self, app: &mut App) {
        let stage = ambition_demo_smash::smash_stage().world.size;
        let world = app.world_mut();
        let mut positions = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::actor::BodyKinematics,
        )>();
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
    /// Seat → the table preparation actually gave that body. Read from the
    /// WORLD, because what a seat WEARS is the only version that can disagree
    /// with what was authored.
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
            // ⭐ PRINTED ON SUCCESS TOO. How long a seat spent off the stage is
            // the premise of every route/recovery claim here, and a number only
            // visible on failure cannot tell a reader whether a GREEN run
            // observed the affordance or merely never asked.
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

/// The one seating path in this file. Two CPUs at the same rung, through the
/// demo shell, watched for `ticks` frames after the countdown.
fn run_a_match(characters: [&str; 2], ticks: usize) -> MatchReport {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // both seats CPU, same rung. `smash_roster_at_levels` is the helper
    // that seats every slot as a CPU; the sibling test in `the_stage_kills` has
    // the scar from using the one that makes seat 0 a human with no controller,
    // which measures one fighter pacing around a statue.
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[5, 5]);
    // Seat → character taken from the roster handed IN. What this cannot prove —
    // that two different fighters were seated — is proved from the world instead,
    // by the two tables differing.
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
    // the warm-up has to outlast the countdown. The stage opens suspended
    // and every fighter carries scripted control for the whole 3-2-1-GO, so a
    // window inside the hold measures fighters that are correctly forbidden to
    // act. Read from the ruleset rather than restating it.
    let countdown = ambition_demo_smash::smash_roster(characters).opening_countdown_ticks;
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
    let report = MatchReport {
        ledger,
        tables,
        characters: seat_characters,
    };
    // every run, not only a failing one. See the module header.
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

/// A FIGHTER WITH SIXTEEN MOVES THROWS MORE THAN ONE OF THEM.
///
/// and the poison is the shared table. `smash_duelist_a` carries eleven moves with no
/// specials at all; if THAT fighter also reached this floor, the number would be measuring the
/// brain's appetite for variety rather than George's repertoire.
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

/// THE RECOVERY IS A MOVE THE CPU THROWS, NOT A MOVE IT OWNS.
///
/// this is the one that could pass vacuously in the most expensive way. Authoring the move fixes
/// nothing on its own; the brain has to offer it.
///
/// the measurement is deliberately weak on WHEN and strict on WHETHER.
/// Pinning the recovery to a particular offstage position would be pinning the
/// tuning of a demo. What must never be true is that the move is never thrown in
/// a match where fighters are being launched off a stage.
///
/// this is the one test that names a move, because it is a claim about
/// GEORGE's own way home rather than about the engine's route affordance. The
/// generic version of the claim is `every_authored_route_gets_pressed` below.
#[test]
fn the_cpu_throws_its_authored_recovery_during_a_match() {
    const WINDOW: usize = 1800;

    let ledger = watch_a_match(ambition_demo_smash::SMASH_GEORGE_BOOUL, WINDOW);
    let seen = ledger.every_move_seen();
    let ascents = seen.get("excluded_middle").copied().unwrap_or(0);
    assert!(
        ascents > 0,
        "over {WINDOW} ticks nobody threw the authored recovery. Everything the \
         two Georges did throw: {seen:?}"
    );
}

/// AND IT IS THE REPERTOIRE DOING IT, not the brain's appetite for variety.
#[test]
fn a_fighter_that_authored_no_special_throws_none() {
    let ledger = watch_a_match(ambition_demo_smash::SMASH_CHARACTER_ID, 600);
    let seen = ledger.every_move_seen();
    assert!(
        !seen.contains_key("excluded_middle"),
        "the stand-in table has no specials, but a seat threw one: {seen:?}"
    );
    // It is not standing still either — the contrast is about the REPERTOIRE,
    // and an empty ledger would make the assertion above meaningless.
    assert!(
        !seen.is_empty(),
        "the stand-in fighters threw nothing at all, so this poison proves nothing"
    );
}

/// TWO DIFFERENT TABLES, ON ONE STAGE, PLAYING DIFFERENT GAMES.
///
/// That answers *"is the repertoire exercised"* and cannot answer the question a viewer actually
/// asks, which is whether the two bodies on screen are doing recognisably different things.
///
/// George authors sixteen moves with four specials and one commanded rise; the
/// stand-in duelist authors eleven with no special and nothing that lifts. So
/// the contrast is real content rather than two names.
///
/// what this asserts is DIFFERENCE, not quality. It says each fighter
/// threw something the other's table does not even contain — so the difference a
/// viewer sees is repertoire and not labelling. It says nothing about the SHAPE
/// of either distribution, which the printed histogram is for.
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

    // THE POISON. A character id the composition does not carry is seated
    // as a stand-in wearing the shared table, and both seats would then be the
    // same fighter twice while every assertion below still passed.
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
        // NON-VACUITY FIRST, because a guard placed AFTER the assertion it
        // protects can never run. This one was written correctly and sat
        // below, so when the claim failed for exactly the reason the guard
        // names, the claim's message got the blame.
        assert!(
            !started.is_empty(),
            "seat {seat} threw nothing at all, so it cannot be compared to \
             anything and the claim below would be about an empty set.\n{report}"
        );

        // compared against what the OPPONENT THREW, not what it COULD
        // throw. Measuring a seat's throws against the other seat's whole
        // TABLE is unpassable whenever one table contains the other — George
        // authors 16 moves and the duelist 11, so the duelist could never throw
        // anything "George's table lacks" no matter how differently it played,
        // and the test would have reported two indistinguishable fighters while
        // they were plainly fighting differently.
        //
        // a viewer sees what was DONE. Containment of authored tables is a
        // fact about content; it is not the claim.
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
        // The tables must still differ in what they OFFER, or a difference in
        // what was thrown is a coin flip rather than a character.
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

/// EVERY FIGHTER THAT AUTHORS A WAY HOME PRESSES IT — the generic form of
/// `the_cpu_throws_its_authored_recovery_during_a_match`, with no move id in it.
///
/// the route set is derived by the same `lift_speed > 0` predicate the brain's
/// `lifting_candidates` proposes from, so this measures the affordance rather
/// than George. A fighter that authors none is skipped, and the guard below
/// refuses to let the whole test become a skip.
///
/// AND HERE IS WHAT IT CANNOT SEE, stated rather than implied. A throw is
/// not a decision. Any move commanding a rise satisfies this, including one
/// authored as a JUGGLE rather than a way home — the Pirate Admiral's `air_up`
/// is exactly that, deliberately. So a green here means *"the fighter pressed
/// something that displaces it"*, NOT *"the fighter recovered with its
/// recovery"*. Only `the_decision_log` below asks the brain what it SELECTED in
/// `Situation::Recovery`. Do not promote this to the stronger claim.
#[test]
fn every_authored_route_gets_pressed() {
    // THE WINDOW IS PATIENCE, NOT THE MEASUREMENT, and it moved on 2026-08-22.
    //
    // This probe watches a whole MATCH and asks whether a route ever got
    // pressed, so what it really measures is when that match happens to put a
    // fighter far enough out to want one. Fixing the airborne-grab gate changed
    // how the two CPUs trade, George's first trip offstage landed later, and at
    // 1800 the affordance had not been exercised yet. Nothing about his
    // recovery changed: `the_cpu_throws_its_authored_recovery_during_a_match`
    // above asks the brain what it SELECTED in `Situation::Recovery` and is
    // green, which is the strong claim this test explicitly does not make.
    //
    // Measured rather than guessed: 2100 fails, 2400 passes. Sitting on the
    // threshold means the next behaviour change flips it again, so this is
    // double the original -- headroom for a probe whose cost is one more
    // simulated match, not a promise that a CPU recovers within N ticks.
    const WINDOW: usize = 3600;

    let m = run_a_match(
        [
            ambition_demo_smash::SMASH_GEORGE_BOOUL,
            ambition_demo_smash::SMASH_CHARACTER_ID,
        ],
        WINDOW,
    );
    let report = m.render();

    let mut fighters_with_a_route = 0;
    for (seat, table) in &m.tables {
        let routes = routes(table);
        if routes.is_empty() {
            continue;
        }
        fighters_with_a_route += 1;
        // ⛔⛔ THE PREMISE FIRST, AND IT IS NOT A FORMALITY. This probe watches
        // one match and asks whether a route was ever pressed, so it can only
        // see the affordance in a match that put the fighter off the stage. It
        // has now gone red twice for behaviour changes that moved WHEN George
        // first goes out — a charge-payoff correction and a charge-pose move —
        // while `the_cpu_throws_its_authored_recovery_during_a_match`, which
        // asks the brain what it SELECTED in `Situation::Recovery`, stayed
        // green both times. Widening the window each time is a hand-kept
        // ledger; naming the premise is the fix.
        let offstage = m.ledger.offstage_ticks.get(seat).copied().unwrap_or(0);
        assert!(
            offstage > 0,
            "seat {seat} never left the stage in {WINDOW} ticks, so this match \
             cannot say anything about whether it uses its route home. That is \
             a statement about the MATCH, not about the fighter — the claim \
             about the brain's selection is \
             `the_cpu_throws_its_authored_recovery_during_a_match`.\n{report}"
        );
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
        fighters_with_a_route > 0,
        "neither seat authors a move that commands a rise, so this test skipped \
         every fighter and asserted nothing.\n{report}"
    );
}

/// `Situation::Recovery` → WHICH ACTION WAS SELECTED, asked of the brain's
/// own decision facts rather than inferred from what the body did.
///
/// Everything above sees a move being THROWN. None of it can see the decision
/// that chose one, nor the ticks where the recovery search ran and endorsed
/// nothing — and "pressed the juggle aerial because the search came back empty"
/// and "pressed the recovery because the kernel found it" look identical in a
/// `MovePlayback` histogram. `fighter_decision` carries `situation`, `attack`
/// (the selected move id), `recovery_routes` (what the repertoire proposed) and
/// `recovery_move` (what the kernel endorsed), so the histogram is a group-by
/// rather than a reconstruction.
///
/// gated on `causal`, which is NOT a default feature — recording costs
/// work per tick and a shipped demo must not pay it:
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
        // the FEATURE and the PLUGIN are two switches, deliberately. The
        // feature compiles the publishers in; only `CausalPlugin` creates the
        // recording they write to.
        app.add_plugins(CausalPlugin);
        // BRAIN only, and the ring is why. `CausalLog` holds 4096 facts and
        // drops the oldest, so `RecordingPolicy::All` over a thirty-second match
        // would leave the histogram silently describing its last second.
        // `dropped()` is reported below either way.
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
        let countdown = ambition_demo_smash::smash_roster(characters).opening_countdown_ticks;
        for _ in 0..(countdown as usize + 30 + WINDOW) {
            app.update();
        }

        let recording = app.world().resource::<CausalRecording>();
        let dropped = recording.dropped();

        // subject → (situation, selected action) → count.
        let mut by_subject: BTreeMap<String, BTreeMap<(String, String), usize>> = BTreeMap::new();
        // and what the recovery SEARCH said, separately.
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
                // the PROPOSALS ride along with the outcome, because
                // `no-route` means two different things — the repertoire offered
                // nothing, or the kernel declined everything it was offered — and
                // only the second is a tuning question. `pressed` is what the
                // decision actually armed.
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
        // the SUBJECT is what makes a two-fighter histogram readable at all; an
        // unattributed stream is one pile.
        assert!(
            by_subject.len() >= 2,
            "both seats are CPUs and the decisions came back under {} subject(s), \
             so the histogram cannot tell the two fighters apart: {by_subject:#?}",
            by_subject.len()
        );
        // the headline: some tick of this match was a recovery decision, and it
        // named what it selected. A run with none means the fighters never left
        // the stage, which makes every recovery claim in this file UNTESTED rather
        // than passing.
        assert!(
            !recovery.is_empty(),
            "no fighter was ever classified `Situation::Recovery` in {WINDOW} \
             ticks, so nothing here measured a recovery decision at all"
        );
    }
}

/// THE DEFENSIVE VOCABULARY IS SOMETHING THE CPU USES, NOT SOMETHING IT OWNS.
///
/// Two mechanics shipped tuned, reachable and unused, because the fighter brain
/// had no verb that reached either: a smash's charge multiplier is paid out
/// against how long Attack stays down, and a tech is armed by the evade press
/// while tumbling. Both are read off the BODY here rather than off the frame the
/// brain emits — a brain asking is not the claim; a body receiving is.
///
/// ⭐ IT RUNS THREE NOISE STREAMS AND ASKS FOR ONE, and that is not laxity — it
/// is what the spread actually looks like. Measured over three runs of
/// `bin/match_report`: damage 0–86–217, tumbling 0–97–121, techs 0–29–54, best
/// charge 0.00–0.00–0.99. One run in three is a fight where almost nothing
/// happens, so a single fixed match asserting "a charge occurred" is a coin
/// flip dressed as a regression test — it passed for a week and then failed on a
/// change that made the fight BETTER.
///
/// The measurement stays strict on WHETHER and deliberately weak on WHEN.
/// Pinning a charge to a percentage or a tech to a position would be pinning
/// demo tuning. What must never be true again is that no stream produces either.
#[test]
fn the_cpu_charges_a_smash_and_techs_a_landing_in_some_match() {
    // NINETY SECONDS, not thirty, and the reason is the event rather than the
    // patience. A charge needs an OPENING — the brain pays a full hold only when
    // the opponent is committed or offstage — and openings arrive on their own
    // schedule. Measured 2026-08-23 across three streams: at 1800 ticks the best
    // charge reached is 0.00 in all three; at 5400 it is 0.99 in two of them.
    // The old window was hunting an event rarer than itself.
    const WINDOW: usize = 5400;
    const STREAMS: u64 = 3;

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
         of it (best fraction seen {best_charge:.2})"
    );
    // THE NON-VACUITY GUARD for the tech half. A run of matches in which nobody
    // is ever launched into a tumble has no landing to tech.
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
    use ambition_platformer2d::actors::features::MotionModel;
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
    let countdown = ambition_demo_smash::smash_roster(characters).opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }
    // The stream is SUPPLIED here rather than modelled: a live fighter's is
    // `participant ⊕ level`, and sweeping it is the whole point — the same
    // reason `bin/ladder_probe` documents for doing it this way.
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

/// ⭐ THE JAB STRING AND THE RAPID JAB, DRIVEN BY A REAL BUTTON, IN THE REAL
/// GAME.
///
/// ⛔ NOT A UNIT TEST ON A TIMELINE, and the difference is the whole point of
/// this file. This run has shipped four mechanics that were green and dead on
/// arrival — a smash charge whose hold could not outlast its own startup, a tech
/// gate that stripped the button for all of hitstun, a launch-trail threshold
/// above the launch speeds that exist, and a post-hit window that refused the
/// second Active window of a multi-window move. Every one of them passed its own
/// unit test. So this one presses a physical button on a physical pad, seats a
/// human on the shipped roster, and reads what the body actually played.
///
/// Holding Attack must walk the whole route: `jab` into `jab2` into `jab3`, and
/// `jab3` must LOOP — the rapid jab is an authored `MoveLoop`, and a flurry that
/// never took a second lap is a third jab with extra vocabulary.
#[test]
fn holding_attack_walks_the_jab_string_into_the_rapid_jab() {
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let character = ambition_demo_smash::SMASH_GEORGE_BOOUL;
    let mut app = build_demo_app();
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
    let countdown =
        ambition_demo_smash::smash_roster([character, character]).opening_countdown_ticks;
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

    // Attack down, and never released, written at the DEVICE seam.
    // `SeatRawFrames` is the pre-latch table a controller publishes into, one hop
    // below `SlotControls` — so the press still crosses seat latching,
    // `ActorControl`, the gesture resolver and the buffer before anything this
    // slice touched sees it. ⛔ the headless demo composes `MinimalPlugins` and
    // has no gamepad plugin at all, so a raw pad event has nothing to read it;
    // `versus_stage` is where the pad-to-seat hop is guarded.
    let mut played: Vec<String> = Vec::new();
    let mut laps = 0.0f32;
    for tick in 0..240 {
        {
            use ambition_platformer2d::characters::control::{PlayerSlot, SeatRawFrames};
            let mut raw = app.world_mut().resource_mut::<SeatRawFrames>();
            let mut frame = raw.get(PlayerSlot(0));
            frame.attack_pressed = tick == 0;
            frame.attack_held = true;
            raw.set(PlayerSlot(0), frame);
        }
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
    // ⛔ AND IT MUST NOT HAVE BOUGHT A ROUTE. George's jab window also names
    // `smash` and `special` — his one way from his fast half to his slow half —
    // and those are VERB names, not move ids. A hold may only take a successor
    // the window names BY MOVE ID, so the string is reachable by holding and the
    // route still costs a deliberate directed press. Without that rule a held
    // button would throw a fully-charged smash out of a jab.
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
