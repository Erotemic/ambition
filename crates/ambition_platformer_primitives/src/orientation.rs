//! Actor orientation under gravity (the "which way is down" upright reflex).
//!
//! The shared body-orientation component ([`ActorRoll`]) and its righting system
//! ([`update_actor_roll`]). Any body that can be reoriented (by a portal flip, a
//! gravity zone, a knockback) eases back toward "feet along gravity" here — and
//! a body RIDING a surface plants its feet on that surface instead, via the
//! per-tick [`SurfaceUpright`] fact its integration publishes. The component and
//! systems are gravity-driven and actor-generic — they operate on the unified
//! [`crate::body::BodyKinematics`] body and the in-crate
//! [`crate::gravity::GravityCtx`], with no sandbox / content dependency.

use bevy::prelude::*;

use crate::body::BodyKinematics;
use crate::gravity::{gravity_upright_angle, upright_angle_for_world_up, GravityCtx};
use crate::projectile::ProjectileGameplay;
use crate::time::SimDt;

/// Shared "which way is down" body orientation, in render-space radians, applied
/// to the body's sprite. The SAME component, righting system
/// ([`update_actor_roll`]), and external transit math drive the player and every
/// actor, so a goblin or shark somersaults under a gravity flip exactly like the
/// player (the unification). External mechanics ADD the rotation a body's
/// velocity underwent; [`update_actor_roll`] then eases the roll back toward
/// "feet along gravity" so the body rights itself toward the current gravity
/// field (in a gravity room it settles to that room's down).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ActorRoll {
    /// Current render-space z-rotation applied to the body's sprite.
    pub angle: f32,
}

/// Per-tick surface-ride fact: while a momentum body RIDES a surface this
/// carries the surface's outward normal — the body's "visual up" — so the
/// righting reflex tilts its feet onto the ground it stands on (a Sonic running
/// up a ramp leans with the ramp). Airborne, and on every body that never
/// rides, it stays `None` and the gravity-upright reflex applies. Published by
/// the body integration phase (the ONLY writer) and rebuilt every tick, like
/// `BodyMotionFacts` — a derived fact, never persistent state.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SurfaceUpright {
    /// World-space outward normal of the ridden surface, if riding.
    pub up: Option<Vec2>,
}

/// Reorientation rate easing `angle` toward gravity-upright (rad/s). Visible but
/// quick — a 180° flip rights itself in ~0.4s as the body arcs.
const ACTOR_ROLL_SPEED: f32 = 8.0;

/// Surface-tracking rate while riding (rad/s). The ground angle IS the pose, so
/// tracking must outrun the tightest authored curvature at top speed — a full
/// loop at ~1200 px/s sweeps ~9 rad/s — or the sprite visibly lags the ground
/// it stands on. Still finite, so a landing on a steep slope reads as a quick
/// settle rather than a pop.
const SURFACE_ROLL_TRACK_SPEED: f32 = 30.0;

/// Attach an [`ActorRoll`] (plus its [`SurfaceUpright`] ride fact) lazily to
/// each non-projectile body that can be reoriented. Projectiles carry
/// [`BodyKinematics`] too, but must not somersault upright mid-flight, so
/// [`ProjectileGameplay`] filters them out.
pub fn ensure_actor_roll(
    mut commands: Commands,
    bodies: Query<
        Entity,
        (
            With<BodyKinematics>,
            Without<ActorRoll>,
            Without<ProjectileGameplay>,
        ),
    >,
) {
    for entity in &bodies {
        commands
            .entity(entity)
            .insert((ActorRoll::default(), SurfaceUpright::default()));
    }
}

/// Ease each actor's roll toward its upright target: feet onto the ridden
/// surface while the body's [`SurfaceUpright`] fact carries one, feet along
/// gravity otherwise. Runs airborne and grounded so a rotated body visibly
/// rights itself toward its local gravity field.
pub fn update_actor_roll(
    time: Res<SimDt>,
    gravity: GravityCtx,
    mut rolls: Query<(&mut ActorRoll, &BodyKinematics, Option<&SurfaceUpright>)>,
) {
    let dt = time.get();
    if dt <= 0.0 {
        return;
    }
    for (mut roll, kin, surface) in &mut rolls {
        // A riding body plants its feet on the ridden surface (fast tracking);
        // everything else rights toward the gravity of the column IT is
        // standing in (localized): resolve from its own position.
        let (target, rate) = match surface.and_then(|s| s.up) {
            Some(up) => (upright_angle_for_world_up(up), SURFACE_ROLL_TRACK_SPEED),
            None => (
                gravity_upright_angle(gravity.dir_at(kin.pos)),
                ACTOR_ROLL_SPEED,
            ),
        };
        let max_step = rate * dt;
        // Shortest signed difference, wrapped to (-π, π], so righting always
        // takes the short way around.
        let mut diff = (target - roll.angle).rem_euclid(std::f32::consts::TAU);
        if diff > std::f32::consts::PI {
            diff -= std::f32::consts::TAU;
        }
        if diff.abs() <= max_step {
            roll.angle = target;
        } else {
            roll.angle += max_step * diff.signum();
        }
        // Keep the stored angle bounded so repeated flips don't grow it.
        roll.angle = roll.angle.rem_euclid(std::f32::consts::TAU);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roll_app() -> App {
        let mut app = App::new();
        app.insert_resource(SimDt { dt: 1.0 / 60.0 });
        app.add_systems(Update, update_actor_roll);
        app
    }

    #[test]
    fn a_riding_body_tilts_its_feet_onto_the_surface() {
        let mut app = roll_app();
        // A 45° ascending-to-the-right slope (y-down world): outward normal
        // tilts up-left.
        let up = Vec2::new(-1.0, -1.0).normalize();
        let body = app
            .world_mut()
            .spawn((
                ActorRoll::default(),
                BodyKinematics::default(),
                SurfaceUpright { up: Some(up) },
            ))
            .id();
        for _ in 0..60 {
            app.update();
        }
        let angle = app.world().get::<ActorRoll>(body).unwrap().angle;
        let expected = upright_angle_for_world_up(up);
        let diff = (angle - expected).rem_euclid(std::f32::consts::TAU);
        let diff = diff.min(std::f32::consts::TAU - diff);
        assert!(
            diff < 1e-3,
            "roll tracked the ridden surface: {angle} vs {expected}"
        );
    }

    #[test]
    fn a_cleared_ride_fact_rights_the_body_back_to_gravity() {
        let mut app = roll_app();
        let body = app
            .world_mut()
            .spawn((
                ActorRoll {
                    angle: std::f32::consts::FRAC_PI_4,
                },
                BodyKinematics::default(),
                SurfaceUpright::default(),
            ))
            .id();
        for _ in 0..60 {
            app.update();
        }
        let angle = app.world().get::<ActorRoll>(body).unwrap().angle;
        let diff = angle.rem_euclid(std::f32::consts::TAU);
        let diff = diff.min(std::f32::consts::TAU - diff);
        assert!(diff < 1e-3, "no ride fact → gravity-upright: {angle}");
    }
}
