//! Headless Ambition sandbox driver.
//!
//! Builds the simulation App with no rendering, audio, or windowing plugins
//! and runs `Update` for a fixed number of ticks, then exits. Useful for
//! environments without a display (CI, remote VMs) and as a foundation for
//! future RL drivers that need deterministic stepping. See
//! `crate::headless::run_headless` for what is and is not exercised.
//!
//! Usage:
//!
//! ```bash
//! cargo run -p ambition_sandbox --bin headless                       # 120 ticks (default)
//! cargo run -p ambition_sandbox --bin headless -- 600                # 600 ticks
//! cargo run -p ambition_sandbox --bin headless -- 600 --dump-trace path/  # dump trace to dir
//! cargo run -p ambition_sandbox --bin headless -- 600 --start-room mob_lab
//! ```
//!
//! `--dump-trace DIR` writes a GameplayTraceBuffer JSON+Markdown dump
//! after the final tick so trace_replay can re-drive the same input
//! sequence later. The dump is named `ambition_trace_<timestamp>.json`
//! per the existing dump_paths convention.

use std::path::PathBuf;

use ambition_sandbox::input::ControlFrame;
use ambition_sandbox::rl_sim::{SandboxSim, SandboxSimOptions, TimestepMode};
use ambition_sandbox::trace::{self, record_simulation_frame, DumpReason, GameplayTraceBuffer};

fn parse_max_ticks(args: &[String]) -> u32 {
    // First positional non-flag arg is the tick count.
    for a in args.iter().skip(1) {
        if a.starts_with('-') {
            continue;
        }
        if let Ok(n) = a.parse() {
            return n;
        }
    }
    120
}

fn parse_named_value(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).cloned();
        }
        let prefix = format!("{name}=");
        if args[i].starts_with(&prefix) {
            return Some(args[i].trim_start_matches(&prefix).to_string());
        }
        i += 1;
    }
    None
}

fn run_with_trace_dump(max_ticks: u32, dump_dir: PathBuf, start_room: Option<String>) -> i32 {
    let mut options = SandboxSimOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if let Some(room) = start_room {
        options = options.with_start_room(room);
    }
    let mut sim = match SandboxSim::new_with_options(options) {
        Ok(s) => s,
        Err(error) => {
            eprintln!("headless run failed: {error}");
            return 1;
        }
    };

    // Build a buffer sized to hold the full run so we can replay.
    let mut buffer = GameplayTraceBuffer::with_capacity(max_ticks as usize + 16, 256);

    // Drive the sim manually so we can record each frame after each step.
    // Idle inputs only -- the trace captures the deterministic gameplay
    // baseline; agents that want a richer trace can replay this binary
    // pattern from their own scripted policy.
    use ambition_sandbox::game_mode::GameMode as GameModeState;
    use ambition_sandbox::player::{PlayerEntity, PlayerMovementAuthority};
    use ambition_sandbox::rooms::RoomSet;
    use ambition_sandbox::GameWorld;
    use bevy::prelude::With;
    use bevy::state::state::State;

    for _ in 0..max_ticks {
        sim.step(ambition_sandbox::AgentAction::default());

        // Read the authoritative player state from the ECS component.
        let player = {
            let mut q = sim
                .world_mut()
                .query_filtered::<&PlayerMovementAuthority, With<PlayerEntity>>();
            q.single(sim.world())
                .map(|a| a.player.clone())
                .unwrap_or_else(|_| {
                    ambition_engine::Player::new_with_abilities(
                        ambition_engine::Vec2::ZERO,
                        ambition_engine::AbilitySet::default(),
                    )
                })
        };
        let world_ref = sim.world();
        let game_world = world_ref.resource::<GameWorld>();
        let control_frame = world_ref.resource::<ControlFrame>();
        let room_set = world_ref.resource::<RoomSet>();
        let game_mode = world_ref.resource::<State<GameModeState>>();
        let sim_state = world_ref.resource::<ambition_sandbox::SandboxSimState>();
        let moving_platforms = world_ref.resource::<ambition_sandbox::MovingPlatformSet>();
        let active_area = room_set.active_spec().id.clone();
        let mode_label = format!("{:?}", game_mode.get());
        let locomotion_state = ambition_engine::LocomotionState::from_player(&player);
        let body_mode_state = ambition_engine::BodyMode::from_player(&player);
        record_simulation_frame(
            &mut buffer,
            &player,
            sim_state,
            &game_world.0,
            *control_frame,
            1.0 / 60.0,
            1.0 / 60.0,
            &mode_label,
            &active_area,
            &moving_platforms.0,
            locomotion_state.label(),
            body_mode_state.label(),
        );
    }

    match trace::write_dump(
        &buffer,
        &DumpReason::Programmatic {
            label: "headless".into(),
        },
        &dump_dir,
    ) {
        Ok(path) => {
            println!(
                "headless run completed: {} ticks; trace dumped to {}",
                max_ticks,
                path.display()
            );
            0
        }
        Err(error) => {
            eprintln!("headless trace dump failed: {error}");
            1
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let max_ticks = parse_max_ticks(&args);
    let dump_dir = parse_named_value(&args, "--dump-trace").map(PathBuf::from);
    let start_room = parse_named_value(&args, "--start-room");

    if let Some(dir) = dump_dir {
        let code = run_with_trace_dump(max_ticks, dir, start_room);
        std::process::exit(code);
    }

    // Plain run: use the existing run_headless entry point. start_room
    // override is only honored on the trace-dump path because the
    // existing run_headless doesn't take options yet (and tests/CI
    // depend on the no-arg form).
    if start_room.is_some() {
        eprintln!(
            "headless: --start-room only takes effect with --dump-trace (run_headless takes no options yet)"
        );
    }
    match ambition_sandbox::run_headless(max_ticks) {
        Ok(report) => {
            println!("{report}");
        }
        Err(error) => {
            eprintln!("headless run failed: {error}");
            std::process::exit(1);
        }
    }
}
