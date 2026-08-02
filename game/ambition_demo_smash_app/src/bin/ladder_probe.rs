//! **Does a rollout buy anything?** (FB6e — `l3_earns_its_depth`)
//!
//! `cargo run -p ambition_demo_smash_app --bin ladder_probe`
//!
//! ⚠ **"every ladder row carries `rollout_depth: 0`" was true when this was
//! written and stopped being true the same day** (`ed6c55d0e`, 2026-07-31):
//! `profile.rs` now ships `if level >= 6 { 12 } else { 0 }`. Read every table
//! below with that in mind — a row's level chooses its depth, so comparing
//! ACROSS levels varies two things at once. Only the `9/d0` vs `9/d12` rows hold
//! the profile fixed, which is why they are the only A/B here worth anything.
//!
//! The reason the depth was withheld is still written down: it cannot be authored
//! until something proves it buys something. FB6a–e built the rollout; nothing had ever measured a
//! fighter WITH it against the same fighter WITHOUT it, because until today no
//! fighter brain could be put on a body in a running match.
//!
//! This is that measurement, at the crudest honest scale: run the same match at
//! each level and time how long the CPU keeps itself alive. Its opponent is a
//! human seat with no controller and cannot attack, so **every stock lost is a
//! self-KO** — the brain walking or jumping off the stage. That makes the number
//! unusually clean: it is not "did it win", it is "did it kill itself", which is
//! the one thing a difficulty curve cannot survive.
//!
//! ⚠ the reported number is TIME, not stocks. It was stocks until 2026-07-31,
//! and stocks turned out to be a saturated metric: every level lost all three,
//! so the column read `3 3 3 3 3` and could not have reported an improvement if
//! one had happened. Ticks-until-first-self-KO has resolution across the whole
//! range between "walks off immediately" and "never dies".
//!
//! **What it measured, 2026-07-31 (morning)** — `level 9`, whole profile fixed,
//! only `rollout_depth` moved:
//!
//! ```text
//!   9/d0      5.0s to first self-KO
//!   9/d12     9.8s to first self-KO
//! ```
//!
//! That was read as the first evidence in this repository that L3 buys
//! anything, and it took three fixes to get a signal at all: the rollout had to
//! roll MOVEMENT lines (it only ever refined attacks), the shadow floor had to
//! have an EDGE, and the veto's horizon had to be long enough to reach one.
//! Before those, this table read `3 3 3 3 3` — a saturated metric over a blind
//! model.
//!
//! ⛔ **and the same evening, over THREE SEEDS, it says the opposite:**
//!
//! ```text
//!   9/d0      5.2s to first self-KO   (identical on every seed)
//!   9/d12     2.7s to first self-KO   (identical on every seed)
//! ```
//!
//! The rollout makes this fighter die SOONER, reproducibly. Two things follow,
//! and the second is the one that matters:
//!
//! * the morning's numbers are not comparable to the evening's — the same
//!   caution the ladder rows carry, and it applies to a table written six hours
//!   apart in the same file;
//! * **at level 9 the outcome does not move with the noise seed at all**, while
//!   levels 1/3/5/6 do. So this A/B is not a sampling accident: within one
//!   build, turning the rollout on costs this fighter half its survival, and
//!   `l3_earns_its_depth` cannot be authored off it.
//!
//! ⛔ it is not a fix either way. Every rung still loses all three stocks inside
//! ~10 s: the brain kills itself, and depth changes only how fast.
//!
//! ## What the two new columns say, and what they rule OUT
//!
//! ```text
//!   1      died at x≈-119 (3L/0R), vmax 400
//!   3      died at x≈-117 (3L/0R), vmax 586
//!   5      died at x≈-116 (3L/0R), vmax 658
//!   6      died at x≈756  (0L/3R), vmax 609
//!   9      died at x≈758  (0L/3R), vmax 752
//!   9/d0   died at x≈752  (0L/3R), vmax 773
//!   9/d12  died at x≈758  (0L/3R), vmax 752
//! ```
//!
//! The platform spans x 110..530 of a 640-wide stage and the blast margin is
//! 120, so `-119` is off the left lip and `756` off the right. Two facts fall
//! straight out, and both narrow the search:
//!
//! * **the direction is a property of the LEVEL, not of the rollout.** Levels
//!   1–5 leave to the left, 6–9 to the right, and level 9 leaves right at BOTH
//!   depths. Whatever the rollout is doing, it is not steering.
//! * **`vmax` is dash speed at both depths** (`dash_speed` is 760; a run tops
//!   out at `MAX_RUN_SPEED` 270). So the fighter DASHES off the stage with the
//!   rollout on and with it off. The rollout does not introduce the dash; it
//!   changes when it happens — 2.7 s instead of 5.2 s.
//!
//! ▢ **so the open question is narrow**: why does the movement veto let a dash
//!   that leaves the stage through? The shadow models a dash as `dash_speed`
//!   held for `dash_time` and then coasts under `ShadowIntent::Hold`, which
//!   zeroes lateral velocity instantly WHEN GROUNDED — while the real body that
//!   dashes off a lip becomes airborne, where `AIR_FRICTION` (650) lets 760 px/s
//!   carry it several hundred px further. Confirming that needs a per-decision
//!   trace of one level-9 match, which this probe does not have and should get
//!   before anybody changes the veto: the last two attempts at this were a
//!   paralysis that read as a 3× improvement and a fix that made the number
//!   worse.
//!
//! ⚠ this is a PROBE, not the ladder rig. It runs one scenario, one opponent,
//! no repeats — enough to say whether depth changes behaviour at all, and not
//! enough to author a row from. §8's scenario suite and the survival/damage
//! ratios are the real thing, and this exists because a first measurement that
//! can be read in one line beats a suite nobody has run.

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
use ambition_platformer2d::characters::brain::{Brain, StateMachineCfg};
use bevy::app::App;

