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

use crate::geometry::{Aabb, AabbExt};
use crate::world::{BlinkWallTier, BlockKind, World};
use crate::{approach, Vec2};

mod events;
mod input;
mod ops;
mod player;
mod tuning;

pub use events::{BlinkEvent, FrameEvents};
pub use input::InputState;
pub use ops::{ComboMark, MovementOp};
pub use player::Player;
pub use tuning::{
    MovementTuning, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE,
    BLINK_GRACE_TIME, BLINK_HOLD_THRESHOLD, BLINK_MAX_DOWNWARD_SPEED, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_TUNING, DOUBLE_JUMP_SPEED, FAST_FALL_ACCEL,
    FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ, FLIGHT_HOVER_SPEED,
    FLIGHT_TERMINAL_SPEED, GLIDE_AIR_ACCEL, GLIDE_FALL_SPEED, GRAVITY, GROUND_FRICTION,
    JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED, ONE_WAY_DROP_THROUGH_GRACE,
    POGO_SPEED, PRECISION_BLINK_AIM_SPEED, PRECISION_BLINK_DISTANCE,
    PRECISION_BLINK_MAX_DOWNWARD_SPEED, RUN_ACCEL, SLASH_RECOIL, WALL_CLIMB_SPEED,
    WALL_JUMP_X, WALL_SLIDE_SPEED,
};

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
    let mut events = FrameEvents::default();

    if input.reset_pressed && player.abilities.reset {
        player.reset_to(world.spawn);
        events.reset = true;
        return events;
    }

    update_facing_and_control_intent(player, input, tuning);
    handle_mode_toggles(player, input, &mut events);
    handle_blink(world, player, input, control_dt, tuning, &mut events);
    handle_attacks(world, player, input, tuning, &mut events);
    handle_dash(player, input, tuning, &mut events);
    handle_jump_release(player, input);

    events
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
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return events;
    }
    let dt = raw_dt.min(1.0 / 30.0);

    // Water contact is queried once per tick and cached on the
    // player so jump-buffer handling, gravity integration, and the
    // post-step reset gate all see the same answer. Source-agnostic:
    // `water_at` covers both IntGrid `Water` cells and entity
    // `WaterVolume` regions.
    player.water_contact = world.water_at(player.aabb());

    // Climbable contact: same one-query-per-tick discipline as
    // `water_contact`. Movement does not yet consume this -- the field
    // is populated for sandbox-side gameplay systems and the
    // RL/headless adapter so they can read a stable answer for the
    // current frame. Full BodyMode::Climbing integration in movement
    // is a follow-up; the data flow is already in place.
    player.climbable_contact = world.climbable_at(player.aabb());

    // Drowning gate: water without the swim ability is a death zone,
    // not a slow-down. Trigger the same reset path the hazard tile
    // uses so the existing flash/sfx/respawn pipeline applies.
    if player.water_contact.is_some() && !player.abilities.swim {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
        return events;
    }

    age_player(player, dt);
    update_simulation_timers(player, dt, tuning);
    handle_jump_buffer(world, player, input, tuning, &mut events);
    integrate_velocity(world, player, input, dt, tuning, &mut events);

    if touching_hazard(world, player) || player.pos.y > world.size.y + 200.0 {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
    }

    events
}

fn age_player(player: &mut Player, dt: f32) {
    player.time_alive += dt;
    player.max_speed = player.max_speed.max(player.vel.length());
    for mark in &mut player.combo {
        mark.age += dt;
    }
    player
        .combo
        .retain(|m| m.age < 4.0 || m.op == MovementOp::Reset);
}

fn update_facing_and_control_intent(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
) {
    if input.axis_x.abs() > 0.1 {
        player.facing = input.axis_x.signum();
    }

    if input.jump_pressed && player.abilities.jump {
        player.jump_buffer_timer = tuning.jump_buffer;
    }
    if input.dash_pressed && player.abilities.dash {
        player.dash_buffer_timer = tuning.dash_buffer;
    }
}

