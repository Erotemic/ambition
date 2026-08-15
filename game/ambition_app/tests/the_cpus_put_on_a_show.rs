#![cfg(feature = "input")]
//! **TWO DIFFERENT FIGHTERS, DRIVEN BY CPUs, IN THE REAL HOST — what do they
//! actually DO?**
//!
//! The claim this exists to test is not *"the moves exist"* and not *"a special
//! occurred at some point"*. It is the one somebody makes while WATCHING:
//!
//! > this engine elegantly supports serious platform-fighter combat.
//!
//! A person watching believes that when the two bodies on screen are playing
//! DIFFERENT games — different pokes, different aerials, different specials, and
//! visibly different answers to being knocked offstage. So this is an INSTRUMENT
//! first and an assertion second: it seats George Booul against the Pirate
//! Admiral, watches a real match through the ordinary press seam, and prints the
//! per-seat histogram of what each body threw. A regression tells you what the
//! CPUs did instead of arguing about what they could have done.
//!
//! ## Why it lives HERE and not in `ambition_demo_smash_app`
//!
//! ⛔ **the demo crate cannot see the Admiral.** George is authored in
//! `ambition_demo_smash`; the Pirate Admiral is authored in `ambition_content`,
//! Ambition's own cast. Only `ambition_app` composes both, so the CROSSOVER —
//! two fighters with independently authored repertoires on one stage — is
//! measurable only here. `the_repertoire_gets_used.rs` over in the demo crate
//! measures George against George; this measures George against somebody else,
//! which is the question "are they behaving DIFFERENTLY" needs.
//!
//! ## Nothing here names a move
//!
//! ⭐ every classification below is read off the body's OWN `ActorMoveset` — a
//! special is a move some `special*` verb reaches, an aerial is a move gated
//! airborne-only, a ROUTE is a move whose authored frame data commands an
//! against-gravity displacement. Add a fighter, author a table, and this
//! measures it without an edit. A test with a list of George's move ids in it
//! would pass forever while a second fighter threw one jab.
//!
//! ⚠ **the thresholds are floors, not targets.** ⛔ do not tune content against
//! them.

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d::actor::{MatchParticipantRoster, MatchSeat};
use ambition_platformer2d::combat::moveset::{ActorMoveset, MovePlayback};
use ambition_platformer2d::entity_catalog::MovesetContract;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

use crate::smash_in_the_host::{active_route, launch_row, shell_host_app};

/// The Admiral's catalog id. ⚠ a literal, and the assertion that it resolved is
/// three lines below — an id nothing carries would otherwise seat a stand-in and
/// this whole file would measure a robot.
const PIRATE_ADMIRAL: &str = "npc_pirate_admiral";

/// Ticks watched after the countdown. Thirty seconds at the pinned 60 Hz — long
/// enough that bodies are launched off the stage and have to come back, which is
/// the half of the fight the offstage questions are about.
const WINDOW: usize = 1800;

/// **What one seat threw, and who it was.**
struct SeatLedger {
    character: String,
    /// Move id → how many times it STARTED.
    started: BTreeMap<String, usize>,
    /// The body's own authored table, so every classification below comes from
    /// the fighter rather than from this file.
    moveset: MovesetContract,
}

impl SeatLedger {
    /// Move ids some `special*` verb reaches.
    fn specials(&self) -> BTreeSet<&str> {
        self.moveset
            .verbs
            .iter()
            .filter(|(verb, _)| verb.starts_with("special"))
            .map(|(_, id)| id.as_str())
            .collect()
    }

    /// Move ids gated airborne-only — the aerials, as the table declares them.
    fn aerials(&self) -> BTreeSet<&str> {
        self.moveset
            .moves
            .iter()
            .filter(|m| m.gates.grounded == Some(false))
            .map(|m| m.id.as_str())
            .collect()
    }

