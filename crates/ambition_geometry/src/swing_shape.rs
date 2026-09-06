//! `SwingShape` — the ORIENTED shape of a swing, as presentation needs it.
//!
//! A [`CombatVolume`] answers "does this overlap that". A slash effect needs a
//! different question answered: *where does the swing start, which way does it
//! go, how far, and how wide is it at each end.* Those are the numbers the
//! sprite generator authors a blade arc from (`cone(origin, dir, length,
//! near_w, far_w)`), and until this type existed there was no way to carry them
//! from the damage volume to the effect that draws it.
//!
//! ## Why a projection and not the hull itself
//!
//! The renderer draws a sprite, and a sprite is a quad; handed a hull it would
//! derive exactly these numbers to place one. Carrying the projection keeps the
//! cue `Copy` and small, keeps hull math in one place, and stays forward
//! compatible: a future mesh path builds a conforming cone from the SAME five
//! numbers with no change to the message.

use crate::{AabbExt, CombatVolume, Vec2};

/// The oriented extent of a swing, world space.
///
/// Deliberately NOT a rectangle: a swing that flares (every blade arc in the
/// game) is a trapezoid, and an oriented rectangle is the degenerate case where
/// `near_half == far_half`. Presentation that only wants a box takes
/// [`SwingShape::oriented_bounds`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwingShape {
    /// A directional sweep: from `origin`, along unit `dir`, for `length`, with
    /// the perpendicular half-width growing `near_half` at the origin to
    /// `far_half` at its widest. A straight poke is `near_half == far_half`.
    Sweep {
        origin: Vec2,
        /// Unit vector. Guaranteed normalized by every constructor here.
        dir: Vec2,
        length: f32,
        near_half: f32,
        far_half: f32,
    },
    /// A body-centred sweep with no forward axis — the aerial-neutral spin that
    /// goes all the way around. This is NOT a degenerate `Sweep`: a sweep has a
    /// direction and a radial swing does not, and collapsing the two would make
    /// the renderer guess an orientation for a shape that has none.
    Radial { center: Vec2, radius: Vec2 },
}

impl SwingShape {
    /// The centre of the swept region — where a quad drawn for this shape sits.
    pub fn center(&self) -> Vec2 {
        match self {
            SwingShape::Sweep {
                origin,
                dir,
                length,
                ..
            } => *origin + *dir * (*length * 0.5),
            SwingShape::Radial { center, .. } => *center,
        }
    }

    /// Half-extents of the swing in its OWN frame: `x` along the swing axis,
    /// `y` across it. For a radial swing the frame is world-axis-aligned.
    ///
    /// This is what a quad-drawing renderer wants; it takes the WIDER end, so
    /// the quad contains the whole sweep rather than clipping the flare.
    pub fn oriented_bounds(&self) -> Vec2 {
        match self {
            SwingShape::Sweep {
                length,
                near_half,
                far_half,
                ..
            } => Vec2::new(*length * 0.5, near_half.max(*far_half)),
            SwingShape::Radial { radius, .. } => *radius,
        }
    }

    /// Rotation of the swing axis, radians CCW from world `+x`. Zero for a
    /// radial swing, which has no axis to rotate to.
    pub fn rotation(&self) -> f32 {
        match self {
            SwingShape::Sweep { dir, .. } => dir.y.atan2(dir.x),
            SwingShape::Radial { .. } => 0.0,
        }
    }

    /// Move the whole swing by `delta`.
    ///
    /// The cue carries a BODY-LOCAL swing so presentation can re-place it on a
    /// moving attacker every frame, which is what stops the drawn blade being
    /// left behind by the damage box that tracks the body. Deriving the shape
    /// needs the volume in the world (that is how the projection knows which end
    /// is the handle), so the subtraction happens after.
    pub fn translated(&self, delta: Vec2) -> Self {
        match *self {
            SwingShape::Sweep {
                origin,
                dir,
                length,
                near_half,
                far_half,
            } => SwingShape::Sweep {
                origin: origin + delta,
                dir,
                length,
                near_half,
                far_half,
            },
            SwingShape::Radial { center, radius } => SwingShape::Radial {
                center: center + delta,
                radius,
            },
        }
    }