fn update_simulation_timers(player: &mut Player, dt: f32, tuning: MovementTuning) {
    player.jump_buffer_timer = dec(player.jump_buffer_timer, dt);
    player.dash_buffer_timer = dec(player.dash_buffer_timer, dt);
    player.coyote_timer = dec(player.coyote_timer, dt);
    player.drop_through_timer = dec(player.drop_through_timer, dt);
    player.dash_cooldown = dec(player.dash_cooldown, dt);
    player.blink_cooldown = dec(player.blink_cooldown, dt);
    player.blink_grace_timer = dec(player.blink_grace_timer, dt);
    player.rebound_cooldown = dec(player.rebound_cooldown, dt);

    if player.on_ground {
        player.coyote_timer = tuning.coyote_time;
        player.refresh_movement_resources(tuning);
    }
}

fn handle_mode_toggles(player: &mut Player, input: InputState, events: &mut FrameEvents) {
    if input.fly_toggle_pressed && player.abilities.fly {
        player.fly_enabled = !player.fly_enabled;
        if player.fly_enabled {
            player.fast_falling = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.dash_timer = 0.0;
            player.blink_grace_timer = 0.0;
        }
        events.op(player, MovementOp::FlyToggle);
    }
}

fn handle_blink(
    world: &World,
    player: &mut Player,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.blink {
        player.blink_hold_active = false;
        player.blink_aiming = false;
        player.blink_hold_timer = 0.0;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
        return;
    }

    if (input.blink_pressed || (input.blink_held && !player.blink_hold_active))
        && player.blink_cooldown <= 0.0
    {
        // Permit a held blink button to arm as soon as cooldown clears. This
        // avoids a bad second-blink case where the user pressed slightly early,
        // the hold was ignored, and bullet-time never engaged.
        player.blink_hold_active = true;
        player.blink_hold_timer = 0.0;
        player.blink_aiming = false;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    }

    if player.blink_hold_active && input.blink_held {
        // Blink hold/aim uses unscaled control time. During precision blink,
        // physics can be nearly frozen, but the destination cursor should still
        // feel like a responsive UI control.
        let control_dt = dt.min(1.0 / 20.0);
        player.blink_hold_timer += control_dt;
        if player.abilities.precision_blink
            && player.blink_hold_timer >= tuning.blink_hold_threshold
        {
            player.blink_aiming = true;
        }
        if player.blink_aiming {
            let aim_input = Vec2::new(input.axis_x, input.axis_y);
            if aim_input.length_squared() > 0.01 {
                player.blink_aim_offset +=
                    aim_input * (tuning.precision_blink_aim_speed * control_dt);
                player.blink_aim_offset = player
                    .blink_aim_offset
                    .clamp_length_max(tuning.precision_blink_distance);
            }
        }
    }

    if player.blink_hold_active && input.blink_released {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        let precision = player.blink_aiming && player.abilities.precision_blink;
        let from = player.pos;
        let to = if precision {
            blink_destination_to_point(world, player, player.pos + player.blink_aim_offset)
        } else {
            blink_destination(world, player, aim, tuning.blink_distance)
        };
        complete_blink(player, from, to, precision, tuning, events);
    }

    // Cancel a partially-started blink if the binding disappeared for any
    // reason without a release event. This avoids sticky bullet-time state when
    // focus changes or a future remapper swaps presets mid-hold.
    if player.blink_hold_active && !input.blink_held && !input.blink_released {
        player.blink_hold_active = false;
        player.blink_aiming = false;
        player.blink_hold_timer = 0.0;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    }
}

