//! `VolumeShape` — an authored hit/hurt shape in LOCAL space.
//!
//! This is the authoring currency for combat geometry: a tiny, serializable
//! value type (RON-friendly) describing a shape *relative to a placement
//! origin*, with `+x` = the actor's forward/facing direction. It is the same
//! type whether the shape is authored *with a sprite* (a per-animation hitbox)
//! or *with an effect* (an explosion's blast radius) — the two authoring homes
//! share one currency.
//!
//! [`VolumeShape::place_at`] is the one "place an effect at the right position"
//! operation: it mirrors the local shape to the actor's facing and orients it to
//! the actor's reference frame (gravity / clung surface), producing the
//! world-space [`CombatVolume`] the damage path consumes. Under vertical gravity
//! and a `Box` shape this is just an `Aabb` (the common, cheapest case).
//!
//! Missing authored data falls back to [`VolumeShape::default`] — a small dummy
//! box — so the game stays playable when a sprite's RON/PNG fails to load and no
//! shape was authored. (Game state differs without the authored data, but it
//! does not break.)

use crate::reference_frame::AccelerationFrame;
use crate::{CombatVolume, Vec2};

/// Half-extent of the [`VolumeShape::default`] dummy box — the fallback used
/// when no shape was authored (e.g. a sprite sheet failed to load). Small but
/// non-zero so combat still functions.
pub const DUMMY_HALF: f32 = 12.0;

/// An authored shape in local space (origin = placement point, `+x` = forward).
/// Resolve to a world [`CombatVolume`] with [`VolumeShape::place_at`].
#[derive(Clone, Debug, PartialEq)]
pub enum VolumeShape {
    /// Axis-aligned box (in the actor's frame) — the common, cheapest case.
    Box { half: Vec2 },
    /// Box rotated `angle` radians (CCW) within the actor's frame.
    Obb { half: Vec2, angle: f32 },
    /// Circle / disc — explosions, radial AoE.
    Circle { radius: f32 },
    /// Arbitrary convex polygon, local points (`+x` forward). Authored for blade
    /// arcs, cones, etc.
    Convex { points: Vec<Vec2> },
}

impl Default for VolumeShape {
    /// The dummy fallback used when no shape was authored.
    fn default() -> Self {
        VolumeShape::Box {
            half: Vec2::splat(DUMMY_HALF),
        }
    }
}

impl VolumeShape {
    /// Explicit dummy fallback (same as [`Default`]) for call sites that resolve
    /// missing authored data.
    pub fn dummy() -> Self {
        VolumeShape::default()
    }

    pub fn box_half(half: Vec2) -> Self {
        VolumeShape::Box { half }
    }

    pub fn circle(radius: f32) -> Self {
        VolumeShape::Circle {
            radius: radius.max(0.0),
        }
    }

    /// Place this local shape into the world at `origin`, mirrored to `facing`
    /// (`< 0` flips local `+x`) and oriented so the actor's local `down` points
    /// along `frame_down` (gravity or a clung surface). Returns the world
    /// [`CombatVolume`] for intersection. Identity orientation under vertical
    /// gravity, so the common case is a plain `Aabb`.
    pub fn place_at(&self, origin: Vec2, facing: f32, frame_down: Vec2) -> CombatVolume {
        let frame = AccelerationFrame::new(frame_down);
        // Frame rotation angle: the angle of the local +x (side) axis. Zero
        // under vertical gravity → boxes stay axis-aligned (fast Aabb path).
        let theta = frame.side.y.atan2(frame.side.x);
        let face = if facing < 0.0 { -1.0 } else { 1.0 };
        // Map a local point (+x forward, +y down) into world space.
        let to_world = |local: Vec2| origin + frame.side * (local.x * face) + frame.down * local.y;

        match self {
            VolumeShape::Box { half } => {
                if theta.abs() < 1.0e-5 {
                    CombatVolume::aabb(crate::Aabb::new(origin, *half))
                } else {
                    CombatVolume::obb(origin, *half, theta)
                }
            }
            VolumeShape::Obb { half, angle } => {
                let world_angle = theta + angle * face;
                if world_angle.abs() < 1.0e-5 {
                    CombatVolume::aabb(crate::Aabb::new(origin, *half))
                } else {
                    CombatVolume::obb(origin, *half, world_angle)
                }
            }
            // A disc is rotation- and mirror-invariant; only its center moves.
            VolumeShape::Circle { radius } => CombatVolume::circle(origin, *radius),
            VolumeShape::Convex { points } => {
                CombatVolume::convex(points.iter().map(|p| to_world(*p)).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AabbExt;

    const DOWN: Vec2 = Vec2::new(0.0, 1.0);

    #[test]
    fn box_upright_is_a_plain_aabb_at_origin() {
        let vol = VolumeShape::box_half(Vec2::new(10.0, 20.0)).place_at(
            Vec2::new(5.0, 7.0),
            1.0,
            DOWN,
        );
        match vol {
            CombatVolume::Aabb(a) => {
                assert_eq!(a.center(), Vec2::new(5.0, 7.0));
                assert_eq!(a.half_size(), Vec2::new(10.0, 20.0));
            }
            other => panic!("expected Aabb under vertical gravity, got {other:?}"),
        }
    }

    #[test]
    fn box_under_sideways_gravity_rotates() {
        // Gravity points +x: the box should rotate ~90°, so a tall box (half
        // 4x20) now reaches far in x and little in y.
        let vol =
            VolumeShape::box_half(Vec2::new(4.0, 20.0)).place_at(Vec2::ZERO, 1.0, Vec2::new(1.0, 0.0));
        let b = vol.bounds();
        assert!(b.half_size().x > 19.0, "rotated box should be wide in x");
        assert!(b.half_size().y < 5.0, "rotated box should be short in y");
    }

    #[test]
    fn convex_mirrors_with_facing() {
        // A forward-poking triangle: right-facing reaches +x, left-facing -x.
        let tri = VolumeShape::Convex {
            points: vec![Vec2::ZERO, Vec2::new(40.0, -8.0), Vec2::new(40.0, 8.0)],
        };
        let right = tri.place_at(Vec2::ZERO, 1.0, DOWN);
        let left = tri.place_at(Vec2::ZERO, -1.0, DOWN);
        assert!(right.bounds().right() > 30.0 && right.bounds().left() >= -0.01);
        assert!(left.bounds().left() < -30.0 && left.bounds().right() <= 0.01);
    }

    #[test]
    fn circle_is_facing_and_frame_invariant() {
        let a = VolumeShape::circle(15.0).place_at(Vec2::new(3.0, 3.0), -1.0, Vec2::new(1.0, 0.0));
        match a {
            CombatVolume::Circle { center, radius } => {
                assert_eq!(center, Vec2::new(3.0, 3.0));
                assert_eq!(radius, 15.0);
            }
            other => panic!("expected Circle, got {other:?}"),
        }
    }

    #[test]
    fn default_is_a_small_dummy_box() {
        assert_eq!(
            VolumeShape::default(),
            VolumeShape::Box {
                half: Vec2::splat(DUMMY_HALF)
            }
        );
    }
}