const TICKS: usize = 3_600; // one minute at 60Hz

/// How many execution-noise seeds each configuration is run under.
///
/// ⚠ **ONE run is not a measurement, and this probe reported one for a week.**
/// The brain's noise stream is seeded from the LEVEL, so a configuration ran
/// exactly one match and its time-to-first-self-KO was reported as the answer.
/// Two numbers from two single samples then get read as a difference: on
/// 2026-07-31 the A/B read `d0 5.2s` against `d12 2.7s` — the rollout appearing
/// to make the fighter die twice as fast — from one match each.
///
/// Overridable: `cargo run --bin ladder_probe -- --seeds 7`.
const DEFAULT_SEEDS: usize = 3;

fn main() {
    warn_if_seam_trace_is_unavailable();
    let seeds = seed_count();
    println!(
        "[ladder_probe] level  first_self_KO   survived   stocks_lost  peak%   \
         (median of {seeds} seeds; opponent cannot attack, so every loss is a self-KO)"
    );
    for level in [1u8, 3, 5, 6, 9] {
        report(&run_seeds(level, None, seeds));
    }

    // ── the A/B that is actually FB6e's question ─────────────────────────
    //
    // The ladder column above confounds depth with everything else a rung
    // changes (reaction, APM, execution noise, read weight). `for_level` turns
    // the rollout on at level 6, so level 5 -> 6 is NOT a depth experiment; it
    // is five changes at once. These two rows hold the whole profile fixed and
    // move `rollout_depth` alone, which is the only comparison that can answer
    // "does L3 earn its depth".
    println!("[ladder_probe] --- same level 9 profile, ONLY rollout_depth varied ---");
    for depth in [0u32, 12] {
        report(&run_seeds(9, Some(depth), seeds));
    }
}

