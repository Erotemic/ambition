//! Quick fighter-depth smoke probe.
//!
//! Runs one fixed match across fighter levels and execution-noise seeds, then prints survival
//! and engagement measurements. The opponent cannot attack, so stock loss measures self-KOs.
//! This is a diagnostic probe, not the multi-scenario ladder calibration rig; do not author
//! difficulty rows from it.

use crate::build_demo_app;
use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
use bevy::app::App;

/// One minute at 60Hz.
///
/// ⚠ **NO LONGER THE SAME BUDGET AS `ladder_rig`, and the two are no longer
/// readable against each other.** That rig defaulted to sixty seconds too, and
/// its comment said so; on 2026-09-04 it was changed to
/// `ambition_demo_smash::SMASH_TIME_LIMIT_TICKS` — the **shipped eight-minute
/// match** — because a bout that cannot end leaves stocks tied and sends every
/// verdict to a damage tiebreak.
///
/// ⭐ **That reasoning does NOT transfer here, which is why this stays sixty.**
/// This probe's opponent cannot attack: a bout has no winner to decide, so
/// nothing depends on it finishing. Stock loss here counts SELF-KOs, and the
/// clock is just the observation window. ⇒ Making it eight minutes would multiply
/// the run time by eight and change nothing about what the number means.
///
/// ⛔ So do not compare a survival time from this probe against one from
/// `ladder_rig`. They are windows onto different questions and, since 2026-09-04,
/// different lengths.
const TICKS: usize = 3_600;

/// How many execution-noise seeds each configuration is run under.
///
/// Overridable: `cargo run --bin smash_tool -- ladder-probe -- --seeds 7`.
const DEFAULT_SEEDS: usize = 3;

/// Say WHICH ladder these numbers describe, before printing any.
///
/// a calibration table that does not name its ladder is worse than no
/// table, and this one could not name it. The rungs here resolve through
/// `FighterBrainProfile::for_level` — the ENGINE FLOOR — because this demo ships
/// no ladder of its own, which `fighter-brain.md` §4 says is exactly what a game
/// that has authored none should get: *"Games/demos ship their own rows — it's
/// content."*
///
/// That is the demo gate."* Loading it here would be one game reading another game's difficulty.
///
/// the numbers differ where this probe's own A/B lives, which is why the
/// line matters: the floor turns `rollout_depth: 12` on at level ≥ 6, so the
/// ladder column below confounds depth with four other changes — while an
/// authored ladder may set 0 on every rung (Ambition's does, deliberately), in
/// which case the same column is a clean reaction/APM/noise sweep. Same table,
/// two different meanings, decided entirely by whose ladder ran.
fn announce_which_ladder_is_under_test() {
    println!(
        "[ladder_probe] LADDER: engine floor (`FighterBrainProfile::for_level`) — \
         this demo authors no ladder of its own. Rungs therefore gain \
         `rollout_depth: 12` at level >= 6, so the level column confounds depth \
         with reaction/APM/noise/read-weight. The forced-depth A/B below is the \
         only clean depth comparison here."
    );
}

#[derive(clap::Args, Debug)]
pub struct LadderProbeArgs {
    /// How many seeds to run.
    #[arg(long, default_value_t = DEFAULT_SEEDS)]
    pub seeds: usize,
}

pub fn run(args: LadderProbeArgs) {
    warn_if_seam_trace_is_unavailable();
    announce_which_ladder_is_under_test();
    let seeds = args.seeds;
    println!(
        "[ladder_probe] level  first_self_KO   survived   stocks_lost  peak%   \
         (median of {seeds} seeds; opponent cannot attack, so every loss is a self-KO)"
    );
    for level in [1u8, 3, 5, 6, 9] {
        report(&run_seeds(level, None, seeds));
    }

    // ── the A/B that is actually FB6e's question ─────────────────────────
    //
    // The ladder column above confounds depth with everything else a rung changes (reaction,
    // APM, execution noise, read weight). `for_level` turns the rollout on at level 6, so level
    // 5 -> 6 is NOT a depth experiment; it is five changes at once.
    println!("[ladder_probe] --- same level 9 profile, ONLY rollout_depth varied ---");
    for depth in [0u32, 12] {
        report(&run_seeds(9, Some(depth), seeds));
    }
}

