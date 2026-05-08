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
//!   The policy here is `epsilon=1.0` random — replace `RandomPolicy`
//!   with a learned policy and you're training.
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_sandbox --bin rl_random_walker            # 600 steps, seed=1
//! cargo run -p ambition_sandbox --bin rl_random_walker -- 1200 42 # 1200 steps, seed=42
//! ```
//!
//! Prints a per-100-step heartbeat plus end-of-run summary (final pos,
//! room, hp, total resets, dash count, jump count, max distance from
//! spawn).

use ambition_sandbox::{AgentAction, AgentObservation, SandboxSim};

/// Simple LCG. Plenty for fuzzing — we don't need cryptographic
/// quality, just deterministic + cheap.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        // Avoid all-zero state.
        Self(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        // Numerical Recipes-style 64-bit LCG.
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    fn unit(&mut self) -> f32 {
        // [0, 1)
        (self.next_u32() as f32) / (u32::MAX as f32 + 1.0)
    }

    fn signed_unit(&mut self) -> f32 {
        // [-1, 1)
        2.0 * self.unit() - 1.0
    }

    fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }
}

/// Per-step action probabilities. Tuned so the agent moves around a lot
/// (walks + jumps frequently), occasionally dashes / blinks, and rarely
/// presses Reset (which would short-circuit the run). No projectile or
/// fly-toggle by default — both are situational and a uniform random
/// agent ends up spamming them.
struct RandomPolicy {
    rng: Lcg,
    /// Sticky horizontal axis. Each frame we pick a new axis with
    /// probability `axis_change_chance`; otherwise the previous axis
    /// holds. This avoids the "vibrate at 0 velocity" failure mode of
    /// purely IID action sampling.
    axis_x: f32,
    axis_change_chance: f32,
    jump_chance: f32,
    jump_hold_chance: f32,
    dash_chance: f32,
    blink_chance: f32,
    attack_chance: f32,
    interact_chance: f32,
    reset_chance: f32,
}

impl RandomPolicy {
    fn new(seed: u64) -> Self {
        Self {
            rng: Lcg::new(seed),
            axis_x: 0.0,
            axis_change_chance: 0.06,
            jump_chance: 0.05,
            jump_hold_chance: 0.5,
            dash_chance: 0.02,
            blink_chance: 0.005,
            attack_chance: 0.01,
            interact_chance: 0.01,
            reset_chance: 0.0005,
        }
    }

    fn act(&mut self) -> AgentAction {
        if self.rng.chance(self.axis_change_chance) {
            // Bias toward the cardinals: half the time -1/+1, half the
            // time pure analog. Keeps the test exercising both "stick
            // pinned" and "stick partial" inputs.
            self.axis_x = if self.rng.chance(0.5) {
                if self.rng.chance(0.5) {
                    1.0
                } else {
                    -1.0
                }
            } else {
                self.rng.signed_unit()
            };
        }
        let jump = self.rng.chance(self.jump_chance);
        // Aim sticks: when a blink fires, set a random aim direction
        // so blink targets get exercised (precision-blink reads aim
        // x/y). Idle frames use small drift so the aim deadzone code
        // sees both 0-magnitude and partial-magnitude stick reads.
        let blink = self.rng.chance(self.blink_chance);
        let (aim_x, aim_y) = if blink {
            // Random unit-ish vector when blinking.
            let dx = self.rng.signed_unit();
            let dy = self.rng.signed_unit();
            (dx, dy)
        } else {
            // Drifting partial-magnitude aim, scaled to stay in the
            // deadzone band most frames. This exercises the aim
            // deadzone code path (filter out drift) without firing
            // blink targets randomly.
            (self.rng.signed_unit() * 0.05, self.rng.signed_unit() * 0.05)
        };
        AgentAction {
            move_x: self.axis_x,
            move_y: 0.0,
            up_pressed: false,
            down_pressed: false,
            jump,
            jump_held: jump || self.rng.chance(self.jump_hold_chance),
            jump_released: false,
            dash: self.rng.chance(self.dash_chance),
            attack: self.rng.chance(self.attack_chance),
            blink,
            blink_held: false,
            blink_released: false,
            pogo: false,
            interact: self.rng.chance(self.interact_chance),
            projectile: false,
            projectile_held: false,
            projectile_released: false,
            fly_toggle: false,
            reset: self.rng.chance(self.reset_chance),
            start: false,
            aim_x,
            aim_y,
        }
    }
}

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
    let mut policy = RandomPolicy::new(seed);
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