/// Finish a blink in one place so every blink variant shares the same
/// post-teleport state policy.
///
/// Blink completion is kept in one place so destination resolution, cooldowns,
/// presentation events, and post-blink state stay consistent across quick and
/// precision variants.
fn complete_blink(
    player: &mut Player,
    from: Vec2,
    to: Vec2,
    precision: bool,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    player.pos = to;
    apply_post_blink_motion(player, precision, tuning);
    player.blink_cooldown = tuning.blink_cooldown;
    player.blink_hold_active = false;
    player.blink_hold_timer = 0.0;
    player.blink_aiming = false;
    player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    let op = if precision {
        MovementOp::PrecisionBlink
    } else {
        MovementOp::Blink
    };
    events.op(player, op);
    events.blinks.push(BlinkEvent {
        from,
        to,
        precision,
    });
}

/// Apply the movement-state aftermath of a completed blink.
///
/// Blink is a topological reposition, not another gravity-preserving dash. This
/// policy is intentionally small and explicit. The real bullet-time invariant is
/// enforced by the split control/simulation clocks above; this function only
/// defines the immediate feel after teleporting.
fn apply_post_blink_motion(player: &mut Player, precision: bool, tuning: MovementTuning) {
    let damping = if precision { 0.35 } else { 0.55 };
    let max_downward = if precision {
        tuning.precision_blink_max_downward_speed
    } else {
        tuning.blink_max_downward_speed
    };

    player.vel.x *= damping;
    if player.vel.y > max_downward {
        player.vel.y = max_downward;
    } else {
        player.vel.y *= damping;
    }

    player.fast_falling = false;
    player.wall_clinging = false;
    player.wall_climbing = false;
    player.dash_timer = 0.0;
    player.blink_grace_timer = tuning.blink_grace_time;
}

fn handle_attacks(
    world: &World,
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.attack {
        return;
    }
    let can_pogo = player.abilities.pogo;
    if input.pogo_pressed && can_pogo {
        if let Some(orb_aabb) = try_pogo(world, player, tuning) {
            events.op(player, MovementOp::Pogo);
            events.pogo_hits.push(orb_aabb);
        } else {
            // Dedicated pogo whiff still gives a tiny correction so it can be
            // tested as a fourth face-button verb without requiring a target.
            player.vel.x -= player.facing * (tuning.slash_recoil * 0.45);
            events.op(player, MovementOp::Slash);
        }
    } else if input.attack_pressed {
        if can_pogo && input.axis_y > 0.25 {
            if let Some(orb_aabb) = try_pogo(world, player, tuning) {
                events.op(player, MovementOp::Pogo);
                events.pogo_hits.push(orb_aabb);
            } else {
                player.vel.x -= player.facing * tuning.slash_recoil;
                events.op(player, MovementOp::Slash);
            }
        } else {
            // A small generated recoil/correction action. It exists to test
            // cancellability and non-commutative feel.
            player.vel.x -= player.facing * tuning.slash_recoil;
            events.op(player, MovementOp::Slash);
        }
    }
}

fn handle_jump_buffer(
    world: &World,
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.jump_buffer_timer > 0.0 {
        // Underwater swimming wins over every other jump path: while
        // submerged with the swim ability, a buffered jump becomes
        // exactly one upward swim stroke and nothing else. This keeps
        // "underwater jump != normal jump" true on a single press,
        // and the `min(-impulse)` floor makes repeated taps reliably
        // rise even if the previous stroke is still climbing.
        if let Some(contact) = player.water_contact {
            if player.abilities.swim {
                let impulse = contact.spec.swim_up_impulse;
                player.vel.y = (player.vel.y - impulse).min(-impulse);
                player.jump_buffer_timer = 0.0;
                player.coyote_timer = 0.0;
                events.op(player, MovementOp::Jump);
                return;
            }
        }

        // Down + jump while standing on a one-way platform means "drop through",
        // not "jump". Cancel the buffered jump so the vertical sweep can take
        // the player past the platform on the next integration step.
        if input.drop_through_pressed && player.on_ground && standing_on_one_way(world, player) {
            player.jump_buffer_timer = 0.0;
            player.on_ground = false;
            player.coyote_timer = 0.0;
            // Latch the drop-through so subsequent frames keep ignoring the
            // one-way until the player has cleared the landing tolerance band.
            // Without this, the gesture only frees the player for a single
            // frame and the resolve-up step snaps them back onto the platform.
            player.drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
            return;
        }
        if player.abilities.wall_jump && player.on_wall && !player.on_ground {
            player.vel.x = player.wall_normal_x * tuning.wall_jump_x;
            player.vel.y = -tuning.jump_speed * 0.94;
            player.on_wall = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            events.op(player, MovementOp::WallJump);
        } else if player.abilities.jump && (player.on_ground || player.coyote_timer > 0.0) {
            player.vel.y = -tuning.jump_speed;
            player.on_ground = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            player.air_jumps_available = player.abilities.air_jump_count(tuning.air_jumps);
            events.op(player, MovementOp::Jump);
        } else if player.abilities.double_jump && player.air_jumps_available > 0 {
            player.vel.y = -tuning.double_jump_speed;
            player.on_ground = false;
            player.on_wall = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.jump_buffer_timer = 0.0;
            player.air_jumps_available -= 1;
            events.op(player, MovementOp::DoubleJump);
        }
    }
}

