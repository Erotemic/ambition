//! `frame` — the engine-level aperture vocabulary (docs/concepts/movement-collision.md).
//!
//! A portal endpoint IS a frame: an origin on a host face, an outward normal,
//! and (for moving portals, CC6) the aperture's own velocity. The pair map
//! between two frames is pure geometry — which is why it lives HERE and not in
//! the portal gameplay crate: `cast` needs it to offer portal-aware casts
//! without inverting layers (engine_core owns aperture GEOMETRY; the portal
//! crate owns portal GAMEPLAY — placement, channels, cooldowns, carve policy,
//! the transit machine).
//!
//! Conventions (pinned; see the planning doc before "fixing" any of these):
//! - `normal` is unit and points OUT of the host wall into the room.
//! - The tangent is DERIVED, never stored: `normal` rotated +90°
//!   (`(-n.y, n.x)`). Handedness is thereby fixed; an inconsistent frame is
//!   unrepresentable.
//! - Local coordinates are `(along, front)`: `along` = tangent component,
//!   `front` = normal component; `front > 0` is the room side.
//! - The pair map ALWAYS flips depth (`front' = -front`: depth sunk INTO the
//!   entry emerges OUT of the exit). The along component depends on the
//!   [`MapConvention`]: `Reflection` (det −1, the game-wide default) preserves
//!   it; `Rotation` (det +1) negates it. At THIS layer the convention is an
//!   explicit parameter — the game-wide flag lives with the portal wrappers
//!   (`ambition_platformer2d::platformer::math::portal_map_vec` dispatches on it).

use crate::Vec2;

/// The along-surface tangent for an aperture normal: `normal` rotated +90°
/// (floor → +x, ceiling → −x, right-wall → −y, left-wall → +y). The ONE
/// handedness definition; every tangent in the engine derives from it.
pub fn tangent_of(normal: Vec2) -> Vec2 {
    Vec2::new(-normal.y, normal.x)
}

/// A portal endpoint IS a frame. World-frame fields (AJ13 naming).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalFrame {
    /// World-space center of the doorway, on the host face.
    pub origin: Vec2,
    /// Unit outward normal, pointing into the room.
    pub normal: Vec2,
    /// The aperture's own motion: the velocity (px/s) of its point at
    /// `pivot`. ZERO for static portals; a moving host publishes its
    /// authoritative mover velocity here (CC6) — never a finite difference of
    /// positions.
    pub velocity: Vec2,
    /// How fast it turns about `pivot` (rad/s; `+` turns `+x` toward `+y`,
    /// as [`crate::SurfaceChain::carried`] turns). Zero unless its host turns
    /// (a plank platform).
    pub spin: f32,
    /// The point it turns about. Any point when `spin` is zero.
    pub pivot: Vec2,
}

impl PortalFrame {
    /// A static frame (no motion) — the common constructor today.
    pub fn fixed(origin: Vec2, normal: Vec2) -> Self {
        Self {
            origin,
            normal,
            velocity: Vec2::ZERO,
            spin: 0.0,
            pivot: origin,
        }
    }

    /// The velocity of the aperture's surface at `p`: its motion and its turn
    /// (`velocity + spin × (p − pivot)`). What a body crossing at `p` moves
    /// relative to.
    pub fn velocity_at(&self, p: Vec2) -> Vec2 {
        let r = p - self.pivot;
        self.velocity + self.spin * Vec2::new(-r.y, r.x)
    }

    /// The derived along-surface tangent ([`tangent_of`]).
    pub fn tangent(&self) -> Vec2 {
        tangent_of(self.normal)
    }

    /// Local coordinates of `p`: `(along, front)` — tangent and normal
    /// components of `p − origin`. `front > 0` = room side (this IS the
    /// portal system's `front_distance`).
    pub fn to_local(&self, p: Vec2) -> Vec2 {
        let d = p - self.origin;
        Vec2::new(d.dot(self.tangent()), d.dot(self.normal))
    }

