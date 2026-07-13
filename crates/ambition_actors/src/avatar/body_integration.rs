//! Home/player body movement, decomposed so it joins the SAME scheduled body
//! integration phase as actors.
//!
//! The home body is NOT a separate gameplay species: [`integrate_home_body`] is
//! the per-body movement core the unified `integrate_sim_bodies` phase calls for
//! every `PlayerEntity`, right beside the actor bodies it integrates in the same
//! system. It runs the LITERAL same engine entry an actor uses
//! (`ae::step_motion`) over the body's `BodyClustersMut`
//! view. The ONLY home-specific work here is:
//!
//! - the pre-sim ledge-platform carry ([`ledge_platform_carry`]) — only the home
//!   body ledge-grabs;
//! - the two-clock precision-blink affordance carried by `InputState::control_dt`
//!   (an INPUT affordance, not a simulation structure);
//! - flagging a body reset ([`PlayerBodyFrameOutput::reset`]) for the separate
//!   home reset POLICY and PRESENTATION phases to consume.
//!
//! It performs NO sandbox reset, NO room reset, and NO presentation — those are
//! home-policy / home-view phases that read the [`PlayerBodyFrameOutput`] hand-off
//! this phase writes.

use bevy::prelude::*;

use ambition_engine_core as ae;

use crate::features::ecs::attack::engine_input_from_actor_control;
use crate::features::FeatureEcsWorldOverlay;
use crate::time::feel::SandboxFeelTuning;
use crate::world::platforms::MovingPlatformState;
use ambition_characters::actor::BodyCombat;
use ambition_world::collision::world_with_sandbox_solids;

/// Movement→(reset/presentation) hand-off for a home/player body, written by the
/// unified body integration phase (`integrate_sim_bodies` → [`integrate_home_body`])
/// and read by the two home-policy phases: the home reset POLICY (sandbox reset on
/// `reset`) and the home PRESENTATION phase (screen shake / landing SFX / per-op
/// anim/SFX/VFX). Body-generic in SHAPE — it carries only integration facts (this
/// frame's `FrameEvents` + the landing inputs + a reset flag), never any player
/// presentation state — so movement stays a pure integrate-and-report phase.
/// A required component of every player body.
#[derive(Component, Default)]
pub struct PlayerBodyFrameOutput {
    /// The movement tick's events (jump/dash/blink ops, blink endpoints, …).
    pub events: ae::FrameEvents,
    /// Grounded state ENTERING the movement tick (for the hard-fall shake edge).
    pub was_grounded: bool,
    /// Fall speed entering the tick — the velocity component ALONG gravity
    /// (hard-fall shake magnitude; frame-agnostic, fable review 2026-07-02 §B
    /// minor: the raw `vel.y` form misfired under sideways gravity).
    pub pre_sim_fall_speed: f32,
    /// The integration flagged a body reset this frame (drown / hazard /
    /// out-of-bounds / death). The body was already teleported to spawn by this
    /// phase; the home reset POLICY consumes this to run the full sandbox reset for
    /// the primary, and the PRESENTATION phase skips the frame.
    pub reset: bool,
}

/// How a ledge-grabbing player should react to the moving platform that carries
/// them this frame: ride along with it, or be knocked off because the carry
/// would shove them into a wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgePlatformCarry {
    Carry,
    KnockOff,
}

/// Decide [`LedgePlatformCarry`] for a ledge-grabbing player about to be carried
/// by `delta`. `world` is the base collision world, which **excludes** the
/// moving platform (it's composited in separately), so a solid overlap here is a
/// genuine *other* wall — meaning the platform would push the player through it
/// (#126 "ledge grab on a moving platform into a wall pushes you through").
/// Pure, so the wall decision is unit-testable without the full phase context.
pub fn ledge_platform_carry(
    world: &ae::World,
    player_aabb: ae::Aabb,
    delta: ae::Vec2,
) -> LedgePlatformCarry {
    use ambition_engine_core::AabbExt;
    let carried = player_aabb.translated(delta);
    let into_wall = world
        .blocks
        .iter()
        .any(|b| matches!(b.kind, ae::BlockKind::Solid) && carried.strict_intersects(b.aabb));
    if into_wall {
        LedgePlatformCarry::KnockOff
    } else {
        LedgePlatformCarry::Carry
    }
}