fn handle_jump_release(player: &mut Player, input: InputState) {
    // Variable jump height is an input/control gesture. It should react even
    // during bullet-time rather than waiting for scaled simulation time.
    if player.abilities.variable_jump && input.jump_released && player.vel.y < -120.0 {
        player.vel.y *= 0.54;
    }
}

fn handle_dash(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_buffer_timer > 0.0
        && player.abilities.dash
        && player.dash_charges_available > 0
        && player.dash_cooldown <= 0.0
    {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        player.vel = aim * tuning.dash_speed;
        player.dash_timer = tuning.dash_time;
        player.dash_cooldown = tuning.dash_cooldown;
        player.dash_buffer_timer = 0.0;
        let op = player.spend_dash_charge();
        events.op(player, op);
    }
}

fn integrate_velocity(
    world: &World,
    player: &mut Player,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_timer > 0.0 {
        player.dash_timer = dec(player.dash_timer, dt);
    } else if player.body_mode == crate::player_state::BodyMode::Climbing
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
}

/// Integrate one frame of climbing. Suspends gravity, drives vertical
/// velocity directly from `input.axis_y * climb_speed`, and scales
/// horizontal motion by `strafe_factor` so the player can align with
/// rungs without sliding off the ladder.
///
/// Caller must guarantee `player.body_mode == BodyMode::Climbing` and
/// `player.climbable_contact.is_some()`. The contact's `spec` provides
/// the per-region tuning (climb_speed / strafe_factor) so authoring
/// can override defaults per ladder.
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

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

fn is_solid_for_axis(kind: BlockKind, axis: Axis) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => matches!(axis, Axis::Y),
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

/// While climbing, blocks that overlap the active climbable region
/// are passable so the player can complete the climb at the top
/// without being blocked by an authored floor / ceiling tile that
/// happens to share footprint with the ladder. Hazards remain
/// dangerous regardless -- this only relaxes Solid / BlinkWall /
/// OneWay collision.
///
/// Generalizes the room-author trick of "carve a gap in the platform
/// where the ladder ends" so future ladder rooms don't need the
/// gap. Source: ladder_lab fix 2026-05-07.
fn block_passable_during_climb(player: &Player, block: &crate::world::Block) -> bool {
    if !matches!(player.body_mode, crate::player_state::BodyMode::Climbing) {
        return false;
    }
    let Some(contact) = player.climbable_contact else {
        return false;
    };
    if matches!(block.kind, BlockKind::Hazard) {
        return false;
    }
    contact.region_aabb.strict_intersects(block.aabb)
}

fn sweep_fraction(time_of_impact: f32) -> f32 {
    time_of_impact.clamp(0.0, 1.0)
}

