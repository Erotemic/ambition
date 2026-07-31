//! **Does a rollout buy anything?** (FB6e — `l3_earns_its_depth`)
//!
//! `cargo run -p ambition_demo_smash_app --bin ladder_probe`
//!
//! Every ladder row in this repository has carried `rollout_depth: 0`, and the
//! reason is written down: the depth cannot be authored until something proves
//! it buys something. FB6a–e built the rollout; nothing had ever measured a
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
//! ⚠ this is a PROBE, not the ladder rig. It runs one scenario, one opponent,
//! no repeats — enough to say whether depth changes behaviour at all, and not
//! enough to author a row from. §8's scenario suite and the survival/damage
//! ratios are the real thing, and this exists because a first measurement that
//! can be read in one line beats a suite nobody has run.

use ambition::actor::{FighterStocks, MatchSeat};
use ambition::characters::brain::{Brain, StateMachineCfg};
use ambition_demo_smash_app::build_demo_app;
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
        "[ladder_probe]   {:<5}   {:>13}  {:>13}   {}         {:.0}%",
        tag,
        spread_label(runs.iter().map(|r| r.first_loss)),
        spread_label(runs.iter().map(|r| r.eliminated)),
        median(runs.iter().map(|r| r.lost as usize)).unwrap_or(0),
        runs.iter().map(|r| r.peak).fold(0.0f32, f32::max) * 100.0,
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
    lost: u32,
    peak: f32,
}

/// Run one match. `forced_depth` overwrites `rollout_depth` on every fighter
/// brain in the world once the bodies exist, holding the rest of the profile
/// fixed — the intervention that makes the A/B an A/B.
fn run_one(level: u8, forced_depth: Option<u32>, noise_seed: u64) -> LadderRun {
    let mut app = build_demo_app();
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
        .write_message(ambition::game_shell::ShellCommand::GoTo(
            ambition::game_shell::ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        ));

    let mut lost = 0u32;
    let mut peak = 0.0f32;
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
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &FighterStocks,
            &ambition::characters::actor::BodyHealth,
        )>();
        let mut present = false;
        for (seat, stocks, health) in q.iter(world) {
            if seat.0 == 1 {
                present = true;
                let now = ambition_demo_smash::STARTING_STOCKS.saturating_sub(stocks.remaining);
                if now > 0 && first_loss.is_none() {
                    first_loss = Some(tick);
                }
                lost = lost.max(now);
                peak = peak.max(health.damage_percent());
            }
        }
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
