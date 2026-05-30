//! Player movement simulation.
//!
//! Pure-Rust kinematic platformer with coyote time, buffered jumps,
//! optional double jumps, optional wall jumps/cling/climb, optional
//! dash/double dash, blink/precision blink, pogo refreshes, rebound
//! pads, hazards, and a symbolic operation trace.
//!
//! Entry points (all cluster-native, no `ae::Player` aggregate):
//!
//! - [`update_player_with_tuning_clusters`] — combined control + sim
//! - [`update_player_control_with_clusters`] — control-phase only
//! - [`update_player_simulation_with_clusters`] — simulation-phase only
//! - [`update_player_*_scratch`] — test wrappers that take a
//!   `PlayerClusterScratch` instead of a `PlayerClustersMut` view
//!
//! Each entry point consumes an [`InputState`], mutates the player's
//! cluster components through a [`crate::engine_core::PlayerClustersMut`]
//! view, and returns [`FrameEvents`] for the Bevy layer to translate
//! into particles, hitstop, sound, or debug overlays. Implementation
//! details live in focused child modules so movement actions,
//! simulation clocks, collision, velocity integration, and blink
//! pathing can evolve independently.

use crate::engine_core::world::World;

mod blink;
pub(crate) mod collision;
mod control;
mod events;
mod input;
mod integration;
mod ops;
mod player;
mod simulation;
mod tuning;

pub use blink::{blink_destination_clusters, blink_destination_to_point_clusters};
pub use events::{BlinkEvent, FrameEvents};
pub use input::InputState;
pub use ops::{ComboMark, MovementOp};
pub use player::{default_player_body_size, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH};
pub use tuning::{
    LedgeMomentumTuning, MovementTuning, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN,
    BLINK_DISTANCE, BLINK_GRACE_TIME, BLINK_HOLD_THRESHOLD, BLINK_MAX_DOWNWARD_SPEED, COYOTE_TIME,
    DASH_BUFFER, DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DODGE_ROLL_COOLDOWN,
    DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL, FAST_FALL_SPEED,
    FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED,
    GLIDE_AIR_ACCEL, GLIDE_FALL_SPEED, GRAVITY, GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED,
    MAX_FALL_SPEED, MAX_RUN_SPEED, ONE_WAY_DROP_THROUGH_GRACE, PARRY_WINDOW_TIME, POGO_SPEED,
    PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE, PRECISION_BLINK_MAX_DOWNWARD_SPEED,
    RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};

#[cfg(test)]
use collision::body_is_side_contact;

