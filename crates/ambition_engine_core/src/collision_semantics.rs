//! Shared collision-semantics kernel: the gravity-relative support/surface
//! truths every actor body agrees on.
//!
//! Two sweeps consume these primitives:
//!
//! - [`crate::movement::collision`] — the controlled-body movement sweep, with
//!   jump-buffer / dash / blink / climb / wall-state affordances layered on top.
//! - `ambition_platformer_primitives::kinematic` — the generic enemy/NPC/actor
//!   sweep.
//!
//! Both used to carry private copies of these helpers. The copies were *almost*
//! identical, which is the dangerous kind of duplication: the two bodies agreed
//! at the design level while being free to drift at the implementation level
//! (one-way landing eligibility, support-face tolerances, non-down gravity).
//! This module is the single source of truth for the low-level semantic kernel
//! so every controlled/scripted/AI/remote actor collides against the same rules.
//! The richer *affordances* (depenetration strategy, wall-cling, climb passage,
//! ability tuning) stay in each sweep — only the pure classification/geometry
//! truths live here.
//!
//! Everything here is a pure function of `(BlockKind, Aabb, gravity_dir, …)` —
//! no `World`, no ECS, no per-frame state — so it is trivially testable across
//! all four cardinal gravity directions (see the `tests` module).

use crate::geometry::{Aabb, AabbExt};
use crate::world::BlockKind;
use crate::Vec2;

/// Resting contact tolerance along the gravity (feet) axis, in pixels. A body
/// whose feet are within this distance of a support face counts as resting on
/// it.
pub const CONTACT_SLOP: f32 = 4.0;

/// One-way landing crossing tolerance, in pixels. A body may land on a one-way
/// surface only if its previous feet coordinate was within this slack of the
/// surface's anti-gravity face — handling discrete timesteps near the surface.
pub const ONE_WAY_CROSSING_SLOP: f32 = 8.0;

/// Minimum motion (along an axis or toward the feet) treated as non-zero.
pub const MOTION_EPS: f32 = 1.0e-5;

/// A world axis. The world is axis-aligned, so sweeps and penetration repair
/// step one world axis at a time even though support/wall *decisions* are
/// expressed in gravity-relative (feet/head/side) terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

impl Axis {
    pub fn perpendicular(self) -> Self {
        match self {
            Axis::X => Axis::Y,
            Axis::Y => Axis::X,
        }
    }
}

/// Whether a world axis currently plays the gravity (feet/head) role or the
/// side (wall) role, given the body's gravity direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisRole {
    Gravity,
    Side,
}

/// The world axis gravity currently runs along (cardinal `gravity_dir`).
pub fn gravity_axis(gravity_dir: Vec2) -> Axis {
    if gravity_dir.x.abs() > gravity_dir.y.abs() {
        Axis::X
    } else {
        Axis::Y
    }
}

/// Classify a world axis as the gravity axis or a side axis for this gravity.
pub fn axis_role(axis: Axis, gravity_dir: Vec2) -> AxisRole {
    if axis == gravity_axis(gravity_dir) {
        AxisRole::Gravity
    } else {
        AxisRole::Side
    }
}

/// True when `delta` carries the body toward its feet (the +gravity direction).
pub fn moving_toward_feet(delta: Vec2, gravity_dir: Vec2) -> bool {
    delta.dot(gravity_dir) > MOTION_EPS
}

/// Surfaces a body can rest on: full solids, blink walls, and one-ways.
pub fn is_support_surface(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Solid | BlockKind::BlinkWall { .. } | BlockKind::OneWay
    )
}

/// Surfaces that block both axes unconditionally (solids and blink walls).
pub fn is_full_collision_surface(kind: BlockKind) -> bool {
    matches!(kind, BlockKind::Solid | BlockKind::BlinkWall { .. })
}

/// Whether `kind` is a collision surface for `axis` under this gravity.
///
/// Full solids/blink walls block both axes. One-way surfaces are collision
/// surfaces only on the current gravity axis (their passability is then decided
/// by the one-way landing rule); they never block on a side axis. Hazards, pogo
/// orbs, and rebound blocks are handled by gameplay logic, not collision.
pub fn is_solid_for_axis(kind: BlockKind, axis: Axis, gravity_dir: Vec2) -> bool {
    match kind {
        BlockKind::Solid | BlockKind::BlinkWall { .. } => true,
        BlockKind::OneWay => axis_role(axis, gravity_dir) == AxisRole::Gravity,
        BlockKind::Hazard | BlockKind::PogoOrb | BlockKind::Rebound { .. } => false,
    }
}

