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
/// the feet) + jump. The `descend` scalar is the resolved player-frame `y` from
/// [`MovementTuning::stick`], so it is gravity- AND input-mode-relative (under
/// inverted gravity, Hybrid reads screen-UP + jump). Computed at the consumer
/// rather than precomputed gravity-blind at the input boundary.
pub(super) fn wants_drop_through(descend: f32, jump_pressed: bool) -> bool {
    descend > 0.35 && jump_pressed
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
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
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

    // Pre-sweep state.
    clusters.wall.on_wall = false;
    let pre_wall_snapshot = clusters.kinematics.vel;
    clusters.wall.wall_normal_x = 0.0;
    clusters.wall.wall_climbing = false;
    let was_clinging = clusters.wall.wall_clinging;
    clusters.wall.wall_clinging = false;

    // The sweeps are still X/Y because the world is axis-aligned, but both the
    // ORDER and the SEMANTICS are local-frame: sweep the controlled body's side
    // axis first (arming wall contact), apply wall abilities against last-frame
    // ground state, clear ground, then sweep the gravity/support axis, which
    // owns landing. ONE sequence and ONE role-aware sweep for every cardinal
    // gravity (fable review 2026-07-02 §B5/§B6 — the per-world-axis sweep pair
    // and the per-branch ordering each broke whichever axis gravity rotated
    // onto). In particular, a sideways body that runs off a ledge loses support
    // before the gravity-axis sweep of the SAME frame, not one frame later
    // because X happened to run before Y.
    let gravity_on_x = tuning.gravity_dir.x != 0.0;
    let (side_axis, gravity_axis) = if gravity_on_x {
        (
            crate::collision_semantics::Axis::Y,
            crate::collision_semantics::Axis::X,
        )
    } else {
        (
            crate::collision_semantics::Axis::X,
            crate::collision_semantics::Axis::Y,
        )
    };

    let drop_through = wants_drop_through(tuning.stick(&input).y, input.jump_pressed)
        || clusters.ground.drop_through_timer > 0.0;

    let sweep = |clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
                 axis: crate::collision_semantics::Axis| {
        let prev_feet_coord = clusters
            .kinematics
            .aabb_oriented(tuning.gravity_dir)
            .feet_coord(tuning.gravity_dir);
        let delta_along = match axis {
            crate::collision_semantics::Axis::X => clusters.kinematics.vel.x,
            crate::collision_semantics::Axis::Y => clusters.kinematics.vel.y,
        } * dt;
        super::collision::sweep_player_axis_clusters(
            world,
            clusters.kinematics,
            clusters.ground,
            clusters.wall,
            clusters.body_mode,
            clusters.env_contact,
            axis,
            delta_along,
            prev_feet_coord,
            drop_through,
            tuning.gravity_dir,
        );
    };

    sweep(clusters, side_axis);
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
    clusters.ground.on_ground = false;
    sweep(clusters, gravity_axis);

    // Emergent platform riding — the SAME rule the shared `step_kinematic` sweep
    // applies to enemies/NPCs: a grounded body resting on a MOVING solid is carried
    // by that solid's gravity-perpendicular velocity (the gravity-axis ride is
    // already handled by gravity + the landing). Static geometry carries `ZERO`, so
    // this is a no-op off moving platforms. This is why the player — and the
    // brain-driven clone, which runs this exact core — ride moving platforms: not a
    // player feature, a property of standing on a moving solid.
    if clusters.ground.on_ground {
        let g = tuning.gravity_dir;
        let oriented = clusters.kinematics.aabb_oriented(g);
        if let Some(support) = crate::collision_semantics::supporting_block(
            world,
            oriented,
            g,
            clusters.ground.drop_through_timer > 0.0,
        ) {
            let v = support.velocity;
            clusters.kinematics.pos += v - v.dot(g) * g;
        }
    }

    if clusters.ground.on_ground {
        crate::body_clusters::refresh_movement_resources_clusters(
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
            crate::body_clusters::refresh_movement_resources_clusters(
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
    kinematics: &mut crate::body_clusters::BodyKinematics,
    flight: &mut crate::body_clusters::BodyFlightState,
    ground: &crate::body_clusters::BodyGroundState,
    blink: &crate::body_clusters::BodyBlinkState,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    abilities: &crate::body_clusters::BodyAbilities,
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
        &mut flight.carried_run,
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
    /// Blink hang-time is active this frame (`BodyBlinkState::grace_timer > 0`).
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
    carried_run: &mut f32,
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
        // Run/friction act along the PHYSICAL run axis (`side`, perpendicular to
        // gravity). The input-frame mode chooses how the stick projects onto it:
        // `stick(...).x` is the run component (Hybrid: just `axis_x`; Screen: the
        // screen stick's run-along-the-ground component). So `+run` walks the body
        // toward THEIR right at any gravity orientation, screen-relative or not.
        let m = crate::AccelerationFrame::new(g).side;
        let run = tuning.stick(&input).x;
        let along = kin_vel.dot(m);
        // CARRIED MOMENTUM: the world has no air drag, but the CONTROLLER has
        // a tight stop assist. `carried_run` is the run-axis velocity the
        // WORLD imparted (a portal fling, knockback) — the floor the
        // hands-off stop assist decays toward instead of zero. Ordinary jump
        // drift (carried = 0) stops on release exactly as before; imparted
        // momentum is conserved until input, a wall, or landing consumes it.
        // Airborne input steers as an equilibrium (accelerates toward the
        // held direction up to the run cap, never brakes speed already
        // beyond it in that direction — the fall cap's `relax`); OPPOSING
        // input brakes at full air control and eats the carried floor with
        // it. `carried_decay` optionally bleeds the floor over time.
        *carried_run = approach(*carried_run, 0.0, tuning.carried_decay * dt);
        let new_along = if ctx.on_ground {
            let mut v = approach(along, run * tuning.max_run_speed, accel * dt);
            if run.abs() <= 0.1 {
                v = approach(v, 0.0, tuning.ground_friction * dt);
            }
            v
        } else if run > 0.1 {
            approach(along, (run * tuning.max_run_speed).max(along), accel * dt)
        } else if run < -0.1 {
            approach(along, (run * tuning.max_run_speed).min(along), accel * dt)
        } else {
            // Hands-off: tight stop assist down to the carried floor.
            approach(along, *carried_run, tuning.air_stop_assist * dt)
        };
        *kin_vel += (new_along - along) * m;
        // The floor never exceeds the actual velocity: opposing input, wall
        // impacts (the sweep zeroes the run component), and grounded friction
        // all shrink it naturally through this clamp.
        *carried_run = carried_run.clamp(new_along.min(0.0), new_along.max(0.0));
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
    kinematics: &mut crate::body_clusters::BodyKinematics,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    flight: &mut crate::body_clusters::BodyFlightState,
    wall: &mut crate::body_clusters::BodyWallState,
    jump: &mut crate::body_clusters::BodyJumpState,
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
    // Resolve raw input into the controlled body's local frame, then project that
    // local intent through the body frame onto the climbable's authored world
    // axes. Today's climbable regions are vertical world-space spans with a
    // small horizontal strafe lane; when climbables grow an explicit authored
    // axis, this projection is the seam that should consume it.
    let local_stick = tuning.stick(&input);
    let body_frame = crate::reference_frame::AccelerationFrame::new(tuning.gravity_dir);
    let world_stick = body_frame.to_world(local_stick);
    let pressing_away_from_gravity = local_stick.y < -0.1;
    let mut target_vel = Vec2::new(
        world_stick.x * spec.climb_speed * spec.strafe_factor,
        world_stick.y * spec.climb_speed,
    );
    if jump.ladder_jump_boost > 0.0 && pressing_away_from_gravity {
        let away_from_feet = -tuning.jump_speed;
        let along_down = target_vel.dot(body_frame.down);
        target_vel += body_frame.down * (away_from_feet - along_down);
    }
    kinematics.vel = target_vel;
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
    kinematics: &mut crate::body_clusters::BodyKinematics,
    flight: &mut crate::body_clusters::BodyFlightState,
    wall: &mut crate::body_clusters::BodyWallState,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
) {
    flight.fast_falling = false;
    wall.wall_clinging = false;
    wall.wall_climbing = false;
    flight.flight_phase += dt * tuning.flight_hover_hz * std::f32::consts::TAU;

    // Free flight consumes controlled-body-local input. Resolve raw input before
    // it reaches `InputState`; this layer only projects local side/down motion
    // into world space.
    let frame = crate::reference_frame::AccelerationFrame::new(tuning.gravity_dir);
    let vel_run = kinematics.vel.dot(frame.side);
    let vel_descend = kinematics.vel.dot(frame.down);
    let local_stick = tuning.stick(&input);

    let target_run = local_stick.x * tuning.flight_terminal_speed;
    let mut target_descend = local_stick.y * tuning.flight_terminal_speed;
    if !tuning.flight_direct_velocity && local_stick.y.abs() <= 0.10 {
        target_descend = flight.flight_phase.sin() * tuning.flight_hover_speed;
    }

    let (mut new_run, mut new_descend) = if tuning.flight_direct_velocity {
        // Direct-velocity free-mover: the controller commanded an exact velocity
        // (`stick × terminal` == its `velocity_target`), so take it verbatim — no
        // accel ramp, drag, hover-bob, or deadzone. Byte-identical to a SNAP float
        // (`step_floating_body`, `accel: None`) so a boss flies through the ONE
        // pipeline without a motion change. The clamp below is a harmless no-op
        // (`|stick| ≤ 1` already bounds this to ±terminal).
        (target_run, target_descend)
    } else {
        let mut new_run = approach(vel_run, target_run, tuning.flight_accel * dt);
        let mut new_descend = approach(vel_descend, target_descend, tuning.flight_accel * dt);

        if local_stick.x.abs() <= 0.10 {
            new_run = approach(new_run, 0.0, tuning.flight_drag * dt);
        }
        if local_stick.y.abs() <= 0.10 {
            new_descend = approach(new_descend, target_descend, tuning.flight_drag * dt);
        }
        (new_run, new_descend)
    };

    new_run = new_run.clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);
    new_descend = new_descend.clamp(-tuning.flight_terminal_speed, tuning.flight_terminal_speed);

    kinematics.vel = frame.to_world(crate::Vec2::new(new_run, new_descend));
}

/// Wall ability ride: while local side input presses into a wall, engage
/// wall-cling (clamp descent along the controlled body's down axis) or, with
/// `wall_climb` + local up/down input, drive motion along that down axis.
/// Records the first transition op so the trace recorder fires
/// `WallCling` / `WallClimb` exactly once per engagement.
///
pub(super) fn apply_wall_abilities_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    abilities: &crate::body_clusters::BodyAbilities,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    tuning: MovementTuning,
    was_clinging: bool,
    events: &mut FrameEvents,
) {
    if !wall.on_wall || ground.on_ground || !abilities.abilities.wall_cling {
        return;
    }
    let frame = crate::reference_frame::AccelerationFrame::new(tuning.gravity_dir);
    let local_stick = tuning.stick(&input);
    let pressing_into_wall =
        local_stick.x.abs() > 0.1 && local_stick.x.signum() == -wall.wall_normal_x;
    if !pressing_into_wall {
        return;
    }
    wall.wall_clinging = true;
    if abilities.abilities.wall_climb && local_stick.y.abs() > 0.25 {
        wall.wall_climbing = true;
        let along_down = kinematics.vel.dot(frame.down);
        kinematics.vel += frame.down * (local_stick.y * tuning.wall_climb_speed - along_down);
        if !was_clinging {
            events.op_clusters(combo_trace, MovementOp::WallClimb);
        }
    } else {
        let descend = kinematics.vel.dot(frame.down);
        if descend > tuning.wall_slide_speed {
            kinematics.vel -= frame.down * (descend - tuning.wall_slide_speed);
        }
        if !was_clinging {
            events.op_clusters(combo_trace, MovementOp::WallCling);
        }
    }
}
