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
//! **What it measured, 2026-07-31** (`level 9`, whole profile fixed, only
//! `rollout_depth` moved):
//!
//! ```text
//!   9/d0      5.0s to first self-KO
//!   9/d12     9.8s to first self-KO
//! ```
//!
//! That is the first evidence in this repository that L3 buys anything, and it
//! took three fixes to get a signal at all: the rollout had to roll MOVEMENT
//! lines (it only ever refined attacks), the shadow floor had to have an EDGE,
//! and the veto's horizon had to be long enough to reach one. Before those, this
//! table read `3 3 3 3 3` — a saturated metric over a blind model.
//!
//! ⛔ it is not a fix. Every rung still loses all three stocks inside ~16 s; the
//! brain kills itself half as fast, which is progress and not competence.
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

fn main() {
    println!(
        "[ladder_probe] level  first_self_KO  survived  stocks_lost  peak%   \
         (opponent cannot attack: every loss is a self-KO)"
    );
    for level in [1u8, 3, 5, 6, 9] {
        report(run_one(level, None));
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
        report(run_one(9, Some(depth)));
    }
}

fn report(r: LadderRun) {
    let tag = match r.forced_depth {
        Some(d) => format!("9/d{d}"),
        None => r.level.to_string(),
    };
    println!(
        "[ladder_probe]   {:<5}   {:>6}      {:>6}       {}         {:.0}%",
        tag,
        tick_label(r.first_loss, TICKS),
        tick_label(r.eliminated, TICKS),
        r.lost,
        r.peak * 100.0,
    );
}

/// `"5.4s"`, or `">60s"` when the event never happened inside the window — the
/// difference matters and a bare tick count hides it.
fn tick_label(tick: Option<usize>, window: usize) -> String {
    match tick {
        Some(t) => format!("{:.1}s", t as f32 / 60.0),
        None => format!(">{}s", window / 60),
    }
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
fn run_one(level: u8, forced_depth: Option<u32>) -> LadderRun {
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
    for tick in 0..TICKS {
        app.update();
        if !depth_applied {
            depth_applied = force_depth(&mut app, forced_depth.unwrap());
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