/// Run the control phase for one frame: reset gesture, facing /
/// jump-buffer / dash-buffer intent, fly toggle, blink hold + release,
/// melee + pogo, dodge roll, dash, shield, variable jump release.
/// All state lives on cluster components.
pub fn update_player_control_with_clusters(
    world: &World,
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    control_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();

    // Reset on edge press, cluster-native.
    if input.reset_pressed && clusters.abilities.abilities.reset {
        crate::engine_core::player_clusters::reset_player_clusters(clusters, world.spawn);
        events.reset = true;
        return events;
    }

    // update_facing_and_control_intent — cluster-native.
    {
        let can_turn = clusters.ground.on_ground || clusters.flight.fly_enabled;
        if can_turn && input.axis_x.abs() > 0.1 {
            clusters.kinematics.facing = input.axis_x.signum();
        }
        if input.jump_pressed && clusters.abilities.abilities.jump {
            clusters.action_buffer.jump = tuning.jump_buffer;
        }
        if input.dash_pressed && clusters.abilities.abilities.dash {
            clusters.action_buffer.dash = tuning.dash_buffer;
        }
    }

    // handle_mode_toggles — cluster-native.
    if input.fly_toggle_pressed && clusters.abilities.abilities.fly {
        clusters.flight.fly_enabled = !clusters.flight.fly_enabled;
        if clusters.flight.fly_enabled {
            clusters.flight.fast_falling = false;
            clusters.wall.wall_clinging = false;
            clusters.wall.wall_climbing = false;
            clusters.dash.timer = 0.0;
            clusters.blink.grace_timer = 0.0;
        }
        events.op_clusters(clusters.combo_trace, ops::MovementOp::FlyToggle);
    }

    // Blink hold / aim / release + melee + pogo dispatch.
    control::handle_blink_clusters(
        world,
        clusters.kinematics,
        clusters.abilities,
        clusters.flight,
        clusters.wall,
        clusters.dash,
        clusters.blink,
        clusters.combo_trace,
        input,
        control_dt,
        tuning,
        &mut events,
    );
    control::handle_attacks_clusters(
        world,
        clusters.kinematics,
        clusters.abilities,
        clusters.ground,
        clusters.dash,
        clusters.jump,
        clusters.combo_trace,
        input,
        tuning,
        &mut events,
    );

    // handle_dodge — cluster-native.
    if clusters.action_buffer.dash > 0.0
        && clusters.abilities.abilities.dodge
        && clusters.ground.on_ground
        && clusters.dodge.cooldown <= 0.0
    {
        let dir = if input.axis_x.abs() > 0.1 {
            input.axis_x.signum()
        } else {
            clusters.kinematics.facing
        };
        clusters.kinematics.vel.x = dir * tuning.dodge_roll_speed;
        clusters.kinematics.vel.y = clusters.kinematics.vel.y.min(0.0);
        clusters.dodge.roll_timer = tuning.dodge_roll_time;
        clusters.dodge.cooldown = tuning.dodge_roll_cooldown;
        clusters.action_buffer.dash = 0.0;
        events.op_clusters(clusters.combo_trace, ops::MovementOp::DodgeRoll);
    }

    // handle_dash — cluster-native (note: spend_dash_charge picks
    // Dash vs DoubleDash op based on charge count before decrement).
    if clusters.action_buffer.dash > 0.0
        && clusters.abilities.abilities.dash
        && clusters.dash.charges_available > 0
        && clusters.dash.cooldown <= 0.0
    {
        let fallback = bevy_math::Vec2::new(clusters.kinematics.facing, 0.0);
        let aim = bevy_math::Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        clusters.kinematics.vel = aim * tuning.dash_speed;
        clusters.dash.timer = tuning.dash_time;
        clusters.dash.cooldown = tuning.dash_cooldown;
        clusters.action_buffer.dash = 0.0;
        let before = clusters.dash.charges_available;
        clusters.dash.charges_available = clusters.dash.charges_available.saturating_sub(1);
        let op = if before >= 2 {
            ops::MovementOp::DoubleDash
        } else {
            ops::MovementOp::Dash
        };
        events.op_clusters(clusters.combo_trace, op);
    }

    // handle_shield — cluster-native.
    if !clusters.abilities.abilities.shield {
        clusters.shield.active = false;
        clusters.shield.parry_window_timer = 0.0;
    } else {
        let can_shield = clusters.dash.timer <= 0.0;
        let want_shield = input.shield_held && can_shield;
        if want_shield && !clusters.shield.active {
            clusters.shield.parry_window_timer = tuning.parry_window_time;
            events.op_clusters(clusters.combo_trace, ops::MovementOp::ShieldUp);
        }
        clusters.shield.active = want_shield;
    }

    // handle_jump_release — cluster-native (variable jump height).
    if clusters.abilities.abilities.variable_jump
        && input.jump_released
        && clusters.kinematics.vel.y < -120.0
    {
        clusters.kinematics.vel.y *= 0.54;
    }

    events
}

