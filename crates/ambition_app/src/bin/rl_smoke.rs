//! Smoke test: visit every room in the sandbox via SandboxSim and run
//! a small random-walker policy for a fixed number of steps. Catches
//! regressions where a specific room panics on construction (LDtk
//! validation, encounter/boss registry init, IntGrid layer parsing,
//! …) or under any random input combination.
//!
//! For each room id (returned by `SandboxSim::room_ids`):
//! 1. Build a fresh `SandboxSim` starting in that room (fixed-60Hz).
//! 2. Run an LCG-seeded random policy for `steps` ticks.
//! 3. Assert: HP stays in [0, hp_max], position stays finite + bounded.
//! 4. Report per-room max distance from spawn + final HP.
//!
//! On the first failure the binary exits non-zero with the room id in
//! the message so the regression is reproducible. Useful as a CI
//! check (mirrors `cargo run --bin headless` but exercises every
//! room rather than just the start).
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_gameplay_core --bin rl_smoke               # 200 steps per room, seed=1
//! cargo run -p ambition_gameplay_core --bin rl_smoke -- 500 42     # 500 steps, seed=42
//! ```

use ambition_app::rl_sim::TimestepMode;
use ambition_app::{RandomWalkPolicy, SandboxSim, SandboxSimOptions};

fn smoke_room(room_id: &str, steps: u32, seed: u64) -> Result<RoomReport, String> {
    let mut sim = SandboxSim::new_with_options(
        SandboxSimOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_start_room(room_id),
    )
    .map_err(|e| format!("room '{room_id}': SandboxSim::new failed: {e}"))?;
    let initial = sim.observation();
    if initial.active_room != room_id {
        // start_room override falls back to authored start when the id
        // doesn't resolve. For the smoke test that's a soft fail (not
        // a panic) since the sim still ran — we report it but don't
        // exit. This usually means a room id with a different
        // active-area name in LDtk.
        eprintln!(
            "  [{room_id}] start_room override fell back to '{}' (likely id-vs-active-area mismatch)",
            initial.active_room
        );
    }
    let mut policy = RandomWalkPolicy::fuzz(seed.wrapping_add(hash_room_id(room_id)));
    let mut max_dist: f32 = 0.0;
    let mut hp_drained = false;

    for step in 0..steps {
        let action = policy.act();
        let obs = sim.step(action);
        if !obs.player_pos.0.is_finite() || !obs.player_pos.1.is_finite() {
            return Err(format!(
                "room '{room_id}' step {step}: non-finite player position {:?}",
                obs.player_pos
            ));
        }
        if obs.player_pos.0.abs() > 1.0e6 || obs.player_pos.1.abs() > 1.0e6 {
            return Err(format!(
                "room '{room_id}' step {step}: player position exploded {:?}",
                obs.player_pos
            ));
        }
        if obs.hp < 0 || obs.hp > obs.hp_max {
            return Err(format!(
                "room '{room_id}' step {step}: hp out of range {} of {}",
                obs.hp, obs.hp_max
            ));
        }
        let dx = obs.player_pos.0 - initial.world_spawn.0;
        let dy = obs.player_pos.1 - initial.world_spawn.1;
        let d = (dx * dx + dy * dy).sqrt();
        if d > max_dist {
            max_dist = d;
        }
        if obs.hp == 0 {
            hp_drained = true;
        }
    }
    let final_obs = sim.observation();
    Ok(RoomReport {
        room_id: room_id.to_string(),
        active_room: final_obs.active_room.clone(),
        ticks: final_obs.tick,
        max_dist,
        final_hp: final_obs.hp,
        max_hp: final_obs.hp_max,
        hp_drained_during_run: hp_drained,
    })
}

#[allow(dead_code)] // some fields are read only via Debug formatting in failures
#[derive(Debug)]
struct RoomReport {
    room_id: String,
    active_room: String,
    ticks: u64,
    max_dist: f32,
    final_hp: i32,
    max_hp: i32,
    hp_drained_during_run: bool,
}

fn hash_room_id(s: &str) -> u64 {
    // Tiny string hash so different rooms get different seeds. Doesn't
    // need to be cryptographic; just want jitter so all rooms don't
    // run the same action sequence.
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// Parse `--rooms a,b,c` from argv. Returns the set of room ids the
/// caller wants to limit the smoke run to, or `None` for "all rooms".
fn parse_rooms_filter(args: &[String]) -> Option<Vec<String>> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--rooms" {
            return args
                .get(i + 1)
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
        }
        if let Some(rest) = args[i].strip_prefix("--rooms=") {
            return Some(rest.split(',').map(|s| s.trim().to_string()).collect());
        }
        i += 1;
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // Skip --rooms args when reading positional STEPS / SEED.
    let positionals: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--rooms") && !a.contains("--rooms="))
        .filter(|a| {
            !args
                .iter()
                .any(|prev| prev == "--rooms" && std::ptr::eq(*a, &args[0]))
        })
        .collect();
    let steps: u32 = positionals
        .first()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200);
    let seed: u64 = positionals.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let filter = parse_rooms_filter(&args);

    // Build a sim once just to enumerate the room ids.
    let scout = SandboxSim::new().expect("SandboxSim::new should succeed");
    let all_room_ids = scout.room_ids();
    drop(scout);

    let room_ids: Vec<String> = match &filter {
        Some(picks) => all_room_ids
            .iter()
            .filter(|id| picks.iter().any(|p| p == *id))
            .cloned()
            .collect(),
        None => all_room_ids.clone(),
    };
    if let Some(ref picks) = filter {
        // Warn about any picks that didn't resolve, so a typo
        // doesn't silently produce a 0-room run.
        for p in picks {
            if !all_room_ids.iter().any(|id| id == p) {
                eprintln!("rl_smoke: warning: --rooms entry '{p}' did not match any known room");
            }
        }
    }
    println!(
        "rl_smoke: visiting {} rooms × {steps} steps (seed={seed}{})",
        room_ids.len(),
        if filter.is_some() {
            ", filtered"
        } else {
            ", all rooms"
        }
    );

    let mut failures = Vec::<String>::new();
    let mut reports = Vec::with_capacity(room_ids.len());
    for room_id in &room_ids {
        print!("  [{:25}] ", room_id);
        match smoke_room(room_id, steps, seed) {
            Ok(report) => {
                println!(
                    "ok   ticks={} active={} max_dist={:.0} hp={}/{}{}",
                    report.ticks,
                    report.active_room,
                    report.max_dist,
                    report.final_hp,
                    report.max_hp,
                    if report.hp_drained_during_run {
                        " (hp_drained_during_run)"
                    } else {
                        ""
                    }
                );
                reports.push(report);
            }
            Err(message) => {
                println!("FAIL");
                failures.push(message);
            }
        }
    }

    println!("--- summary ---");
    println!("rooms ok    : {}", reports.len());
    println!("rooms failed: {}", failures.len());
    for f in &failures {
        eprintln!("  - {f}");
    }
    if !failures.is_empty() {
        std::process::exit(1);
    }
}
