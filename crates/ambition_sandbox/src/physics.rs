//! Shared world physics applied to every actor body.
//!
//! [`GravityField`] is the world's gravity state (a redirectable down). The
//! goal of this module is that adding a new *global* force later — wind, a
//! tractor field, a gravity well — is a one-place change that reaches **every**
//! actor automatically:
//!
//! - **Free bodies** (thrown / ground items, projectiles) integrate through
//!   [`apply_world_forces`] — the single per-frame "apply global forces to a
//!   velocity" call. Add a force to `GravityField` + one line here and they all
//!   pick it up.
//! - **Collision-bound controllers** (the player, enemies) own bespoke swept-AABB
//!   integrators whose ground / jump logic is axis-based, so they consume
//!   [`GravityField::vertical_sign`] (which way is "down" along Y). They read the
//!   same `GravityField`, so a gravity flip moves them too.
//!
//! We deliberately keep this lightweight rather than reaching for a full
//! rigid-body engine: the platformer controllers are custom (parry2d swept
//! AABBs) for feel. **Avian2D remains available (ADR 0007)** for the day real
//! rigid-body physics — debris, ragdoll, stacked/complex collisions — is
//! genuinely needed; that's the escape hatch, not this seam.

use bevy::prelude::*;

/// The world's gravity direction (unit vector, in the y-DOWN world frame) —
/// default straight down. Change `dir` and every actor reorients + falls the new
/// way: it's the gravity-room / gravity-effect hook. Set by e.g. the
/// `GravityFlipSwitch`; consumed by the player / enemy / item / projectile
/// integrators and by the portal orient-to-gravity roll.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GravityField {
    pub dir: Vec2,
}

impl Default for GravityField {
    fn default() -> Self {
        // +Y is down in the world frame, so default gravity points +Y.
        Self {
            dir: Vec2::new(0.0, 1.0),
        }
    }
}

impl GravityField {
    /// Gravity acceleration vector for a body whose gravity magnitude is
    /// `magnitude` (px/s²). Used by free bodies that can fall in any direction.
    pub fn gravity_accel(&self, magnitude: f32) -> Vec2 {
        self.dir.normalize_or_zero() * magnitude
    }

    /// Sign of gravity along Y: `+1` = down (normal), `-1` = up (flipped). Used
    /// by the axis-based collision controllers (player / enemies).
    pub fn vertical_sign(&self) -> f32 {
        if self.dir.y >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }
}

/// The room's **ambient** gravity — the default an actor falls under when it's
/// not inside any [`GravityZone`]. Flipped by the `GravityFlipSwitch` and
/// (later) authored per room. [`resolve_active_gravity`] copies this (or an
/// overlapping zone's direction) into the live [`GravityField`] each frame, so
/// the switch sets the ambient while zones override locally.
#[derive(Resource, Clone, Copy, Debug)]
pub struct BaseGravity {
    pub dir: Vec2,
}

impl Default for BaseGravity {
    fn default() -> Self {
        Self {
            dir: Vec2::new(0.0, 1.0),
        }
    }
}

/// An authored region with its own gravity direction — the building block of a
/// "gravity room". While the player is inside the zone's `aabb`, the world
/// [`GravityField`] points along `dir` (everything falls that way and the player
/// reorients via the shared `ActorRoll`); on exit it reverts to [`BaseGravity`].
#[derive(Component, Clone, Copy, Debug)]
pub struct GravityZone {
    /// World-space region (engine coords) the zone covers.
    pub aabb: crate::engine_core::Aabb,
    /// Gravity direction inside the zone (e.g. `(0,-1)` = up).
    pub dir: Vec2,
}