fn sweep_player_x(world: &World, player: &mut Player, delta_x: f32) {
    let delta = Vec2::new(delta_x, 0.0);
    if delta.x.abs() <= 1.0e-5 {
        resolve_axis(world, player, Axis::X);
        return;
    }

    if let Some(hit) = world.first_body_sweep(player.aabb(), delta, |block| {
        is_solid_for_axis(block.kind, Axis::X)
            && !matches!(block.kind, BlockKind::OneWay)
            && !block_passable_during_climb(player, block)
    }) {
        let toi_fraction = sweep_fraction(hit.time_of_impact);
        player.pos.x += delta.x * toi_fraction;
        let body = player.aabb();
        let immediate_contact = hit.time_of_impact <= 1.0e-5;
        let overlap_x = (body.right().min(hit.block.aabb.right())
            - body.left().max(hit.block.aabb.left()))
        .max(0.0);
        let overlap_y = (body.bottom().min(hit.block.aabb.bottom())
            - body.top().max(hit.block.aabb.top()))
        .max(0.0);
        // Skip the horizontal snap in two failure-mode cases:
        // 1. The contact is dominantly *vertical* (player's head poking
        //    into a wide ceiling, or feet poking into a wide floor). The
        //    perpendicular `resolve_vertical` pass owns this contact;
        //    pushing horizontally toward the block's far edge would
        //    catapult the player across the entire room.
        // 2. The contact is dominantly horizontal but the player is
        //    *already moving away* from the block (e.g. wall-jump pushed
        //    them off a wall they were sub-pixel-penetrating). The
        //    delta-direction snap logic uses delta.x sign to pick a face;
        //    when the player is on the far side from where delta.x points
        //    that pick is wrong and pushes them through the block.
        let vertical_dominant = immediate_contact && overlap_y > 0.0 && overlap_x > overlap_y;
        let body_to_right_of_block = body.center().x > hit.block.aabb.center().x;
        let moving_away_from_block =
            (body_to_right_of_block && delta.x > 0.0) || (!body_to_right_of_block && delta.x < 0.0);
        let horizontal_overlap_moving_away =
            immediate_contact && overlap_x > 0.0 && moving_away_from_block;
        if vertical_dominant || horizontal_overlap_moving_away {
            player.pos.x += delta.x * (1.0 - toi_fraction);
        } else {
            // Pick the snap face from the player's *position relative to
            // the block*, not from delta.x sign. The two only agree when
            // the player is approaching from the side delta.x implies;
            // for a pre-existing overlap they can disagree, which is the
            // tunneling failure mode addressed above.
            if body_to_right_of_block {
                player.pos.x += hit.block.aabb.right() - body.left();
                player.wall_normal_x = 1.0;
            } else {
                player.pos.x += hit.block.aabb.left() - body.right();
                player.wall_normal_x = -1.0;
            }
            player.vel.x = 0.0;
            player.on_wall = true;
        }
    } else {
        player.pos.x += delta.x;
    }

    // Shape casts catch fast motion; positional resolution remains as a cheap
    // penetration repair for starts inside geometry or stacked contacts.
    resolve_axis(world, player, Axis::X);
}

fn sweep_player_y(
    world: &World,
    player: &mut Player,
    delta_y: f32,
    prev_bottom: f32,
    drop_through: bool,
) {
    let delta = Vec2::new(0.0, delta_y);
    if delta.y.abs() <= 1.0e-5 {
        resolve_vertical(world, player, prev_bottom, drop_through);
        return;
    }

    let start_body = player.aabb();
    if let Some(hit) = world.first_body_sweep(player.aabb(), delta, |block| {
        if !is_solid_for_axis(block.kind, Axis::Y) {
            return false;
        }
        if block_passable_during_climb(player, block) {
            return false;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = delta.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            return landing_from_above && !drop_through;
        }
        // AMBITION_REVIEW(spatial): reject blocks the body is already
        // overlapping dominantly on the y axis. Concrete repro: a player
        // wall-clinging on a tall left-side wall whose top is at world
        // y=0 used to get a `time_of_impact = 0` hit on the wall during
        // the downward y-sweep, then snap the body's bottom to the wall's
        // top — teleporting the player from `(62, 1678)` to `(62, -23)`
        // (= `0 - half_height`). The fix mirrors `resolve_axis(Axis::X)`'s
        // overlap-shape guard: when the existing x-overlap is non-zero
        // and the y-overlap is larger, this is a *side-wall* contact and
        // belongs to the x-axis sweep/resolve. The vertical sweep should
        // not see it. See `docs/lessons_learned.md` for the trace
        // signature and the regression test.
        if body_is_side_contact(start_body, block.aabb) {
            return false;
        }
        true
    }) {
        player.pos.y += delta.y * sweep_fraction(hit.time_of_impact);
        let body = player.aabb();
        if delta.y > 0.0 || body.center().y < hit.block.aabb.center().y {
            player.pos.y += hit.block.aabb.top() - body.bottom();
            player.on_ground = true;
        } else {
            player.pos.y += hit.block.aabb.bottom() - body.top();
        }
        player.vel.y = 0.0;
    } else {
        player.pos.y += delta.y;
    }

    resolve_vertical(world, player, prev_bottom, drop_through);
}

