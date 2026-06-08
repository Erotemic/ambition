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
//! cargo run -p ambition_sandbox --bin headless -- 600 --start-room goblin_encounter
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
    use ambition_sandbox::player::{PlayerEntity, PlayerSafetyState};
    use ambition_sandbox::rooms::RoomSet;
    use ambition_sandbox::GameWorld;
    use bevy::prelude::With;
    use bevy::state::state::State;

    for _ in 0..max_ticks {
        sim.step(ambition_sandbox::AgentAction::default());

        // Clone the resources record_simulation_frame needs as owned
        // values so the immutable borrow on `sim` ends before we take
        // the mutable cluster borrow below. ae::World + Vec<...> + the
        // ClockState resource are all `Clone`, so this is cheap
        // for a once-per-tick trace dump.
        let (clock, control_frame, active_area, mode_label, moving_platforms, game_world) = {
            let world_ref = sim.world();
            let clock = *world_ref.resource::<ambition_sandbox::ClockState>();
            let control_frame = *world_ref.resource::<ControlFrame>();
            let room_set = world_ref.resource::<RoomSet>();
            let game_mode = world_ref.resource::<State<GameModeState>>();
            let moving_platforms = world_ref.resource::<ambition_sandbox::MovingPlatformSet>();
            let game_world = world_ref.resource::<GameWorld>();
            let active_area = room_set.active_spec().id.clone();
            let mode_label = format!("{:?}", game_mode.get());
            (
                clock,
                control_frame,
                active_area,
                mode_label,
                moving_platforms.0.clone(),
                game_world.0.clone(),
            )
        };

        let safety = {
            let mut safety_q = sim
                .world_mut()
                .query_filtered::<&PlayerSafetyState, With<PlayerEntity>>();
            safety_q.single(sim.world()).copied().unwrap_or_default()
        };

        // Query the player's 18 cluster components in one shot via
        // `PlayerClusterQueryData::as_clusters_mut()` so the trace
        // recorder can read them through a `PlayerClustersMut` view.
        let mut cluster_q = sim
            .world_mut()
            .query_filtered::<ambition_sandbox::engine_core::PlayerClusterQueryData, With<PlayerEntity>>();
        let Ok(mut cluster_item) = cluster_q.single_mut(sim.world_mut()) else {
            continue;
        };
        let clusters = cluster_item.as_clusters_mut();
        let locomotion_state = ambition_sandbox::engine_core::LocomotionState::from_clusters(
            clusters.ground,
            clusters.wall,
            clusters.flight,
            clusters.dash,
            clusters.blink,
            clusters.ledge,
        );
        let body_mode_state =
            ambition_sandbox::engine_core::BodyMode::from_clusters(clusters.body_mode);
        record_simulation_frame(
            &mut buffer,
            &clusters,
            &clock,
            &safety,
            &game_world,
            control_frame,
            1.0 / 60.0,
            1.0 / 60.0,
            &mode_label,
            &active_area,
            &moving_platforms,
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
