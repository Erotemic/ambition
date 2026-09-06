//! Per-body movement-frame resolution.
//!
//! [`FrameResolveSet`] publishes one [`ResolvedMotionFrame`] per integrated body
//! after environmental contributions are known and before frame-relative input,
//! movement, combat, and support consumers run. [`FrameEnv::resolve`] is the
//! composition rule for localized gravity and non-orienting world acceleration.
//! The resolved frame is transient derived state and is recomputed after restore.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_core::{AabbExt, AccelerationFrame, MotionFrame};

use crate::gravity::GravityCtx;

/// Environment-resolved movement frame for the current simulation tick.
/// Published only by [`FrameResolveSet`]; newly spawned bodies use the default
/// until the next resolution pass.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ResolvedMotionFrame {
    frame: MotionFrame,
}

impl Default for ResolvedMotionFrame {
    fn default() -> Self {
        Self {
            frame: MotionFrame::from_direction(
                ambition_platformer2d_core::DEFAULT_GRAVITY_DIR,
                0.0,
            ),
        }
    }
}

impl ResolvedMotionFrame {
    /// The resolved frame value. This is what crosses the kernel boundary as
    /// `MotionStepContext::frame` and what every input/combat/ability consumer
    /// interprets its body-relative quantities in.
    pub fn get(&self) -> MotionFrame {
        self.frame
    }

    /// Convenience: the frame's toward-feet direction in world space.
    pub fn down(&self) -> Vec2 {
        self.frame.down()
    }

    /// Convenience: the frame's reference basis.
    pub fn basis(&self) -> AccelerationFrame {
        self.frame.basis()
    }

    /// Publish this tick's resolved frame. Only the frame-resolution phase may
    /// call this; workspace policy guards this named mutation seam.
    pub fn publish_resolved_frame(&mut self, frame: MotionFrame) {
        self.frame = frame;
    }
}

/// An authored region contributing world-space acceleration WITHOUT defining
/// orientation — wind, conveyor updrafts, tractor fields. A body overlapping the
/// region accumulates `accel` on top of its gravity contribution; its reference
/// basis still comes from the gravity environment alone. This is the
/// counterexample that keeps basis and acceleration independent: lateral force
/// never rotates a body's frame, and a zero-gravity body inside a force zone
/// still knows which way its feet point.
#[derive(Component, Clone, Copy, Debug)]
pub struct ForceZone {
    /// World-space region (engine coords) the force covers.
    pub aabb: ambition_platformer2d_core::Aabb,
    /// World-space acceleration (px/s²) applied to overlapping bodies. NOT
    /// scaled by the body's gravity response — gravity response scales gravity.
    pub accel: Vec2,
}

/// Per-tick snapshot of every [`ForceZone`], mirroring
/// [`GravityZones`](crate::gravity::GravityZones) so the resolution phase reads
/// one resource. Rebuilt by [`collect_force_zones`] in the zone-snapshot phase.
#[derive(Resource, Default, Clone, Debug)]
pub struct ForceZones {
    pub zones: Vec<(ambition_platformer2d_core::Aabb, Vec2)>,
}

impl ForceZones {
    /// Sum of the world-space acceleration contributions grabbing `body` (the
    /// same body-overlap rule gravity zones use).
    pub fn accel_for(&self, body: ambition_platformer2d_core::Aabb) -> Vec2 {
        self.zones
            .iter()
            .filter(|(aabb, _)| body.strict_intersects(*aabb))
            .map(|(_, accel)| *accel)
            .sum()
    }
}

/// Rebuild the [`ForceZones`] snapshot from live components. Scheduled with the
/// gravity-zone snapshot, before the frame resolution phase.
pub fn collect_force_zones(mut snapshot: ResMut<ForceZones>, zones: Query<&ForceZone>) {
    snapshot.zones.clear();
    snapshot
        .zones
        .extend(zones.iter().map(|z| (z.aabb, z.accel)));
}

/// The frame resolution phase: publishes every integrated body's
/// [`ResolvedMotionFrame`] for the tick. Configured after the environment's
/// zone snapshot and before `Platformer2dSimulationPhaseMonolith::CoreSimulation`, so controller
/// interpretation, brains, combat, and integration all read this tick's value.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct FrameResolveSet;

/// The complete frame environment: gravity (orientation + gravity acceleration)
/// plus non-orienting force contributions. This is the resolver's ONLY input
/// bundle; nothing else composes a body frame.
#[derive(SystemParam)]
pub struct FrameEnv<'w> {
    pub gravity: GravityCtx<'w>,
    pub forces: Option<Res<'w, ForceZones>>,
}

