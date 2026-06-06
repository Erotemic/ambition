//! Actor orientation under gravity (the "which way is down" upright reflex).
//!
//! This is the proto-runtime home for the shared body-orientation component and
//! its righting system. Any body that can be reoriented (by a portal flip, a
//! gravity zone, a knockback) eases back toward "feet along gravity" here. The
//! component and systems are gravity-driven and actor-generic — no portal
//! dependency — so non-portal mechanics can rely on them.

use bevy::prelude::*;

use crate::physics::gravity_upright_angle;
use crate::player::{PlayerEntity, PlayerKinematics, PrimaryPlayer};

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

/// Attach an [`ActorRoll`] to every body that can be reoriented — the player
/// plus all non-player actors (enemies / NPCs / bosses, all via the unified
/// `BodyKinematics`) — lazily, so no bundle needs to know about this module.
pub fn ensure_actor_roll(
    mut commands: Commands,
    player: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>, Without<ActorRoll>)>,
    bodies: Query<
        Entity,
        (
            With<crate::features::BodyKinematics>,
            Without<PlayerEntity>,
            Without<ActorRoll>,
        ),
    >,
) {
    for entity in &player {
        commands.entity(entity).insert(ActorRoll::default());
    }
    for entity in &bodies {
        commands.entity(entity).insert(ActorRoll::default());
    }
}

/// Continuously ease EVERY actor's roll toward "feet along gravity" (the
/// orient-to-gravity reflex) — player and non-player alike. Runs whether
/// airborne or grounded, so after something rotates a body it visibly rights
/// itself toward the current gravity field; in a gravity room it settles to that
/// room's down.
pub fn update_actor_roll(
    time: Res<crate::WorldTime>,
    gravity: crate::physics::GravityCtx,
    mut rolls: Query<(
        &mut ActorRoll,
        Option<&PlayerKinematics>,
        Option<&crate::features::BodyKinematics>,
    )>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    let max_step = ACTOR_ROLL_SPEED * dt;
    for (mut roll, pkin, bkin) in &mut rolls {
        // Each body rights toward the gravity of the column IT is standing in
        // (localized): resolve from its own position, falling back to the
        // player's field when position is unavailable. Player carries
        // `PlayerKinematics`; enemies / NPCs / bosses carry the unified
        // `BodyKinematics`.
        let pos = pkin.map(|k| k.pos).or_else(|| bkin.map(|k| k.pos));
        let gravity_dir = match pos {
            Some(p) => gravity.dir_at(p),
            None => gravity.field_dir(),
        };
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
