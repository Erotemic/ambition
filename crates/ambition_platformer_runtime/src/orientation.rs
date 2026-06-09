//! Actor orientation under gravity (the "which way is down" upright reflex).
//!
//! The shared body-orientation component ([`ActorRoll`]) and its righting system
//! ([`update_actor_roll`]). Any body that can be reoriented (by a portal flip, a
//! gravity zone, a knockback) eases back toward "feet along gravity" here. The
//! component and systems are gravity-driven and actor-generic — they operate on
//! the unified [`crate::body::BodyKinematics`] body and the in-crate
//! [`crate::gravity::GravityCtx`], with no sandbox / content dependency.

use bevy::prelude::*;

use crate::body::BodyKinematics;
use crate::gravity::{gravity_upright_angle, GravityCtx};
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

/// Reorientation rate easing `angle` toward gravity-upright (rad/s). Visible but
/// quick — a 180° flip rights itself in ~0.4s as the body arcs.
const ACTOR_ROLL_SPEED: f32 = 8.0;

/// Attach an [`ActorRoll`] to every body that can be reoriented — every entity
/// carrying the unified [`BodyKinematics`] (the player plus all non-player
/// actors: enemies / NPCs / bosses) — lazily, so no bundle needs to know about
/// this module. With the kinematics unification all those archetypes share one
/// component, so this is a single `With<BodyKinematics>` query (no player/actor
/// branch).
///
/// Projectiles are EXCLUDED (`Without<ProjectileGameplay>`, Stage 19 Phase
/// 3c-i): once player projectiles become entities carrying the shared
/// [`BodyKinematics`] (Phase 3c-ii) they would otherwise be handed an
/// [`ActorRoll`] and auto-righted to gravity like an actor. A projectile is
/// not an actor — it must not somersault upright mid-flight — so the
/// projectile marker filters it out here (and transitively out of
/// [`update_actor_roll`], which only iterates `ActorRoll` carriers).
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
        commands.entity(entity).insert(ActorRoll::default());
    }
}

/// Continuously ease EVERY actor's roll toward "feet along gravity" (the
/// orient-to-gravity reflex) — player and non-player alike. Runs whether
/// airborne or grounded, so after something rotates a body it visibly rights
/// itself toward the current gravity field; in a gravity room it settles to that
/// room's down. With the kinematics unification every `ActorRoll` carrier holds
/// the one [`BodyKinematics`], so this is a single query (the dual-arm
/// player/actor branch collapsed in Stage 16 / S5).
pub fn update_actor_roll(
    time: Res<SimDt>,
    gravity: GravityCtx,
    mut rolls: Query<(&mut ActorRoll, &BodyKinematics)>,
) {
    let dt = time.get();
    if dt <= 0.0 {
        return;
    }
    let max_step = ACTOR_ROLL_SPEED * dt;
    for (mut roll, kin) in &mut rolls {
        // Each body rights toward the gravity of the column IT is standing in
        // (localized): resolve from its own position.
        let gravity_dir = gravity.dir_at(kin.pos);
        let target = gravity_upright_angle(gravity_dir);
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