/// True when `body`'s vertical range fits entirely inside `block`'s
/// vertical range — i.e. the body is *alongside* the block, not above
/// or below it. The y-axis sweep / resolve cannot legitimately produce
/// a contact in that geometry: any landing has the body's bottom
/// approaching the block's top from above, and any ceiling hit has the
/// body's top approaching the block's bottom from below. A body
/// fully nested inside the block's y-range can only be touching the
/// block on its left or right *side*, which the x-axis sweep / resolve
/// owns.
///
/// This is the symmetric counterpart to the `overlap_x > overlap_y`
/// guard `resolve_axis(Axis::X)` already uses. The first revision of
/// this fix required `overlap_x > 0` (strict penetration), which
/// missed the trace's *exact-edge-touching* case (`body.left ==
/// wall.right` to within float precision). The current predicate
/// catches both edge-touching and penetrating side contacts because
/// the y-range test is independent of x-overlap.
///
/// Tolerance: a 1e-4 epsilon on the y-range bounds so a body whose
/// top exactly equals the block's top (e.g. a player standing at the
/// same height as a one-tile-tall ledge corner) is still classified
/// as a side-contact when the bottom is also inside.
fn body_is_side_contact(body: Aabb, block: Aabb) -> bool {
    const Y_NESTED_EPS: f32 = 1.0e-4;
    body.top() >= block.top() - Y_NESTED_EPS && body.bottom() <= block.bottom() + Y_NESTED_EPS
}

// AMBITION_REVIEW(spatial): one-way platform contact test. The 4px vertical
// epsilon mirrors the landing tolerance used by the vertical sweep; if either
// is changed the other should follow.
fn standing_on_one_way(world: &World, player: &Player) -> bool {
    let body = player.aabb();
    for block in &world.blocks {
        if !matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        let horizontally_overlaps =
            body.right() > block.aabb.left() + 1.0 && body.left() < block.aabb.right() - 1.0;
        let near_top = (body.bottom() - block.aabb.top()).abs() <= 4.0;
        if horizontally_overlaps && near_top {
            return true;
        }
    }
    false
}

fn resolve_axis(world: &World, player: &mut Player, axis: Axis) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, axis) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            continue;
        }
        match axis {
            Axis::X => {
                // Only resolve as a horizontal contact when the overlap is
                // shallower in x than in y. Otherwise this is a vertical
                // contact (player's head poking into a wide ceiling, or feet
                // poking into a wide floor) and the appropriate axis is the
                // perpendicular `resolve_vertical` pass — pushing
                // horizontally instead can catapult the player across the
                // entire room (the floor/ceiling block spans the whole
                // width, so its near edge is far away). Concrete repro: a
                // wall-jump off the left wall while feet barely overlap the
                // floor used to teleport the player tens of pixels left
                // through the wall.
                let overlap_x = (aabb.right().min(block.aabb.right())
                    - aabb.left().max(block.aabb.left()))
                .max(0.0);
                let overlap_y = (aabb.bottom().min(block.aabb.bottom())
                    - aabb.top().max(block.aabb.top()))
                .max(0.0);
                if overlap_x > overlap_y {
                    continue;
                }
                if aabb.center().x < block.aabb.center().x {
                    let push = block.aabb.left() - aabb.right();
                    player.pos.x += push;
                    player.wall_normal_x = -1.0;
                } else {
                    let push = block.aabb.right() - aabb.left();
                    player.pos.x += push;
                    player.wall_normal_x = 1.0;
                }
                player.vel.x = 0.0;
                player.on_wall = true;
            }
            Axis::Y => {}
        }
        aabb = player.aabb();
    }
}

