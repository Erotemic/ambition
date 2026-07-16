//! One trusted, frame-aware movement kernel with swappable physics policies.
//!
//! [`step_motion`] is the ONLY movement entry. Every movable body carries one
//! explicit [`MotionModel`], and every policy receives the same immutable
//! [`MotionFrame`] resolved once by the environment from a reference basis and
//! the complete world-space acceleration for that body tick. The active frame
//! is environmental state: it is neither authored into model parameters nor
//! cached in model-private runtime state, and every directional quantity
//! crossing this boundary carries its frame in its type (see
//! [`InputState`]).
//!
//! Axis-swept action-platformer movement, [`surface momentum`](SurfaceMotion),
//! and the [`adhesive crawler`](CrawlerState) are sibling implementations. They
//! own different private state and contact logic, but they share one body-state
//! authority, one typed local-input contract, one world context, one frame, and
//! one deterministic dispatch seam. The phase-level axis functions below are
//! kernel-private implementation vocabulary — production integration calls
//! [`step_motion`], never an individual solver arm.

use crate::world::World;
use crate::MotionFrame;

mod abilities;
mod adhesive_crawler;
mod authority;
mod blink;
pub(crate) mod collision;
mod control;
mod events;
mod facts;
mod input;
mod integration;
mod kernel;
mod model;
mod ops;
mod player;
mod simulation;
pub(crate) mod surface_momentum;
mod tuning;

pub use adhesive_crawler::{AdhesiveCrawlerMotion, CrawlAttachment, CrawlerParams, CrawlerState};
pub use surface_momentum::{
    DepthOcclusions, MomentumParams, OcclusionSpan, RouteDeparture, SurfaceMotion, SurfaceRef,
};

pub use abilities::resolve_shield;
pub use blink::{blink_destination_clusters, blink_destination_to_point_clusters};
// The ONE hazard-touch rule, exported so external observers apply the SAME
// predicate the kernel applies — never a duplicated near-copy.
pub use authority::{
    carry_body, constrain_body_pose, reconcile_transit, transit_body, TransitVelocity,
};
pub use collision::{touching_hazard_aabb, touching_rebound_aabb};
pub use events::{BlinkEvent, FrameEvents};
pub use facts::{BodyMotionFacts, LedgeFacts};
pub use input::InputState;
/// Screen-vertical input → gravity-relative "descend" intent (the vertical
/// sibling of the run-axis transform). Every crouch/pogo/drop-through/fast-fall
/// gate and gravity-relative vertical movement reads input through this so a
/// gravity flip moves them all together. See its doc for the convention.
pub use integration::gravity_descend;
/// The canonical "launch at `speed` opposite `gravity_dir`" velocity primitive
/// shared by jump, wall-kick, and pogo so a gravity flip moves them all. Any
/// pogo/jump impulse outside the engine (e.g. the sandbox attack path) MUST go
/// through this instead of a hardcoded `vel.y = -speed`.
pub use integration::set_jump_velocity;
/// The actor-generic normal-mode physics SPINE: gravity-relative gravity, run,
/// fast-fall/glide gates, and the fall cap. The player feeds it its rich ability
/// clusters; enemies/NPCs feed it [`NormalSpineCtx::bare`] + per-actor tuning, so
/// every actor falls + runs through the SAME core (the non-player-centric seam).
pub use integration::{integrate_normal_spine, NormalSpineCtx};
pub use kernel::{step_motion, MotionStepContext, MotionStepResult, SupportFact};
pub use model::{
    knock_off_ledge, switch_motion_model, AxisManeuverState, AxisSweptMotion, MotionModel,
    MotionModelKind, MotionModelSpec, SurfaceMomentumMotion,
};
pub use ops::{ComboMark, MovementOp};
pub use player::{default_player_body_size, DEFAULT_PLAYER_BODY_HEIGHT, DEFAULT_PLAYER_BODY_WIDTH};
pub use tuning::{
    AxisLocomotion, AxisSweptParams, FlightTuning, LedgeMomentumTuning, MovementTuning,
    TraversalAbilityTuning, AIR_ACCEL, AIR_FRICTION, AIR_JUMPS, BLINK_COOLDOWN, BLINK_DISTANCE,
    BLINK_GRACE_TIME, BLINK_HOLD_THRESHOLD, BLINK_MAX_DOWNWARD_SPEED, COYOTE_TIME, DASH_BUFFER,
    DASH_COOLDOWN, DASH_SPEED, DASH_TIME, DEFAULT_AXIS_SWEPT_PARAMS, DEFAULT_GRAVITY_DIR,
    DEFAULT_TUNING, DODGE_ROLL_COOLDOWN, DODGE_ROLL_SPEED, DODGE_ROLL_TIME, DOUBLE_JUMP_SPEED,
    FAST_FALL_ACCEL, FAST_FALL_SPEED, FLIGHT_ACCEL, FLIGHT_DRAG, FLIGHT_HOVER_HZ,
    FLIGHT_HOVER_SPEED, FLIGHT_TERMINAL_SPEED, GLIDE_AIR_ACCEL, GLIDE_FALL_SPEED, GRAVITY,
    GROUND_FRICTION, JUMP_BUFFER, JUMP_SPEED, MAX_FALL_SPEED, MAX_RUN_SPEED,
    ONE_WAY_DROP_THROUGH_GRACE, PARRY_WINDOW_TIME, POGO_SPEED, PRECISION_BLINK_AIM_SPEED,
    PRECISION_BLINK_DISTANCE, PRECISION_BLINK_MAX_DOWNWARD_SPEED, RUN_ACCEL, SLASH_RECOIL,
    WALL_CLIMB_SPEED, WALL_JUMP_X, WALL_SLIDE_SPEED,
};

