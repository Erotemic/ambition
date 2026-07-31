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
//! each level and count how many stocks the CPU loses. Its opponent is a human
//! seat with no controller and cannot attack, so **every stock lost is a
//! self-KO** — the brain walking or jumping off the stage. That makes the number
//! unusually clean: it is not "did it win", it is "did it kill itself", which is
//! the one thing a difficulty curve cannot survive.
//!
//! ⚠ this is a PROBE, not the ladder rig. It runs one scenario, one opponent,
//! no repeats — enough to say whether depth changes behaviour at all, and not
//! enough to author a row from. §8's scenario suite and the survival/damage
//! ratios are the real thing, and this exists because a first measurement that
//! can be read in one line beats a suite nobody has run.

use ambition::actor::{FighterStocks, MatchSeat};
use ambition_demo_smash_app::build_demo_app;
use bevy::prelude::*;

const TICKS: usize = 3_600; // one minute at 60Hz

fn main() {
    println!("[ladder_probe] level  stocks_lost  peak%   (opponent cannot attack: every loss is a self-KO)");
    for level in [1u8, 3, 5, 6, 9] {
        let (lost, peak) = run_one(level);
        println!("[ladder_probe]   {level}        {lost}         {:.0}%", peak * 100.0);
    }
}

fn run_one(level: u8) -> (u32, f32) {
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
    for _ in 0..TICKS {
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            &FighterStocks,
            &ambition::characters::actor::BodyHealth,
        )>();
        for (seat, stocks, health) in q.iter(world) {
            if seat.0 == 1 {
                lost = lost.max(
                    ambition_demo_smash::STARTING_STOCKS.saturating_sub(stocks.remaining),
                );
                peak = peak.max(health.damage_percent());
            }
        }
    }
    // A fighter that despawned lost everything it had left.
    let world = app.world_mut();
    let mut alive = world.query::<&MatchSeat>();
    if !alive.iter(world).any(|seat| seat.0 == 1) {
        lost = ambition_demo_smash::STARTING_STOCKS;
    }
    (lost, peak)
}