    /// Inverse of [`Self::to_local`].
    pub fn from_local(&self, l: Vec2) -> Vec2 {
        self.origin + l.x * self.tangent() + l.y * self.normal
    }
}

/// Frame + opening extent: THE aperture vocabulary portal-aware casts and the
/// piece/straddle geometry consume. Carve depth / capture margins are NOT
/// here — they are portal-crate gameplay policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalAperture {
    pub frame: PortalFrame,
    /// Opening half-extent along [`PortalFrame::tangent`].
    pub half_length: f32,
}

/// A rectangle turned to a portal's face: `half_along` along the face's
/// tangent and `half_out` along its normal, about `center`. A portal on a
/// slope has its capture, its approach and its carve as one of these; for a
/// floor, wall or ceiling portal it is an axis-aligned box, and every test
/// here gives the box's answer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceRect {
    pub center: Vec2,
    /// The face's unit outward normal.
    pub normal: Vec2,
    pub half_along: f32,
    pub half_out: f32,
}

impl FaceRect {
    pub fn tangent(&self) -> Vec2 {
        tangent_of(self.normal)
    }

    /// The axis-aligned box that bounds it (itself, for a cardinal face).
    pub fn bounds(&self) -> ambition_geometry::geometry::Aabb {
        let (t, n) = (self.tangent(), self.normal);
        let half = Vec2::new(
            t.x.abs() * self.half_along + n.x.abs() * self.half_out,
            t.y.abs() * self.half_along + n.y.abs() * self.half_out,
        );
        ambition_geometry::geometry::Aabb::new(self.center, half)
    }

    /// Do it and `aabb` overlap by more than a touch? Separating axes: the
    /// box's two and the rectangle's two. For a cardinal face this is the
    /// boxes' `strict_intersects`.
    pub fn strict_intersects(&self, aabb: ambition_geometry::geometry::Aabb) -> bool {
        use ambition_geometry::geometry::AabbExt;
        let (c, h) = (aabb.center(), aabb.half_size());
        let d = c - self.center;
        let (t, n) = (self.tangent(), self.normal);
        let apart = |axis: Vec2, own: f32| {
            let box_reach = h.x * axis.x.abs() + h.y * axis.y.abs();
            d.dot(axis).abs() >= box_reach + own
        };
        let rect_reach = |axis: Vec2| self.half_along * t.dot(axis).abs() + self.half_out * n.dot(axis).abs();
        !(apart(Vec2::X, rect_reach(Vec2::X))
            || apart(Vec2::Y, rect_reach(Vec2::Y))
            || apart(t, self.half_along)
            || apart(n, self.half_out))
    }

    /// The part of the segment `a..b` inside it, as the interval of the
    /// segment's parameter (`0` at `a`, `1` at `b`), if any.
    pub fn clip_segment(&self, a: Vec2, b: Vec2) -> Option<(f32, f32)> {
        let (t, n) = (self.tangent(), self.normal);
        let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
        for (axis, half) in [(t, self.half_along), (n, self.half_out)] {
            let p0 = (a - self.center).dot(axis);
            let dp = (b - a).dot(axis);
            if dp.abs() < 1.0e-9 {
                if p0.abs() > half {
                    return None;
                }
                continue;
            }
            let (t0, t1) = ((-half - p0) / dp, (half - p0) / dp);
            lo = lo.max(t0.min(t1));
            hi = hi.min(t0.max(t1));
        }
        (hi > lo).then_some((lo, hi))
    }
}

/// Which linear map glues an aperture pair. The two differ ONLY in the sign
/// of the along-surface term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum MapConvention {
    /// det −1: along-surface component PRESERVED — falling right through two
    /// floor portals exits still moving right. The DEFAULT, for the composition
    /// that states none.
    #[default]
    Reflection,
    /// det +1: the bare rotation taking `−n_in` onto `n_out`; opposite-facing
    /// thin-wall pairs become the identity map.
    Rotation,
}

