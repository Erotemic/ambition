use crate::geometry::AabbExt;
use crate::world::World;
use crate::Vec2;

/// Move `value` toward `target` by at most `delta`. Inlined from the
/// removed `ae::scalar::approach`.
fn approach(value: f32, target: f32, delta: f32) -> f32 {
    if value < target {
        (value + delta).min(target)
    } else {
        (value - delta).max(target)
    }
}

/// Clamp the velocity component ALONG `gravity_dir` (the fall direction) to
/// `cap`, leaving the perpendicular (movement) component untouched. The
/// gravity-direction-relative form of `vel.y = vel.y.min(cap)`.
fn cap_fall_speed(vel: &mut crate::Vec2, gravity_dir: crate::Vec2, cap: f32) {
    let along = vel.dot(gravity_dir);
    if along > cap {
        *vel -= (along - cap) * gravity_dir;
    }
}

/// Launch the body at `speed` OPPOSITE `gravity_dir` (a jump / pogo / wall-kick
/// vertical impulse), preserving the perpendicular (movement-axis) component.
/// The gravity-direction-relative form of `vel.y = -speed * gravity_sign`.
pub fn set_jump_velocity(vel: &mut crate::Vec2, gravity_dir: crate::Vec2, speed: f32) {
    let perp = *vel - vel.dot(gravity_dir) * gravity_dir;
    *vel = perp - speed * gravity_dir;
}

/// Screen-vertical input (`axis_y`, +Y = screen-down) → the gravity-relative
/// "descend" (toward-the-feet) intent that gates crouch / pogo / drop-through /
/// fast-fall and drives gravity-relative vertical movement. The vertical sibling
/// of the run axis ([`crate::AccelerationFrame::control_frame`]'s `side`): that
/// keeps the run axis player-relative, this keeps the gate axis sign-consistent.
///
/// CONVENTION — this game's; change it HERE and every gate moves together. The
/// gate stays on the up/down keys; its sign flips only when gravity rotates PAST
/// ±90° from screen-down (its screen-down component goes negative). So down AND
/// sideways gravity read screen-down as "descend"; only past horizontal (gravity
/// pointing up-ish) does screen-up become "descend". For default down gravity
/// this is the identity, so normal play is byte-identical.
pub fn gravity_descend(axis_y: f32, gravity_dir: crate::Vec2) -> f32 {
    let gate_sign = if gravity_dir.y < 0.0 { -1.0 } else { 1.0 };
    axis_y * gate_sign
}

