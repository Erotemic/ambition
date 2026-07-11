//! `frame` — the engine-level aperture vocabulary (collision-and-ccd.md §7, CC5).
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
//!   (`ambition_platformer_primitives::math::portal_map_vec` dispatches on it).

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
    /// The aperture's own motion (px/s). ZERO for static portals; a moving
    /// host publishes its authoritative mover velocity here (CC6) — never a
    /// finite difference of positions.
    pub velocity: Vec2,
}

impl PortalFrame {
    /// A static frame (velocity zero) — the common constructor today.
    pub fn fixed(origin: Vec2, normal: Vec2) -> Self {
        Self {
            origin,
            normal,
            velocity: Vec2::ZERO,
        }
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

/// Which linear map glues an aperture pair. The two differ ONLY in the sign
/// of the along-surface term.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapConvention {
    /// det −1 (the game-wide default): along-surface component PRESERVED —
    /// falling right through two floor portals exits still moving right.
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

/// Galilean velocity composition — THE moving-portal rule (CC6):
/// `v_out = R(v − a.velocity) + b.velocity`. For static frames (velocity
/// zero) this is exactly [`map_vec`].
pub fn map_velocity(a: &PortalFrame, b: &PortalFrame, convention: MapConvention, v: Vec2) -> Vec2 {
    map_vec(a, b, convention, v - a.velocity) + b.velocity
}

#[cfg(test)]
mod tests;