    /// Scale the swing's EXTENT about its origin without moving or turning it.
    ///
    /// This is the presentation margin — art reads better when it overshoots
    /// the damage volume slightly. It is a separate, named step precisely
    /// because the scalar it replaced (`SLASH_EFFECT_SCALE = 2.0`, applied to
    /// the longer side of a bounding box) was indistinguishable from the shape
    /// derivation itself.
    pub fn scaled(&self, factor: f32) -> Self {
        let factor = factor.max(0.0);
        match *self {
            SwingShape::Sweep {
                origin,
                dir,
                length,
                near_half,
                far_half,
            } => SwingShape::Sweep {
                origin,
                dir,
                length: length * factor,
                near_half: near_half * factor,
                far_half: far_half * factor,
            },
            SwingShape::Radial { center, radius } => SwingShape::Radial {
                center,
                radius: radius * factor,
            },
        }
    }
}

/// Fraction of the swing's length treated as "the near end" when measuring the
/// width a sweep starts at. The near edge of an authored cone is a single
/// segment, so any small window recovers the authored `near_w` exactly; the
/// window exists so a hull with a slightly bevelled root does not report a
/// near width of zero.
const NEAR_BAND: f32 = 0.15;

/// A swing axis shorter than this is not a direction, it is noise.
const MIN_AXIS_LEN: f32 = 1.0e-3;

impl CombatVolume {
    /// Project this combat volume into a renderable swing shape rooted at
    /// attacker position `from`. Circles remain radial; other volumes use the
    /// attacker-to-centroid axis to identify the rooted end.
    ///
    /// This axis is an approximation: vertically offset authored sweeps can tilt
    /// toward their centroid. The exact fix is to retain the authored sweep axis
    /// in `CombatVolume` rather than infer it from geometry.
    pub fn swing_shape(&self, from: Vec2) -> SwingShape {
        if let CombatVolume::Circle { center, radius } = self {
            return SwingShape::Radial {
                center: *center,
                radius: Vec2::splat(*radius),
            };
        }
        let axis = self.center() - from;
        let Some(dir) = normalized(axis) else {
            // Degenerate: the volume is centred on the attacker, so there is no
            // outward direction to sweep along. Read it as radial rather than
            // inventing a facing — this is the aerial spin's shape anyway.
            let bounds = self.bounds();
            return SwingShape::Radial {
                center: bounds.center(),
                radius: bounds.half_size(),
            };
        };
        let perp = Vec2::new(-dir.y, dir.x);
        let points = self.outline();
        // Station each point along the axis and measure how far it lies off it.
        let mut t_min = f32::INFINITY;
        let mut t_max = f32::NEG_INFINITY;
        for p in &points {
            let t = (*p - from).dot(dir);
            t_min = t_min.min(t);
            t_max = t_max.max(t);
        }
        let length = (t_max - t_min).max(0.0);
        let near_cut = t_min + length * NEAR_BAND;
        let mut near_half: f32 = 0.0;
        let mut far_half: f32 = 0.0;
        for p in &points {
            let d = *p - from;
            let off = d.dot(perp).abs();
            far_half = far_half.max(off);
            if d.dot(dir) <= near_cut {
                near_half = near_half.max(off);
            }
        }
        SwingShape::Sweep {
            origin: from + dir * t_min,
            dir,
            length,
            near_half,
            // A hull whose widest station IS its near edge (a retreating wedge)
            // would otherwise report a far end wider than the shape.
            far_half: far_half.max(near_half),
        }
    }