impl FrameEnv<'_> {
    /// Resolve one body's frame: THE composition rule.
    ///
    /// - The reference basis comes from the localized gravity direction the
    ///   body's AABB overlaps (zone-or-ambient). Orientation is defined even
    ///   when the resulting acceleration is zero.
    /// - The gravity contribution is that direction scaled by the body's
    ///   authored `gravity_response` (tuning gravity × surface scale; an aerial
    ///   body's 0 keeps orientation with zero gravity acceleration).
    /// - Force-zone contributions accumulate in world space, unscaled by the
    ///   gravity response and without rotating the basis.
    pub fn resolve(
        &self,
        body: ambition_platformer2d_core::Aabb,
        gravity_response: f32,
    ) -> MotionFrame {
        let dir = self.gravity.dir_for(body);
        let gravity_acceleration = dir * gravity_response.max(0.0);
        let external_acceleration = self
            .forces
            .as_deref()
            .map(|forces| forces.accel_for(body))
            .unwrap_or(Vec2::ZERO);
        MotionFrame::with_accelerations(
            AccelerationFrame::new(dir),
            gravity_acceleration,
            external_acceleration,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gravity::{BaseGravity, GravityZone, GravityZones};
    use ambition_platformer2d_core::Aabb;

    fn env_app() -> App {
        let mut app = App::new();
        app.init_resource::<crate::gravity::GravityField>();
        app.init_resource::<BaseGravity>();
        app.init_resource::<GravityZones>();
        app.init_resource::<ForceZones>();
        app.add_systems(
            Update,
            (crate::gravity::collect_gravity_zones, collect_force_zones),
        );
        app
    }

    fn resolve_in(app: &mut App, body: Aabb, response: f32) -> MotionFrame {
        let mut state: bevy::ecs::system::SystemState<FrameEnv> =
            bevy::ecs::system::SystemState::new(app.world_mut());
        let env = state.get(app.world()).expect("frame env params");
        env.resolve(body, response)
    }

    #[test]
    fn basis_comes_from_gravity_orientation_even_at_zero_response() {
        let mut app = env_app();
        app.world_mut().spawn(GravityZone {
            aabb: Aabb::new(Vec2::new(100.0, 0.0), Vec2::new(50.0, 50.0)),
            dir: Vec2::new(-1.0, 0.0), // left
        });
        app.update();
        let body = Aabb::new(Vec2::new(100.0, 0.0), Vec2::new(10.0, 10.0));
        let frame = resolve_in(&mut app, body, 0.0);
        assert_eq!(frame.down(), Vec2::new(-1.0, 0.0), "orientation retained");
        assert_eq!(frame.acceleration(), Vec2::ZERO, "zero response, zero pull");
    }

    #[test]
    fn force_zone_accumulates_without_rotating_the_basis() {
        let mut app = env_app();
        app.world_mut().spawn(ForceZone {
            aabb: Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(50.0, 50.0)),
            accel: Vec2::new(300.0, 0.0), // lateral wind
        });
        app.update();
        let body = Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let frame = resolve_in(&mut app, body, 900.0);
        assert_eq!(frame.down(), Vec2::new(0.0, 1.0), "wind does not rotate");
        assert_eq!(frame.gravity_acceleration(), Vec2::new(0.0, 900.0));
        assert_eq!(frame.external_acceleration(), Vec2::new(300.0, 0.0));
        assert_eq!(
            frame.acceleration(),
            Vec2::new(300.0, 900.0),
            "gravity and wind contributions compose in world space"
        );
    }

    #[test]
    fn multiple_contributions_compose_and_gravity_response_scales_only_gravity() {
        let mut app = env_app();
        app.world_mut().spawn(ForceZone {
            aabb: Aabb::new(Vec2::ZERO, Vec2::new(50.0, 50.0)),
            accel: Vec2::new(200.0, 0.0),
        });
        app.world_mut().spawn(ForceZone {
            aabb: Aabb::new(Vec2::ZERO, Vec2::new(50.0, 50.0)),
            accel: Vec2::new(0.0, -100.0),
        });
        app.update();
        let body = Aabb::new(Vec2::ZERO, Vec2::new(10.0, 10.0));
        // Zero gravity response: force zones still reach the body, unscaled.
        let frame = resolve_in(&mut app, body, 0.0);
        assert_eq!(frame.gravity_acceleration(), Vec2::ZERO);
        assert_eq!(frame.external_acceleration(), Vec2::new(200.0, -100.0));
        assert_eq!(frame.acceleration(), Vec2::new(200.0, -100.0));
        assert_eq!(frame.down(), Vec2::new(0.0, 1.0));
    }

    #[test]
    fn zone_selection_uses_body_overlap_not_center_point() {
        let mut app = env_app();
        app.world_mut().spawn(GravityZone {
            aabb: Aabb::new(Vec2::new(100.0, 0.0), Vec2::new(20.0, 20.0)),
            dir: Vec2::new(0.0, -1.0),
        });
        app.update();
        // Body center OUTSIDE the zone but its AABB overlaps it: grabbed.
        let straddling = Aabb::new(Vec2::new(70.0, 0.0), Vec2::new(15.0, 15.0));
        let frame = resolve_in(&mut app, straddling, 900.0);
        assert_eq!(
            frame.down(),
            Vec2::new(0.0, -1.0),
            "a zone grabs a body the body TOUCHES"
        );
    }
}
