//! Count what two CPUs actually DO to each other over a match.
//!
//! `cargo run -p ambition_demo_smash_app --bin match_report -- [SECONDS] [CHARACTER] [--runs N]`
//!
//! ⭐ **With `--features causal` it also prints WHAT THE BRAIN DECIDED, grouped by
//! the situation it was answering.** The outcome half of this report says what a
//! fight did; the decision half says why, and separating a behaviour change from
//! its second-order consequences needs both at once. Three hand edits to the
//! fighter's movement scores were reverted in one night for want of exactly this
//! pairing: the change did what it said to the verb it named, and the damage came
//! two steps away, through the situation classifier.
//!
//! Every mechanic in this demo is authored, tuned and reachable; the question
//! that keeps going unanswered is whether anybody USES it. Three separate
//! slices — the smash charge, directional influence, the tech — shipped green
//! and inert, and each one was caught by counting in a real match rather than by
//! a unit test. This is that counting, made cheap enough to run after any change
//! that claims to affect how a fight goes.
//!
//! It is observational and has no pass/fail threshold. The one guard that DOES
//! assert lives in `tests/the_repertoire_gets_used.rs`; this prints the whole
//! vocabulary so a number that moved can be seen next to the ones that did not.
//!
//! ⛔ `--runs N` IS NOT DECORATION, AND ONE RUN IS NOT A MEASUREMENT. Two
//! fighters carry an execution-noise stream each, and a single thirty-second
//! sample of a fight is noisy enough that tuning against it makes things worse:
//! measured 2026-08-23, an option-scorer change judged on one run took the smash
//! suite from two failures to four. With `--runs` the spread is printed as
//! `min–median–max`, which is the shape a threshold should be picked off.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

