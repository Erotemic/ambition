//! Authoritative projectile-spawn request vocabulary.
//!
//! A firing system decides a projectile's authored/body state and when its first
//! simulation step belongs. This module owns the one request channel used by
//! every projectile producer; materialization is handled by [`crate::materialize`].
//!
//! The historical split was transport-shaped rather than semantic:
//! named controlled-body fire wrote a dedicated pool message, while actor/item/boss fire
//! disguised projectile creation as a VFX effect. Both roads ended
//! by constructing the same [`crate::LiveProjectile`] entity. The request below
//! states the two facts that actually differ — presentation vocabulary and
//! first-step timing — without assigning simulation authority to a VFX enum or
//! resurrecting producer/faction storage pools.

use bevy::prelude::{Entity, Message};

use crate::{
    InFlightProjectile, ProjectileBody, ProjectileKind, ProjectileSpec, WorldHitPolicy,
};

/// When a newly materialized projectile begins advancing.
///
/// This is an explicit gameplay-timing fact, not a proxy for who fired the shot.
/// Actor/item/boss projectiles historically materialize before the projectile
/// step and therefore advance on the firing tick. The charged named-body-fire
/// path historically fires after that step and therefore begins on the next tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileStart {
    /// Materialize before this tick's projectile step.
    StepThisTick,
    /// Materialize after this tick's projectile step; first advance next tick.
    StepNextTick,
}

/// How the presentation layer identifies a projectile.
///
/// These variants intentionally say nothing about faction. A sentry or wielded
/// weapon can emit an open-visual projectile on a player's side, and allegiance
/// is frozen separately from the real owner entity after materialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectilePresentation {
    /// A named projectile vocabulary entry (Fireball / Hadouken / …).
    /// The kind component and its registered visual id are both stamped.
    NamedKind(ProjectileKind),
    /// An open visual id authored by an actor/item/boss technique. The empty
    /// string means the generic open-projectile look.
    OpenVisual(String),
}

/// One authoritative request to materialize a projectile entity.
///
/// `owner == Entity::PLACEHOLDER` means there is deliberately no firing body
/// (for example, an environmental volley). A real owner is stamped as
/// [`crate::ProjectileOwner`] and is the source of faction/team allegiance and
/// presentation-source inheritance.
#[derive(Message, Clone, Debug)]
pub struct ProjectileSpawnRequest {
    pub owner: Entity,
    pub projectile: InFlightProjectile,
    pub presentation: ProjectilePresentation,
    pub start: ProjectileStart,
}

impl ProjectileSpawnRequest {
    /// Build an open-visual projectile from authored spawn data.
    pub fn open(
        owner: Entity,
        spawn: ambition_projectile_spec::ProjectileSpawn,
        start: ProjectileStart,
    ) -> Self {
        let visual_id = spawn.visual_id.clone();
        Self {
            owner,
            projectile: build_in_flight_projectile(spawn),
            presentation: ProjectilePresentation::OpenVisual(visual_id),
            start,
        }
    }

    /// Materialize a named projectile whose gameplay body is already prepared.
    pub fn named(
        owner: Entity,
        projectile: InFlightProjectile,
        kind: ProjectileKind,
        start: ProjectileStart,
    ) -> Self {
        Self {
            owner,
            projectile,
            presentation: ProjectilePresentation::NamedKind(kind),
            start,
        }
    }
}

/// Lower substrate-neutral authored spawn data into the shared in-flight body.
///
/// This is the one request→body mapping for open-visual shots. Keeping the lowering with the
/// request makes the ownership literal and lets the dead pool resource disappear.
pub fn build_in_flight_projectile(
    request: ambition_projectile_spec::ProjectileSpawn,
) -> InFlightProjectile {
    let speed = request.speed.max(1.0);
    let dir = if request.dir.length() < 1.0e-4 {
        ambition_platformer2d_core::Vec2::new(1.0, 0.0)
    } else {
        request.dir / request.dir.length()
    };
    let spec = ProjectileSpec {
        origin: request.origin,
        direction: dir,
        damage: request.damage.max(1),
        speed,
        max_lifetime: request.max_lifetime.max(0.2),
        half_extent: request.half_extent,
        gravity: request.gravity.max(0.0),
        bounces: request.bounces,
        world_hit: if request.bounce_on_world_contact {
            WorldHitPolicy::Bouncing
        } else {
            WorldHitPolicy::ExpireOnContact
        },
        // Open authored spawn data has no named charge vocabulary. Named body
        // fire has already applied its charge tier to damage/size before it
        // reaches this request seam.
        charge_tier: 0,
        boomerang_return_s: request.boomerang_return_s,
        splash_half_extent: request.splash_half_extent.max(0.0),
    };
    InFlightProjectile {
        body: ProjectileBody::from_spec(spec),
    }
}

#[cfg(test)]
mod tests {
    use ambition_platformer2d_core as ae;

    use super::build_in_flight_projectile;
    use crate::WorldHitPolicy;

    fn spawn(speed: f32, damage: i32) -> ambition_projectile_spec::ProjectileSpawn {
        ambition_projectile_spec::ProjectileSpawn {
            origin: ae::Vec2::ZERO,
            dir: ae::Vec2::new(1.0, 0.0),
            speed,
            damage,
            max_lifetime: 1.0,
            half_extent: ae::Vec2::new(8.0, 8.0),
            gravity: 0.0,
            visual_id: "bolt".into(),
            bounces: 0,
            bounce_on_world_contact: false,
            splash_half_extent: 0.0,
            boomerang_return_s: None,
        }
    }

    #[test]
    fn open_request_lowering_preserves_authored_flight() {
        let projectile = build_in_flight_projectile(spawn(120.0, 3));
        assert!((projectile.body.kin.vel.x - 120.0).abs() < 1.0e-4);
        assert_eq!(projectile.body.game.damage, 3);
        assert_eq!(projectile.body.game.bounces_remaining, 0);
        assert_eq!(projectile.body.game.world_hit, WorldHitPolicy::ExpireOnContact);
    }

    #[test]
    fn open_request_lowering_keeps_the_historical_safety_clamps() {
        let mut request = spawn(0.0, 0);
        request.dir = ae::Vec2::ZERO;
        request.max_lifetime = 0.0;
        request.gravity = -5.0;
        let projectile = build_in_flight_projectile(request);
        assert!(projectile.body.kin.vel.x >= 1.0);
        assert_eq!(projectile.body.kin.vel.y, 0.0);
        assert!(projectile.body.game.damage >= 1);
        assert!(projectile.body.game.max_lifetime >= 0.2);
        assert_eq!(projectile.body.game.gravity, 0.0);
    }
}