#[cfg(test)]
use collision::body_is_side_contact;

/// Frame-explicit axis-swept control phase — kernel-private. The current
/// acceleration frame is supplied by the environment, never read from or
/// written into model parameters. `state` is the axis policy's model-private
/// maneuver state ([`AxisManeuverState`]), threaded from the active
/// [`AxisSweptMotion`] variant.
pub(crate) fn update_body_control_in_frame(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    control_dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) -> FrameEvents {
    let mut events = FrameEvents::default();

    // Reset on edge press: the body only FLAGS the request; the body's owner
    // applies its reset policy (respawn for the home body, damage/ignore for
    // an actor).
    if input.reset_pressed && clusters.abilities.abilities.reset {
        events.reset = true;
        return events;
    }

    abilities::apply_intent(
        clusters.kinematics,
        clusters.ground,
        clusters.flight,
        state,
        clusters.abilities,
        input,
        tuning,
    );

    abilities::apply_fly_toggle(
        clusters.flight,
        state,
        clusters.abilities,
        clusters.combo_trace,
        input,
        &mut events,
    );

    // Blink hold / aim / release + melee + pogo dispatch.
    control::handle_blink_clusters(
        world,
        clusters.kinematics,
        clusters.abilities,
        clusters.blink,
        state,
        clusters.combo_trace,
        input,
        control_dt,
        frame,
        tuning,
        &mut events,
    );
    control::handle_attacks_clusters(
        clusters.kinematics,
        clusters.abilities,
        clusters.combo_trace,
        input,
        frame,
        tuning,
        &mut events,
    );

    abilities::apply_dodge(
        clusters.kinematics,
        clusters.dodge,
        state,
        clusters.ground,
        clusters.abilities,
        clusters.combo_trace,
        input,
        frame,
        tuning,
        &mut events,
    );

    abilities::apply_dash(
        clusters.kinematics,
        clusters.dash,
        state,
        clusters.abilities,
        clusters.combo_trace,
        input,
        frame,
        tuning,
        &mut events,
    );

    abilities::apply_shield(
        clusters.shield,
        state,
        clusters.abilities,
        clusters.combo_trace,
        input,
        tuning,
        &mut events,
    );

    abilities::apply_jump_release(clusters.kinematics, clusters.abilities, input, frame);

    events
}

/// Frame-explicit axis-swept simulation phase — kernel-private. The same
/// immutable frame reaches every gravity-relative limb.
pub(crate) fn update_body_simulation_in_frame(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    raw_dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
) -> FrameEvents {
    // §3.1 SweepSample: both endpoints are captured INSIDE the kernel —
    // `prev` at sim-phase entry, `curr` at exit — so any position change
    // outside this window (blink in the control phase, respawn policy after
    // this returns, portal/room/scripted teleports in other systems) is
    // excluded from the motion record BY CONSTRUCTION. Early returns still
    // pass through the write below (a zero-dt tick records a zero-length
    // segment, never a stale one).
    let entry_pos = clusters.kinematics.pos;
    let entry_vel = clusters.kinematics.vel;
    let events = update_body_simulation_inner(world, clusters, state, input, raw_dt, frame, tuning);
    if let Some(sweep) = clusters.sweep.as_deref_mut() {
        *sweep = crate::body_clusters::SweepSample {
            prev: entry_pos,
            curr: clusters.kinematics.pos,
            vel: entry_vel,
            half: clusters.kinematics.size * 0.5,
        };
    }
    events
}

