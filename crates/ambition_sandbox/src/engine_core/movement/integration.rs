use crate::engine_core::geometry::AabbExt;
use crate::engine_core::world::World;
use crate::engine_core::Vec2;

/// Move `value` toward `target` by at most `delta`. Inlined from the
/// removed `ae::scalar::approach`.
fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

use super::collision::{sweep_player_x, sweep_player_y, touching_rebound};
use super::dec;
use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::player::Player;
use super::tuning::MovementTuning;

pub(super) fn integrate_velocity(
    world: &World,
    player: &mut Player,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_timer > 0.0 {
        player.dash_timer = dec(player.dash_timer, dt);
    } else if player.body_mode == crate::engine_core::player_state::BodyMode::Climbing
        && player.climbable_contact.is_some()
    {
        integrate_climb(player, input, dt);
    } else if player.fly_enabled && player.abilities.fly {
        integrate_flight(player, input, dt, tuning);
    } else {
        let blink_hang_active = player.blink_grace_timer > 0.0 && player.vel.y >= 0.0;
        // Water makes gravity gentler and adds linear drag. We
        // multiply gravity by the region's `gravity_scale` (Mario-
        // style: still sinks, just slower) and apply per-frame drag
        // to both axes so directional inputs feel more like swimming
        // strokes than running. The fall cap below also gets lowered
        // to the per-region cap so the player doesn't accelerate to
        // dash speeds in deep water.
        let water_gravity_scale = player
            .water_contact
            .map(|c| c.spec.gravity_scale)
            .unwrap_or(1.0);
        if !blink_hang_active {
            player.vel.y += tuning.gravity * water_gravity_scale * dt;
        }
        if input.fast_fall_pressed && player.abilities.fast_fall && !player.on_ground {
            player.fast_falling = true;
        }
        if player.fast_falling && !blink_hang_active && player.water_contact.is_none() {
            player.vel.y += tuning.fast_fall_accel * dt;
        }

        // Glide: hold-jump while airborne and falling. Fast-fall and
        // water/blink-hang preempt it (the player explicitly chose
        // those alternatives), so glide only takes hold when none of
        // those modes are active. The actual fall cap lookup below
        // reads `player.gliding`.
        player.gliding = player.abilities.glide
            && !player.on_ground
            && !player.fast_falling
            && !blink_hang_active
            && player.water_contact.is_none()
            && input.jump_held
            && player.vel.y > 0.0;

        if player.abilities.move_horizontal {
            let accel = if player.on_ground {
                tuning.run_accel
            } else if player.gliding {
                tuning.glide_air_accel
            } else {
                tuning.air_accel
            };
            let target_vx = input.axis_x * tuning.max_run_speed;
            player.vel.x = approach(player.vel.x, target_vx, accel * dt);

            let friction = if player.on_ground {
                tuning.ground_friction
            } else {
                tuning.air_friction
            };
            if input.axis_x.abs() <= 0.1 {
                player.vel.x = approach(player.vel.x, 0.0, friction * dt);
            }
        }

        if let Some(contact) = player.water_contact {
            // Water drag is a linear-per-tick decay applied AFTER the
            // gravity / horizontal accel pass so the gravity-applied
            // velocity also gets damped.
            let drag = contact.spec.drag.clamp(0.0, 1.0);
            player.vel.x *= 1.0 - drag;
            player.vel.y *= 1.0 - drag;
            player.vel.y = player.vel.y.min(contact.spec.max_fall_speed);
        } else {
            let fall_cap = if player.fast_falling {
                tuning.fast_fall_speed
            } else if player.gliding {
                tuning.glide_fall_speed
            } else {
                tuning.max_fall_speed
            };
            player.vel.y = player.vel.y.min(fall_cap);
        }
    }

    // Resolve horizontal motion with a Parry-backed swept AABB. This
    // establishes wall contact for wall verbs without letting high-speed dash
    // or future knockback skip through a thin wall.
    player.on_wall = false;
    // Snapshot the player's intended velocity BEFORE the X sweep
    // collides them against any wall and BEFORE `apply_wall_abilities`
    // clamps `vel.y` to `wall_slide_speed`. This is the "approach
    // velocity" the ledge-grab momentum carry wants — by the time
    // `try_start_ledge_grab` reads `player.vel` at the end of this
    // function, both `vel.x` (wall collision zero) and `vel.y`
    // (wall-slide clamp) have been mangled. Committed back into
    // `pre_wall_vel` further down only if the frame ended airborne
    // AND free (no wall-cling, no ground), so wall-clinging frames
    // don't overwrite the last good airborne sample.
    let pre_wall_snapshot = player.vel;
    player.wall_normal_x = 0.0;
    player.wall_climbing = false;
    let was_clinging = player.wall_clinging;
    player.wall_clinging = false;
    sweep_player_x(world, player, player.vel.x * dt);

    apply_wall_abilities(player, input, tuning, was_clinging, events);

    // Resolve vertical motion. Previous bottom determines one-way behavior.
    let prev_bottom = player.aabb().bottom();
    player.on_ground = false;
    let drop_through = input.drop_through_pressed || player.drop_through_timer > 0.0;
    sweep_player_y(world, player, player.vel.y * dt, prev_bottom, drop_through);

    if player.on_ground {
        player.refresh_movement_resources(tuning);
        player.blink_grace_timer = 0.0;
        player.fast_falling = false;
        player.gliding = false;
        player.wall_clinging = false;
        player.wall_climbing = false;
        player.drop_through_timer = 0.0;
    }

    if player.abilities.rebound && player.rebound_cooldown <= 0.0 {
        if let Some(impulse) = touching_rebound(world, player) {
            player.vel = impulse;
            player.refresh_movement_resources(tuning);
            player.on_ground = false;
            player.rebound_cooldown = 0.18;
            events.op(player, MovementOp::Rebound);
        }
    }

    // End-of-integration: if the frame settled into airborne free
    // flight (no ground, no wall-cling), commit the pre-wall snapshot
    // as the most recent valid `pre_wall_vel`. The ledge-grab
    // momentum-carry path reads this so wall-cling and wall-collision
    // can't shred the approach velocity before the grab captures it.
    if !player.on_ground && !player.wall_clinging {
        player.pre_wall_vel = pre_wall_snapshot;
        player.pre_wall_vel_age = 0.0;
    }
}