// ⛔ THIS BLOCK IS ORPHANED AND NEEDS ITS OWNER. It was a `///` doc on the
// `thread_local!` below, which is not a place a doc comment attaches — so it
// warned, and the warning was invisible because this binary only builds under
// `--features causal`. Its four subjects (printing an explanation, clearing the
// log each tick, the per-subject `vel_x`, the seam's 1-in-5 sampling) describe
// several different items, so it reads like docs left behind by items that were
// deleted or moved. Demoted to `//` rather than reattached: silencing the
// warning is unambiguous, deciding which item each paragraph belonged to is not.
// Print the joined explanation for every subject that acted this tick, then
// clear the log.
//
// `[fighter …]` lines on this same stream carry NO TICK, and must not be
// aligned with `[seam] t=N` by adjacency. That is deliberate and correct —
// `trace_decision`'s own doc explains it: a brain five hops below the ECS does
// not know the world's clock, and a counter guessed there would be a second
// clock no other domain could join against. The fact it publishes IS stamped;
// only the stderr rendering is not.
//
// They may or may not; adjacent lines here are not evidence either way. Compare only `t=`-stamped
// lines with each other.
//
// cleared every tick on purpose. A ladder run is thousands of ticks with
// several bodies each; a log that accumulated all of it would be a memory
// profile of the probe rather than a trace. The question here is always "what
// happened on THIS tick", so the tick is the natural scope.
// Per-subject `vel_x` as of the previous tick, so an UNCLAIMED velocity step can
// be detected instead of eyeballed.
//
// the seam line samples 1-in-5 between decisions, so a three-tick ramp is
// invisible to it — which is how S51's `-99`/tick ramp survived six reading
// cycles. The data was there every tick; only the printing was sampled.
#[cfg(feature = "causal")]
thread_local! {
    /// the detector itself now lives in `ambition_causal` — this probe was
    /// where it was first needed, not where it belongs. The trace that motivated
    /// it was taken in the SANDBOX composition and this binary runs the smash
    /// LADDER; two hosts that never see each other's bodies, one question.
    static UNCLAIMED: std::cell::RefCell<ambition_platformer2d::causal::UnclaimedStepDetector> =
        std::cell::RefCell::new(ambition_platformer2d::causal::UnclaimedStepDetector::new());
}

/// 1.01 is float slop and nothing else.
#[cfg(feature = "causal")]
const UNCLAIMED_STEP_THRESHOLD: f32 = {
    let per_tick = if ambition_platformer2d::engine_core::RUN_ACCEL
        > ambition_platformer2d::engine_core::AIR_ACCEL
    {
        ambition_platformer2d::engine_core::RUN_ACCEL
    } else {
        ambition_platformer2d::engine_core::AIR_ACCEL
    } / 60.0;
    per_tick * 1.01
};