/// Signed separation between the body's feet face and the surface's head face
/// along the gravity axis. Zero at perfect rest; positive when the feet are
/// past (penetrating) the support face.
pub fn support_face_separation(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> f32 {
    body.feet_coord(gravity_dir) - surface.head_coord(gravity_dir)
}

/// True when the body's center is on the support side of the surface (the side
/// gravity pulls it onto), i.e. it could be resting on rather than under it.
pub fn body_on_support_side(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> bool {
    body.center().dot(gravity_dir) <= surface.center().dot(gravity_dir)
}

/// The position delta that snaps the body's feet face exactly onto the
/// surface's head face along the gravity axis.
pub fn snap_feet_to_surface(body: Aabb, surface: Aabb, gravity_dir: Vec2) -> Vec2 {
    gravity_dir * (surface.head_coord(gravity_dir) - body.feet_coord(gravity_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::aabb_from_min_size;
    use crate::world::BlinkWallTier;

    const CARDINALS: [Vec2; 4] = [
        Vec2::new(0.0, 1.0),  // down
        Vec2::new(0.0, -1.0), // up
        Vec2::new(1.0, 0.0),  // right
        Vec2::new(-1.0, 0.0), // left
    ];

    #[test]
    fn gravity_axis_and_role_are_cardinal_consistent() {
        for dir in CARDINALS {
            let g = gravity_axis(dir);
            assert_eq!(axis_role(g, dir), AxisRole::Gravity);
            assert_eq!(axis_role(g.perpendicular(), dir), AxisRole::Side);
        }
    }

    #[test]
    fn one_way_blocks_only_on_the_gravity_axis() {
        for dir in CARDINALS {
            let g = gravity_axis(dir);
            assert!(is_solid_for_axis(BlockKind::OneWay, g, dir));
            assert!(!is_solid_for_axis(BlockKind::OneWay, g.perpendicular(), dir));
            // Full solids block both axes in every frame.
            assert!(is_solid_for_axis(BlockKind::Solid, g, dir));
            assert!(is_solid_for_axis(BlockKind::Solid, g.perpendicular(), dir));
        }
    }

    #[test]
    fn non_collision_kinds_never_block() {
        for dir in CARDINALS {
            let g = gravity_axis(dir);
            for kind in [BlockKind::Hazard, BlockKind::PogoOrb] {
                assert!(!is_solid_for_axis(kind, g, dir));
                assert!(!is_solid_for_axis(kind, g.perpendicular(), dir));
                assert!(!is_support_surface(kind));
            }
        }
    }

    #[test]
    fn support_classification_matches_intent() {
        assert!(is_support_surface(BlockKind::Solid));
        assert!(is_support_surface(BlockKind::OneWay));
        assert!(is_support_surface(BlockKind::BlinkWall {
            tier: BlinkWallTier::Soft
        }));
        assert!(is_full_collision_surface(BlockKind::Solid));
        assert!(!is_full_collision_surface(BlockKind::OneWay));
    }

    #[test]
    fn moving_toward_feet_is_gravity_relative() {
        // Toward feet means along +gravity_dir in every frame.
        assert!(moving_toward_feet(Vec2::new(0.0, 5.0), Vec2::new(0.0, 1.0)));
        assert!(!moving_toward_feet(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0)));
        assert!(moving_toward_feet(Vec2::new(-5.0, 0.0), Vec2::new(-1.0, 0.0)));
        assert!(!moving_toward_feet(Vec2::new(5.0, 0.0), Vec2::new(-1.0, 0.0)));
    }

    #[test]
    fn feet_snap_and_separation_are_gravity_relative() {
        // Body resting just above a floor (down gravity): feet face is the
        // bottom; separation small-negative; snap pushes down onto the head.
        let floor = aabb_from_min_size(Vec2::new(0.0, 100.0), Vec2::new(100.0, 20.0));
        let body = Aabb::new(Vec2::new(40.0, 88.0), Vec2::new(10.0, 10.0));
        let dir = Vec2::new(0.0, 1.0);
        // feet at y=98, floor head at y=100 -> separation -2.
        assert!((support_face_separation(body, floor, dir) - (-2.0)).abs() < 1e-3);
        assert!(body_on_support_side(body, floor, dir));
        let snap = snap_feet_to_surface(body, floor, dir);
        assert!((snap.y - 2.0).abs() < 1e-3 && snap.x.abs() < 1e-6);
    }
}
