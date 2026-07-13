//! The authoritative per-body movement frame: resolved once, consumed everywhere.
//!
//! ADR 0024's frame law says every body-relative operation in a movement tick —
//! controller interpretation, the active movement policy, jumps/dashes/blinks,
//! knockback and launch directions, support publication — must consume ONE
//! environment-resolved reference/acceleration frame for that body and tick. This
//! module owns the ECS half of that law:
//!
//! - [`ResolvedMotionFrame`] is the transient per-body artifact. The frame
//!   resolution phase ([`FrameResolveSet`]) publishes it exactly once per
//!   integrated body per sim tick, after the tick's environmental contributions
//!   (gravity zones, force zones, ambient flips) are snapshotted and before any
//!   consumer reads it. Consumers NEVER rebuild an equivalent frame from
//!   `GravityCtx::dir_at`, `GravityField`, or a hardcoded down — they read this
//!   component.
//! - [`FrameEnv`] is the resolver's input bundle: the gravity environment plus
//!   non-gravity acceleration contributions. [`FrameEnv::resolve`] is the ONE
//!   composition rule — an explicit reference basis from the localized gravity
//!   direction (selected by body-AABB overlap, the engine's zone-grab rule),
//!   plus accumulated world-space acceleration contributions where the body's
//!   authored gravity response scales ONLY the gravity contribution.
//! - [`ForceZone`] is a non-orienting acceleration contribution (wind, a tractor
//!   field): it adds world-space acceleration without rotating the reference
//!   basis, proving basis and acceleration are independently resolved.
//!
//! The resolved frame is deliberately NOT authored body state, NOT part of
//! [`MotionModel`](ambition_engine_core::MotionModel), and NOT snapshot state:
//! restore rewinds bodies and environment, and the next resolution phase
//! recomputes the frame from the live restored world.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_engine_core::{AabbExt, AccelerationFrame, MotionFrame};

use crate::gravity::GravityCtx;

/// The body's environment-resolved reference/acceleration frame for the current
/// sim tick — THE value every frame-relative consumer of this body reads.
///
/// Published only by the frame resolution phase (see [`FrameResolveSet`]); the
/// field is private so gameplay code cannot casually author it. A body spawned
/// mid-tick carries the default (screen-down basis, zero acceleration) until the
/// next resolution phase.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ResolvedMotionFrame {
    frame: MotionFrame,
}

impl Default for ResolvedMotionFrame {
    fn default() -> Self {
        Self {
            frame: MotionFrame::from_direction(ambition_engine_core::DEFAULT_GRAVITY_DIR, 0.0),
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

    /// Publish this tick's resolved frame. ONLY the frame resolution phase may
    /// call this (guarded by workspace policy, not just convention).
    pub fn publish(&mut self, frame: MotionFrame) {
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
    pub aabb: ambition_engine_core::Aabb,
    /// World-space acceleration (px/s²) applied to overlapping bodies. NOT
    /// scaled by the body's gravity response — gravity response scales gravity.
    pub accel: Vec2,
}

/// Per-tick snapshot of every [`ForceZone`], mirroring
/// [`GravityZones`](crate::gravity::GravityZones) so the resolution phase reads
/// one resource. Rebuilt by [`collect_force_zones`] in the zone-snapshot phase.
#[derive(Resource, Default, Clone, Debug)]
pub struct ForceZones {
    pub zones: Vec<(ambition_engine_core::Aabb, Vec2)>,
}

impl ForceZones {
    /// Sum of the world-space acceleration contributions grabbing `body` (the
    /// same body-overlap rule gravity zones use).
    pub fn accel_for(&self, body: ambition_engine_core::Aabb) -> Vec2 {
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
/// zone snapshot and before `SandboxSet::CoreSimulation`, so controller
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
    pub fn resolve(&self, body: ambition_engine_core::Aabb, gravity_response: f32) -> MotionFrame {
        let dir = self.gravity.dir_for(body);
        let mut accel = dir * gravity_response.max(0.0);
        if let Some(forces) = self.forces.as_deref() {
            accel += forces.accel_for(body);
        }
        MotionFrame::new(AccelerationFrame::new(dir), accel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gravity::{BaseGravity, GravityZone, GravityZones};
    use ambition_engine_core::Aabb;

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
        let env = state.get(app.world());
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
