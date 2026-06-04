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

use super::dec;
use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::MovementTuning;

/// Apply one frame of velocity integration to the player: mode-select
/// between dash / climb / flight / normal physics, run the per-mode
/// integration, sweep the kinematics through X then Y collisions,
/// apply wall abilities + rebound + end-of-frame `pre_wall_vel`
/// bookkeeping. Reads and writes every relevant cluster directly.
pub(super) fn integrate_velocity_clusters(
    world: &World,
    clusters: &mut crate::engine_core::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    use crate::engine_core::player_state::BodyMode;

    let climbing = clusters.body_mode.body_mode == BodyMode::Climbing
        && clusters.env_contact.climbable.is_some();
    if !climbing {
        clusters.jump.ladder_jump_boost = 0.0;
    }

    if clusters.dash.timer > 0.0 {
        clusters.dash.timer = dec(clusters.dash.timer, dt);
    } else if climbing {
        integrate_climb_clusters(
            clusters.kinematics,
            clusters.env_contact,
            clusters.flight,
            clusters.wall,
            clusters.jump,
            input,
            dt,
            tuning,
        );
    } else if clusters.flight.fly_enabled && clusters.abilities.abilities.fly {
        integrate_flight_clusters(
            clusters.kinematics,
            clusters.flight,
            clusters.wall,
            input,
            dt,
            tuning,
        );
    } else {
        let blink_hang_active =
            clusters.blink.grace_timer > 0.0 && clusters.kinematics.vel.y >= 0.0;
        let water_gravity_scale = clusters
            .env_contact
            .water
            .map(|c| c.spec.gravity_scale)
            .unwrap_or(1.0);
        if !blink_hang_active {
            clusters.kinematics.vel.y +=
                tuning.gravity * tuning.gravity_sign * water_gravity_scale * dt;
        }
        if input.fast_fall_pressed
            && clusters.abilities.abilities.fast_fall
            && !clusters.ground.on_ground
        {
            clusters.flight.fast_falling = true;
        }
        if clusters.flight.fast_falling
            && !blink_hang_active
            && clusters.env_contact.water.is_none()
        {
            clusters.kinematics.vel.y += tuning.fast_fall_accel * tuning.gravity_sign * dt;
        }
        clusters.flight.gliding = clusters.abilities.abilities.glide
            && !clusters.ground.on_ground
            && !clusters.flight.fast_falling
            && !blink_hang_active
            && clusters.env_contact.water.is_none()
            && input.jump_held
            && clusters.kinematics.vel.y > 0.0;

        if clusters.abilities.abilities.move_horizontal {
            let accel = if clusters.ground.on_ground {
                tuning.run_accel
            } else if clusters.flight.gliding {
                tuning.glide_air_accel
            } else {
                tuning.air_accel
            };
            let target_vx = input.axis_x * tuning.max_run_speed;
            clusters.kinematics.vel.x = approach(clusters.kinematics.vel.x, target_vx, accel * dt);
            let friction = if clusters.ground.on_ground {
                tuning.ground_friction
            } else {
                tuning.air_friction
            };
            if input.axis_x.abs() <= 0.1 {
                clusters.kinematics.vel.x = approach(clusters.kinematics.vel.x, 0.0, friction * dt);
            }
        }

        if let Some(contact) = clusters.env_contact.water {
            let drag = contact.spec.drag.clamp(0.0, 1.0);
            clusters.kinematics.vel.x *= 1.0 - drag;
            clusters.kinematics.vel.y *= 1.0 - drag;
            clusters.kinematics.vel.y = clusters.kinematics.vel.y.min(contact.spec.max_fall_speed);
        } else {
            let fall_cap = if clusters.flight.fast_falling {
                tuning.fast_fall_speed
            } else if clusters.flight.gliding {
                tuning.glide_fall_speed
            } else {
                tuning.max_fall_speed
            };
            clusters.kinematics.vel.y = clusters.kinematics.vel.y.min(fall_cap);
        }
    }

    // Pre-X-sweep state.
    clusters.wall.on_wall = false;
    let pre_wall_snapshot = clusters.kinematics.vel;
    clusters.wall.wall_normal_x = 0.0;
    clusters.wall.wall_climbing = false;
    let was_clinging = clusters.wall.wall_clinging;
    clusters.wall.wall_clinging = false;

    // X-sweep — fully cluster-native.
    let dt_x = clusters.kinematics.vel.x * dt;
    super::collision::sweep_player_x_clusters(
        world,
        clusters.kinematics,
        clusters.wall,
        clusters.body_mode,
        clusters.env_contact,
        dt_x,
    );

    apply_wall_abilities_clusters(
        clusters.kinematics,
        clusters.ground,
        clusters.wall,
        clusters.abilities,
        clusters.combo_trace,
        input,
        tuning,
        was_clinging,
        events,
    );

    // Pre-Y-sweep state.
    let prev_bottom = clusters.kinematics.aabb().bottom();
    clusters.ground.on_ground = false;
    let drop_through = input.drop_through_pressed || clusters.ground.drop_through_timer > 0.0;
    let dt_y = clusters.kinematics.vel.y * dt;
    super::collision::sweep_player_y_clusters(
        world,
        clusters.kinematics,
        clusters.ground,
        clusters.body_mode,
        clusters.env_contact,
        dt_y,
        prev_bottom,
        drop_through,
        tuning.gravity_sign,
    );

    // World-bounds containment (runs for every mode -- walk, climb, flight). The
    // collision de-penetration can rarely push the body out a WIDE block's far
    // edge and past the world: flying deep into the thin ceiling near a corner
    // shoves the body out the ceiling's far X edge, after which nothing stops it
    // leaving the envelope (the "outside world (y)" OOB the fly traces showed).
    // Clamp the body back inside so it can never leave the world -- the player
    // counterpart of the boss soft world-bounds clamp.
    {
        let half = clusters.kinematics.size * 0.5;
        let pos = &mut clusters.kinematics.pos;
        pos.x = pos.x.clamp(half.x, (world.size.x - half.x).max(half.x));
        pos.y = pos.y.clamp(half.y, (world.size.y - half.y).max(half.y));
    }

    if clusters.ground.on_ground {
        crate::engine_core::player_clusters::refresh_movement_resources_clusters(
            clusters.abilities,
            &mut *clusters.dash,
            &mut *clusters.jump,
            tuning,
        );
        clusters.blink.grace_timer = 0.0;
        clusters.flight.fast_falling = false;
        clusters.flight.gliding = false;
        clusters.wall.wall_clinging = false;
        clusters.wall.wall_climbing = false;
        clusters.ground.drop_through_timer = 0.0;
    }

    if clusters.abilities.abilities.rebound && clusters.ground.rebound_cooldown <= 0.0 {
        if let Some(impulse) =
            super::collision::touching_rebound_aabb(world, clusters.kinematics.aabb())
        {
            clusters.kinematics.vel = impulse;
            crate::engine_core::player_clusters::refresh_movement_resources_clusters(
                clusters.abilities,
                &mut *clusters.dash,
                &mut *clusters.jump,
                tuning,
            );
            clusters.ground.on_ground = false;
            clusters.ground.rebound_cooldown = 0.18;
            events.op_clusters(clusters.combo_trace, MovementOp::Rebound);
        }
    }

    // End-of-integration: if the frame settled into airborne free
    // flight, commit the pre-wall snapshot as the most recent valid
    // `pre_wall_vel`.
    if !clusters.ground.on_ground && !clusters.wall.wall_clinging {
        clusters.wall.pre_wall_vel = pre_wall_snapshot;
        clusters.wall.pre_wall_vel_age = 0.0;
    }
}