/// Run the simulation phase for one frame: cache water/climbable
/// contact, age timers + combo trace, advance the active ledge grab,
/// handle the buffered jump, integrate velocity through collision,
/// re-probe ledge starts, and finally fire the hazard reset gate.
/// All state lives on cluster components.
pub fn update_player_simulation_with_clusters(
    world: &World,
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return events;
    }
    let dt = raw_dt.min(1.0 / 30.0);

    // Cache water + climbable contact once per tick so movement,
    // jump-buffer, and integration all see the same answer. Also
    // clear a stale ledge grab if the ability is no longer enabled.
    clusters.env_contact.water = world.water_at(clusters.kinematics.aabb());
    clusters.env_contact.climbable = world.climbable_at(clusters.kinematics.aabb());
    if !clusters.abilities.abilities.ledge_grab {
        clusters.ledge.grab = None;
    }

    // Drowning gate — cluster-native reset.
    if clusters.env_contact.water.is_some() && !clusters.abilities.abilities.swim {
        crate::engine_core::player_clusters::reset_player_clusters(clusters, world.spawn);
        events.hazard = true;
        events.reset = true;
        return events;
    }

    // age_player + update_simulation_timers — cluster-native inline.
    {
        clusters.lifetime.time_alive += dt;
        let speed = clusters.kinematics.vel.length();
        clusters.lifetime.max_speed = clusters.lifetime.max_speed.max(speed);
        for mark in clusters.combo_trace.combo.iter_mut() {
            mark.age += dt;
        }
        clusters
            .combo_trace
            .combo
            .retain(|m| m.age < 4.0 || m.op == ops::MovementOp::Reset);

        let dec = |v: f32| (v - dt).max(0.0);
        clusters.action_buffer.jump = dec(clusters.action_buffer.jump);
        clusters.action_buffer.dash = dec(clusters.action_buffer.dash);
        clusters.ground.coyote_timer = dec(clusters.ground.coyote_timer);
        clusters.ground.drop_through_timer = dec(clusters.ground.drop_through_timer);
        clusters.jump.ladder_jump_boost = dec(clusters.jump.ladder_jump_boost);
        clusters.dash.cooldown = dec(clusters.dash.cooldown);
        clusters.blink.cooldown = dec(clusters.blink.cooldown);
        clusters.blink.grace_timer = dec(clusters.blink.grace_timer);
        clusters.ground.rebound_cooldown = dec(clusters.ground.rebound_cooldown);
        clusters.dodge.roll_timer = dec(clusters.dodge.roll_timer);
        clusters.dodge.cooldown = dec(clusters.dodge.cooldown);
        clusters.shield.parry_window_timer = dec(clusters.shield.parry_window_timer);
        clusters.ledge.release_cooldown = dec(clusters.ledge.release_cooldown);
        if clusters.wall.wall_clinging || clusters.ground.on_ground {
            clusters.wall.pre_wall_vel_age += dt;
        }
        if clusters.ground.on_ground {
            clusters.ground.coyote_timer = tuning.coyote_time;
            crate::engine_core::player_clusters::refresh_movement_resources_clusters(
                clusters.abilities,
                clusters.dash,
                clusters.jump,
                tuning,
            );
        }
    }

    // Active ledge-grab tick. Returns true if it consumed the frame
    // (the rest of the simulation phase short-circuits).
    if crate::engine_core::ledge_grab::tick_active_ledge_grab_clusters(
        clusters,
        input,
        dt,
        tuning,
        &mut events,
    ) {
        return events;
    }

    // Consume the buffered jump (or convert to swim stroke /
    // drop-through / wall-jump / double-jump).
    simulation::handle_jump_buffer_clusters(
        world,
        clusters.action_buffer,
        clusters.env_contact,
        clusters.abilities,
        clusters.body_mode.body_mode,
        clusters.kinematics,
        clusters.ground,
        clusters.wall,
        clusters.jump,
        clusters.combo_trace,
        input,
        tuning,
        &mut events,
    );

    integration::integrate_velocity_clusters(world, clusters, input, dt, tuning, &mut events);

    // Probe for a fresh ledge grab now that the integration step
    // settled the new position. Required for the auto-snap-on-fall
    // recovery path (slow drifts ignore this; fast falls latch).
    crate::engine_core::ledge_grab::try_start_ledge_grab_clusters(
        world,
        clusters,
        input,
        &mut events,
    );

    // Hazard reset — cluster-native.
    if collision::touching_hazard_aabb(world, clusters.kinematics.aabb())
        || clusters.kinematics.pos.y > world.size.y + 200.0
    {
        crate::engine_core::player_clusters::reset_player_clusters(clusters, world.spawn);
        events.hazard = true;
        events.reset = true;
    }

    events
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

/// Combined cluster-native player tick: control phase then simulation
/// phase, using `tuning`. `InputState::control_dt` overrides `raw_dt`
/// for the control phase when positive (so bullet-time slowing
/// gravity does not slow input).
pub fn update_player_with_tuning_clusters(
    world: &World,
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let control_dt = if input.control_dt > 0.0 {
        input.control_dt
    } else {
        raw_dt
    };
    let mut events =
        update_player_control_with_clusters(world, clusters, input, control_dt, tuning);
    let sim_events = update_player_simulation_with_clusters(world, clusters, input, raw_dt, tuning);
    events.extend(sim_events);
    events
}

/// `DEFAULT_TUNING` convenience wrapper for
/// [`update_player_with_tuning_clusters`]. Useful in adapter sites
/// (RL, headless drivers, lightweight integration tests) that don't
/// need custom tuning knobs.
pub fn update_player_clusters(
    world: &World,
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_with_tuning_clusters(world, clusters, input, raw_dt, DEFAULT_TUNING)
}

/// `PlayerClusterScratch`-based test wrapper: builds the cluster view
/// in-place and dispatches to `update_player_with_tuning_clusters`.
pub fn update_player_with_tuning_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    update_player_with_tuning_clusters(world, &mut clusters, input, raw_dt, tuning)
}

/// Convenience wrapper using `DEFAULT_TUNING`.
pub fn update_player_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_with_tuning_scratch(world, scratch, input, raw_dt, DEFAULT_TUNING)
}

/// `PlayerClusterScratch`-based control-phase wrapper for tests.
pub fn update_player_control_with_tuning_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    control_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    update_player_control_with_clusters(world, &mut clusters, input, control_dt, tuning)
}

pub fn update_player_control_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    control_dt: f32,
) -> FrameEvents {
    update_player_control_with_tuning_scratch(world, scratch, input, control_dt, DEFAULT_TUNING)
}

/// `PlayerClusterScratch`-based simulation-phase wrapper for tests.
pub fn update_player_simulation_with_tuning_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut clusters = scratch.as_mut();
    update_player_simulation_with_clusters(world, &mut clusters, input, raw_dt, tuning)
}

pub fn update_player_simulation_scratch(
    world: &World,
    scratch: &mut crate::engine_core::player_clusters::PlayerClusterScratch,
    input: InputState,
    raw_dt: f32,
) -> FrameEvents {
    update_player_simulation_with_tuning_scratch(world, scratch, input, raw_dt, DEFAULT_TUNING)
}

#[cfg(test)]
mod tests;