/// Resolve the live [`GravityField`] each frame: it points along the first
/// [`GravityZone`] the player overlaps, else the room's [`BaseGravity`]. This is
/// the one writer of `GravityField.dir`, so zones and the ambient switch compose
/// cleanly (zone overrides ambient while inside). Reorientation is handled for
/// free by the shared `update_actor_roll`, which eases every body toward the new
/// gravity.
pub fn resolve_active_gravity(
    base: Option<Res<BaseGravity>>,
    zones: Query<&GravityZone>,
    players: Query<
        &crate::player::PlayerKinematics,
        (
            With<crate::player::PlayerEntity>,
            With<crate::player::PrimaryPlayer>,
        ),
    >,
    mut gravity: ResMut<GravityField>,
) {
    use crate::engine_core::AabbExt;
    let base_dir = base.map_or(Vec2::new(0.0, 1.0), |b| b.dir);
    let target = players
        .single()
        .ok()
        .and_then(|kin| {
            let body = crate::engine_core::Aabb::new(kin.pos, kin.size * 0.5);
            zones
                .iter()
                .find(|z| body.strict_intersects(z.aabb))
                .map(|z| z.dir)
        })
        .unwrap_or(base_dir);
    gravity.dir = target.normalize_or_zero();
}

/// Apply the world's per-frame global forces to a free body's velocity. This is
/// the ONE place new global forces get added, so every caller (items,
/// projectiles, …) inherits them. Today it's just gravity; future forces go
/// right here.
pub fn apply_world_forces(vel: &mut Vec2, gravity_magnitude: f32, gravity: &GravityField, dt: f32) {
    *vel += gravity.gravity_accel(gravity_magnitude) * dt;
    // ── add new global forces here (wind, drag fields, gravity wells) ──
}

/// Render-space z-rotation that stands a body upright under `gravity_dir`: it
/// points the sprite's local +Y ("up") along world-up (`-gravity`), accounting
/// for the y-down→y-up render flip. Default gravity → angle 0. (Lives here with
/// the gravity state; consumed by the portal orient-to-gravity roll.)
pub fn gravity_upright_angle(gravity_dir: Vec2) -> f32 {
    let g = gravity_dir.normalize_or_zero();
    if g == Vec2::ZERO {
        return 0.0;
    }
    // World up = -g; the render frame flips y.
    let render_up = Vec2::new(-g.x, g.y);
    render_up.y.atan2(render_up.x) - std::f32::consts::FRAC_PI_2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_sign_and_accel_track_the_direction() {
        let down = GravityField::default();
        assert_eq!(down.vertical_sign(), 1.0);
        assert_eq!(down.gravity_accel(1000.0), Vec2::new(0.0, 1000.0));
        let up = GravityField {
            dir: Vec2::new(0.0, -1.0),
        };
        assert_eq!(up.vertical_sign(), -1.0);
        assert_eq!(up.gravity_accel(1000.0), Vec2::new(0.0, -1000.0));
    }

    #[test]
    fn gravity_zone_overrides_ambient_while_inside_then_reverts() {
        let mut app = App::new();
        app.init_resource::<GravityField>();
        app.init_resource::<BaseGravity>();
        app.add_systems(Update, resolve_active_gravity);
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerEntity,
                crate::player::PrimaryPlayer,
                crate::player::PlayerKinematics {
                    pos: Vec2::new(0.0, 0.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.world_mut().spawn(GravityZone {
            aabb: crate::engine_core::Aabb::new(Vec2::new(200.0, 0.0), Vec2::new(60.0, 60.0)),
            dir: Vec2::new(0.0, -1.0), // up
        });

        // Outside the zone → ambient (down).
        app.update();
        assert!(app.world().resource::<GravityField>().dir.y > 0.0, "starts ambient down");

        // Inside the zone → gravity points up.
        app.world_mut()
            .get_mut::<crate::player::PlayerKinematics>(player)
            .unwrap()
            .pos = Vec2::new(200.0, 0.0);
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y < 0.0,
            "inside the gravity-up zone, gravity points up"
        );

        // Leave the zone → reverts to ambient down.
        app.world_mut()
            .get_mut::<crate::player::PlayerKinematics>(player)
            .unwrap()
            .pos = Vec2::new(0.0, 0.0);
        app.update();
        assert!(
            app.world().resource::<GravityField>().dir.y > 0.0,
            "exiting the zone reverts to ambient gravity"
        );
    }

    #[test]
    fn apply_world_forces_adds_gravity_over_dt() {
        let g = GravityField::default();
        let mut vel = Vec2::ZERO;
        apply_world_forces(&mut vel, 1200.0, &g, 0.5);
        assert_eq!(vel, Vec2::new(0.0, 600.0));
    }
}