#[cfg(feature = "causal")]
fn trace_seam(app: &mut App, tick: usize) {
    let Some(log) = app
        .world()
        .get_resource::<ambition_platformer2d::causal::CausalRecording>()
    else {
        return;
    };
    let Some(stamped) = log.tick() else { return };
    for subject in log.subjects_on(stamped) {
        let explanation = log.explain(stamped, &subject);
        let received = explanation.first("control_frame_received");
        let decided = explanation.first("fighter_decision");
        // Every kernel movement operation this tick — `Slash`, `Dash`,
        // `WallJump`, `LedgeClimbStart`, … — not just the first. A tick can
        // carry several and the interesting one is rarely the earliest.
        let operations: Vec<String> = explanation
            .facts()
            .iter()
            .filter(|fact| fact.kind() == "movement_operation")
            .filter_map(|fact| fact.get("operation").map(|value| format!("{value}")))
            .collect();
        // A subject with neither is some other domain's; skip rather than print
        // an empty row for it.
        if received.is_none() && decided.is_none() {
            continue;
        }
        // UNCLAIMED VELOCITY STEPS — checked on EVERY tick, before the
        // sampling filter below. This is the detector S51 needed and did not
        // have: a step larger than the integrator can produce, with no kernel
        // operation naming a writer.
        //
        // it prints every fact KIND on the tick rather than only the operations,
        // because the lesson that thread paid for twice is that a filter is a
        // hypothesis — `knockback_applied` sat in the log through six cycles
        // because the query asked for `contains("hit")`.
        if let Some(vx) = received
            .and_then(|fact| fact.get("vel_x"))
            .and_then(|value| format!("{value}").parse::<f32>().ok())
        {
            let subject_key = format!("{subject}");
            let found = UNCLAIMED.with(|cell| {
                cell.borrow_mut().observe(
                    tick as u64,
                    &subject_key,
                    vx,
                    !operations.is_empty(),
                    UNCLAIMED_STEP_THRESHOLD,
                )
            });
            if let Some(step) = found {
                let kinds: Vec<&str> = explanation.facts().iter().map(|f| f.kind()).collect();
                let show = |name: &str| {
                    received
                        .and_then(|fact| fact.get(name))
                        .map(|value| format!("{value}"))
                        .unwrap_or_else(|| "-".to_string())
                };
                eprintln!(
                    "[unclaimed] t={tick} {subject} dvx={:+.4} ({:.2} -> {:.2}) pos=({},{}) vel_y={} ground={} ops=[] kinds={kinds:?}",
                    step.delta(),
                    step.before,
                    step.after,
                    show("pos_x"),
                    show("pos_y"),
                    show("vel_y"),
                    show("on_ground"),
                );
            }
        }
        // Print every decision event; only sample the frames between decisions.
        // Decision cadence is independent of per-tick sampling.
        if decided.is_none() && tick % 5 != 0 {
            continue;
        }
        let field = |fact: Option<&ambition_platformer2d::causal::CausalFact>, name: &str| {
            fact.and_then(|f| f.get(name))
                .map(|value| format!("{value}"))
                .unwrap_or_else(|| "-".to_string())
        };
        eprintln!(
            "[seam] t={tick} {subject} asked={} holding={} facing={} vx={} ground={} \
             dash_charges={} chose={} ops=[{}]",
            field(decided, "emit_locomotion_x"),
            field(received, "locomotion_x"),
            // FACING is not decoration.
            field(received, "facing"),
            field(received, "vel_x"),
            field(received, "on_ground"),
            field(received, "dash_charges"),
            field(decided, "chose"),
            // THE KERNEL'S OWN OPERATION, and the field that finally
            // answered this thread. The seam line reported
            // what the brain asked and what the body held and was silent about
            // what the ENGINE did — so three ticks of `Slash` were invisible,
            // and `Slash` subtracts `side * facing * slash_recoil` from velocity
            // on every press. The body was recoiling from its own attacks and
            // the trace built to explain it could not say so.
            operations.join(","),
        );
    }
    app.world_mut()
        .resource_mut::<ambition_platformer2d::causal::CausalRecording>()
        .clear();
}

/// Whether the brain/seam trace is on. Same switch the brain reads, so the two
/// halves of the trace are never half-enabled.
///
/// the `[seam]` half needs `--features causal` — it reads the causal log
/// now instead of hand-querying components, and the log is a default-off
/// dependency. [`warn_if_seam_trace_is_unavailable`] says so out loud rather
/// than letting the trace come back missing half its lines.
/// Say it, rather than printing nothing.
fn warn_if_seam_trace_is_unavailable() {
    #[cfg(not(feature = "causal"))]
    if trace_enabled() {
        eprintln!(
            "[ladder_probe] AMBITION_FIGHTER_TRACE is on, but the [seam] half needs \
             the causal log: re-run with `--features causal`. The [fighter] lines \
             below are the brain's own trace and are NOT the whole picture."
        );
    }
}

#[cfg_attr(not(feature = "causal"), allow(dead_code))]
fn trace_enabled() -> bool {
    static ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("AMBITION_FIGHTER_TRACE").is_ok_and(|value| value != "0")
    });
    *ENABLED
}

/// One configuration, run under `seeds` different execution-noise streams.
///
/// this rig SUPPLIES its streams and does not model how a real fighter gets
/// one. A live CPU's stream is `participant ⊕ level`
/// (`brain_builders::fighter_cognition_seed`); sweeping `i` here is the point of
/// the probe — it is measuring the SPREAD across streams — so it deliberately
/// does not go through that seam. do not "fix" this to match the builder, and
/// do not read this loop as evidence of what the builder does.
fn run_seeds(level: u8, forced_depth: Option<u32>, seeds: usize) -> Vec<LadderRun> {
    (0..seeds.max(1))
        .map(|i| {
            run_one(
                level,
                forced_depth,
                0x5F37_7A11_u64.wrapping_mul(i as u64 + 1),
            )
        })
        .collect()
}