    /// **Move ids whose authored frame data commands a rise** — the routes home,
    /// recognised by geometry exactly the way `lifting_candidates` recognises
    /// them for the brain. This is the set the recovery search is allowed to
    /// propose from, so "did the fighter use its authored recovery" is asked of
    /// the same derivation the brain asks.
    fn routes(&self) -> BTreeSet<&str> {
        self.moveset
            .moves
            .iter()
            .filter(|m| m.frame_data().lift_speed > 0.0)
            .map(|m| m.id.as_str())
            .collect()
    }

    fn count_within(&self, ids: &BTreeSet<&str>) -> usize {
        self.started
            .iter()
            .filter(|(id, _)| ids.contains(id.as_str()))
            .map(|(_, n)| *n)
            .sum()
    }

    fn distinct(&self) -> usize {
        self.started.len()
    }

    fn report(&self) -> String {
        format!(
            "  seat wearing {:<24} distinct={} specials={} aerials={} routes={} threw={:?}\n    \
             (authored: {} specials, {} aerials, {} routes)",
            self.character,
            self.distinct(),
            self.count_within(&self.specials()),
            self.count_within(&self.aerials()),
            self.count_within(&self.routes()),
            self.started,
            self.specials().len(),
            self.aerials().len(),
            self.routes().len(),
        )
    }
}

/// Watch a match and count every move START, per seat.
///
/// A START is an entity whose playback names a different move than last tick, or
/// the same move with a clock that went backwards (the press landed again). The
/// same reading `the_repertoire_gets_used` takes, for the same reason: nothing
/// here can observe a move the body did not actually perform.
struct Watcher {
    started: BTreeMap<usize, BTreeMap<String, usize>>,
    live: BTreeMap<Entity, (String, f32)>,
}

impl Watcher {
    fn new() -> Self {
        Self {
            started: BTreeMap::new(),
            live: BTreeMap::new(),
        }
    }

    fn sample(&mut self, app: &mut App) {
        let world = app.world_mut();
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
}

/// Seat these two characters as CPUs at the same rung in the REAL host, run the
/// match, and report per seat.
fn watch_a_crossover(characters: [&str; 2], level: u8, ticks: usize) -> Vec<SeatLedger> {
    let mut app = shell_host_app();
    for _ in 0..6 {
        app.update();
    }

    // ⚠ **enter the experience the way a player does.** Writing the roster and
    // jumping straight to the gameplay route skips whatever the provider sets up
    // on the way in, and a fixture that skips the entry is measuring a stage
    // nobody reaches.
    launch_row(&mut app, "Smash");
    for _ in 0..20 {
        app.update();
    }

    // ⚠ **both seats CPU, same rung.** `smash_roster_at_levels` is the helper
    // that seats every slot as a CPU; the one that makes seat 0 a human leaves a
    // statue for the other fighter to pace around.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            characters,
            &[level, level],
        ));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));
    for _ in 0..120 {
        app.update();
        if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
            break;
        }
    }
    assert_eq!(
        active_route(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the match never reached the stage, so nothing below measures a fight"
    );

    // ⛔ **the warm-up has to outlast the countdown.** The stage opens suspended
    // and every fighter carries scripted control for the whole 3-2-1-GO, so a
    // window inside the hold measures fighters that are correctly forbidden to
    // act. Read it from the ruleset rather than restating it.
    let countdown = ambition_demo_smash::smash_roster(characters).opening_countdown_ticks;
    for _ in 0..(countdown + 60) {
        app.update();
    }

    // The bodies, and the tables they are wearing. Read from the WORLD: a seat's
    // repertoire is whatever preparation actually gave it, which is the only
    // version that can disagree with the authoring.
    let seat_tables: BTreeMap<usize, MovesetContract> = {
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, &ActorMoveset)>();
        q.iter(world)
            .map(|(seat, moveset)| (seat.0, moveset.0.clone()))
            .collect()
    };
    let seat_characters: BTreeMap<usize, String> = app
        .world()
        .resource::<MatchParticipantRoster>()
        .participants
        .iter()
        .enumerate()
        .map(|(index, p)| (index, p.character.to_string()))
        .collect();
    assert_eq!(
        seat_tables.len(),
        2,
        "expected two seated bodies carrying movesets, found {}: {:?}",
        seat_tables.len(),
        seat_characters
    );

    let mut watcher = Watcher::new();
    for _ in 0..ticks {
        app.update();
        watcher.sample(&mut app);
    }

    seat_tables
        .into_iter()
        .map(|(seat, moveset)| SeatLedger {
            character: seat_characters
                .get(&seat)
                .cloned()
                .unwrap_or_else(|| format!("seat {seat}")),
            started: watcher.started.get(&seat).cloned().unwrap_or_default(),
            moveset,
        })
        .collect()
}

