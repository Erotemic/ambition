//! The production ROLLBACK host, headless, in one room, for N ticks: wall time
//! per tick, from the clock, with no Tracy layer in the process.
//!
//! Why this exists next to `--headless -- --start-room X`: that flag selects
//! the direct sandbox host, which installs no rollback session, so its
//! per-tick numbers are not the shipped simulation's. This drives the same
//! `Platformer2dSimHarness` the integration tests use, with the shipped local
//! session (a sync test at `check_distance: 0`), so `GgrsSchedule` is the
//! thing being timed.
//!
//! ```sh
//! AMBITION_ACTOR_POPULATION_CAP=64 cargo run --example hall_bench --profile profiling -- --ticks 3000
//! ```
//!
//! Set `AMBITION_PROFILE_CENSUS=1` for the `[census] sim_phases` rows on stderr.

use std::time::Instant;

use ambition_app::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions,
    TimestepMode,
};

fn main() {
    let mut room = "hall_of_characters".to_string();
    let mut ticks: usize = 3000;
    let mut warmup: usize = 300;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--room" => room = it.next().expect("--room ID"),
            "--ticks" => ticks = it.next().expect("--ticks N").parse().expect("--ticks N"),
            "--warmup" => warmup = it.next().expect("--warmup N").parse().expect("--warmup N"),
            other => {
                eprintln!("hall_bench: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }

    let options = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(room.clone())
        // The shipped local session: a sync test that never re-simulates.
        .with_sync_test_rollback_settings(0, 12);
    let mut sim = Platformer2dSimHarness::new_with_options(options).expect("harness");

    let idle = AgentAction::default();
    for _ in 0..warmup {
        sim.step(idle);
    }
    // LOUD if the room is not the one asked for: a benchmark of the wrong room
    // reports a number for the right one.
    let active = sim.observation().active_room;
    assert_eq!(active, room, "after {warmup} ticks the active room is {active:?}, not {room:?}");

    // One-second windows of wall time per tick, so the printed number is a
    // median over windows rather than a mean a single hitch can own.
    let mut windows: Vec<f64> = Vec::new();
    let mut window_start = Instant::now();
    let mut window_ticks = 0usize;
    let run_start = Instant::now();
    for _ in 0..ticks {
        sim.step(idle);
        window_ticks += 1;
        let elapsed = window_start.elapsed();
        if elapsed.as_secs_f64() >= 1.0 {
            windows.push(elapsed.as_secs_f64() * 1000.0 / window_ticks as f64);
            window_start = Instant::now();
            window_ticks = 0;
        }
    }
    let total_ms = run_start.elapsed().as_secs_f64() * 1000.0;
    windows.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = windows.get(windows.len() / 2).copied().unwrap_or(total_ms / ticks as f64);
    let cap = std::env::var("AMBITION_ACTOR_POPULATION_CAP").unwrap_or_default();
    println!(
        "[hall_bench] room={room} cap={cap:?} ticks={ticks} windows={} ms_per_tick median={median:.3} mean={:.3} min={:.3} max={:.3}",
        windows.len(),
        total_ms / ticks as f64,
        windows.first().copied().unwrap_or(f64::NAN),
        windows.last().copied().unwrap_or(f64::NAN),
    );
}