fn report(runs: &[LadderRun]) {
    let first = runs.first().expect("a configuration runs at least once");
    let tag = match first.forced_depth {
        Some(d) => format!("9/d{d}"),
        None => first.level.to_string(),
    };
    println!(
        "[ladder_probe]   {:<5}   {:>13}  {:>13}   {}         {:.0}%   {}",
        tag,
        spread_label(runs.iter().map(|r| r.first_loss)),
        spread_label(runs.iter().map(|r| r.eliminated)),
        median(runs.iter().map(|r| r.lost as usize)).unwrap_or(0),
        runs.iter().map(|r| r.peak).fold(0.0f32, f32::max) * 100.0,
        format!(
            "{}, vmax {:.0}",
            death_side(runs),
            runs.iter().map(|r| r.peak_speed).fold(0.0f32, f32::max)
        ),
    );
}

/// `"5.4s"` when every seed agrees, `"5.4s ±1.2"` when they do not.
///
/// The spread is the half the single-sample version could not print, and it is
/// the number that says whether a difference between two rows means anything.
fn spread_label(values: impl Iterator<Item = Option<usize>> + Clone) -> String {
    let all: Vec<Option<usize>> = values.collect();
    // A seed where the event never happened is not a large number — it is a
    // different outcome, and averaging it in would invent a time.
    let never = all.iter().filter(|v| v.is_none()).count();
    let happened: Vec<usize> = all.iter().filter_map(|v| *v).collect();
    if happened.is_empty() {
        return format!(">{}s", TICKS / 60);
    }
    let mid = median(happened.iter().copied()).unwrap_or(0) as f32 / 60.0;
    let low = *happened.iter().min().unwrap() as f32 / 60.0;
    let high = *happened.iter().max().unwrap() as f32 / 60.0;
    let never_tag = if never > 0 {
        format!(" +{never} never")
    } else {
        String::new()
    };
    if (high - low).abs() < 0.05 {
        format!("{mid:.1}s{never_tag}")
    } else {
        format!("{mid:.1}s [{low:.1}-{high:.1}]{never_tag}")
    }
}

/// WHERE it died, as a side rather than a number. The stage is centered, so
/// the authored stage centre separates left exits from right exits without
/// duplicating a room width or platform extent in this diagnostic.
fn death_side(runs: &[LadderRun]) -> String {
    let xs: Vec<f32> = runs.iter().filter_map(|r| r.death_x).collect();
    if xs.is_empty() {
        return "no self-KO".to_string();
    }
    let stage_midpoint = ambition_demo_smash::stage_centre().x;
    let left = xs.iter().filter(|x| **x < stage_midpoint).count();
    let right = xs.len() - left;
    let mean = xs.iter().sum::<f32>() / xs.len() as f32;
    format!("died at x≈{mean:.0} ({left}L/{right}R)")
}

fn median(values: impl Iterator<Item = usize>) -> Option<usize> {
    let mut sorted: Vec<usize> = values.collect();
    sorted.sort_unstable();
    sorted.get(sorted.len() / 2).copied()
}

struct LadderRun {
    level: u8,
    forced_depth: Option<u32>,
    /// Tick at which the fighter first lost a stock to itself.
    first_loss: Option<usize>,
    /// Tick at which it ran out of stocks entirely.
    eliminated: Option<usize>,
    /// The fastest horizontal speed the body ever reached, in px/s.
    ///
    /// A run tops out near `MAX_RUN_SPEED`; a dash is an impulse several times
    /// that. So this separates "walked off" from "dashed off" without having to
    /// instrument the brain's chosen verb — and those are different bugs.
    peak_speed: f32,
    /// Where the body was standing on the tick before its FIRST self-KO.
    ///
    /// A death off the LEFT and a death off the RIGHT are different bugs — one is the veto
    /// steering, the other is the veto blind.
    death_x: Option<f32>,
    lost: u32,
    peak: f32,
}