/// Ladder integration: drive vel.y from `axis_y * climb_speed`,
/// scale x by `strafe_factor`, and clear transient flight flags.
/// Suspends gravity by overwriting `vel.y` rather than accumulating.
pub(super) fn integrate_climb_clusters(
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    flight: &mut crate::engine_core::player_clusters::PlayerFlightState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    jump: &mut crate::engine_core::player_clusters::PlayerJumpState,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
) {
    let Some(contact) = env_contact.climbable else {
        kinematics.vel = Vec2::ZERO;
        jump.ladder_jump_boost = 0.0;
        return;
    };
    let spec = contact.spec;
    let target_vy = if jump.ladder_jump_boost > 0.0 && input.axis_y < -0.1 {
        -tuning.jump_speed * tuning.gravity_sign
    } else {
        input.axis_y * spec.climb_speed
    };
    kinematics.vel.y = target_vy;
    let target_vx = input.axis_x * spec.climb_speed * spec.strafe_factor;
    kinematics.vel.x = target_vx;
    flight.fast_falling = false;
    flight.gliding = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    let _ = dt;
}

/// Free-flight integration: accelerate toward stick input, idle-hover
/// bob phase when sticks are centered, hard clamp to the flight
/// terminal speed. Clears fast-fall + wall-cling flags by mode.
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

/// Wall ability ride: while pressed into a wall (axis_x against the
/// wall normal), engage wall-cling (clamp `vel.y` to `wall_slide_speed`)
/// or, with `wall_climb` + |axis_y| > 0.25, drive `vel.y` directly.
/// Records the first transition op so the trace recorder fires
/// `WallCling` / `WallClimb` exactly once per engagement.
///
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
