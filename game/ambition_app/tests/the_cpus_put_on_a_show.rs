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

/// The Admiral's catalog id. ⚠ a literal, and an id this composition does not
/// carry is INVISIBLE — the seat quietly gets a stand-in wearing the demo's
/// shared table. That is what the two-tables poison in the first test catches.
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

/// How many bodies are seated in the match right now.
///
/// ⚠ the poll's condition, and the first number the failure message reports.
/// `MatchSeat` is inserted by `activate_the_prepared_match` on the entity
/// `realize_seat` just spawned, so a body that has this has been REALISED — it
/// is the honest "the stage is populated" signal, where a frame count is a guess.
fn seats_in_the_world(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&MatchSeat>();
    q.iter(world).count()
}

/// Entities carrying a moveset at all, seated or not — the term that separates
/// "the bodies have no repertoire" from "the two components are on different
/// entities".
fn movesets_in_the_world(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut q = world.query::<&ActorMoveset>();
    q.iter(world).count()
}

/// **Bring a two-CPU crossover match to the point where both bodies exist**, and
/// hand back the app plus who is in which seat.
///
/// ⚠ **one seating path, reached through a hook.** `compose` runs after the
/// `App` exists and before its first frame — the one moment a caller can install
/// something like `CausalPlugin`. The alternative was a second copy of this
/// procedure inside the causal test, and it would have inherited the
/// zero-bodies bug this function was rewritten to fix; `build_visible_app_with`
/// carries the same hook for the same reason, and its doc records that the fork
/// it replaced cost five bugs.
fn seat_a_crossover(
    characters: [&str; 2],
    level: u8,
    compose: impl FnOnce(&mut App),
) -> (
    App,
    BTreeMap<usize, String>,
    BTreeMap<usize, MovesetContract>,
) {
    let mut app = shell_host_app();
    compose(&mut app);
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
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[level, level]);
    // ⚠ seat → character taken from the roster we HAND IN, not read back later:
    // the resource is the select screen's and it is removed on the way back out,
    // so a read after the match is a lifetime question rather than a fact about
    // who is fighting. What this cannot prove — that the ids resolved to two
    // different fighters — is proved from the world instead, by the two seats'
    // tables differing.
    let seat_characters: BTreeMap<usize, String> = roster
        .participants
        .iter()
        .enumerate()
        .map(|(index, p)| (index, p.character.to_string()))
        .collect();
    // Kept, because the poll below may have to hand it in again — see there.
    let roster_for_the_stage = roster.clone();
    app.world_mut().insert_resource(roster);
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

    // ⛔⛔ **WAIT FOR THE BODIES, do not assume a frame count produced them.**
    //
    // The first version ran `countdown + 60` fixed frames and then queried, and
    // it found ZERO seated bodies on a stage whose room had demonstrably loaded.
    // A fixed warm-up encodes a guess about how long SEATING takes, and seating
    // in the host is a lifecycle: the route activates, the room loads,
    // `prepare_the_match` reads the roster into a `PreparedMatch`, and only then
    // does `activate_the_prepared_match` realise the bodies. Every one of those
    // is a separate frame, and `prepare_the_match` returns early — silently —
    // until its geometry, its registry AND its roster are all present at once.
    //
    // ⚠ so the loop polls for the thing it needs instead of for a clock, and the
    // cap is generous because a slow step here is not the property under test.
    let mut waited = 0usize;
    let mut re_supplied = 0usize;
    let countdown = countdown as usize;
    let cap = countdown + 900;
    while waited < cap {
        app.update();
        waited += 1;
        let seated_now = seats_in_the_world(&mut app);
        // ⚠ both terms: two bodies AND past the hold. The stage opens suspended
        // and seats its fighters before the count ends, so breaking on the bodies
        // alone would drop the window straight back inside the 3-2-1-GO — the
        // defect the old fixed warm-up was written to avoid, and it must survive
        // the rewrite.
        if seated_now >= 2 && waited > countdown {
            break;
        }
        // ⚠ **the roster is an INPUT the stage consumes, and this fixture hands
        // it in from the select route — which legitimately clears it**, so that
        // a player coming back to the screen gets a fresh match instead of one
        // already decided. That is a race a fixture can lose silently: the
        // resource disappears, `prepare_the_match` returns early forever, the
        // route still activates, and the stage stands empty.
        //
        // ⛔⛔ **AND IT STOPS THE INSTANT A BODY IS SEATED**, which is the whole
        // reason this is safe. `versus.rs` has a test named
        // `a_half_seated_match_is_not_handed_a_different_roster` — handing a
        // roster to a match that has already seated somebody is a known way to
        // corrupt one, and a fixture doing it every frame would be manufacturing
        // exactly that state.
        //
        // ⛔ nor is it papering over the race: `re_supplied` is PRINTED on the
        // green path, so a non-zero count beside a passing run is the select
        // screen eating the roster — a real finding, and a different one from
        // anything about the brain.
        if seated_now == 0
            && app
                .world()
                .get_resource::<MatchParticipantRoster>()
                .is_none()
        {
            app.world_mut()
                .insert_resource(roster_for_the_stage.clone());
            re_supplied += 1;
        }
    }

    // The bodies, and the tables they are wearing. Read from the WORLD: a seat's
    // repertoire is whatever preparation actually gave it, which is the only
    // version that can disagree with the authoring.
    //
    // ⚠ **`Option<&ActorMoveset>`, so the failure below can tell three different
    // bugs apart.** A required column collapses "no body was ever seated", "the
    // body exists and preparation gave it no moveset" and "the two components
    // landed on different entities" into one `found 0`, and they want three
    // different fixes.
    let seated: Vec<(usize, Option<MovesetContract>)> = {
        let world = app.world_mut();
        let mut q = world.query::<(&MatchSeat, Option<&ActorMoveset>)>();
        q.iter(world)
            .map(|(seat, moveset)| (seat.0, moveset.map(|m| m.0.clone())))
            .collect()
    };
    let seat_tables: BTreeMap<usize, MovesetContract> = seated
        .iter()
        .filter_map(|(seat, moveset)| moveset.clone().map(|m| (*seat, m)))
        .collect();

    // Every diagnostic term read BEFORE the assertion, so the message is one
    // format and no borrow of the world outlives it.
    let with_seat = seated.len();
    let seated_without_moveset = seated.iter().filter(|(_, m)| m.is_none()).count();
    let movesets_anywhere = movesets_in_the_world(&mut app);
    let roster_state = match app.world().get_resource::<MatchParticipantRoster>() {
        None => "NO — something removed it between the insert and preparation",
        Some(r) if r.participants.len() == 2 => "yes, 2 participants",
        Some(_) => "yes, but not 2 participants",
    };
    let session_state = match app
        .world()
        .get_resource::<ambition_platformer2d::game_shell::ActiveGameplaySession>()
    {
        None => "no session resource",
        Some(s) if s.0.is_some() => "yes",
        Some(_) => "resource present, no session",
    };
    assert_eq!(
        seat_tables.len(),
        2,
        "no two seated bodies carrying movesets after {waited} frames \
         (countdown {countdown}).\n  \
         bodies with MatchSeat:             {with_seat}\n  \
         ...of those, WITHOUT ActorMoveset: {seated_without_moveset}\n  \
         entities with ActorMoveset at all: {movesets_anywhere}\n  \
         roster resource still present:     {roster_state}\n  \
         times the fixture re-supplied it:  {re_supplied}\n  \
         gameplay session active:           {session_state}\n  \
         asked for: {seat_characters:?}\n\
         Read it like this. MatchSeat 0 ⇒ SEATING never happened: the route \
         activated but `prepare_the_match` never turned a roster into a \
         `PreparedMatch`, so read the roster row — that is a FIXTURE bug. \
         MatchSeat 2 with movesets 0 ⇒ the bodies exist and preparation gave \
         them no repertoire, which is a PRODUCT defect. MatchSeat 0 with \
         movesets > 0 ⇒ the two components really are on different entities, \
         which contradicts `activate_the_prepared_match` inserting `MatchSeat` \
         on the entity `realize_seat` returned."
    );

    // ⚠ printed on the GREEN path too: how long seating actually took, and
    // whether the select route was eating the roster underneath it. Both are
    // facts about the fixture that a passing run would otherwise hide.
    eprintln!(
        "[smash crossover] seated 2 bodies after {waited} frames (countdown \
         {countdown}); roster re-supplied {re_supplied} time(s)"
    );

    (app, seat_characters, seat_tables)
}