/// Run one match.
fn run_one(level: u8, forced_depth: Option<u32>, noise_seed: u64) -> LadderRun {
    let mut app = build_demo_app();
    // The log the seam trace reads. Installed only when tracing, so an ordinary
    // ladder run pays nothing for an inspector nobody opened.
    #[cfg(feature = "causal")]
    if trace_enabled() {
        app.add_plugins(ambition_platformer2d::causal::CausalPlugin);
        ambition_platformer2d::causal::record_domains(
            &mut app,
            ambition_platformer2d::causal::RecordingPolicy::only([
                ambition_platformer2d::causal::domains::MOVEMENT,
                ambition_platformer2d::causal::domains::BRAIN,
            ]),
        );
    }
    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_level(
            [
                ambition_demo_smash::SMASH_CHARACTER_ID,
                ambition_demo_smash::SMASH_OPPONENT_ID,
            ],
            level,
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));

    let mut lost = 0u32;
    let mut peak = 0.0f32;
    let mut last_x: Option<f32> = None;
    let mut peak_speed = 0.0f32;
    let mut death_x: Option<f32> = None;
    let mut first_loss = None;
    let mut eliminated = None;
    let mut depth_applied = forced_depth.is_none();
    let mut seed_applied = false;
    for tick in 0..TICKS {
        app.update();
        if !depth_applied {
            depth_applied = force_depth(&mut app, forced_depth.unwrap());
        }
        if !seed_applied {
            seed_applied = force_noise_seed(&mut app, noise_seed);
        }
        // the APPLIED control, beside the body it is supposed to move.
        // `AMBITION_FIGHTER_TRACE=1` prints what the BRAIN emitted; this prints what reached
        // `ActorControl`, which is the seam between the brain phase and the movement phase.
        //
        // Now there is one: the engine publishes the typed fact, and this reads it.
        //
        // What that buys is the whole point of the thread. The brain's `fighter_decision` fact and
        // the body's `control_frame_received` fact carry the SAME subject, so one `explain` returns
        // both — "asked for -1.0, holding -1.0, travelling +588" is one line instead of two streams
        // a human correlates by eye.
        #[cfg(feature = "causal")]
        if trace_enabled() {
            trace_seam(&mut app, tick);
        }
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &FighterStocks,
            &ambition_platformer2d::characters::actor::BodyHealth,
            &ambition_platformer2d::platformer::body::BodyKinematics,
        )>();
        let mut present = false;
        let mut seat_x = None;
        for (seat, stocks, health, kinematics) in q.iter(world) {
            if seat.0 == 1 {
                present = true;
                seat_x = Some(kinematics.pos.x);
                peak_speed = peak_speed.max(kinematics.vel.x.abs());
                let now = ambition_demo_smash::STARTING_STOCKS.saturating_sub(stocks.remaining);
                if now > 0 && first_loss.is_none() {
                    first_loss = Some(tick);
                    death_x = last_x;
                }
                lost = lost.max(now);
                peak = peak.max(health.damage_percent());
            }
        }
        last_x = seat_x.or(last_x);
        // Elimination despawns the body, so its absence AFTER it was present is
        // the signal — there is no last frame to read a zero off.
        if !present && eliminated.is_none() && first_loss.is_some() {
            eliminated = Some(tick);
        }
    }
    // A fighter that despawned lost everything it had left.
    let world = app.world_mut();
    let mut alive = world.query::<&MatchSeat>();
    if !alive.iter(world).any(|seat| seat.0 == 1) {
        lost = ambition_demo_smash::STARTING_STOCKS;
    }
    assert!(
        depth_applied,
        "the depth override never found a fighter brain to apply to; \
         this run measured the DEFAULT profile and its number is a lie"
    );
    assert!(
        seed_applied,
        "the noise seed never reached a fighter brain, so every seed in this \
         column ran the SAME match and the spread it prints is zero by \
         construction"
    );
    LadderRun {
        level,
        forced_depth,
        first_loss,
        eliminated,
        lost,
        peak,
        death_x,
        peak_speed,
    }
}

/// Overwrite `rollout_depth` on every fighter brain present. Overwrite the execution-noise
/// stream on every fighter brain present, so one configuration can be run under several.
fn force_noise_seed(app: &mut App, seed: u64) -> bool {
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { state, .. }) = &mut *brain {
            state.noise = seed;
            found = true;
        }
    }
    found
}

fn force_depth(app: &mut App, depth: u32) -> bool {
    let world = app.world_mut();
    let mut q = world.query::<&mut Brain>();
    let mut found = false;
    for mut brain in q.iter_mut(world) {
        if let Brain::StateMachine(StateMachineCfg::Fighter { cfg, .. }) = &mut *brain {
            cfg.profile.rollout_depth = depth;
            if depth == 0 {
                cfg.profile.rollout_k = 0;
            }
            found = true;
        }
    }
    found
}