fn crossover() -> Vec<SeatLedger> {
    watch_a_crossover(
        [ambition_demo_smash::SMASH_GEORGE_BOOUL, PIRATE_ADMIRAL],
        7,
        WINDOW,
    )
}

fn render(seats: &[SeatLedger]) -> String {
    seats
        .iter()
        .map(SeatLedger::report)
        .collect::<Vec<_>>()
        .join("\n")
}

/// **THE SHOWCASE MEASUREMENT.** One match, everything it produced, printed.
///
/// The assertions are the four claims a viewer would make, in the order they
/// would notice them failing. They are floors; the print is the product.
#[test]
fn a_cpu_match_between_two_authored_fighters_shows_two_repertoires() {
    const DISTINCT_FLOOR: usize = 5;

    let seats = crossover();
    let report = render(&seats);
    eprintln!("[smash crossover, {WINDOW} ticks]\n{report}");

    // 0. Both fighters are the ones asked for. An id nothing carries is seated as
    //    something else and every number below would be about a stand-in.
    let wearing: Vec<&str> = seats.iter().map(|s| s.character.as_str()).collect();
    assert!(
        wearing.contains(&ambition_demo_smash::SMASH_GEORGE_BOOUL)
            && wearing.contains(&PIRATE_ADMIRAL),
        "the stage seated {wearing:?} instead of the two authored fighters"
    );

    // 1. **Each fighter reaches for more than one thing.** A body with sixteen
    //    moves that throws one of them is the failure this whole pass exists to
    //    catch, and it passes every authoring test ever written.
    for seat in &seats {
        assert!(
            seat.distinct() >= DISTINCT_FLOOR,
            "{} started only {} distinct moves in {WINDOW} ticks (floor \
             {DISTINCT_FLOOR}).\n{report}",
            seat.character,
            seat.distinct(),
        );
    }

    // 2. **Specials and aerials are USED, not merely owned.** Both fighters
    //    author four specials and five-plus aerials; a CPU that only ever jabs
    //    and dashes is the `double jump → generic attack` fighter this measures
    //    against.
    for seat in &seats {
        assert!(
            seat.count_within(&seat.specials()) > 0,
            "{} authored {} specials and threw none.\n{report}",
            seat.character,
            seat.specials().len(),
        );
        assert!(
            seat.count_within(&seat.aerials()) > 0,
            "{} authored {} aerials and threw none.\n{report}",
            seat.character,
            seat.aerials().len(),
        );
    }

    // 3. **THE TWO FIGHTERS ARE PLAYING DIFFERENT GAMES.** Not "the ids differ" —
    //    the ids differ because the tables differ, which proves nothing. What is
    //    asserted is that each body threw something the OTHER body's table does
    //    not even contain, so the difference on screen is repertoire and not
    //    naming.
    let [left, right] = [&seats[0], &seats[1]];
    for (mine, theirs) in [(left, right), (right, left)] {
        let theirs_ids: BTreeSet<&str> =
            theirs.moveset.moves.iter().map(|m| m.id.as_str()).collect();
        let unique: Vec<&str> = mine
            .started
            .keys()
            .map(String::as_str)
            .filter(|id| !theirs_ids.contains(id))
            .collect();
        assert!(
            !unique.is_empty(),
            "{} threw nothing its opponent's table does not also contain, so the \
             two bodies are indistinguishable to a viewer.\n{report}",
            mine.character,
        );
    }
}

