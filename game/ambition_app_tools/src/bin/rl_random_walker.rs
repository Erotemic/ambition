//! Random-walker RL driver for the Ambition sandbox.
//!
//! Drives `Platformer2dSimHarness` with a small LCG-seeded random policy so the
//! simulation gets exercised without a human at the keyboard. Useful as:
//!
//! - Fuzz harness — a long random walk surfaces movement / collision
//!   bugs that don't show up in scripted tests (sticky walls, OOB
//!   teleports, mid-air-stuck states, etc.).
//! - End-to-end Platformer2dSimHarness demonstration — one of the simplest
//!   possible RL agents you can write against the Ambition step API.
//!   The policy here is `epsilon=1.0` random — replace `RandomWalkPolicy`
//!   with a learned policy and you're training.
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_app_tools --bin rl_random_walker            # 600 steps, seed=1
//! cargo run -p ambition_app_tools --bin rl_random_walker -- 1200 42 # 1200 steps, seed=42
//! ```
//!
//! Prints a per-100-step heartbeat plus end-of-run summary (final pos,
//! room, hp, total resets, dash count, jump count, max distance from
//! spawn).

use ambition_app::{AgentObservation, AmbitionSim, Platformer2dSimHarness, RandomWalkPolicy};

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
    let mut sim = match Platformer2dSimHarness::new() {
        Ok(sim) => sim,
        Err(error) => {
            eprintln!("rl_random_walker: failed to construct Platformer2dSimHarness: {error}");
            std::process::exit(1);
        }
    };
    // the unclaimed-velocity detector, in the composition the S51 ramp was
    // actually observed in. That trace was taken on `Seat(0)` — the SANDBOX's
    // own player — and the first detector was wired into the smash ladder, which
    // has two seated duelists and no sandbox player. It could never have seen the
    // ramp at any threshold. This binary is the right host: a long random walk is
    // already the repo's fuzz harness for "movement / collision bugs that do not
    // show up in scripted tests", which is exactly the population here.
    #[cfg(feature = "causal")]
    let mut unclaimed = {
        sim.app_mut()
            .add_plugins(ambition_platformer2d::causal::CausalPlugin);
        ambition_platformer2d::causal::record_domains(
            sim.app_mut(),
            ambition_platformer2d::causal::RecordingPolicy::All,
        );
        ambition_platformer2d::causal::UnclaimedStepDetector::new()
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
        #[cfg(feature = "causal")]
        report_unclaimed_steps(&mut sim, &mut unclaimed);
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
    #[cfg(feature = "causal")]
    report_vacuity();
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

/// what a ZERO from this detector must be able to mean.
///
/// A run that reports no unclaimed steps is indistinguishable from a run where
/// the recorder published nothing at all — and this session has already produced
/// three false absences from exactly that confusion. So the binary counts what it
/// SAW, and says so at the end: ticks observed, facts read, subjects carrying a
/// control frame. A zero beside `facts_seen: 0` is an instrument that was never
/// live; a zero beside a large count is a finding.
#[cfg(feature = "causal")]
#[derive(Default)]
struct Vacuity {
    ticks_seen: u64,
    facts_seen: usize,
    frames_seen: u64,
    no_tick: u64,
}

#[cfg(feature = "causal")]
thread_local! {
    static VACUITY: std::cell::RefCell<Vacuity> = std::cell::RefCell::new(Vacuity::default());
}

#[cfg(feature = "causal")]
fn report_vacuity() {
    VACUITY.with(|v| {
        let v = v.borrow();
        eprintln!(
            "[unclaimed] instrument: ticks_seen={} facts_seen={} control_frames={} ticks_without_a_stamp={}",
            v.ticks_seen, v.facts_seen, v.frames_seen, v.no_tick
        );
        if v.facts_seen == 0 {
            eprintln!(
                "[unclaimed] ⛔ NOTHING WAS RECORDED — a zero above is the instrument \
                 being absent, not the sandbox being clean."
            );
        }
    });
}

/// Feed this tick's velocities to the detector and print anything no operation
/// claimed.
///
/// the bound is derived from the kernel's own constants, never written
/// down as a number: a hardcoded threshold was wrong twice — once 5.8× too low
/// from a grep that missed `pub const RUN_ACCEL`, once too HIGH from a safety
/// margin for per-character tuning that does not exist in the tree, which put the
/// bar above the very ramp this is here to find.
#[cfg(feature = "causal")]
fn report_unclaimed_steps(
    sim: &mut Platformer2dSimHarness,
    detector: &mut ambition_platformer2d::causal::UnclaimedStepDetector,
) {
    use ambition_platformer2d::engine_core::{AIR_ACCEL, RUN_ACCEL};

    let max_step = if RUN_ACCEL > AIR_ACCEL {
        RUN_ACCEL
    } else {
        AIR_ACCEL
    } / 60.0
        * 1.01;
    let Some(log) = sim
        .world_mut()
        .get_resource::<ambition_platformer2d::causal::CausalRecording>()
    else {
        return;
    };
    let Some(tick) = log.tick() else {
        VACUITY.with(|v| v.borrow_mut().no_tick += 1);
        return;
    };
    let mut findings = Vec::new();
    VACUITY.with(|v| {
        let mut v = v.borrow_mut();
        v.ticks_seen += 1;
        v.facts_seen += log.len();
    });
    for subject in log.subjects_on(tick) {
        let explanation = log.explain(tick, &subject);
        let Some(frame) = explanation.first("control_frame_received") else {
            continue;
        };
        VACUITY.with(|v| v.borrow_mut().frames_seen += 1);
        let Some(vel_x) = frame
            .get("vel_x")
            .and_then(|value| format!("{value}").parse::<f32>().ok())
        else {
            continue;
        };
        let had_operation = explanation
            .facts()
            .iter()
            .any(|fact| fact.kind() == "movement_operation");
        if let Some(step) =
            detector.observe(tick, &format!("{subject}"), vel_x, had_operation, max_step)
        {
            let show = |name: &str| {
                frame
                    .get(name)
                    .map(|value| format!("{value}"))
                    .unwrap_or_else(|| "-".to_string())
            };
            findings.push(format!(
                "[unclaimed] t={} {} dvx={:+.4} ({:.2} -> {:.2}) pos=({},{}) ground={}",
                step.tick,
                step.subject,
                step.delta(),
                step.before,
                step.after,
                show("pos_x"),
                show("pos_y"),
                show("on_ground"),
            ));
        }
    }
    for line in findings {
        eprintln!("{line}");
    }
    // Clear after each tick so findings describe the current tick and the probe
    // does not accumulate an unbounded history.
    sim.world_mut()
        .get_resource_mut::<ambition_platformer2d::causal::CausalRecording>()
        .map(|mut log| log.clear());
}