fn update_body_simulation_inner(
    world: &World,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    state: &mut AxisManeuverState,
    input: InputState,
    raw_dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
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
        state.ledge_grab = None;
    }

    // Drowning gate — body flags hazard + reset; the owner applies its policy.
    if clusters.env_contact.water.is_some() && !clusters.abilities.abilities.swim {
        events.hazard = true;
        events.reset = true;
        return events;
    }

    // Age lifetime + timers + combo trace — cluster + maneuver-state inline.
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
        state.buffer_jump = dec(state.buffer_jump);
        state.buffer_dash = dec(state.buffer_dash);
        state.coyote_timer = dec(state.coyote_timer);
        state.drop_through_timer = dec(state.drop_through_timer);
        clusters.jump.ladder_jump_boost = dec(clusters.jump.ladder_jump_boost);
        clusters.jump.ladder_drop_through_timer = dec(clusters.jump.ladder_drop_through_timer);
        clusters.dash.cooldown = dec(clusters.dash.cooldown);
        clusters.blink.cooldown = dec(clusters.blink.cooldown);
        state.blink_grace_timer = dec(state.blink_grace_timer);
        state.rebound_cooldown = dec(state.rebound_cooldown);
        state.dodge_roll_timer = dec(state.dodge_roll_timer);
        clusters.dodge.cooldown = dec(clusters.dodge.cooldown);
        clusters.shield.parry_window_timer = dec(clusters.shield.parry_window_timer);
        clusters.ledge.release_cooldown = dec(clusters.ledge.release_cooldown);
        if state.wall_clinging || clusters.ground.on_ground {
            state.pre_wall_vel_age += dt;
        }
        if clusters.ground.on_ground {
            state.coyote_timer = tuning.locomotion.coyote_time;
            crate::body_clusters::refresh_movement_resources_clusters(
                clusters.abilities,
                clusters.dash,
                clusters.jump,
                tuning.locomotion.air_jumps,
            );
        }
    }

    // Active ledge-grab tick. Returns true if it consumed the frame
    // (the rest of the simulation phase short-circuits).
    if crate::ledge_grab::tick_active_ledge_grab_clusters_in_frame(
        clusters,
        state,
        input,
        dt,
        frame,
        tuning,
        &mut events,
    ) {
        return events;
    }

    // Consume the buffered jump (or convert to swim stroke /
    // drop-through / wall-jump / double-jump).
    simulation::handle_jump_buffer_clusters(
        world,
        state,
        clusters.env_contact,
        clusters.abilities,
        clusters.body_mode.body_mode,
        clusters.flight.fly_enabled,
        clusters.kinematics,
        clusters.ground,
        clusters.wall,
        clusters.jump,
        clusters.combo_trace,
        input,
        frame,
        tuning,
        &mut events,
    );

    integration::integrate_velocity_clusters(
        world,
        clusters,
        state,
        input,
        dt,
        frame,
        tuning,
        &mut events,
    );

    // Probe for a fresh ledge grab now that the integration step
    // settled the new position. Required for the auto-snap-on-fall
    // recovery path (slow drifts ignore this; fast falls latch).
    crate::ledge_grab::try_start_ledge_grab_clusters_in_frame(
        world,
        clusters,
        state,
        input,
        frame,
        &mut events,
    );

    // Hazard / out-of-bounds gate — body flags hazard + reset; the owner
    // applies its policy. "Fell out of the world" is gravity-relative:
    // distance past the world AABB measured ALONG the fall direction (fable
    // review 2026-07-02 §B7 — the old `pos.y > size.y + 200` only caught the
    // bottom edge, so under up/sideways gravity a body could fall forever).
    let pos = clusters.kinematics.pos;
    let clamped = crate::Vec2::new(
        pos.x.clamp(0.0, world.size.x),
        pos.y.clamp(0.0, world.size.y),
    );
    let fell_out = (pos - clamped).dot(frame.down()) > 200.0;
    if collision::touching_hazard_aabb(world, clusters.kinematics.aabb()) || fell_out {
        events.hazard = true;
        events.reset = true;
    }

    events
}

fn dec(value: f32, dt: f32) -> f32 {
    (value - dt).max(0.0)
}

/// Axis-swept implementation arm behind [`step_motion`] — kernel-private. All
/// frame-sensitive control and integration receives the exact same per-tick
/// frame value, and both phases share the SAME model-private maneuver state
/// borrowed from the active [`AxisSweptMotion`] variant.
/// `InputState::control_dt` overrides `raw_dt` for the control phase when
/// positive (so bullet-time slowing gravity does not slow input).
pub(crate) fn update_body_with_frame_clusters(
    world: &World,
    axis: &mut AxisSweptMotion,
    clusters: &mut crate::body_clusters::BodyClustersMut<'_>,
    input: InputState,
    frame: MotionFrame,
    raw_dt: f32,
) -> FrameEvents {
    let tuning = axis.params;
    let state = &mut axis.state;
    let control_dt = if input.control_dt > 0.0 {
        input.control_dt
    } else {
        raw_dt
    };
    let mut events =
        update_body_control_in_frame(world, clusters, state, input, control_dt, frame, tuning);
    let sim_events =
        update_body_simulation_in_frame(world, clusters, state, input, raw_dt, frame, tuning);
    events.extend(sim_events);
    events
}

#[cfg(test)]
mod tests;