fn resolve_vertical(world: &World, player: &mut Player, prev_bottom: f32, drop_through: bool) {
    let mut aabb = player.aabb();
    for block in &world.blocks {
        if !is_solid_for_axis(block.kind, Axis::Y) || !aabb.strict_intersects(block.aabb) {
            continue;
        }
        if matches!(block.kind, BlockKind::OneWay) {
            let landing_from_above = player.vel.y >= 0.0 && prev_bottom <= block.aabb.top() + 8.0;
            if !landing_from_above || drop_through {
                continue;
            }
        }
        // AMBITION_REVIEW(spatial): symmetric to `resolve_axis(Axis::X)`.
        // If the body's y-range is entirely nested inside the block's
        // y-range, this is a side-wall contact — the x-axis sweep /
        // resolve owns it. Pushing vertically here can catapult the
        // player to the wall block's top edge if the wall spans the
        // full room height (concrete repro: wall-clinging on a tall
        // left wall whose top is at world y=32 used to teleport the
        // player to y = top - half_height = 9). Skipping OneWay
        // because OneWay is by construction wider than tall.
        if !matches!(block.kind, BlockKind::OneWay) && body_is_side_contact(aabb, block.aabb) {
            continue;
        }
        if aabb.center().y < block.aabb.center().y {
            let push = block.aabb.top() - aabb.bottom();
            player.pos.y += push;
            player.on_ground = true;
        } else {
            let push = block.aabb.bottom() - aabb.top();
            player.pos.y += push;
        }
        player.vel.y = 0.0;
        aabb = player.aabb();
    }
}

/// Attempt a pogo bounce. Returns the AABB of the orb-like block that was
/// hit (for the sandbox to route damage to a breakable pogo orb), or `None`
/// if no valid target was under the player's feet. Non-PogoOrb hits return
/// the AABB too so callers don't need to second-guess the kind, but the
/// sandbox damage path filters for orbs by matching against breakables
/// flagged `pogo_refresh`.
fn try_pogo(world: &World, player: &mut Player, tuning: MovementTuning) -> Option<Aabb> {
    let feet = player.aabb();
    let hitbox = Aabb::new(
        Vec2::new(feet.center().x, feet.bottom() + 18.0),
        Vec2::new(feet.half_size().x * 0.76, 22.0),
    );
    let hit = world.blocks.iter().find(|block| {
        let valid_target = matches!(
            block.kind,
            BlockKind::PogoOrb
                | BlockKind::Solid
                | BlockKind::BlinkWall { .. }
                | BlockKind::Rebound { .. }
        );
        valid_target && hitbox.strict_intersects(block.aabb)
    });
    if let Some(block) = hit {
        let aabb = block.aabb;
        player.vel.y = -tuning.pogo_speed;
        player.refresh_movement_resources(tuning);
        player.on_ground = false;
        Some(aabb)
    } else {
        None
    }
}

fn touching_hazard(world: &World, player: &Player) -> bool {
    let aabb = player.aabb();
    world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, BlockKind::Hazard) && aabb.strict_intersects(b.aabb))
}

fn touching_rebound(world: &World, player: &Player) -> Option<Vec2> {
    let aabb = player.aabb();
    world.blocks.iter().find_map(|b| match b.kind {
        BlockKind::Rebound { impulse } if aabb.strict_intersects(b.aabb) => Some(impulse),
        _ => None,
    })
}