/// The per-body home movement core — control phase **and** simulation phase in ONE
/// combined kernel call, `ae::step_motion`: the literal same
/// engine entry a brain-driven actor uses (`ActorMut::integrate_body`). Called by
/// the unified `integrate_sim_bodies` phase for every `PlayerEntity`, so the home
/// body and every actor integrate through one function inside one scheduled system.
///
/// THE TWO-CLOCK SPLIT IS AN INPUT AFFORDANCE, NOT A SIMULATION STRUCTURE.
/// Precision-blink bullet-time keeps the player's aim responsive while the world
/// slows. It is carried entirely by `InputState::control_dt`: the human sets
/// `control_dt = real frame_dt` (so the engine runs the control phase at real time
/// and the simulation phase at `sim_dt`), while a brain leaves `control_dt = 0`.
///
/// `ActorControl` is the single source of truth for input — the brain translates
/// every verb the simulation consumes. The hitstun gate applies inside
/// `engine_input_from_actor_control`.
///
/// On a flagged reset (drown / hazard / out-of-bounds / death) the body teleports
/// to spawn (engine-level body reset, the same on every body) and `frame_out.reset`
/// is set. The SANDBOX reset + ROOM reset are HOME POLICY, run by a separate phase
/// that reads this flag — this function never performs them.
#[allow(clippy::too_many_arguments)]
pub fn integrate_home_body(
    actor_control: ambition_characters::actor::control::ActorControlFrame,
    world: &ae::World,
    clusters: &mut ae::BodyClustersMut<'_>,
    combat: &BodyCombat,
    hurtbox: &mut ae::CenteredAabb,
    frame_out: &mut PlayerBodyFrameOutput,
    moving_platforms: &[MovingPlatformState],
    motion_model: &mut crate::features::MotionModel,
    motion_frame: ae::MotionFrame,
    axis_tuning: ae::MovementTuning,
    feel: SandboxFeelTuning,
    frame_dt: f32,
    scaled_dt: f32,
    feature_ecs_overlay: &FeatureEcsWorldOverlay,
) {
    let input = engine_input_from_actor_control(
        actor_control,
        feel,
        combat.hitstun_timer,
        combat.recoil_lock_timer,
        frame_dt,
    );
    let sim_dt = if combat.hitstop_timer > 0.0 {
        0.0
    } else {
        scaled_dt
    };

    // Live authored tuning refreshes only the active axis policy's parameters.
    // The environmental acceleration frame is supplied separately and therefore
    // cannot be frozen into, or reset with, movement-model configuration.
    if let crate::features::MotionModel::AxisSwept(axis) = motion_model {
        axis.params = axis_tuning.axis_swept_params();
    }

    // Ledge-platform carry is an axis-swept model-private affordance.  The
    // movement dispatch itself remains one call for every policy.
    if matches!(motion_model, crate::features::MotionModel::AxisSwept(_)) {
        let player_aabb_pre = clusters.kinematics.aabb();
        let player_size_pre = clusters.kinematics.size;
        let active_ledge_platform = clusters.ledge.grab.and_then(|grab| {
            moving_platforms.iter().position(|platform| {
                platform.matches_ledge_contact_in_frame(
                    grab.contact,
                    player_size_pre,
                    motion_frame.down(),
                )
            })
        });
        if let Some(platform_delta) =
            active_ledge_platform.map(|idx| moving_platforms[idx].last_delta())
        {
            match ledge_platform_carry(world, player_aabb_pre, platform_delta) {
                LedgePlatformCarry::KnockOff => {
                    clusters.ledge.knock_off_on_hit();
                }
                LedgePlatformCarry::Carry => {
                    // Parent-frame carry (ADR 0024 external-constraint
                    // authority): the platform moves the grabbed body.
                    ae::movement::carry_body(clusters.kinematics, platform_delta);
                    if let Some(grab) = clusters.ledge.grab.as_mut() {
                        grab.contact.anchor += platform_delta;
                        grab.contact.climb_target += platform_delta;
                    }
                }
            }
        }
    }

    let collision_world = world_with_sandbox_solids(world, moving_platforms, feature_ecs_overlay);
    let was_grounded = clusters.ground.on_ground;
    let pre_sim_fall_speed = clusters.kinematics.vel.dot(motion_frame.down());
    let result = ae::step_motion(
        motion_model,
        clusters,
        ae::MotionStepContext {
            world: &collision_world,
            input,
            frame: motion_frame,
            facing_intent: actor_control.facing,
            dt: sim_dt,
        },
    );

    // Respawn is home-body policy. The pure kernel only reports the reset event.
    if result.events.reset {
        ae::reset_body_clusters(motion_model, clusters, world.spawn);
    }

    *frame_out = PlayerBodyFrameOutput {
        was_grounded,
        pre_sim_fall_speed,
        reset: result.events.reset,
        events: result.events,
    };

    use ambition_engine_core::AabbExt;
    let body = crate::features::collision_aabb(&crate::features::SimpleActorGeometry {
        pos: clusters.kinematics.pos,
        size: clusters.kinematics.size,
        facing: clusters.kinematics.facing,
        frame_down: -result.surface_normal,
    });
    hurtbox.center = body.center();
    hurtbox.half_size = body.half_size();
}

/// Advance the world's moving platforms ONCE per frame, ahead of every body
/// integration (home + actors), so every body rides this frame's platform
/// positions. Peeled out of the per-entity body loop so it can't multiply. Uses
/// the PRIMARY player's hitstop so platforms freeze during the player's hitstop.
pub fn advance_moving_platforms(
    world_time: Res<ambition_time::WorldTime>,
    mut platforms: ResMut<ambition_world::collision::MovingPlatformSet>,
    primary_combat: Query<&BodyCombat, crate::actor::PrimaryPlayerOnly>,
) {
    let Ok(combat) = primary_combat.single() else {
        return;
    };
    let sim_dt = if combat.hitstop_timer > 0.0 {
        0.0
    } else {
        world_time.scaled_dt
    };
    for platform in platforms.0.iter_mut() {
        platform.update(sim_dt);
    }
}

#[cfg(test)]
mod home_momentum_tests;
#[cfg(test)]
mod ledge_carry_tests;