/// Cluster-native variant of [`integrate_climb`].
#[allow(dead_code)]
pub(super) fn integrate_climb_clusters(
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    flight: &mut crate::engine_core::player_clusters::PlayerFlightState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    input: InputState,
    dt: f32,
) {
    let Some(contact) = env_contact.climbable else {
        kinematics.vel = Vec2::ZERO;
        return;
    };
    let spec = contact.spec;
    let target_vy = input.axis_y * spec.climb_speed;
    kinematics.vel.y = target_vy;
    let target_vx = input.axis_x * spec.climb_speed * spec.strafe_factor;
    kinematics.vel.x = target_vx;
    flight.fast_falling = false;
    flight.gliding = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    let _ = dt;
}

/// Cluster-native variant of [`integrate_flight`].
#[allow(dead_code)]
pub(super) fn integrate_flight_clusters(
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    flight: &mut crate::engine_core::player_clusters::PlayerFlightState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
) {
    flight.fast_falling = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    flight.flight_phase += dt * tuning.flight_hover_hz * std::f32::consts::TAU;

    let target_x = input.axis_x * tuning.flight_terminal_speed;
    let mut target_y = input.axis_y * tuning.flight_terminal_speed;
    if input.axis_y.abs() <= 0.10 {
        target_y = flight.flight_phase.sin() * tuning.flight_hover_speed;
    }

    kinematics.vel.x = approach(kinematics.vel.x, target_x, tuning.flight_accel * dt);
    kinematics.vel.y = approach(kinematics.vel.y, target_y, tuning.flight_accel * dt);

    if input.axis_x.abs() <= 0.10 {
        kinematics.vel.x = approach(kinematics.vel.x, 0.0, tuning.flight_drag * dt);
    }
    if input.axis_y.abs() <= 0.10 {
        kinematics.vel.y = approach(kinematics.vel.y, target_y, tuning.flight_drag * dt);
    }

    kinematics.vel.x = kinematics
        .vel
        .x
        .clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
    kinematics.vel.y = kinematics
        .vel
        .y
        .clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
}