/// The "drop through a one-way platform" gesture: press the descend gate (toward
/// gravity) + jump. Gravity-relative via [`gravity_descend`], so under inverted
/// gravity it reads screen-UP + jump. Computed here at the consumer (where
/// `gravity_dir` is known) rather than precomputed gravity-blind at the input
/// boundary.
pub(super) fn wants_drop_through(
    axis_y: f32,
    jump_pressed: bool,
    gravity_dir: crate::Vec2,
) -> bool {
    gravity_descend(axis_y, gravity_dir) > 0.35 && jump_pressed
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
    clusters: &mut crate::player_clusters::PlayerClustersMut<'_>,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    use crate::player_state::BodyMode;

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
        // Normal mode — the shared physics spine (gravity-direction-relative).
        integrate_normal_clusters(
            clusters.kinematics,
            clusters.flight,
            clusters.ground,
            clusters.blink,
            clusters.env_contact,
            clusters.abilities,
            input,
            dt,
            tuning,
        );
    }

    // Pre-X-sweep state.
    clusters.wall.on_wall = false;
    let pre_wall_snapshot = clusters.kinematics.vel;
    clusters.wall.wall_normal_x = 0.0;
    clusters.wall.wall_climbing = false;
    let was_clinging = clusters.wall.wall_clinging;
    clusters.wall.wall_clinging = false;

    // Under sideways gravity X is the GRAVITY axis (wall-walking): the X sweep is
    // the gravity sweep, so on_ground is set by a probe after the sweeps and the
    // vertical-only wall abilities (cling / wall-jump) are skipped for this slice.
    let gravity_on_x = tuning.gravity_dir.x != 0.0;

    // X-sweep — fully cluster-native.
    let dt_x = clusters.kinematics.vel.x * dt;
    super::collision::sweep_player_x_clusters(
        world,
        clusters.kinematics,
        clusters.wall,
        clusters.body_mode,
        clusters.env_contact,
        dt_x,
        tuning.gravity_dir,
    );

    if !gravity_on_x {
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
    }

    // Pre-Y-sweep state.
    let prev_bottom = clusters
        .kinematics
        .aabb_oriented(tuning.gravity_dir)
        .bottom();
    if !gravity_on_x {
        // Y is the gravity axis (down/up): reset on_ground before the Y sweep
        // grounds the player. Under sideways gravity the probe below owns it.
        clusters.ground.on_ground = false;
    }
    let drop_through = wants_drop_through(input.axis_y, input.jump_pressed, tuning.gravity_dir)
        || clusters.ground.drop_through_timer > 0.0;
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
        tuning.gravity_dir,
    );

    // Wall-walking ground probe: under sideways gravity the X (gravity-axis)
    // sweep has stopped the body against the wall; ground it when a surface sits
    // right there on the gravity side, and clear the spurious wall contact.
    if gravity_on_x {
        clusters.ground.on_ground = super::collision::grounded_against_gravity(
            world,
            clusters.kinematics.aabb_oriented(tuning.gravity_dir),
            tuning.gravity_dir,
        );
        clusters.wall.on_wall = false;
    }

    if clusters.ground.on_ground {
        crate::player_clusters::refresh_movement_resources_clusters(
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
        if let Some(impulse) = super::collision::touching_rebound_aabb(
            world,
            clusters.kinematics.aabb_oriented(tuning.gravity_dir),
        ) {
            clusters.kinematics.vel = impulse;
            crate::player_clusters::refresh_movement_resources_clusters(
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
/// Normal-mode integration — the shared physics SPINE (not a composable limb):
/// gravity-direction-relative gravity, fast-fall, glide-gate, run/friction, and
/// the fall-speed cap. The fourth mode-select branch alongside dash (skip),
/// climb, and flight. Everything projects onto `tuning.gravity_dir` so sideways /
/// flipped gravity Just Works — the property enemies/NPCs inherit when they move
/// onto this spine (and the reason their Y-only `gravity_sign` fall bug vanishes).
pub(super) fn integrate_normal_clusters(
    kinematics: &mut crate::player_clusters::BodyKinematics,
    flight: &mut crate::player_clusters::PlayerFlightState,
    ground: &crate::player_clusters::PlayerGroundState,
    blink: &crate::player_clusters::PlayerBlinkState,
    env_contact: &crate::player_clusters::PlayerEnvironmentContact,
    abilities: &crate::player_clusters::PlayerAbilities,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
) {
    // The player adapter: project its rich clusters into the actor-generic
    // spine context (ability components → gating flags) and run the one spine.
    integrate_normal_spine(
        &mut kinematics.vel,
        &mut flight.fast_falling,
        &mut flight.gliding,
        NormalSpineCtx {
            on_ground: ground.on_ground,
            blink_grace: blink.grace_timer > 0.0,
            water: env_contact.water,
            can_fast_fall: abilities.abilities.fast_fall,
            can_glide: abilities.abilities.glide,
            can_move_horizontal: abilities.abilities.move_horizontal,
        },
        input,
        dt,
        tuning,
    );
}

/// Read-only gating the normal-mode spine consults. Every field models a player
/// ability/contact; an actor that carries none of those components feeds
/// `on_ground` + `can_move_horizontal` and leaves the rest at their "absent"
/// values, getting pure gravity + run + fall-cap. This is the pay-for-use seam:
/// the spine is the SAME core the player runs with its abilities switched on.
#[derive(Clone, Copy)]
pub struct NormalSpineCtx {
    pub on_ground: bool,
    /// Blink hang-time is active this frame (`PlayerBlinkState::grace_timer > 0`).
    pub blink_grace: bool,
    pub water: Option<crate::world::WaterContact>,
    pub can_fast_fall: bool,
    pub can_glide: bool,
    pub can_move_horizontal: bool,
}

impl NormalSpineCtx {
    /// The gating a bare actor (enemy/NPC) with no player ability components
    /// presents: it moves horizontally (run) and falls, nothing else.
    pub fn bare(on_ground: bool) -> Self {
        Self {
            on_ground,
            blink_grace: false,
            water: None,
            can_fast_fall: false,
            can_glide: false,
            can_move_horizontal: true,
        }
    }
}

/// Normal-mode integration — the shared physics SPINE, actor-generic. Applies
/// gravity-direction-relative gravity, fast-fall, glide-gate, run/friction, and
/// the fall-speed cap to ANY body's `vel`, gated only by the small
/// [`NormalSpineCtx`]. Everything projects onto `tuning.gravity_dir` so sideways /
/// flipped gravity Just Works. The player feeds it via `integrate_normal_clusters`;
/// enemies/NPCs feed it via [`NormalSpineCtx::bare`] (+ per-actor `tuning`).
pub fn integrate_normal_spine(
    kin_vel: &mut Vec2,
    fast_falling: &mut bool,
    gliding: &mut bool,
    ctx: NormalSpineCtx,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
) {
    let g = tuning.gravity_dir;
    // Fall-direction speed BEFORE this frame's gravity (terminal velocity is an
    // equilibrium gravity accelerates UP TO, not a brake on an over-cap fling).
    let fall_along_before = kin_vel.dot(g).max(0.0);
    let blink_hang_active = ctx.blink_grace && kin_vel.dot(g) >= 0.0;
    let water_gravity_scale = ctx.water.map(|c| c.spec.gravity_scale).unwrap_or(1.0);
    if !blink_hang_active {
        *kin_vel += tuning.gravity * g * water_gravity_scale * dt;
    }
    if input.fast_fall_pressed && ctx.can_fast_fall && !ctx.on_ground {
        *fast_falling = true;
    }
    if *fast_falling && !blink_hang_active && ctx.water.is_none() {
        *kin_vel += tuning.fast_fall_accel * g * dt;
    }
    *gliding = ctx.can_glide
        && !ctx.on_ground
        && !*fast_falling
        && !blink_hang_active
        && ctx.water.is_none()
        && input.jump_held
        && kin_vel.dot(g) > 0.0;

    if ctx.can_move_horizontal {
        let accel = if ctx.on_ground {
            tuning.run_accel
        } else if *gliding {
            tuning.glide_air_accel
        } else {
            tuning.air_accel
        };
        // Run/friction act along the control frame's `side` axis so `axis_x = +1`
        // walks the body toward THEIR right at any gravity orientation.
        let m = crate::AccelerationFrame::new(g)
            .control_frame(tuning.input_frame_mode)
            .side;
        let along = kin_vel.dot(m);
        let target = input.axis_x * tuning.max_run_speed;
        let mut new_along = approach(along, target, accel * dt);
        let friction = if ctx.on_ground {
            tuning.ground_friction
        } else {
            tuning.air_friction
        };
        if input.axis_x.abs() <= 0.1 {
            new_along = approach(new_along, 0.0, friction * dt);
        }
        *kin_vel += (new_along - along) * m;
    }

    if let Some(contact) = ctx.water {
        let drag = contact.spec.drag.clamp(0.0, 1.0);
        *kin_vel *= 1.0 - drag;
        cap_fall_speed(kin_vel, g, contact.spec.max_fall_speed);
    } else {
        // `relax` = treat the cap as an equilibrium (never decelerate an over-cap
        // fling like a portal exit). GLIDING is an intentional brake, so it keeps a
        // hard clamp; terminal velocity + fast-fall do not.
        let (fall_cap, relax) = if *fast_falling {
            (tuning.fast_fall_speed, true)
        } else if *gliding {
            (tuning.glide_fall_speed, false)
        } else {
            (tuning.max_fall_speed, true)
        };
        let effective_cap = if relax {
            fall_cap.max(fall_along_before)
        } else {
            fall_cap
        };
        cap_fall_speed(kin_vel, g, effective_cap);
    }
}

pub(super) fn integrate_climb_clusters(
    kinematics: &mut crate::player_clusters::BodyKinematics,
    env_contact: &crate::player_clusters::PlayerEnvironmentContact,
    flight: &mut crate::player_clusters::PlayerFlightState,
    wall: &mut crate::player_clusters::PlayerWallState,
    jump: &mut crate::player_clusters::PlayerJumpState,
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
    // The boost GATE ("press away from gravity") is gravity-relative; the climb
    // SPEED stays raw `axis_y` (screen-vertical along the ladder, already
    // gravity-symmetric since it's a direct screen-space velocity).
    let pressing_away_from_gravity = gravity_descend(input.axis_y, tuning.gravity_dir) < -0.1;
    let target_vy = if jump.ladder_jump_boost > 0.0 && pressing_away_from_gravity {
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
    kinematics: &mut crate::player_clusters::BodyKinematics,
    flight: &mut crate::player_clusters::PlayerFlightState,
    wall: &mut crate::player_clusters::PlayerWallState,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
) {
    flight.fast_falling = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    flight.flight_phase += dt * tuning.flight_hover_hz * std::f32::consts::TAU;

    // Free flight respects the reference frame: under sideways/up gravity the stick
    // maps through the player's CONTROL frame (run = `side`, descend = `down`), so
    // "right" moves the player player-right. We do the whole integration in those
    // frame components and map back to world at the end — under normal gravity the
    // control frame is the identity, so this is byte-identical.
    let control = crate::reference_frame::AccelerationFrame::new(tuning.gravity_dir)
        .control_frame(tuning.input_frame_mode);
    let vel_run = kinematics.vel.dot(control.side);
    let vel_descend = kinematics.vel.dot(control.down);

    let target_run = input.axis_x * tuning.flight_terminal_speed;
    let mut target_descend = input.axis_y * tuning.flight_terminal_speed;
    if input.axis_y.abs() <= 0.10 {
        target_descend = flight.flight_phase.sin() * tuning.flight_hover_speed;
    }

    let mut new_run = approach(vel_run, target_run, tuning.flight_accel * dt);
    let mut new_descend = approach(vel_descend, target_descend, tuning.flight_accel * dt);

    if input.axis_x.abs() <= 0.10 {
        new_run = approach(new_run, 0.0, tuning.flight_drag * dt);
    }
    if input.axis_y.abs() <= 0.10 {
        new_descend = approach(new_descend, target_descend, tuning.flight_drag * dt);
    }

    new_run = new_run.clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
    new_descend = new_descend.clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);

    kinematics.vel = control.to_world(crate::Vec2::new(new_run, new_descend));
}

/// Wall ability ride: while pressed into a wall (axis_x against the
/// wall normal), engage wall-cling (clamp `vel.y` to `wall_slide_speed`)
/// or, with `wall_climb` + |axis_y| > 0.25, drive `vel.y` directly.
/// Records the first transition op so the trace recorder fires
/// `WallCling` / `WallClimb` exactly once per engagement.
///
pub(super) fn apply_wall_abilities_clusters(
    kinematics: &mut crate::player_clusters::BodyKinematics,
    ground: &crate::player_clusters::PlayerGroundState,
    wall: &mut crate::player_clusters::PlayerWallState,
    abilities: &crate::player_clusters::PlayerAbilities,
    combo_trace: &mut crate::player_clusters::PlayerComboTrace,
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
