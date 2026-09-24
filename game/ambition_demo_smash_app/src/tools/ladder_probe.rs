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
/// Unlike `ladder_rig`, this does not use the shipped match length. The
/// opponent here cannot attack, so no bout has a winner to decide: stock
/// loss counts self-KOs and the clock is only the observation window. Do
/// not compare survival times from this probe with `ladder_rig`'s.
const TICKS: usize = 3_600;

/// How many execution-noise seeds each configuration is run under.
///
/// Overridable: `cargo run --bin smash_tool -- ladder-probe -- --seeds 7`.
const DEFAULT_SEEDS: usize = 3;

/// Name the ladder before printing any numbers.
///
/// The rungs resolve through `FighterBrainProfile::for_level` (the engine
/// floor), because this demo ships no ladder of its own (`fighter-brain.md`
/// §4: games and demos ship their own rows). The floor turns
/// `rollout_depth: 12` on at level 6 and above, so the level column
/// confounds depth with reaction, APM, noise, and read weight. With an
/// authored ladder that sets depth 0 on every rung, the same column would be
/// a clean reaction/APM/noise sweep.
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

    // The A/B for FB6e's question. The level column confounds depth with
    // every other rung change; level 5 -> 6 changes five things at once. This
    // varies only `rollout_depth` on one level-9 profile.
    println!("[ladder_probe] --- same level 9 profile, ONLY rollout_depth varied ---");
    for depth in [0u32, 12] {
        report(&run_seeds(9, Some(depth), seeds));
    }
}

// Per-subject `vel_x` from the previous tick, so an unclaimed velocity
// step is detected instead of eyeballed. The seam line samples 1 tick in 5
// between decisions, so a short ramp is invisible to it; the detector
// checks every tick.
#[cfg(feature = "causal")]
thread_local! {
    /// The detector lives in `ambition_causal`; this probe is one user.
    static UNCLAIMED: std::cell::RefCell<ambition_platformer2d::causal::UnclaimedStepDetector> =
        std::cell::RefCell::new(ambition_platformer2d::causal::UnclaimedStepDetector::new());
}

/// The largest per-tick velocity step the integrator can produce, plus 1%
/// for float slop.
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

/// Print the joined explanation for every subject that acted this tick,
/// then clear the log. The tick is the natural scope, and an accumulated
/// log would grow over thousands of ticks.
///
/// `[fighter …]` lines on the same stream carry no tick (a brain deep below
/// the ECS does not know the world clock; the published fact is stamped).
/// Do not align them with `[seam] t=N` lines by adjacency. Compare only
/// `t=`-stamped lines.
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
        // Every kernel movement operation this tick (`Slash`, `Dash`, `WallJump`,
        // `LedgeClimbStart`, …), not just the first.
        let operations: Vec<String> = explanation
            .facts()
            .iter()
            .filter(|fact| fact.kind() == "movement_operation")
            .filter_map(|fact| fact.get("operation").map(|value| format!("{value}")))
            .collect();
        // A subject with neither fact belongs to another domain; skip it.
        if received.is_none() && decided.is_none() {
            continue;
        }
        // Unclaimed velocity steps, checked on every tick before the sampling
        // filter: a step larger than the integrator can produce, with no kernel
        // operation naming a writer. Print every fact kind on the tick, not only
        // operations: a filter is a hypothesis and can hide the writer.
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
            // Facing matters: `Slash` recoil depends on it.
            field(received, "facing"),
            field(received, "vel_x"),
            field(received, "on_ground"),
            field(received, "dash_charges"),
            field(decided, "chose"),
            // The kernel's own operation. `Slash` subtracts
            // `side * facing * slash_recoil` from velocity on every press, so
            // without this field a body recoiling from its own attacks is
            // unexplained.
            operations.join(","),
        );
    }
    app.world_mut()
        .resource_mut::<ambition_platformer2d::causal::CausalRecording>()
        .clear();
}

/// The `[seam]` half of the trace reads the causal log and needs
/// `--features causal`. Warn instead of printing half the trace silently.
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

/// Whether the brain/seam trace is on. The brain reads the same switch, so
/// the two halves of the trace are never half-enabled.
#[cfg_attr(not(feature = "causal"), allow(dead_code))]
fn trace_enabled() -> bool {
    static ENABLED: std::sync::LazyLock<bool> = std::sync::LazyLock::new(|| {
        std::env::var("AMBITION_FIGHTER_TRACE").is_ok_and(|value| value != "0")
    });
    *ENABLED
}

/// One configuration, run under `seeds` different execution-noise streams.
///
/// This rig supplies its streams. A live CPU's stream is
/// `participant ⊕ level` (`brain_builders::fighter_cognition_seed`); the
/// probe sweeps `i` to measure the spread across streams. Do not change it
/// to match the builder, and do not read it as evidence of what the
/// builder does.
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
/// The spread says whether a difference between two rows means anything.
fn spread_label(values: impl Iterator<Item = Option<usize>> + Clone) -> String {
    let all: Vec<Option<usize>> = values.collect();
    // A seed where the event never happened is a different outcome, not a
    // large number; averaging it in would invent a time.
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

/// Where it died, as a side rather than a number. The stage is centered, so
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
    /// A run tops out near `MAX_RUN_SPEED`; a dash is several times that. So
    /// this separates "walked off" from "dashed off", which are different bugs.
    peak_speed: f32,
    /// Where the body stood on the tick before its first self-KO. A death off
    /// the left and off the right are different bugs: one is the veto steering,
    /// the other is the veto blind.
    death_x: Option<f32>,
    lost: u32,
    peak: f32,
}

/// Run one match.
fn run_one(level: u8, forced_depth: Option<u32>, noise_seed: u64) -> LadderRun {
    let mut app = build_demo_app();
    // The log the seam trace reads. Installed only when tracing.
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
        // The applied control beside the body it should move.
        // `AMBITION_FIGHTER_TRACE=1` prints what the brain emitted; this prints
        // what reached `ActorControl`. The brain's `fighter_decision` and the
        // body's `control_frame_received` share a subject, so one `explain`
        // returns both on one line.
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
        // Elimination despawns the body, so its absence after it was present is
        // the signal.
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

/// Overwrite the execution-noise stream on every fighter brain present, so
/// one configuration can run under several.
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
