//! Count what two CPUs actually DO to each other over a match.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- match-report -- [SECONDS] [CHARACTER] [--runs N]`
//!
//! With `--features causal` it also prints what the brain decided, grouped by
//! the situation it answered. The outcome half says what a fight did; the
//! decision half says why. A behaviour change can have second-order effects
//! (for example, through the situation classifier), and both halves are
//! needed to see them.
//!
//! Mechanics can be authored, tuned, and reachable, and still unused by the
//! CPUs. Counting in a real match finds that; unit tests do not. Run this
//! after any change that claims to affect how a fight goes.
//!
//! It is observational and has no pass/fail threshold. The asserting guard is
//! in `tests/the_repertoire_gets_used.rs`.
//!
//! Use `--runs N`: one run is not a measurement. Each fighter has an
//! execution-noise stream, and one thirty-second sample is too noisy to tune
//! against. With `--runs` the spread prints as `min–median–max`.

use crate::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::combat::moveset::MovePlayback;
use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

/// One seat's tally. Ticks unless the name says otherwise.
#[derive(Default, Clone)]
struct Tally {
    /// Peak percent, not the final reading. A KO resets a body to zero, so the
    /// last value says how recently somebody died.
    damage: i32,
    hitstun: usize,
    tumbling: usize,
    knocked_down: usize,
    evading: usize,
    /// Ticks this body could not be struck at all. Separates "the CPUs are
    /// defensive" from "the CPUs are unhittable".
    unhittable: usize,
    /// Which of the terms in `body_vulnerable` was false, counted separately,
    /// so the owning term can be fixed.
    unhit_invuln: usize,
    unhit_evading: usize,
    /// The ledge's share of `unhit_evading`. It is a part of that column, so
    /// the columns do not add up to `unhittable`. The ledge has its own
    /// intangibility flag, so it can be tuned apart from the dodge roll.
    unhit_ledge: usize,
    unhit_parry_window: usize,
    unhit_iframes: usize,
    /// How often this fighter changes its walking direction: sign changes in
    /// its locomotion intent, ignoring ticks with no intent.
    ///
    /// The initial dash (D217) re-arms on each new direction, so a body that
    /// flips often restarts its dash instead of travelling. In the kernel, a
    /// body flipping every 4 ticks covers 675px where a steady one covers 1339.
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
    /// The fastest launch this body was handed, and the speed its own tuning
    /// says a launch must beat to become a tumble. Together they separate the
    /// two causes of "nobody tumbled".
    ///
    /// Sampled only on hitstun. Every attack lunges (George's lunge reads
    /// 1500 px/s against a 500 px/s tumble threshold), so plain top speed is
    /// not a launch.
    top_speed: f32,
    tumble_speed: f32,
    /// Ticks within a body-width or two of the nearest opponent. "Moves thrown"
    /// cannot tell a quiet match from one where nobody was in range.
    in_range: usize,
    /// Times this body's percent fell back to zero from a live reading: a KO,
    /// seen at the one edge that survives the body being replaced.
    kos: usize,
    /// Every move start, by id. The decision histogram says what the brain
    /// pressed; this says what the body threw. They differ where the runtime
    /// takes a cancel window's nomination, which is where chains live.
    started: std::collections::BTreeMap<String, usize>,
    /// Launches handed to this body: rising edges of hitstun, the same edge
    /// `top_speed` is sampled on. Separates one big hit from forty.
    launches: usize,
    /// Ticks inside the hard control lock at the start of a launch
    /// (`BodyCombat::recoil_lock_timer`), which presentation reads as the
    /// launch beat. Beside `launches` it gives the beat length in practice;
    /// `0` means the beat is inert.
    beat_ticks: usize,
    /// The speed a launched body flies at, one sample per tick of involuntary
    /// flight (`hitstun > 0 || tumbling`, the predicate `LaunchedBodiesView`
    /// publishes).
    ///
    /// Not `top_speed`, which is the speed when the launch was written.
    /// Gravity keeps working in flight. Presentation gates its launch cues on
    /// this distribution, so fit such thresholds to it.
    flight_speeds: Vec<f32>,
    /// Ticks this body spent held by somebody. A refused grab and a grab never
    /// attempted look the same in the move table.
    held: usize,
}

#[derive(clap::Args, Debug)]
pub struct MatchReportArgs {
    /// Seconds of match to simulate.
    #[arg(default_value_t = 30)]
    pub seconds: usize,
    /// Which fighter to report on.
    #[arg(default_value_t = ambition_demo_smash::SMASH_GEORGE_BOOUL.to_string())]
    pub character: String,
    /// How many runs to average over. Zero is treated as one, as it always was.
    #[arg(long, default_value_t = 1)]
    pub runs: usize,
    /// Load an authored difficulty ladder from a `.ron` and install it.
    ///
    /// Without it, this reports the engine floor, not the shipped game:
    /// `build_demo_app` installs no `AuthoredFighterLadder`, so every seat
    /// carries `UtilityWeights::default()` (the level-9 row) at level-5
    /// reaction, APM, and noise.
    #[arg(long, value_name = "PATH")]
    pub ladder: Option<String>,
}