/// One seat's tally. Ticks unless the name says otherwise.
#[derive(Default, Clone)]
struct Tally {
    /// PEAK percent, not the final reading. A KO resets a body to zero, so the
    /// last value read is a measure of how recently somebody died rather than of
    /// how much the fight did — and a run in which both fighters were killed
    /// reads as a run in which nothing happened.
    damage: i32,
    hitstun: usize,
    tumbling: usize,
    knocked_down: usize,
    evading: usize,
    /// Ticks this body could not be struck at all — the damage rule's own
    /// answer, inverted. The number that separates "the CPUs are defensive" from
    /// "the CPUs are unhittable".
    unhittable: usize,
    /// WHICH of the four terms in `body_vulnerable` was false, counted
    /// separately. "A quarter of the match is untouchable" is a symptom; which
    /// term owns it is the fix.
    unhit_invuln: usize,
    unhit_evading: usize,
    /// The LEDGE's share of `unhit_evading` — a refinement of it, not a
    /// sibling, so the two columns do not add up to `unhittable`.
    ///
    /// Worth its own column because the ledge was invisible until its
    /// intangibility was split off the dodge roll's timer: a body camped on an
    /// edge and a body mid-evade both read as `dodge_rolling`, so "evading 659"
    /// could have been either, and nobody could tune one without the other.
    unhit_ledge: usize,
    unhit_parry_window: usize,
    unhit_iframes: usize,
    /// HOW OFTEN THIS FIGHTER CHANGES ITS MIND about which way to walk —
    /// counted as sign changes in its own locomotion intent, ignoring ticks it
    /// asked for nothing.
    ///
    /// ⭐ IT IS HERE BECAUSE THE INITIAL DASH IS PAID FOR PER CHANGE, not per
    /// tick: the phase re-arms on a new direction, and a body that re-arms
    /// constantly restarts its dash instead of travelling. Measured in the
    /// kernel: a body flipping every 4 ticks covers 675px where a steady one
    /// covers 1339. This column is the other half of that — whether a real
    /// fighter flips anywhere near that often.
    steer_flips: usize,
    /// Ticks this fighter asked for a direction at all, so `steer_flips` has a
    /// denominator and "rarely flips" cannot mean "rarely moves".
    steer_held: usize,
    shielding: usize,
    parries_caught: usize,
    tech_armed: usize,
    charge_held: usize,
    /// The highest charge fraction this seat ever reached.
    best_charge: f32,
    /// Distinct move starts, so a match that throws one move reads as one.
    moves_started: usize,
    /// The fastest launch this body was ever handed, and the speed its own
    /// tuning says a launch has to beat to become a tumble. Printed together
    /// because "nobody tumbled" has two very different causes and only these two
    /// numbers separate them.
    ///
    /// ⛔ hitstun-gated on purpose. Plain top speed is not a launch: every attack
    /// in this engine lunges, and George's lunge alone reads 1500 px/s against a
    /// 500 px/s tumble threshold — a number that says a body was thrown when
    /// nothing threw it.
    top_speed: f32,
    tumble_speed: f32,
    /// Ticks spent within a body-width or two of the nearest opponent. A match
    /// where nothing happens is usually a match where nobody was ever in range,
    /// and "moves thrown" cannot tell those apart.
    in_range: usize,
    /// Times this body's percent fell back to zero from a live reading — a KO,
    /// observed at the one edge that survives the body being removed and
    /// replaced.
    kos: usize,
    /// Every move START, by id. The decision histogram says what the brain
    /// PRESSED; this says what the body actually threw — and the two differ
    /// wherever the runtime takes a cancel window's nomination, which is exactly
    /// where a chain lives.
    started: std::collections::BTreeMap<String, usize>,
    /// Launches HANDED to this body: rising edges of hitstun, the same edge
    /// `top_speed` is sampled on. The peak alone cannot say whether a match had
    /// one big hit or forty.
    launches: usize,
    /// Ticks inside the HARD control lock at the front of a launch
    /// (`BodyCombat::recoil_lock_timer`) while launched — the window
    /// presentation reads as the launch BEAT, and the only thing that separates
    /// a body thrown this instant from one that has been tumbling for a second.
    /// Beside `launches` it says how long a beat lasts in practice; `0` means
    /// the beat is inert and every launch trail is the same trail.
    beat_ticks: usize,
    /// THE SPEED A LAUNCHED BODY ACTUALLY FLIES AT, one sample per tick it
    /// spends in involuntary flight (`hitstun > 0 || tumbling` — the same
    /// predicate `LaunchedBodiesView` publishes).
    ///
    /// ⛔ NOT `top_speed`. That is the speed at the tick the launch was WRITTEN,
    /// which is the right statistic for "how hard do hits throw people" and the
    /// wrong one for anything that watches a body in flight: gravity keeps
    /// working, and a launched body reaching 1500 px/s in a match whose reported
    /// peak launch was 1000 is ordinary, not an anomaly. Presentation gates its
    /// launch cues on THIS distribution, so a threshold picked off the other one
    /// is fitted to a number the gate never sees.
    flight_speeds: Vec<f32>,
    /// Ticks this body spent HELD by somebody. A grab is the most visible beat
    /// in the genre that a CPU can simply never throw, and "moves started"
    /// cannot see it: a grab that is refused and a grab that is never attempted
    /// look identical from the move table.
    held: usize,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seconds: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(30);
    let mut character = ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string();
    let mut runs = 1usize;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--runs" => {
                runs = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|n| *n > 0)
                    .unwrap_or(1);
            }
            other => character = other.to_string(),
        }
    }

    #[cfg(feature = "causal")]
    let mut decisions = DecisionTally::new();
    let mut carried: Vec<String> = Vec::new();
    let all: Vec<Vec<Tally>> = (0..runs)
        .map(|i| {
            run_one(
                &character,
                seconds,
                0x5F37_7A11_u64.wrapping_mul(i as u64 + 1),
                #[cfg(feature = "causal")]
                &mut decisions,
                &mut carried,
            )
        })
        .collect();

    // ⛔ AN EMPTY REPORT IS AN ANSWER NOBODY CAN READ. A character this demo's
    // composition does not carry seats no fighter, every tally stays zero, and
    // the tables below print their headers over nothing — which reads as "the
    // fight was quiet" rather than "you asked about somebody who is not here".
    // Measured 2026-08-23 with `npc_pirate_admiral`, which `app_it` seats
    // successfully because it runs the FULL app; this bin runs the demo shell.
    if all.iter().all(|run| {
        run.iter()
            .all(|tally| tally.damage == 0 && tally.moves_started == 0)
    }) {
        eprintln!(
            "match_report: nothing was seated for '{character}'. This binary composes the \
             SMASH DEMO shell, and the ids its catalog ACTUALLY carries are: {}. A character \
             the composition does not have seats no fighter brain, so every column would be \
             zero.",
            if carried.is_empty() {
                "NONE — the roster resource resolved empty".to_string()
            } else {
                carried
                    .iter()
                    .map(|id| format!("`{id}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        );
        std::process::exit(2);
    }
    if runs == 1 {
        report_one(&character, seconds, &all[0], &carried);
    } else {
        report_spread(&character, seconds, &all, &carried);
    }
    #[cfg(feature = "causal")]
    report_decisions(&decisions);
}

/// Every `(situation, verb)` the brain chose, counted. Empty without the causal
/// feature, which is why the printer is behind the same gate.
#[cfg(feature = "causal")]
type DecisionTally = std::collections::BTreeMap<(String, String), usize>;

/// Count this tick's fighter decisions off the causal log.
///
/// ⭐ THE FACT, not the trace line. `AMBITION_FIGHTER_TRACE=1` prints the same
/// content as prose and counting it means a regex over wording somebody may
/// improve; `first("fighter_decision").get("chose")` is a field lookup. That is
/// the reason the fact exists and its own doc says so.
#[cfg(feature = "causal")]
fn collect_decisions(app: &App, into: &mut DecisionTally) {
    let Some(log) = app
        .world()
        .get_resource::<ambition_platformer2d::causal::CausalRecording>()
    else {
        return;
    };
    let Some(stamped) = log.tick() else { return };
    for subject in log.subjects_on(stamped) {
        let explanation = log.explain(stamped, &subject);
        let Some(decided) = explanation.first("fighter_decision") else {
            continue;
        };
        let situation = decided
            .get("situation")
            .map(|value| format!("{value}"))
            .unwrap_or_else(|| "?".to_string());
        let chose = decided
            .get("chose")
            .map(|value| format!("{value}"))
            .unwrap_or_else(|| "?".to_string());
        *into
            .entry((situation.clone(), format!("move {chose}")))
            .or_default() += 1;
        // THE ATTACK IS A SECOND DECISION, and counting only the movement verb
        // hid that. The mechanics lane found the jab chain inert CPU-versus-CPU
        // and had to reach for a separate move-id histogram to see it, because
        // this one reported no attack row at all — it was not that the brain
        // never attacked, it was that the instrument only asked one of the two
        // questions the fact answers. `"none"` is a real answer here, distinct
        // from a move called none.
        let attack = decided
            .get("attack")
            .map(|value| format!("{value}"))
            .unwrap_or_else(|| "?".to_string());
        if attack != "none" {
            *into
                .entry((situation, format!("attack {attack}")))
                .or_default() += 1;
        }
    }
}

#[cfg(feature = "causal")]
fn report_decisions(decisions: &DecisionTally) {
    if decisions.is_empty() {
        println!(
            "\nno fighter decisions were recorded — the causal feature is on but \
             nothing published, which is a defect in the recording rather than a \
             quiet fight"
        );
        return;
    }
    println!("\nwhat the brain decided, by the question it was answering:");
    let total: usize = decisions.values().sum();
    let mut rows: Vec<_> = decisions.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for ((situation, chose), count) in rows {
        println!(
            "  {:<14} {:<28} {:>6}  {:>5.1}%",
            situation,
            chose,
            count,
            100.0 * *count as f32 / total as f32
        );
    }
    println!("  {:<14} {:<28} {:>6}", "", "total", total);
}

/// One match, under one execution-noise stream.
fn run_one(
    character: &str,
    seconds: usize,
    noise_seed: u64,
    #[cfg(feature = "causal")] decisions: &mut DecisionTally,
    // ⭐ OUT-PARAM, because the header must MEASURE the composition rather than
    // assert it, and this is the only place a built app is in hand. Filled on
    // every run; they agree, and reading it here costs nothing.
    carried: &mut Vec<String>,
) -> Vec<Tally> {
    let mut app = build_demo_app();
    #[cfg(feature = "causal")]
    {
        app.add_plugins(ambition_platformer2d::causal::CausalPlugin);
        ambition_platformer2d::causal::record_domains(
            &mut app,
            ambition_platformer2d::causal::RecordingPolicy::only([
                ambition_platformer2d::causal::domains::BRAIN,
            ]),
        );
    }
    for _ in 0..30 {
        app.update();
    }
    // Resolved at `Startup` from the ids the assembled catalog actually carries.
    // Absent only if the demo's own plugin did not run, in which case the header
    // says so rather than naming a list nobody verified.
    *carried = app
        .world()
        .get_resource::<ambition_demo_smash::select::SmashRoster>()
        .map(|roster| roster.0.clone())
        .unwrap_or_default();
    // BOTH SEATS CPU. `SmashSelect::roster` makes every locked seat a HUMAN,
    // which is right for a couch game and wrong here: a report driven through it
    // measures two fighters standing still while nobody presses anything.
    let characters = [character, character];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[5, 5]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Past the ceremony: every fighter carries scripted control for the whole
    // 3-2-1-GO, so ticks inside the hold measure bodies that are forbidden to
    // act. Read the count from the ruleset rather than restating it.
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    // THE STREAM IS FORCED, and this rig supplies it rather than modelling how a
    // live fighter gets one — the point is the SPREAD across streams, exactly as
    // `ladder_probe` documents for the same reason.
    {
        use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
        let world = app.world_mut();
        let mut q = world.query::<&mut Brain>();
        for (index, mut brain) in q.iter_mut(world).enumerate() {
            if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
                state.noise = noise_seed.wrapping_mul(index as u64 + 1).wrapping_add(1);
            }
        }
    }
    let ticks = seconds * 60;
    let mut totals: Vec<Tally> = vec![Tally::default(); 4];
    let mut live_move: Vec<Option<(String, f32)>> = vec![None; 4];
    let mut parry_was: Vec<f32> = vec![0.0; 4];
    let mut hitstun_was: Vec<f32> = vec![0.0; 4];
    let mut last_damage: Vec<i32> = vec![0; 4];
    let mut steer_was: Vec<f32> = vec![0.0; 4];
    for _ in 0..ticks {
        app.update();
        sample(
            &mut app,
            &mut totals,
            &mut live_move,
            &mut parry_was,
            &mut hitstun_was,
            &mut last_damage,
            &mut steer_was,
        );
        #[cfg(feature = "causal")]
        collect_decisions(&app, decisions);
    }

    totals
}

fn report_one(character: &str, seconds: usize, totals: &[Tally], carried: &[String]) {
    println!("match_report: {character} vs {character}, {seconds}s of CPU-versus-CPU");
    println!("{}\n", composition_scope(carried));
    println!(
        "{:<6} {:>7} {:>7} {:>8} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>7} {:>8} {:>8}",
        "seat",
        "damage",
        "moves",
        "hitstun",
        "tumbling",
        "downed",
        "evading",
        "unhit",
        "shield",
        "parries",
        "techs",
        "charge",
        "launch",
        "tumble@",
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.damage == 0 && tally.moves_started == 0 {
            continue;
        }
        println!(
            "{:<6} {:>7} {:>7} {:>8} {:>8} {:>7} {:>8} {:>7} {:>7} {:>7} {:>7} {:>6.2} {:>8.0} {:>8.0}",
            seat,
            tally.damage,
            tally.moves_started,
            tally.hitstun,
            tally.tumbling,
            tally.knocked_down,
            tally.evading,
            tally.unhittable,
            tally.shielding,
            tally.parries_caught,
            tally.tech_armed,
            tally.best_charge,
            tally.top_speed,
            tally.tumble_speed,
        );
    }
    let mut moves: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for tally in totals {
        for (id, count) in &tally.started {
            *moves.entry(id.as_str()).or_default() += count;
        }
    }
    if !moves.is_empty() {
        let mut rows: Vec<_> = moves.into_iter().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        println!("\nwhat the bodies actually threw:");
        for (id, count) in rows.iter().take(12) {
            println!("  {id:<28} {count:>5}");
        }
    }
    // ⭐ HOW OFTEN A FIGHTER CHANGES ITS MIND about which way to walk. The
    // initial dash (D217) is paid per direction CHANGE, not per tick — the
    // phase re-arms on a new direction — so a body that flips often restarts
    // its dash instead of travelling. Measured in the kernel: flipping every 4
    // ticks covers 675px where a steady body covers 1339. This says whether a
    // real fighter is anywhere near that.
    println!("\nhow often each body changes its walking direction:");
    println!(
        "{:<6} {:>12} {:>12} {:>18}",
        "seat", "steered", "flips", "ticks per flip"
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.steer_held == 0 {
            continue;
        }
        println!(
            "{:<6} {:>12} {:>12} {:>18.1}",
            seat,
            tally.steer_held,
            tally.steer_flips,
            tally.steer_held as f32 / tally.steer_flips.max(1) as f32
        );
    }
    println!("\nwhy each body could not be struck, by the term that refused:");
    println!(
        "{:<6} {:>10} {:>10} {:>10} {:>14} {:>10}",
        "seat", "invuln", "evading", "of-ledge", "parry-window", "i-frames"
    );
    for (seat, tally) in totals.iter().enumerate() {
        if tally.unhittable == 0 {
            continue;
        }
        println!(
            "{:<6} {:>10} {:>10} {:>10} {:>14} {:>10}",
            seat,
            tally.unhit_invuln,
            tally.unhit_evading,
            // A SHARE of the column before it, not a sibling: these do not sum
            // to `unhittable`.
            tally.unhit_ledge,
            tally.unhit_parry_window,
            tally.unhit_iframes
        );
    }
    println!(
        "\nticks are counts of SAMPLED TICKS in that state; damage is final percent, \
         parries and techs are events, charge is the best fraction reached."
    );
    // THE ONE READING WORTH SAYING OUT LOUD. A match where nobody is ever
    // launched is not a match, however much damage it accumulates, and that
    // exact state shipped once already.
    if totals.iter().all(|t| t.tumbling == 0) {
        println!(
            "\n⚠ NOBODY TUMBLED. Hits are landing and nothing is being launched — \
             check the tumble threshold against the launches actually resolved."
        );
    }
    if totals.iter().all(|t| t.best_charge <= 0.0) {
        println!("\n⚠ NOBODY CHARGED A SMASH. The multiplier is authored and unpaid.");
    }
}

fn sample(
    app: &mut App,
    totals: &mut [Tally],
    live_move: &mut [Option<(String, f32)>],
    parry_was: &mut [f32],
    hitstun_was: &mut [f32],
    last_damage: &mut [i32],
    steer_was: &mut [f32],
) {
    // THE STEER, sampled on its own pass because it lives on the control frame
    // rather than on the body — and counted as SIGN CHANGES, which is the unit
    // the initial dash is actually paid in.
    {
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &ambition_platformer2d::characters::control::ActorControl,
        )>();
        let steers: Vec<(usize, f32)> = q
            .iter(world)
            .map(|(seat, control)| (seat.0, control.0.locomotion.vec().x))
            .collect();
        for (seat, x) in steers {
            let Some(slot) = totals.get_mut(seat) else {
                continue;
            };
            let dir = if x.abs() > 0.5 { x.signum() } else { 0.0 };
            if dir != 0.0 {
                slot.steer_held += 1;
                if steer_was[seat] != 0.0 && dir != steer_was[seat] {
                    slot.steer_flips += 1;
                }
                steer_was[seat] = dir;
            }
        }
    }
    let world = app.world_mut();
    let mut q = world.query::<(
        &MatchSeat,
        &BodyHealth,
        &BodyCombat,
        &ae::BodyKinematics,
        Option<&MovePlayback>,
        Option<&ae::BodyMotionFacts>,
        Option<&ae::BodyShieldState>,
        Option<&ambition_platformer2d::actor::MotionModel>,
        Option<&ambition_platformer2d::combat::capture::CapturedBy>,
    )>();
    let rows: Vec<_> = q
        .iter(world)
        .map(
            |(seat, health, combat, kin, playback, facts, shield, model, captured)| {
                (
                    seat.0,
                    health.damage_taken(),
                    // THE DAMAGE RULE'S OWN ANSWER, asked here rather than
                    // reconstructed: a report that guessed at eligibility would
                    // be a second opinion about the thing it measures.
                    (
                        health.health.invulnerable.any(),
                        facts.is_some_and(|f| f.evading()),
                        // The ledge's own intangibility, which used to be
                        // spelled as a dodge roll and so could not be counted.
                        facts.is_some_and(|f| f.ledge_intangible),
                        shield.is_some_and(|s| s.parrying()),
                        !combat.vulnerable(),
                    ),
                    combat.hitstun_timer,
                    kin.vel.length(),
                    match model {
                        Some(ae::MotionModel::AxisSwept(axis)) => {
                            axis.params.abilities.tumble_speed
                        }
                        _ => 0.0,
                    },
                    playback.map(|p| (p.spec.id.clone(), p.t, p.smash_charge_fraction())),
                    facts.copied(),
                    shield.copied(),
                    match model {
                        Some(ae::MotionModel::AxisSwept(axis)) => Some(axis.state.tech_press_timer),
                        _ => None,
                    },
                    kin.pos.x,
                    captured.is_some(),
                    combat.recoil_lock_timer,
                )
            },
        )
        .collect();
    // WAS ANYBODY IN RANGE? A quiet match and a busy one both throw moves; only
    // the distance between the bodies tells them apart.
    const IN_RANGE_PX: f32 = 120.0;
    let positions: Vec<(usize, f32)> = rows.iter().map(|row| (row.0, row.10)).collect();
    for (
        seat,
        damage,
        vulnerable,
        hitstun,
        speed,
        tumble_speed,
        playback,
        facts,
        shield,
        tech_timer,
        here,
        captured,
        recoil_lock,
    ) in rows
    {
        let Some(tally) = totals.get_mut(seat) else {
            continue;
        };
        if damage == 0 && last_damage[seat] > 0 {
            tally.kos += 1;
        }
        last_damage[seat] = damage;
        tally.damage = tally.damage.max(damage);
        if captured {
            tally.held += 1;
        }
        if positions
            .iter()
            .any(|(other, x)| *other != seat && (x - here).abs() <= IN_RANGE_PX)
        {
            tally.in_range += 1;
        }
        // ON THE RISING EDGE OF HITSTUN, which is the tick the launch was
        // written. Sampling any later reads gravity's work as the attacker's:
        // a body launched downward is faster every tick it falls, and the
        // threshold it had to beat was the one it left with.
        if hitstun > 0.0 && hitstun_was[seat] <= 0.0 {
            tally.top_speed = tally.top_speed.max(speed);
            tally.launches += 1;
        }
        if hitstun > 0.0 && recoil_lock > 0.0 {
            tally.beat_ticks += 1;
        }
        if hitstun > 0.0 || facts.is_some_and(|f| f.tumbling) {
            tally.flight_speeds.push(speed);
        }
        hitstun_was[seat] = hitstun;
        tally.tumble_speed = tumble_speed;
        if hitstun > 0.0 {
            tally.hitstun += 1;
        }
        if let Some(facts) = facts {
            if facts.tumbling {
                tally.tumbling += 1;
            }
            if facts.knocked_down {
                tally.knocked_down += 1;
            }
            if facts.evading() {
                tally.evading += 1;
            }
        }
        let (invuln, evading, ledge, parry_window, iframes) = vulnerable;
        if invuln || evading || parry_window || iframes {
            tally.unhittable += 1;
        }
        if invuln {
            tally.unhit_invuln += 1;
        }
        if evading {
            tally.unhit_evading += 1;
        }
        if ledge {
            tally.unhit_ledge += 1;
        }
        if parry_window {
            tally.unhit_parry_window += 1;
        }
        if iframes {
            tally.unhit_iframes += 1;
        }
        if let Some(shield) = shield {
            if shield.active {
                tally.shielding += 1;
            }
            // An EVENT, not a state: the timer is counted on the tick it rises.
            if shield.parry_caught_timer > parry_was[seat] {
                tally.parries_caught += 1;
            }
            parry_was[seat] = shield.parry_caught_timer;
        }
        if tech_timer.is_some_and(|t| t > 0.0) {
            tally.tech_armed += 1;
        }
        if let Some((id, t, charge)) = playback {
            let fresh = match &live_move[seat] {
                Some((last_id, last_t)) => last_id != &id || t < *last_t,
                None => true,
            };
            if fresh {
                tally.moves_started += 1;
                *tally.started.entry(id.clone()).or_default() += 1;
            }
            live_move[seat] = Some((id, t));
            if let Some(fraction) = charge {
                tally.charge_held += 1;
                tally.best_charge = tally.best_charge.max(fraction);
            }
        } else {
            live_move[seat] = None;
        }
    }
}

/// `min–median–max` across runs, which is the shape a threshold should be picked
/// off. One number from one run is a sample of a noisy process, and this rig
/// ⭐⭐ WHAT COMPOSITION THIS MEASUREMENT IS OF, printed on every run.
///
/// ⛔ A PROOF THAT DOES NOT CARRY ITS SCOPE GETS BELIEVED BEYOND IT, and this
/// has now cost two separate findings. This binary composes the SMASH DEMO
/// SHELL, whose character catalog is a fraction of the full app's — twice in one
/// day a lane proved a change live here, correctly, and the regression it missed
/// existed only in the full app: once for a respawn interval that reads clean on
/// George and costs `npc_pirate_admiral` two-thirds of its damage, and once for
/// a perception bound that only bites characters this shell cannot seat.
///
/// So the header says which game was measured. A reader who then quotes the
/// number at the shipped roster is making a claim this line already refused.
/// ⛔⛔ THIS USED TO ASSERT THREE HARD-CODED IDS, AND A HAND-KEPT LIST DESCRIBING A
/// COMPOSED ONE GOES STALE — which is the exact failure this header exists to
/// prevent. `SmashRoster` is RESOLVED AT `Startup` from the ids the assembled
/// catalog actually carries, so the honest header reads that resource instead of
/// naming constants beside it. ⇒ if the demo shell's catalog grows, this line
/// grows with it and nobody has to notice.
fn composition_scope(carried: &[String]) -> String {
    let names = if carried.is_empty() {
        "NOTHING — the catalog resolved empty".to_string()
    } else {
        carried
            .iter()
            .map(|id| format!("`{id}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "⚠ composition: the SMASH DEMO SHELL, which carries {} id(s) — {names}. The shipped \
         app's grid may be larger, and a number taken here is a claim about THIS composition \
         only - `app_it` is where the full roster lives.",
        carried.len(),
    )
}

/// exists because a change judged on one made the suite worse.
fn report_spread(character: &str, seconds: usize, all: &[Vec<Tally>], carried: &[String]) {
    println!(
        "match_report: {character} vs {character}, {seconds}s × {} runs, per-run TOTALS across both seats\n{}\n",
        all.len(),
        composition_scope(carried)
    );
    let spread = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(pick).sum::<f32>())
            .collect();
        values.sort_by(f32::total_cmp);
        let median = values[values.len() / 2];
        format!(
            "{:.0}–{:.0}–{:.0}",
            values.first().copied().unwrap_or(0.0),
            median,
            values.last().copied().unwrap_or(0.0)
        )
    };
    let peak = |pick: fn(&Tally) -> f32| -> String {
        let mut values: Vec<f32> = all
            .iter()
            .map(|run| run.iter().map(pick).fold(0.0f32, f32::max))
            .collect();
        values.sort_by(f32::total_cmp);
        format!(
            "{:.2}–{:.2}–{:.2}",
            values.first().copied().unwrap_or(0.0),
            values[values.len() / 2],
            values.last().copied().unwrap_or(0.0)
        )
    };
    println!("  damage      {}", spread(|t| t.damage as f32));
    println!("  moves       {}", spread(|t| t.moves_started as f32));
    println!("  hitstun     {}", spread(|t| t.hitstun as f32));
    println!("  tumbling    {}", spread(|t| t.tumbling as f32));
    println!("  downed      {}", spread(|t| t.knocked_down as f32));
    println!("  evading     {}", spread(|t| t.evading as f32));
    println!("  unhittable  {}", spread(|t| t.unhittable as f32));
    println!("  shielding   {}", spread(|t| t.shielding as f32));
    // THE INITIAL DASH IS PAID PER DIRECTION CHANGE, so this is the number that
    // says whether the phase would help a fighter or pin one. See D217.
    println!("  steer held  {}", spread(|t| t.steer_held as f32));
    println!("  steer flips {}", spread(|t| t.steer_flips as f32));
    println!(
        "  ⇒ steered ticks per flip {}",
        spread(|t| t.steer_held as f32 / t.steer_flips.max(1) as f32)
    );
    println!("  parries     {}", spread(|t| t.parries_caught as f32));
    println!("  techs       {}", spread(|t| t.tech_armed as f32));
    println!("  in range    {}", spread(|t| t.in_range as f32));
    println!("  KOs         {}", spread(|t| t.kos as f32));
    println!("  held        {}", spread(|t| t.held as f32));
    println!("  launches    {}", spread(|t| t.launches as f32));
    println!("  launch beat {}", spread(|t| t.beat_ticks as f32));
    println!("  best charge {}", peak(|t| t.best_charge));
    println!("  peak launch {}", peak(|t| t.top_speed));
    // POOLED, not min–median–max: this is a distribution over TICKS OF FLIGHT,
    // not a per-run total, and a percentile of it is what a presentation gate
    // on flight speed is actually choosing.
    let mut flight: Vec<f32> = all
        .iter()
        .flat_map(|run| run.iter().flat_map(|t| t.flight_speeds.iter().copied()))
        .collect();
    flight.sort_by(f32::total_cmp);
    if !flight.is_empty() {
        let at = |q: f32| flight[((flight.len() - 1) as f32 * q) as usize];
        println!(
            "  flight speed p25 {:.0}  p50 {:.0}  p75 {:.0}  p90 {:.0}  p99 {:.0}  max {:.0}  (n={})",
            at(0.25),
            at(0.50),
            at(0.75),
            at(0.90),
            at(0.99),
            flight[flight.len() - 1],
            flight.len(),
        );
    }
    println!(
        "\nmin–median–max across runs. Counts are summed over both seats; charge and \
         launch are the best either seat reached. `flight speed` is pooled over every \
         tick of involuntary flight in every run — the distribution a presentation gate \
         on flight speed actually sees.\n\n⚠ THE MATCHUP IS PART OF THE SAMPLE SIZE. \
         Every run here is one character against itself. Weight and fall speed move \
         every distribution above, so a constant fitted to this is fitted to this \
         matchup — write it down beside the sample size."
    );
}