/// The pair map for a free vector, in normal form — THE single
/// implementation (`portal_map_vec_reflection` / `_rotation` delegate here).
/// For cardinal normals every product is by 0/±1, so this is bit-identical
/// to the historical arithmetic (the CC5 parity gate leans on that).
pub fn map_vec_between(v: Vec2, n_in: Vec2, n_out: Vec2, convention: MapConvention) -> Vec2 {
    let into = -v.dot(n_in);
    let along = v.dot(tangent_of(n_in));
    match convention {
        MapConvention::Reflection => into * n_out + along * tangent_of(n_out),
        MapConvention::Rotation => into * n_out - along * tangent_of(n_out),
    }
}

/// The pair map for a free vector between two frames.
pub fn map_vec(a: &PortalFrame, b: &PortalFrame, convention: MapConvention, v: Vec2) -> Vec2 {
    map_vec_between(v, a.normal, b.normal, convention)
}

/// Map a world point near frame `a` to the corresponding point near frame
/// `b`: depth sunk INTO the entry becomes depth emerging OUT of the exit
/// (`a.origin` maps to `b.origin`); the along-surface offset follows the
/// convention.
pub fn map_point(a: &PortalFrame, b: &PortalFrame, convention: MapConvention, p: Vec2) -> Vec2 {
    b.origin + map_vec_between(p - a.origin, a.normal, b.normal, convention)
}

/// THE moving-portal rule (CC6): a body crossing at `p` keeps its velocity
/// relative to the surface it crosses. `v_out = R(v − a.velocity_at(p)) +
/// b.velocity_at(p′)`, where `p′` is `p` mapped to `b`, so a portal that
/// moves or turns gives what crosses it its surface's motion at that point.
/// For static frames this is exactly [`map_vec`].
pub fn map_velocity(a: &PortalFrame, b: &PortalFrame, convention: MapConvention, p: Vec2, v: Vec2) -> Vec2 {
    map_vec(a, b, convention, v - a.velocity_at(p)) + b.velocity_at(map_point(a, b, convention, p))
}

/// A rigid map of the plane: an orthogonal linear part (det ±1: a turn, or a
/// turn and a mirror) and a translation. A portal pair's map is one
/// ([`Iso2::through`]); a body carried through several pairs is in the chart
/// their composition gives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Iso2 {
    pub linear: bevy_math::Mat2,
    pub translation: Vec2,
}

impl Iso2 {
    pub const IDENTITY: Self = Self { linear: bevy_math::Mat2::IDENTITY, translation: Vec2::ZERO };

    /// The pair map from `enter` to `exit` ([`map_point`] and [`map_vec`]).
    pub fn through(enter: &PortalFrame, exit: &PortalFrame, convention: MapConvention) -> Self {
        let linear = bevy_math::Mat2::from_cols(
            map_vec(enter, exit, convention, Vec2::X),
            map_vec(enter, exit, convention, Vec2::Y),
        );
        Self { linear, translation: exit.origin - linear * enter.origin }
    }

    pub fn point(&self, p: Vec2) -> Vec2 {
        self.linear * p + self.translation
    }

    pub fn vec(&self, v: Vec2) -> Vec2 {
        self.linear * v
    }

    /// `self`, then `next`.
    pub fn then(&self, next: &Iso2) -> Self {
        Self { linear: next.linear * self.linear, translation: next.point(self.translation) }
    }

    /// The map back. The linear part is orthogonal, so its inverse is its
    /// transpose.
    pub fn inverse(&self) -> Self {
        let linear = self.linear.transpose();
        Self { linear, translation: -(linear * self.translation) }
    }

    /// +1 for a turn, −1 for a mirror.
    pub fn det(&self) -> f32 {
        self.linear.determinant()
    }

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }
}

pub mod thread;

#[cfg(test)]
mod tests;
