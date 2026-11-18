//! Physical projectile construction and material admission.

use crate::{InFlightProjectile, ProjectileBody, ProjectileSpec, WorldHitPolicy};
use bevy::prelude::Entity;

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
        properties: request.properties,
        guard: request.guard,
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
            properties: Default::default(),
            guard: Default::default(),
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
        assert_eq!(
            projectile.body.game.world_hit,
            WorldHitPolicy::ExpireOnContact
        );
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

/// Prepare a projectile in the chart reached from its emitter's body root.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ProjectileBirth<'w, 's> {
    geometry: Option<
        bevy::prelude::Res<'w, ambition_platformer2d_shared_tangle::body::geometry::BodyGeometry>,
    >,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms<'w, 's>,
}

impl ProjectileBirth<'_, '_> {
    /// Admit before the producer spends resources or emits a request.
    pub fn prepare(
        &self,
        owner: Entity,
        reference: ambition_platformer2d_core::Vec2,
        mut body: ProjectileBody,
    ) -> Result<ProjectileBody, ambition_platformer2d_core::frame::thread::Refusal> {
        let room = self.rooms.of(owner);
        let admission = self.geometry.as_deref().and_then(|geometry| {
            geometry.motion_for(owner, room).map(|admission| {
                ambition_platformer2d_core::frame::thread::MotionAdmission {
                    budget: geometry.budget_for(None, room),
                    ..admission
                }
            })
        });
        let [acceleration] = ambition_platformer2d_core::movement::prepare_body_materialization(
            body.kin,
            reference,
            admission,
            [body.acceleration.0],
        )?
        .commit(&mut body.kin, None);
        body.acceleration.0 = acceleration;
        Ok(body)
    }
}