pub fn run(args: MatchReportArgs) {
    let seconds: usize = args.seconds;
    let character = args.character;
    let runs = if args.runs > 0 { args.runs } else { 1 };

    #[cfg(feature = "causal")]
    let mut decisions = DecisionTally::new();
    let mut carried: Vec<String> = Vec::new();
    // Parse once: a parse failure is a caller error and must stop before any
    // match runs.
    let authored_ladder = args.ladder.as_deref().map(|path| {
        let text = std::fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("[match_report] --ladder {path}: {err}");
            std::process::exit(2);
        });
        let ladder =
            ambition_platformer2d::characters::brain::fighter::FighterBrainLadder::from_ron(&text)
                .unwrap_or_else(|err| {
                    eprintln!("[match_report] --ladder {path} did not parse: {err}");
                    std::process::exit(2);
                });
        ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder(ladder)
    });
    // Name the ladder before the first number. Do it here, not in a report
    // function: `runs == 1` and `runs > 1` take different report paths.
    println!(
        "match_report: ladder: {}",
        if authored_ladder.is_some() {
            "the AUTHORED rows"
        } else {
            "⛔ the ENGINE FLOOR — no --ladder given, so every seat carries \
             UtilityWeights::default() (== the level-9 row) at level-5 reaction/APM. \
             NOT the shipped fighter."
        }
    );
    let all: Vec<Vec<Tally>> = (0..runs)
        .map(|i| {
            run_one(
                &character,
                seconds,
                0x5F37_7A11_u64.wrapping_mul(i as u64 + 1),
                #[cfg(feature = "causal")]
                &mut decisions,
                &mut carried,
                authored_ladder.as_ref(),
            )
        })
        .collect();

    // An empty report is unreadable. A character this composition does not
    // carry seats no fighter, and every tally stays zero, which looks like a
    // quiet fight. This binary runs the demo shell, not the full app.
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
/// Reads the fact, not the `AMBITION_FIGHTER_TRACE=1` prose:
/// `first("fighter_decision").get("chose")` is a field lookup that wording
/// changes cannot break.
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
        // The attack is a second decision, so count it too. `"none"` is a real
        // answer here, not a move called none.
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
    // Out-param: the header must measure the composition, and this is where
    // a built app is in hand. Filled on every run.
    carried: &mut Vec<String>,
    authored_ladder: Option<&ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder>,
) -> Vec<Tally> {
    let mut app = build_demo_app();
    if let Some(ladder) = authored_ladder {
        app.world_mut().insert_resource(ladder.clone());
    }
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
    // Resolved at `Startup` from the ids the assembled catalog carries. Absent
    // only if the demo's own plugin did not run; the header then says so.
    *carried = app
        .world()
        .get_resource::<ambition_demo_smash::select::SmashRoster>()
        .map(|roster| roster.0.clone())
        .unwrap_or_default();
    // Both seats are CPUs. `SmashSelect::roster` makes every locked seat
    // human, and nobody presses anything here.
    let characters = [character, character];
    let roster = ambition_demo_smash::smash_roster_at_levels(characters, &[5, 5]);
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // Wait past the 3-2-1-GO: fighters are held for the whole ceremony. Read
    // the count from the ruleset.
    let countdown = ambition_demo_smash::smash_roster(characters)
        .rules
        .opening_countdown_ticks;
    for _ in 0..(countdown as usize + 30) {
        app.update();
    }

    // Force the stream: the point is the spread across streams (as
    // `ladder_probe` does).
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
        // Print the total first and say when rows are withheld, so a new move
        // is not dropped in silence.
        println!("\nwhat the bodies actually threw: {} distinct", rows.len());
        for (id, count) in rows.iter().take(12) {
            println!("  {id:<28} {count:>5}");
        }
        if rows.len() > 12 {
            println!("  … and {} more not shown", rows.len() - 12);
        }
    }
    // How often a fighter changes its walking direction. See
    // `Tally::steer_flips`.
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
            // A share of the column before it: these do not sum to `unhittable`.
            tally.unhit_ledge,
            tally.unhit_parry_window,
            tally.unhit_iframes
        );
    }
    println!(
        "\nticks are counts of SAMPLED TICKS in that state; damage is final percent, \
         parries and techs are events, charge is the best fraction reached."
    );
    // Flag a match where nobody was launched, however much damage it has.
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
    // The steer lives on the control frame, not the body, so sample it in its
    // own pass. Count sign changes: the initial dash is paid per change.
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
                    // Ask the damage rule's own answer; do not reconstruct eligibility.
                    (
                        health.health.invulnerable.any(),
                        facts.is_some_and(|f| f.evading()),
                        // The ledge's own intangibility.
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
    // Was anybody in range? Only the distance between the bodies separates a
    // quiet match from a busy one.
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
        // Sample on the rising edge of hitstun, the tick the launch was written.
        // Later ticks include gravity's work.
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
            // An event, not a state: count the tick the timer rises.
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

/// The composition this measurement is of, printed on every run.
///
/// This binary composes the smash demo shell, whose catalog is a fraction
/// of the full app's. A change can look fine here and regress only in the
/// full app, for characters this shell cannot seat. The list comes from
/// `SmashRoster`, which is resolved at `Startup` from the catalog, so it
/// cannot go stale.
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

/// `min–median–max` across runs: one run is a sample of a noisy process.
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
    // The initial dash is paid per direction change (D217).
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
    // Pooled, not min–median–max: this is a distribution over ticks of flight,
    // and a presentation gate on flight speed picks a percentile of it.
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