fn integrate_climb(player: &mut Player, input: InputState, dt: f32) {
    let Some(contact) = player.climbable_contact else {
        // Defensive: if the contact disappears mid-climb, don't crash;
        // just zero velocity for this tick. The sandbox-side body-mode
        // driver should clear `Climbing` next frame.
        player.vel = Vec2::ZERO;
        return;
    };
    let spec = contact.spec;

    // Vertical: full input authority at `climb_speed`. This engine's
    // +Y is downward, so axis_y > 0 climbs down (matches the input
    // convention where `down_pressed` sets axis_y > 0).
    let target_vy = input.axis_y * spec.climb_speed;
    // Approach the target hard so climbing feels deterministic — no
    // accel ramp on a ladder; you're either moving or you're not.
    player.vel.y = target_vy;

    // Horizontal: scaled by strafe_factor. Player can nudge sideways
    // to align with the next rung but can't fly off horizontally.
    let target_vx = input.axis_x * spec.climb_speed * spec.strafe_factor;
    player.vel.x = target_vx;

    // Climbing zeroes a few transient flags so they don't survive the
    // mode (mirrors `integrate_flight`'s zero-out pattern).
    player.fast_falling = false;
    player.gliding = false;
    player.wall_clinging = false;
    player.wall_climbing = false;

    // Suppress dt-warnings: the above is purely current-frame velocity
    // assignment; `dt` only matters for accel-style integration. Keep
    // the parameter so signatures stay parallel with `integrate_flight`.
    let _ = dt;
}

fn integrate_flight(player: &mut Player, input: InputState, dt: f32, tuning: MovementTuning) {
    player.fast_falling = false;
    player.wall_clinging = false;
    player.wall_climbing = false;
    player.flight_phase += dt * tuning.flight_hover_hz * std::f32::consts::TAU;

    let target_x = input.axis_x * tuning.flight_terminal_speed;
    let mut target_y = input.axis_y * tuning.flight_terminal_speed;
    if input.axis_y.abs() <= 0.10 {
        target_y = player.flight_phase.sin() * tuning.flight_hover_speed;
    }

    player.vel.x = approach(player.vel.x, target_x, tuning.flight_accel * dt);
    player.vel.y = approach(player.vel.y, target_y, tuning.flight_accel * dt);

    if input.axis_x.abs() <= 0.10 {
        player.vel.x = approach(player.vel.x, 0.0, tuning.flight_drag * dt);
    }
    if input.axis_y.abs() <= 0.10 {
        player.vel.y = approach(player.vel.y, target_y, tuning.flight_drag * dt);
    }

    player.vel.x = player
        .vel
        .x
        .clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
    player.vel.y = player
        .vel
        .y
        .clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
}

fn apply_wall_abilities(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    was_clinging: bool,
    events: &mut FrameEvents,
) {
    if !player.on_wall || player.on_ground || !player.abilities.wall_cling {
        return;
    }
    // Pressing toward the wall means axis_x is opposite the collision normal.
    let pressing_into_wall =
        input.axis_x.abs() > 0.1 && input.axis_x.signum() == -player.wall_normal_x;
    if !pressing_into_wall {
        return;
    }

    player.wall_clinging = true;
    if player.abilities.wall_climb && input.axis_y.abs() > 0.25 {
        player.wall_climbing = true;
        player.vel.y = input.axis_y * tuning.wall_climb_speed;
        if !was_clinging {
            events.op(player, MovementOp::WallClimb);
        }
    } else {
        if player.vel.y > tuning.wall_slide_speed {
            player.vel.y = tuning.wall_slide_speed;
        }
        if !was_clinging {
            events.op(player, MovementOp::WallCling);
        }
    }
}

/// Cluster-native variant of [`apply_wall_abilities`]. Field-for-field
/// translation reading / writing through cluster components.
///
/// Currently unused — sister to the in-flight `integrate_velocity`
/// cluster migration. The wrapping `integrate_velocity_clusters`
/// hasn't been written yet, so this stays opted out of the
/// dead-code lint until it has a caller.
#[allow(dead_code)]
pub(super) fn apply_wall_abilities_clusters(
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    ground: &crate::engine_core::player_clusters::PlayerGroundState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
    combo_trace: &mut crate::engine_core::player_clusters::PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    was_clinging: bool,
    events: &mut FrameEvents,
) {
    if !wall.on_wall || ground.on_ground || !abilities.abilities.wall_cling {
        return;
    }
    let pressing_into_wall =
        input.axis_x.abs() > 0.1 && input.axis_x.signum() == -wall.wall_normal_x;
    if !pressing_into_wall {
        return;
    }
    wall.wall_clinging = true;
    if abilities.abilities.wall_climb && input.axis_y.abs() > 0.25 {
        wall.wall_climbing = true;
        kinematics.vel.y = input.axis_y * tuning.wall_climb_speed;
        if !was_clinging {
            events.op_clusters(combo_trace, MovementOp::WallClimb);
        }
    } else {
        if kinematics.vel.y > tuning.wall_slide_speed {
            kinematics.vel.y = tuning.wall_slide_speed;
        }
        if !was_clinging {
            events.op_clusters(combo_trace, MovementOp::WallCling);
        }
    }
}