/// **DOES THE CPU USE THE CHARACTER'S OWN WAY HOME?**
///
/// ⛔⛔ this is the one that could pass vacuously in the most expensive way. L2
/// used to return an EMPTY attack list in `Situation::Recovery` — *"a body past
/// the blastzone has exactly one problem"* — so a fighter carrying a real Up-B
/// drifted and jumped at a stage it could not reach while holding the thing that
/// would have saved it. Authoring fixes nothing on its own; the brain has to
/// offer it and the kernel has to endorse it.
///
/// ⚠ **deliberately weak on WHEN, strict on WHETHER.** Pinning a recovery to a
/// particular offstage position pins the tuning of a demo. What must never be
/// true is that a fighter with an authored route never presses it in a match
/// where bodies are being launched off a stage.
///
/// ⭐ and the route is derived, never named: `lift_speed > 0` is the same
/// predicate `lifting_candidates` proposes from, so the two fighters' DIFFERENT
/// answers (a vertical ascent and a lateral haul) are both covered by one claim.
///
/// ⛔⛔ **AND HERE IS WHAT THIS TEST CANNOT SEE, stated rather than implied.** A
/// throw is not a decision. `lift_speed > 0` is satisfied by any move that
/// commands a rise, and a fighter may author one that is a JUGGLE rather than a
/// way home — the Pirate Admiral's `air_up` is exactly that, deliberately (its
/// authoring comment calls it "a deliberate poison for the recovery
/// affordance"). So a green here means *"the fighter pressed something that
/// displaces it"* and NOT *"the fighter recovered with its recovery"*. The
/// per-move histogram printed above is where a reader sees which one it was, and
/// `with_the_decision_log` below is the only thing here that asks the brain what
/// it SELECTED in `Situation::Recovery`. Do not promote this to the stronger
/// claim.
#[test]
fn each_fighter_presses_an_authored_route() {
    let seats = crossover();
    let report = render(&seats);
    eprintln!("[smash crossover recovery, {WINDOW} ticks]\n{report}");

    for seat in &seats {
        let routes = seat.routes();
        assert!(
            !routes.is_empty(),
            "{} authors no move that commands a rise, so this fighter has no way \
             home to measure and the claim below would be vacuous.\n{report}",
            seat.character,
        );
        assert!(
            seat.count_within(&routes) > 0,
            "{} carries {} authored route(s) home ({routes:?}) and pressed none \
             across {WINDOW} ticks — the shape of a CPU that recovers with legacy \
             drift-and-jump while holding a real recovery.\n{report}",
            seat.character,
            routes.len(),
        );
    }
}

/// **`Situation::Recovery` → WHICH ACTION WAS SELECTED**, asked of the brain's
/// own decision facts rather than inferred from what the body did.
///
/// The test above sees a route being THROWN; it cannot see the decision that
/// chose it, nor the ticks where the search ran and endorsed nothing. Both are
/// fields on `fighter_decision` now, so the histogram is a group-by rather than
/// a reconstruction.
///
/// ⚠ **gated on `causal`, which is NOT a default feature** — recording costs
/// work per tick and a shipped game must not pay it. Run with
/// `cargo test -p ambition_app --features causal --test app_it -- the_cpus_put_on_a_show`.
#[cfg(feature = "causal")]
mod with_the_decision_log {
    use super::*;
    // ⚠ through the FACADE, not through `ambition_causal` directly: the crate is
    // an optional dependency and the facade's `causal` module is the surface a
    // game composes against.
    use ambition_platformer2d::causal::{
        domains, CausalFact, CausalPlugin, CausalRecording, FactValue, RecordingPolicy,
    };