/// Print the joined explanation for every subject that acted this tick, then
/// clear the log.
///
/// ⛔ **`[fighter …]` lines on this same stream carry NO TICK, and must not be
/// aligned with `[seam] t=N` by adjacency.** That is deliberate and correct —
/// `trace_decision`'s own doc explains it: a brain five hops below the ECS does
/// not know the world's clock, and a counter guessed there would be a second
/// clock no other domain could join against. The fact it publishes IS stamped;
/// only the stderr rendering is not.
///
/// ⚠ but the two interleave on one stream, and reading them as a pair is a
/// mistake waiting to be made — I made it on 2026-08-02 and briefly concluded
/// the seam and the brain disagreed about a body's velocity (`vx=760` beside
/// `vx=-270`). They may or may not; adjacent lines here are not evidence either
/// way. Compare only `t=`-stamped lines with each other.
///
/// ⚠ **cleared every tick on purpose.** A ladder run is thousands of ticks with
/// several bodies each; a log that accumulated all of it would be a memory
/// profile of the probe rather than a trace. The question here is always "what
/// happened on THIS tick", so the tick is the natural scope.
/// Per-subject `vel_x` as of the previous tick, so an UNCLAIMED velocity step can
/// be detected instead of eyeballed.
///
/// ⛔ **the seam line samples 1-in-5 between decisions, so a three-tick ramp is
/// invisible to it** — which is how S51's `-99`/tick ramp survived six reading
/// cycles. The data was there every tick; only the printing was sampled.
#[cfg(feature = "causal")]
thread_local! {
    static PREV_VEL_X: std::cell::RefCell<std::collections::HashMap<String, f32>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// The last tick this detector saw, so a MATCH BOUNDARY can be recognised.
    ///
    /// ⛔ **without this the detector manufactures its own findings.** The probe
    /// runs many matches in one process under one subject id; carrying a velocity
    /// across the boundary compares the last tick of match N with the first of
    /// match N+1 and reports the difference as an unclaimed step. On the first run
    /// that produced 6 spurious `760.00 -> 0.00` rows at t≤5, character-identical
    /// to the 106 REAL mid-match ones — an artifact shaped exactly like the thing
    /// it hunts, which is this repository's most expensive recurring bug.
    static LAST_TICK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The largest per-tick `vel_x` change the integrator can make WITHOUT announcing
/// an operation — **derived from the kernel's own constants, never from a grep.**
///
/// ⛔ **the first version of this hardcoded `25.0`, from a grep that found a
/// maximum of 900 px/s² and concluded `900/60 = 15`/tick.** That grep matched
/// `field: <number>` struct initialisers and never saw
/// `pub const RUN_ACCEL: f32 = 5200.0` — the actual ceiling, 5.8× higher. The
/// wrong number was used to "eliminate" the integrator from S51's `-99`/tick
/// ramp by arithmetic, and 99.17/tick is comfortably INSIDE `5200/60 = 86.67`
/// plus a per-character override. Modelling the constant set instead of asking
/// for it produced a confident false elimination; importing the constants makes
/// that particular mistake unrepresentable.
///
/// The margin covers per-character tuning that raises `run_accel` above the
/// engine default (the catalog allows it) without swamping the signal.
#[cfg(feature = "causal")]
const UNCLAIMED_STEP_THRESHOLD: f32 = {
    let per_tick = if ambition_platformer2d::engine_core::RUN_ACCEL
        > ambition_platformer2d::engine_core::AIR_ACCEL
    {
        ambition_platformer2d::engine_core::RUN_ACCEL
    } else {
        ambition_platformer2d::engine_core::AIR_ACCEL
    } / 60.0;
    per_tick * 1.5
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
    // A tick that did not advance is a new match: drop the carried velocities so
    // the boundary cannot be read as a step (see `LAST_TICK`).
    LAST_TICK.with(|last| {
        if tick <= last.get() {
            PREV_VEL_X.with(|cell| cell.borrow_mut().clear());
        }
        last.set(tick);
    });
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
        // ⭐ **UNCLAIMED VELOCITY STEPS — checked on EVERY tick, before the
        // sampling filter below.** This is the detector S51 needed and did not
        // have: a step larger than the integrator can produce, with no kernel
        // operation naming a writer.
        //
        // ⚠ it prints every fact KIND on the tick rather than only the operations,
        // because the lesson that thread paid for twice is that a filter is a
        // hypothesis — `knockback_applied` sat in the log through six cycles
        // because the query asked for `contains("hit")`.
        if let Some(vx) = received
            .and_then(|fact| fact.get("vel_x"))
            .and_then(|value| format!("{value}").parse::<f32>().ok())
        {
            let key = format!("{subject}");
            let previous = PREV_VEL_X.with(|cell| cell.borrow().get(&key).copied());
            if let Some(prev) = previous {
                let step = vx - prev;
                if step.abs() > UNCLAIMED_STEP_THRESHOLD && operations.is_empty() {
                    let kinds: Vec<&str> =
                        explanation.facts().iter().map(|fact| fact.kind()).collect();
                    // ⭐ **POSITION discriminates the candidates, and it was in
                    // the fact all along.** A respawn TELEPORTS the body to its
                    // spawn (a large `pos_x` jump on the same tick); a wall stop
                    // or a dash ending does not move it at all. Printing the pose
                    // beside the velocity separates three hypotheses without
                    // building another instrument.
                    let show = |name: &str| {
                        received
                            .and_then(|fact| fact.get(name))
                            .map(|value| format!("{value}"))
                            .unwrap_or_else(|| "-".to_string())
                    };
                    let (pos_x, ground) = (show("pos_x"), show("on_ground"));
                    eprintln!(
                        "[unclaimed] t={tick} {subject} dvx={step:+.4} ({prev:.2} -> {vx:.2}) pos_x={pos_x} ground={ground} ops=[] kinds={kinds:?}"
                    );
                }
            }
            PREV_VEL_X.with(|cell| cell.borrow_mut().insert(key, vx));
        }
        // ⚠ **every DECISION prints, and the frames between them are sampled.**
        // The first version sampled both at 1-in-5 and every decision row came
        // back `asked=- chose=-`, which reads as "the brain decided nothing" and
        // is instead "you looked on the wrong tick". A decision fires on its own
        // cadence — that is what `decision_interval_ticks` IS — so sampling it
        // like a per-tick quantity drops most of them and misreports the rest.
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
            // ⚠ FACING is not decoration. `locomotion.x` is in the body's LOCAL
            // frame, so "holds -1 and travels +270" is a defect only if facing
            // does not explain the sign. Reading the pair without it is how a
            // convention gets reported as a bug.
            field(received, "facing"),
            field(received, "vel_x"),
            field(received, "on_ground"),
            field(received, "dash_charges"),
            field(decided, "chose"),
            // ⛔ **THE KERNEL'S OWN OPERATION, and the field that finally
            // answered this thread** (S51, 2026-08-02). The seam line reported
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
/// ⚠ **the `[seam]` half needs `--features causal`** — it reads the causal log
/// now instead of hand-querying components, and the log is a default-off
/// dependency. [`warn_if_seam_trace_is_unavailable`] says so out loud rather
/// than letting the trace come back missing half its lines.
/// **Say it, rather than printing nothing.**
///
/// `AMBITION_FIGHTER_TRACE=1` on a build without `causal` used to be the worst
/// kind of answer: the `[fighter]` lines appear, the `[seam]` lines do not, and
/// nothing explains the difference — so the reader concludes the seam had
/// nothing to report, which is the opposite of true.
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

/// `--seeds N`, else [`DEFAULT_SEEDS`].
fn seed_count() -> usize {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--seeds" {
            if let Some(n) = args.next().and_then(|n| n.parse().ok()) {
                return n;
            }
        }
    }
    DEFAULT_SEEDS
}