/// Seat the crossover, watch it for `ticks`, and report per seat.
fn watch_a_crossover(characters: [&str; 2], level: u8, ticks: usize) -> Vec<SeatLedger> {
    let (mut app, seat_characters, seat_tables) = seat_a_crossover(characters, level, |_| {});

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

    // 0. **THE POISON: two seats, two TABLES.** An id this composition does not
    //    carry seats a stand-in, and a stand-in wears the demo's shared fighter
    //    table — so both seats would be the same fighter twice and every number
    //    below would be about one repertoire measured against itself. Asked of
    //    the world rather than of the ids, because it is the KIT that has to
    //    differ, not the label.
    assert_ne!(
        seats[0].moveset,
        seats[1].moveset,
        "both seats are wearing the same authored table, so this match is one \
         fighter twice: {:?}",
        seats.iter().map(|s| &s.character).collect::<Vec<_>>()
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
        // ⛔ **the SAME seating path as every other test in this file.** This
        // used to spell the whole procedure out again, and when the fixed
        // warm-up turned out not to seat any bodies, the copy would have
        // inherited the bug — silently, because a match with no fighters
        // publishes no decisions and the failure would have read "the `causal`
        // feature stopped reaching `ambition_characters`". The composition hook
        // is what makes one path serve both.
        let (mut app, _seats, _tables) = seat_a_crossover(
            [ambition_demo_smash::SMASH_GEORGE_BOOUL, PIRATE_ADMIRAL],
            7,
            |app| {
                // ⚠ **the FEATURE and the PLUGIN are two switches, deliberately.**
                // The feature compiles the publishers in; only `CausalPlugin`
                // creates the recording they write to. Installed before the first
                // frame so nothing published during composition is missed.
                app.add_plugins(CausalPlugin);
                // ⚠ **BRAIN only, and the ring is why.** `CausalLog` holds 4096
                // facts and drops the oldest; `RecordingPolicy::All` over a
                // thirty-second match wraps it many times over, so the histogram
                // would silently describe the last second of the fight.
                // `dropped()` is reported below either way.
                ambition_platformer2d::causal::record_domains(
                    app,
                    RecordingPolicy::only([domains::BRAIN]),
                );
            },
        );
        for _ in 0..WINDOW {
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

        eprintln!(
            "[fighter decisions] {decisions} facts, {dropped} dropped by the ring\n{by_subject:#?}"
        );
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
