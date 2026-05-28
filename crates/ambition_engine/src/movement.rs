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
pub(crate) mod collision;
mod control;
mod events;
mod input;
mod integration;
mod ops;
mod player;
mod simulation;
mod tuning;

pub use blink::{
    blink_destination, blink_destination_clusters, blink_destination_to_point,
    blink_destination_to_point_clusters,
};
pub use events::{BlinkEvent, FrameEvents};
pub use input::InputState;
pub use ops::{ComboMark, MovementOp};
pub use player::{
    default_player_body_size, Player, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH,
};
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

/// Cluster-ref entry point for the control phase. Operates on cluster
/// refs natively for the easy parts (reset, facing, jump/dash buffer,
/// mode toggles, dodge, dash, shield, jump release). Complex inner
/// helpers (handle_blink, handle_attacks) still take `&mut Player`
/// via a localized scratchpad.
pub fn update_player_control_with_clusters(
    world: &World,
    clusters: &mut crate::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    control_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();

    // Reset on edge press, cluster-native.
    if input.reset_pressed && clusters.abilities.abilities.reset {
        crate::player_clusters::reset_player_clusters(clusters, world.spawn);
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

    // handle_mode_toggles — cluster-native (event push needs the
    // scratchpad's Player because FrameEvents::op records combo).
    if input.fly_toggle_pressed && clusters.abilities.abilities.fly {
        clusters.flight.fly_enabled = !clusters.flight.fly_enabled;
        if clusters.flight.fly_enabled {
            clusters.flight.fast_falling = false;
            clusters.wall.wall_clinging = false;
            clusters.wall.wall_climbing = false;
            clusters.dash.timer = 0.0;
            clusters.blink.grace_timer = 0.0;
        }
        // events.op also writes player.combo — round-trip the combo
        // alone is cheaper than a full to_player/write_from_player.
        clusters.combo_trace.combo.push(ops::ComboMark {
            op: ops::MovementOp::FlyToggle,
            age: 0.0,
        });
        if clusters.combo_trace.combo.len() > 18 {
            let excess = clusters.combo_trace.combo.len() - 18;
            clusters.combo_trace.combo.drain(0..excess);
        }
    }

    // handle_blink + handle_attacks still on Player — scratchpad.
    let mut player = clusters.to_player();
    control::handle_blink_pub(world, &mut player, input, control_dt, tuning, &mut events);
    control::handle_attacks_pub(world, &mut player, input, tuning, &mut events);
    clusters.write_from_player(player);

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
        clusters.combo_trace.combo.push(ops::ComboMark {
            op: ops::MovementOp::DodgeRoll,
            age: 0.0,
        });
        if clusters.combo_trace.combo.len() > 18 {
            let excess = clusters.combo_trace.combo.len() - 18;
            clusters.combo_trace.combo.drain(0..excess);
        }
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
        clusters.combo_trace.combo.push(ops::ComboMark { op, age: 0.0 });
        if clusters.combo_trace.combo.len() > 18 {
            let excess = clusters.combo_trace.combo.len() - 18;
            clusters.combo_trace.combo.drain(0..excess);
        }
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
            clusters.combo_trace.combo.push(ops::ComboMark {
                op: ops::MovementOp::ShieldUp,
                age: 0.0,
            });
            if clusters.combo_trace.combo.len() > 18 {
                let excess = clusters.combo_trace.combo.len() - 18;
                clusters.combo_trace.combo.drain(0..excess);
            }
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

/// Cluster-ref entry point for the simulation phase.
///
/// Phase 3d: operates on cluster refs natively. Inner helpers that
/// still take `&mut Player` (integrate_velocity, handle_jump_buffer,
/// ledge_grab functions) get a localized scratchpad — `to_player` /
/// `write_from_player` is called around each unrefactored helper.
/// As Phase 3d progresses, more inner helpers gain cluster-ref
/// variants and the scratchpad calls shrink.
pub fn update_player_simulation_with_clusters(
    world: &World,
    clusters: &mut crate::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return events;
    }
    let dt = raw_dt.min(1.0 / 30.0);

    // Cluster-native setup: water/climbable contact + ledge clear.
    clusters.env_contact.water = world.water_at(clusters.kinematics.aabb());
    clusters.env_contact.climbable = world.climbable_at(clusters.kinematics.aabb());
    if !clusters.abilities.abilities.ledge_grab {
        clusters.ledge.grab = None;
    }

    // Drowning gate — cluster-native reset.
    if clusters.env_contact.water.is_some() && !clusters.abilities.abilities.swim {
        crate::player_clusters::reset_player_clusters(clusters, world.spawn);
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
        clusters.combo_trace.combo.retain(|m| {
            m.age < 4.0 || m.op == ops::MovementOp::Reset
        });

        let dec = |v: f32| (v - dt).max(0.0);
        clusters.action_buffer.jump = dec(clusters.action_buffer.jump);
        clusters.action_buffer.dash = dec(clusters.action_buffer.dash);
        clusters.ground.coyote_timer = dec(clusters.ground.coyote_timer);
        clusters.ground.drop_through_timer = dec(clusters.ground.drop_through_timer);
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
            crate::player_clusters::refresh_movement_resources_clusters(
                clusters.abilities,
                clusters.dash,
                clusters.jump,
                tuning,
            );
        }
    }

    // Inner helpers that still take &mut Player — localized scratchpad.
    let mut player = clusters.to_player();
    if crate::ledge_grab::tick_active_ledge_grab(&mut player, input, dt, tuning, &mut events) {
        clusters.write_from_player(player);
        return events;
    }
    clusters.write_from_player(player);

    // handle_jump_buffer is now cluster-native (no Player scratchpad).
    simulation::handle_jump_buffer_clusters(
        world,
        clusters.action_buffer,
        clusters.env_contact,
        clusters.abilities,
        clusters.kinematics,
        clusters.ground,
        clusters.wall,
        clusters.jump,
        clusters.combo_trace,
        input,
        tuning,
        &mut events,
    );

    let mut player = clusters.to_player();
    integration::integrate_velocity(world, &mut player, input, dt, tuning, &mut events);
    crate::ledge_grab::try_start_ledge_grab(world, &mut player, input, &mut events);
    clusters.write_from_player(player);

    // Hazard reset — cluster-native.
    if collision::touching_hazard_aabb(world, clusters.kinematics.aabb())
        || clusters.kinematics.pos.y > world.size.y + 200.0
    {
        crate::player_clusters::reset_player_clusters(clusters, world.spawn);
        events.hazard = true;
        events.reset = true;
    }

    events
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

#[cfg(test)]
mod tests;