    /// World-space outline points — the corners a projection measures against.
    fn outline(&self) -> Vec<Vec2> {
        match self {
            CombatVolume::Aabb(a) => {
                let (c, h) = (a.center(), a.half_size());
                vec![
                    Vec2::new(c.x - h.x, c.y - h.y),
                    Vec2::new(c.x + h.x, c.y - h.y),
                    Vec2::new(c.x + h.x, c.y + h.y),
                    Vec2::new(c.x - h.x, c.y + h.y),
                ]
            }
            CombatVolume::Obb {
                center,
                half,
                rotation,
            } => crate::combat_volume::obb_corners(*center, *half, *rotation),
            CombatVolume::Convex { points, .. } => points.clone(),
            // Unreachable in practice; the coarse box is the honest answer.
            CombatVolume::Circle { center, radius } => {
                let h = Vec2::splat(*radius);
                vec![
                    Vec2::new(center.x - h.x, center.y - h.y),
                    Vec2::new(center.x + h.x, center.y - h.y),
                    Vec2::new(center.x + h.x, center.y + h.y),
                    Vec2::new(center.x - h.x, center.y + h.y),
                ]
            }
        }
    }
}

fn normalized(v: Vec2) -> Option<Vec2> {
    let len = v.length();
    (len > MIN_AXIS_LEN).then(|| v / len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Aabb;

    /// The protagonist's `attack_side` hull, in the body-local frame the
    /// manifest authors it in (feet at the origin, +x forward, y down), scaled
    /// out of frame pixels. Reproduced from
    /// `robot_side.py::attack_hitboxes`'s `cone(...)` call so the projection is
    /// tested against the shape an artist actually wrote.
    fn attack_side_hull() -> Vec<Vec2> {
        vec![
            Vec2::new(-9.18, -13.68),
            Vec2::new(162.34, 37.52),
            Vec2::new(193.21, -41.84),
            Vec2::new(162.34, -121.20),
            Vec2::new(-9.18, -70.00),
        ]
    }

    #[test]
    fn a_cone_projects_back_to_the_numbers_it_was_authored_from() {
        let hull = CombatVolume::convex(attack_side_hull());
        // The attacker is at the body, behind the hull's near edge.
        let SwingShape::Sweep {
            length,
            near_half,
            far_half,
            dir,
            ..
        } = hull.swing_shape(Vec2::new(-9.18, -41.84))
        else {
            panic!("a forward blade arc is a sweep, not a radial swing");
        };
        // The authored cone: origin at the near edge, length w*1.34 plus an 18%
        // tip, near half-width h*0.22, far half-width h*0.62 — in the same
        // pixel units the hull is written in.
        assert!(dir.x > 0.99, "swing points forward, got {dir:?}");
        assert!(
            (length - 202.39).abs() < 1.0,
            "near edge to tip is the authored length, got {length}"
        );
        assert!(
            (near_half - 28.16).abs() < 1.0,
            "near half-width is the authored near_w, got {near_half}"
        );
        assert!(
            (far_half - 79.36).abs() < 1.0,
            "far half-width is the authored far_w, NOT the tip's zero width, \
             got {far_half}"
        );
    }

    /// The cue this type replaces, reproduced exactly: `slash_effect_size` took
    /// the volume's AABB, kept its LONGER side, doubled it, and the renderer
    /// splatted that into a square.
    fn legacy_square_side(volume: &CombatVolume) -> f32 {
        ((volume.bounds().half_size() * 2.0).max_element() * 2.0).max(24.0)
    }

    #[test]
    fn the_projection_is_tighter_than_the_square_it_replaces() {
        let hull = CombatVolume::convex(attack_side_hull());
        let half = hull.swing_shape(Vec2::new(-9.18, -41.84)).oriented_bounds();
        let old_side = legacy_square_side(&hull);
        let old_area = old_side * old_side;
        let new_area = (half.x * 2.0) * (half.y * 2.0);
        assert!(
            new_area < old_area * 0.25,
            "the drawn quad should be a small fraction of the square it \
             replaces: {new_area} vs {old_area}"
        );
    }

    #[test]
    fn the_quad_takes_the_swings_height_not_its_length() {
        // The square could only ever be as tall as the swing is LONG.
        let hull = CombatVolume::convex(attack_side_hull());
        let half = hull.swing_shape(Vec2::new(-9.18, -41.84)).oriented_bounds();
        assert!(
            half.y * 2.0 < 170.0,
            "quad height tracks the cone's flare, got {}",
            half.y * 2.0
        );
        assert!(
            half.y < half.x,
            "this swing reaches further than it is tall, and the quad should \
             say so: {half:?}"
        );
    }

    #[test]
    fn a_rotated_swing_keeps_its_extent_while_its_bounding_box_inflates() {
        // Under non-screen-down gravity the strike hull rotates. An
        // axis-aligned box grows for no reason; the oriented projection does
        // not, which is the whole argument for carrying an axis at all.
        let upright = CombatVolume::convex(attack_side_hull());
        let from = Vec2::new(-9.18, -41.84);
        let (sin, cos) = std::f32::consts::FRAC_PI_4.sin_cos();
        let rot = |p: Vec2| {
            let d = p - from;
            from + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
        };
        let tilted = CombatVolume::convex(attack_side_hull().into_iter().map(rot).collect());

        let upright_half = upright.swing_shape(from).oriented_bounds();
        let tilted_half = tilted.swing_shape(from).oriented_bounds();
        assert!(
            (upright_half - tilted_half).length() < 1.0,
            "the swing is the same swing: {upright_half:?} vs {tilted_half:?}"
        );
        // The axis-aligned box it replaces does not have that property: the
        // same swing, tilted, covers 20% more area for no reason. (Its LONGER
        // side happens to shrink for this particular wedge — which is its own
        // argument against deriving a size from one, since the number moves
        // with the gravity frame in whichever direction the shape happens to
        // favour.)
        let area = |v: &CombatVolume| {
            let s = v.bounds().half_size() * 2.0;
            s.x * s.y
        };
        assert!(
            area(&tilted) > area(&upright) * 1.15,
            "the bounding box it replaces DOES inflate when the swing tilts: \
             {} -> {}",
            area(&upright),
            area(&tilted)
        );
    }

    #[test]
    fn a_plain_box_swing_reads_as_a_straight_poke() {
        // The prefab fallback rect: 36 wide, 32 tall, centred 21.6 ahead.
        let vol = CombatVolume::aabb(Aabb::new(Vec2::new(21.6, 0.0), Vec2::new(18.0, 16.0)));
        let SwingShape::Sweep {
            near_half,
            far_half,
            length,
            ..
        } = vol.swing_shape(Vec2::ZERO)
        else {
            panic!("a forward box is a sweep");
        };
        assert!((length - 36.0).abs() < 0.01, "spans the box, got {length}");
        assert!(
            (near_half - far_half).abs() < 0.01,
            "a box does not flare: {near_half} vs {far_half}"
        );
    }

    #[test]
    fn a_circle_is_radial_and_a_body_centred_volume_is_too() {
        let circle = CombatVolume::circle(Vec2::new(4.0, 0.0), 20.0);
        assert!(matches!(
            circle.swing_shape(Vec2::ZERO),
            SwingShape::Radial { .. }
        ));
        // The aerial spin: a hull centred ON the attacker has no outward axis.
        let ring = CombatVolume::convex(vec![
            Vec2::new(30.0, 0.0),
            Vec2::new(0.0, -30.0),
            Vec2::new(-30.0, 0.0),
            Vec2::new(0.0, 30.0),
        ]);
        assert!(matches!(
            ring.swing_shape(Vec2::ZERO),
            SwingShape::Radial { .. }
        ));
    }

    #[test]
    fn scaling_grows_the_extent_and_leaves_the_axis_alone() {
        let vol = CombatVolume::aabb(Aabb::new(Vec2::new(20.0, 0.0), Vec2::new(10.0, 5.0)));
        let shape = vol.swing_shape(Vec2::ZERO);
        let grown = shape.scaled(2.0);
        assert_eq!(shape.rotation(), grown.rotation(), "rotation is untouched");
        assert_eq!(
            grown.oriented_bounds(),
            shape.oriented_bounds() * 2.0,
            "extent doubles in both axes"
        );
    }
}
