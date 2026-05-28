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
    use ambition_sandbox::player::{
        PlayerAbilities, PlayerActionBuffer, PlayerBlinkState, PlayerBodyModeState,
        PlayerComboTrace, PlayerDashState, PlayerDodgeState, PlayerEntity,
        PlayerEnvironmentContact, PlayerFlightState, PlayerGroundState, PlayerJumpState,
        PlayerKinematics, PlayerLedgeState, PlayerLifetime, PlayerMana, PlayerOffense,
        PlayerSafetyState, PlayerShieldState, PlayerWallState,
    };
    use ambition_sandbox::rooms::RoomSet;
    use ambition_sandbox::GameWorld;
    use bevy::prelude::With;
    use bevy::state::state::State;

    for _ in 0..max_ticks {
        sim.step(ambition_sandbox::AgentAction::default());

        // Assemble a snapshot ae::Player from the per-cluster ECS
        // components so the trace writer keeps its existing
        // `record_simulation_frame(&Player, ...)` signature.
        let player = {
            let mut single = || -> Option<ambition_engine::Player> {
                let mut kin_q =
                    sim.world_mut()
                        .query_filtered::<&PlayerKinematics, With<PlayerEntity>>();
                let mut ground_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerGroundState, With<PlayerEntity>>();
                let mut wall_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerWallState, With<PlayerEntity>>();
                let mut jump_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerJumpState, With<PlayerEntity>>();
                let mut dash_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerDashState, With<PlayerEntity>>();
                let mut flight_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerFlightState, With<PlayerEntity>>();
                let mut blink_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerBlinkState, With<PlayerEntity>>();
                let mut ledge_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerLedgeState, With<PlayerEntity>>();
                let mut dodge_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerDodgeState, With<PlayerEntity>>();
                let mut shield_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerShieldState, With<PlayerEntity>>();
                let mut body_mode_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerBodyModeState, With<PlayerEntity>>();
                let mut env_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerEnvironmentContact, With<PlayerEntity>>();
                let mut mana_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerMana, With<PlayerEntity>>();
                let mut offense_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerOffense, With<PlayerEntity>>();
                let mut action_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerActionBuffer, With<PlayerEntity>>();
                let mut lifetime_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerLifetime, With<PlayerEntity>>();
                let mut combo_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerComboTrace, With<PlayerEntity>>();
                let mut abilities_q = sim
                    .world_mut()
                    .query_filtered::<&PlayerAbilities, With<PlayerEntity>>();
                let world_ref = sim.world();
                let kin = kin_q.single(world_ref).ok()?;
                let ground = ground_q.single(world_ref).ok()?;
                let wall = wall_q.single(world_ref).ok()?;
                let jump = jump_q.single(world_ref).ok()?;
                let dash = dash_q.single(world_ref).ok()?;
                let flight = flight_q.single(world_ref).ok()?;
                let blink = blink_q.single(world_ref).ok()?;
                let ledge = ledge_q.single(world_ref).ok()?;
                let dodge = dodge_q.single(world_ref).ok()?;
                let shield = shield_q.single(world_ref).ok()?;
                let body_mode = body_mode_q.single(world_ref).ok()?;
                let env = env_q.single(world_ref).ok()?;
                let mana = mana_q.single(world_ref).ok()?;
                let offense = offense_q.single(world_ref).ok()?;
                let action = action_q.single(world_ref).ok()?;
                let lifetime = lifetime_q.single(world_ref).ok()?;
                let combo = combo_q.single(world_ref).ok()?;
                let abilities = abilities_q.single(world_ref).ok()?;
                Some(ambition_engine::Player {
                    abilities: abilities.abilities,
                    pos: kin.pos,
                    vel: kin.vel,
                    size: kin.size,
                    base_size: kin.base_size,
                    facing: kin.facing,
                    on_ground: ground.on_ground,
                    on_wall: wall.on_wall,
                    wall_normal_x: wall.wall_normal_x,
                    dash_charges_available: dash.charges_available,
                    air_jumps_available: jump.air_jumps_available,
                    fly_enabled: flight.fly_enabled,
                    flight_phase: flight.flight_phase,
                    blink_cooldown: blink.cooldown,
                    blink_hold_active: blink.hold_active,
                    blink_hold_timer: blink.hold_timer,
                    blink_aiming: blink.aiming,
                    blink_aim_offset: blink.aim_offset,
                    blink_grace_timer: blink.grace_timer,
                    fast_falling: flight.fast_falling,
                    gliding: flight.gliding,
                    wall_clinging: wall.wall_clinging,
                    wall_climbing: wall.wall_climbing,
                    dash_timer: dash.timer,
                    dash_cooldown: dash.cooldown,
                    dash_buffer_timer: action.dash,
                    jump_buffer_timer: action.jump,
                    coyote_timer: ground.coyote_timer,
                    rebound_cooldown: ground.rebound_cooldown,
                    drop_through_timer: ground.drop_through_timer,
                    combo: combo.combo.clone(),
                    max_speed: lifetime.max_speed,
                    time_alive: lifetime.time_alive,
                    resets: lifetime.resets,
                    damage_multiplier: offense.damage_multiplier,
                    mana: mana.meter,
                    invincible: offense.invincible,
                    body_mode: body_mode.body_mode,
                    water_contact: env.water,
                    climbable_contact: env.climbable,
                    ledge_grab: ledge.grab,
                    pre_wall_vel: wall.pre_wall_vel,
                    pre_wall_vel_age: wall.pre_wall_vel_age,
                    ledge_release_cooldown: ledge.release_cooldown,
                    dodge_roll_timer: dodge.roll_timer,
                    dodge_roll_cooldown: dodge.cooldown,
                    shield_active: shield.active,
                    parry_window_timer: shield.parry_window_timer,
                })
            };
            single().unwrap_or_else(|| {
                ambition_engine::Player::new_with_abilities(
                    ambition_engine::Vec2::ZERO,
                    ambition_engine::AbilitySet::default(),
                )
            })
        };
        let safety = {
            let mut q = sim
                .world_mut()
                .query_filtered::<&PlayerSafetyState, With<PlayerEntity>>();
            q.single(sim.world()).copied().unwrap_or_default()
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
            &safety,
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