/// Compute the furthest safe blink destination along `aim`.
///
/// Blink should feel like a topological reposition, but it must not place the
/// player inside solid geometry. The implementation uses a Parry-backed shape
/// cast for hard blockers, then samples the remaining path so blink-through
/// walls can be crossed without becoming valid resting positions.
pub fn blink_destination(world: &World, player: &Player, aim: Vec2, max_distance: f32) -> Vec2 {
    let direction = aim.normalize_or(Vec2::new(player.facing, 0.0));
    blink_destination_to_point(world, player, player.pos + direction * max_distance)
}

/// Compute a safe blink destination toward a deliberate target point.
///
/// The path may cross configured blink walls if the player's ability set allows
/// it, but the final resting AABB must be free of solid geometry. This lets
/// blink-through upgrades become meaningful without ever depositing the player
/// inside a wall.
pub fn blink_destination_to_point(world: &World, player: &Player, target: Vec2) -> Vec2 {
    let start = player.pos;
    let half = player.size * 0.5;
    let mut target = target;
    target.x = target.x.clamp(half.x, world.size.x - half.x);
    target.y = target.y.clamp(half.y, world.size.y - half.y);
    let delta = target - start;
    let distance = delta.length();
    if distance <= 1.0e-5 {
        return start;
    }

    let start_body = Aabb::new(start, half);
    let max_t = world
        .first_body_sweep(start_body, delta, |block| {
            blink_path_blocker(player, block.kind)
        })
        .map(|hit| hit.time_of_impact)
        .unwrap_or(1.0);
    let sweep_target = start + delta * max_t;
    last_free_blink_position(world, player, start, sweep_target, half)
}

fn blink_path_blocker(player: &Player, kind: BlockKind) -> bool {
    match kind {
        BlockKind::Solid => true,
        BlockKind::BlinkWall { tier } => !player_can_blink_through(player, tier),
        BlockKind::OneWay | BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {
            false
        }
    }
}

fn last_free_blink_position(
    world: &World,
    player: &Player,
    start: Vec2,
    target: Vec2,
    half: Vec2,
) -> Vec2 {
    let delta = target - start;
    let distance = delta.length();
    if distance <= 1.0e-5 {
        return start;
    }

    let steps = ((distance / 14.0).ceil() as usize).clamp(8, 64);
    let mut last_safe = start;
    for step in 1..=steps {
        let t = step as f32 / steps as f32;
        let candidate = start + delta * t;
        let candidate_aabb = Aabb::new(candidate, half);
        match blink_collision(world, player, candidate_aabb) {
            BlinkCollision::Free => last_safe = candidate,
            BlinkCollision::PassThrough => {}
            BlinkCollision::Blocked => break,
        }
    }
    last_safe
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlinkCollision {
    Free,
    PassThrough,
    Blocked,
}

fn blink_collision(world: &World, player: &Player, aabb: Aabb) -> BlinkCollision {
    let mut pass_through = false;
    for block in &world.blocks {
        if !aabb.strict_intersects(block.aabb) {
            continue;
        }
        match block.kind {
            BlockKind::Solid => return BlinkCollision::Blocked,
            BlockKind::BlinkWall { tier } => {
                if player_can_blink_through(player, tier) {
                    pass_through = true;
                } else {
                    return BlinkCollision::Blocked;
                }
            }
            BlockKind::OneWay => pass_through = true,
            BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => {}
        }
    }
    if pass_through {
        BlinkCollision::PassThrough
    } else {
        BlinkCollision::Free
    }
}

fn player_can_blink_through(player: &Player, tier: BlinkWallTier) -> bool {
    match tier {
        BlinkWallTier::Soft => player.abilities.blink_through_soft_walls,
        BlinkWallTier::Hard => player.abilities.blink_through_hard_walls,
    }
}

#[cfg(test)]
mod tests;
