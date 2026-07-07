//! Random-walker RL driver for the Ambition sandbox.
//!
//! Drives `SandboxSim` with a small LCG-seeded random policy so the
//! simulation gets exercised without a human at the keyboard. Useful as:
//!
//! - **Fuzz harness** — a long random walk surfaces movement / collision
//!   bugs that don't show up in scripted tests (sticky walls, OOB
//!   teleports, mid-air-stuck states, etc.).
//! - **End-to-end SandboxSim demonstration** — one of the simplest
//!   possible RL agents you can write against the Ambition step API.
//!   The policy here is `epsilon=1.0` random — replace `RandomWalkPolicy`
//!   with a learned policy and you're training.
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_app --bin rl_random_walker            # 600 steps, seed=1
//! cargo run -p ambition_app --bin rl_random_walker -- 1200 42 # 1200 steps, seed=42
//! ```
//!
//! Prints a per-100-step heartbeat plus end-of-run summary (final pos,
//! room, hp, total resets, dash count, jump count, max distance from
//! spawn).

use ambition_app::{AgentObservation, RandomWalkPolicy, SandboxSim};

#[derive(Default, Clone, Copy)]
struct RunStats {
    jumps: u32,
    dashes: u32,
    blinks: u32,
    attacks: u32,
    interacts: u32,
    resets: u32,
    damage_events: u32,
    max_dist_from_spawn: f32,
    rooms_visited: u32,
}

fn run_random_walk(steps: u32, seed: u64) {
    let mut sim = match SandboxSim::new() {
        Ok(sim) => sim,
        Err(error) => {
            eprintln!("rl_random_walker: failed to construct SandboxSim: {error}");
            std::process::exit(1);
        }
    };
    let mut policy = RandomWalkPolicy::demo(seed);
    let mut stats = RunStats::default();
    let initial = sim.observation();
    let mut last_room = initial.active_room.clone();
    let mut last_recently_damaged = initial.recently_damaged;

    println!(
        "rl_random_walker: seed={seed} steps={steps} initial_room={} hp={}/{} pos=({:.1},{:.1})",
        initial.active_room, initial.hp, initial.hp_max, initial.player_pos.0, initial.player_pos.1
    );

    for step in 1..=steps {
        let action = policy.act();
        if action.jump {
            stats.jumps += 1;
        }
        if action.dash {
            stats.dashes += 1;
        }
        if action.blink {
            stats.blinks += 1;
        }
        if action.attack {
            stats.attacks += 1;
        }
        if action.interact {
            stats.interacts += 1;
        }
        if action.reset {
            stats.resets += 1;
        }
        let obs = sim.step(action);
        let dx = obs.player_pos.0 - initial.world_spawn.0;
        let dy = obs.player_pos.1 - initial.world_spawn.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > stats.max_dist_from_spawn {
            stats.max_dist_from_spawn = dist;
        }
        if obs.active_room != last_room {
            stats.rooms_visited += 1;
            println!(
                "  [step {step:>5}] room transition: {} -> {} (pos=({:.1},{:.1}))",
                last_room, obs.active_room, obs.player_pos.0, obs.player_pos.1
            );
            last_room = obs.active_room.clone();
        }
        if obs.recently_damaged && !last_recently_damaged {
            stats.damage_events += 1;
        }
        last_recently_damaged = obs.recently_damaged;
        if step % 100 == 0 {
            print_heartbeat(step, &obs, &stats);
        }
    }

    let final_obs = sim.observation();
    println!("--- run complete ---");
    println!("final tick      : {}", final_obs.tick);
    println!("final room      : {}", final_obs.active_room);
    println!(
        "final pos       : ({:.1}, {:.1}) (max distance from spawn: {:.1})",
        final_obs.player_pos.0, final_obs.player_pos.1, stats.max_dist_from_spawn
    );
    println!(
        "final hp        : {}/{} ({:.0}%)",
        final_obs.hp,
        final_obs.hp_max,
        final_obs.hp_fraction() * 100.0
    );
    println!("player resets   : {}", final_obs.resets);
    println!("rooms visited   : {}", stats.rooms_visited + 1); // +1 for initial
    println!("damage events   : {}", stats.damage_events);
    println!(
        "actions sent    : jumps={} dashes={} blinks={} attacks={} interacts={} resets={}",
        stats.jumps, stats.dashes, stats.blinks, stats.attacks, stats.interacts, stats.resets
    );
}

fn print_heartbeat(step: u32, obs: &AgentObservation, stats: &RunStats) {
    println!(
        "  [step {step:>5}] room={} pos=({:.1},{:.1}) vel=({:.1},{:.1}) hp={}/{} jumps={} dashes={}",
        obs.active_room,
        obs.player_pos.0,
        obs.player_pos.1,
        obs.player_vel.0,
        obs.player_vel.1,
        obs.hp,
        obs.hp_max,
        stats.jumps,
        stats.dashes
    );
}

fn parse_arg<T: std::str::FromStr>(args: &[String], idx: usize) -> Option<T> {
    args.get(idx).and_then(|raw| raw.parse().ok())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let steps: u32 = parse_arg(&args, 1).unwrap_or(600);
    let seed: u64 = parse_arg(&args, 2).unwrap_or(1);
    run_random_walk(steps, seed);
}