/// One configuration, run under `seeds` different execution-noise streams.
///
/// The seeds are `0..seeds` mixed through the same splitmix constant the brain
/// builder uses, so they are as unrelated to each other as any two levels'
/// streams are — and they are FIXED, so this stays reproducible.
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

/// WHERE it died, as a side rather than a number. The platform spans x 110..530
/// of a 640-wide stage; `<110` is off the left lip and `>530` off the right, and
/// which one it is separates a veto that steers wrong from a veto that is blind.
fn death_side(runs: &[LadderRun]) -> String {
    let xs: Vec<f32> = runs.iter().filter_map(|r| r.death_x).collect();
    if xs.is_empty() {
        return "no self-KO".to_string();
    }
    let left = xs.iter().filter(|x| **x < 320.0).count();
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
    /// The platform spans x 110..530 of a 640-wide stage, so a death off the
    /// LEFT and a death off the RIGHT are different bugs — one is the veto
    /// steering, the other is the veto blind. A time alone cannot tell them
    /// apart, and this probe reported only times until 2026-07-31.
    death_x: Option<f32>,
    lost: u32,
    peak: f32,
}

/// Run one match. `forced_depth` overwrites `rollout_depth` on every fighter
/// brain in the world once the bodies exist, holding the rest of the profile
/// fixed — the intervention that makes the A/B an A/B.
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
        // ⚠ **the APPLIED control, beside the body it is supposed to move.**
        // `AMBITION_FIGHTER_TRACE=1` prints what the BRAIN emitted; this prints
        // what reached `ActorControl`, which is the seam between the brain phase
        // and the movement phase. The two together answer the question the brain
        // trace could only pose: on 2026-07-31 the brain emitted full LEFT for
        // three decisions while the body accelerated RIGHT, and nothing said
        // whether the intent was lost on the way or ignored on arrival.
        // **THE SEAM, AS A JOINED EXPLANATION.**
        //
        // This was a hand-rolled query printing its own `[seam]` line, which
        // made it the SECOND observer of one seam alongside
        // `record_body_control_frame` — same components, free to drift in
        // coverage. Now there is one: the engine publishes the typed fact, and
        // this reads it.
        //
        // What that buys is the whole point of the thread. The brain's
        // `fighter_decision` fact and the body's `control_frame_received` fact
        // carry the SAME subject, so one `explain` returns both — "asked for
        // -1.0, holding -1.0, travelling +588" is one line instead of two
        // streams a human correlates by eye. On 2026-07-31 that correlation was
        // the open question and nothing could state it.
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
                    // The position on the tick BEFORE: a respawn has already
                    // moved the body by the time the stock count drops.
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

/// Overwrite `rollout_depth` on every fighter brain present. Returns whether it
/// found one — the caller asserts on that, because an override that silently
/// applied to nothing turns an A/B into two identical runs reported as a result.
/// Overwrite the execution-noise stream on every fighter brain present, so one
/// configuration can be run under several. Returns whether it found one, for
/// the same reason `force_depth` does: an override that applied to nothing turns
/// N runs into N copies of one run, reported as a distribution.
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