    fn text<'a>(fact: &'a CausalFact, key: &str) -> Option<&'a str> {
        match fact.get(key) {
            Some(FactValue::Text(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    /// One row per `(subject, situation, selected action)`.
    #[test]
    fn the_recovery_decisions_name_the_action_they_selected() {
        let mut app = shell_host_app();
        // ⚠ **the FEATURE and the PLUGIN are two switches, deliberately.** The
        // feature compiles the publishers in; only `CausalPlugin` creates the
        // recording they write to. Installed before the first frame so nothing
        // published during composition is missed.
        app.add_plugins(CausalPlugin);
        for _ in 0..6 {
            app.update();
        }
        // ⚠ **BRAIN only, and the ring is why.** `CausalLog` holds 4096 facts and
        // drops the oldest; `RecordingPolicy::All` over a thirty-second match
        // wraps it many times over, so the histogram would silently describe the
        // last second of the fight. Narrowing the policy is what makes the whole
        // window readable. `dropped()` is reported below either way.
        ambition_platformer2d::causal::record_domains(
            &mut app,
            RecordingPolicy::only([domains::BRAIN]),
        );

        launch_row(&mut app, "Smash");
        for _ in 0..20 {
            app.update();
        }
        app.world_mut()
            .insert_resource(ambition_demo_smash::smash_roster_at_levels(
                [ambition_demo_smash::SMASH_GEORGE_BOOUL, PIRATE_ADMIRAL],
                &[7, 7],
            ));
        app.world_mut()
            .write_message(ShellCommand::GoTo(ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            )));
        for _ in 0..120 {
            app.update();
            if active_route(&app).as_deref() == Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE) {
                break;
            }
        }
        for _ in 0..(WINDOW + 200) {
            app.update();
        }

        let recording = app.world().resource::<CausalRecording>();
        let dropped = recording.dropped();

        // situation → selected action → count, per subject.
        let mut by_subject: BTreeMap<String, BTreeMap<(String, String), usize>> = BTreeMap::new();
        // and what the recovery SEARCH said, separately: a route, "home already",
        // or nothing found under a named search bound.
        let mut recovery_routes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        let mut decisions = 0usize;
        for fact in recording
            .facts()
            .filter(|fact| fact.detail.kind == "fighter_decision")
        {
            decisions += 1;
            let subject = fact
                .subject
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unattributed".to_string());
            let situation = text(fact, "situation").unwrap_or("?").to_string();
            let action = text(fact, "attack").unwrap_or("?").to_string();
            *by_subject
                .entry(subject.clone())
                .or_default()
                .entry((situation.clone(), action))
                .or_default() += 1;
            if situation == "Recovery" {
                // ⚠ **the PROPOSALS ride along with the outcome**, because
                // `no-route` means two different things — the repertoire offered
                // nothing, or the kernel declined everything it was offered — and
                // only the second is a tuning question. `pressed` is what the
                // decision actually armed, which on the negative branch is the
                // ranking fallback rather than the search's answer (see the ⛔
                // note in `decision.rs`); that is the number this histogram
                // exists to expose.
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
                *recovery_routes
                    .entry(subject)
                    .or_default()
                    .entry(outcome)
                    .or_default() += 1;
            }
        }

        eprintln!("[fighter decisions] {decisions} facts, {dropped} dropped by the ring\n{by_subject:#?}");
        eprintln!("[recovery routes]\n{recovery_routes:#?}");

        assert!(
            decisions > 0,
            "the composed host ran a whole CPU match and published no \
             `fighter_decision` fact — either no fighter brain is seated, or the \
             `causal` feature stopped reaching `ambition_characters`"
        );
        // ⚠ the SUBJECT is what makes a two-fighter histogram readable at all; an
        // unattributed stream is one pile.
        assert!(
            by_subject.len() >= 2,
            "both seats are CPUs and the decisions came back under {} subject(s), \
             so the histogram cannot tell the two fighters apart: {by_subject:#?}",
            by_subject.len()
        );
        // ⭐ the headline: some tick of this match was a recovery decision, and it
        // named what it selected. A run with none means the fighters never left
        // the stage, which makes every recovery claim in this file untested
        // rather than passing.
        assert!(
            !recovery_routes.is_empty(),
            "no fighter was ever classified `Situation::Recovery` in {WINDOW} \
             ticks, so nothing here measured a recovery decision at all"
        );
    }
}
