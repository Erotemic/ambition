//! Player movement simulation.
//!
//! This module contains the code that makes the current prototype feel like a
//! platformer: coyote time, buffered jumps, optional double jumps, optional
//! wall jumps/cling/climb, optional dash/double dash, blink/precision blink,
//! pogo refreshes, rebound pads, hazards, and a symbolic operation trace.
//!
//! The update function is intentionally renderer-free. It consumes a plain
//! `InputState`, mutates a `Player`, and returns `FrameEvents` that the Bevy
//! layer can turn into particles, hitstop, sound, or debug overlays.
//!
//! The public module remains a stable facade. Implementation details live in
//! focused child modules so movement actions, simulation clocks, collision,
//! velocity integration, and blink pathing can evolve independently.

use crate::world::World;

mod blink;
mod collision;
mod control;
mod events;
mod input;
mod integration;
mod ops;
mod player;
mod simulation;
mod tuning;

pub use blink::{blink_destination, blink_destination_to_point};
pub use events::{BlinkEvent, FrameEvents};
pub use input::InputState;
pub use ops::{ComboMark, MovementOp};
pub use player::{default_player_body_size, Player, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH};
pub use tuning::{
    MovementTuning, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE,
    BLINK_GRACE_TIME, BLINK_HOLD_THRESHOLD, BLINK_MAX_DOWNWARD_SPEED, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GLIDE_AIR_ACCEL, GLIDE_FALL_SPEED, GRAVITY, GROUND_FRICTION,
    JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, ONE_WAY_DROP_THROUGH_GRACE, POGO_SPEED,
    PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, PRECISION_BLINK_MAX_DOWNWARD_SPEED,
    RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};

#[cfg(test)]
use collision::body_is_side_contact;

pub fn update_player(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_with_tuning(world, player, input, raw_dt, DEFAULT_TUNING)
}

/// Advance the player for callers that do not care about separate clocks.
///
/// This compatibility wrapper uses the same duration for control and simulation.
/// The Bevy sandbox uses the split functions below so bullet-time can freeze
/// physical evolution while keeping input/aim control responsive.
pub fn update_player_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let control_dt = if input.control_dt > 0.0 {
        input.control_dt
    } else {
        raw_dt
    };
    let mut events = update_player_control_with_tuning(world, player, input, control_dt, tuning);
    let sim_events = update_player_simulation_with_tuning(world, player, input, raw_dt, tuning);
    events.extend(sim_events);
    events
}

/// Process player intent and instantaneous actions using real, unscaled time.
///
/// Input should remain responsive during bullet-time: the blink aim cursor,
/// button-hold thresholds, toggles, dash presses, attack presses, and jump
/// buffering are control-layer concepts. They advance from real frame time,
/// not from slowed simulation time.
pub fn update_player_control(
    world: &World,
    player: &mut Player,
    input: InputState,
    control_dt: f32,
) -> FrameEvents {
    update_player_control_with_tuning(world, player, input, control_dt, DEFAULT_TUNING)
}

pub fn update_player_control_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    control_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    control::update_player_control_with_tuning(world, player, input, control_dt, tuning)
}

/// Advance physical world evolution using scaled game time.
///
/// Gravity, velocity integration, timers, coyote time, cooldowns, enemies,
/// platforms, and particles should all consume this same scaled timestep. Tiny
/// positive values are preserved so near-frozen bullet-time is honored; only
/// large frame spikes are capped.
pub fn update_player_simulation(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_simulation_with_tuning(world, player, input, raw_dt, DEFAULT_TUNING)
}

pub fn update_player_simulation_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    simulation::update_player_simulation_with_tuning(world, player, input, raw_dt, tuning)
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

#[cfg(test)]
mod tests;
