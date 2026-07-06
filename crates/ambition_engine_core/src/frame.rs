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
mod tests {
    use super::*;

    const UP: Vec2 = Vec2::new(0.0, -1.0); // y-down world: a floor's outward normal
    const RIGHT: Vec2 = Vec2::new(1.0, 0.0);

    fn floor_at(x: f32, y: f32) -> PortalFrame {
        PortalFrame::fixed(Vec2::new(x, y), UP)
    }

    #[test]
    fn tangent_handedness_is_rot90_ccw() {
        // Pinned: floor (up-normal in y-down) → tangent +x is rot90 of (0,-1)
        // = (1, 0); right-wall normal (1,0) → (0,1) etc.
        assert_eq!(tangent_of(Vec2::new(0.0, -1.0)), Vec2::new(1.0, 0.0));
        assert_eq!(tangent_of(Vec2::new(1.0, 0.0)), Vec2::new(0.0, 1.0));
        assert_eq!(tangent_of(Vec2::new(0.0, 1.0)), Vec2::new(-1.0, 0.0));
        assert_eq!(tangent_of(Vec2::new(-1.0, 0.0)), Vec2::new(0.0, -1.0));
    }

    #[test]
    fn local_roundtrip_is_exact_for_cardinals_and_tight_for_angles() {
        let f = floor_at(100.0, 300.0);
        let p = Vec2::new(112.5, 297.25);
        assert_eq!(f.from_local(f.to_local(p)), p);

        let angled = PortalFrame::fixed(
            Vec2::new(40.0, 8.0),
            Vec2::new(
                std::f32::consts::FRAC_1_SQRT_2,
                -std::f32::consts::FRAC_1_SQRT_2,
            ),
        );
        let q = Vec2::new(37.0, 15.0);
        let rt = angled.from_local(angled.to_local(q));
        assert!(
            (rt - q).length() < 1e-4,
            "roundtrip drifted: {rt:?} vs {q:?}"
        );
    }

    #[test]
    fn front_is_positive_on_the_room_side() {
        let f = floor_at(100.0, 300.0);
        // Above the floor (smaller y in y-down) is the room side.
        assert!(f.to_local(Vec2::new(100.0, 290.0)).y > 0.0);
        assert!(f.to_local(Vec2::new(100.0, 310.0)).y < 0.0);
    }

    #[test]
    fn reflection_preserves_along_and_flips_depth() {
        // Two floor portals; a point sunk 5 into the entry, 3 along.
        let a = floor_at(0.0, 0.0);
        let b = floor_at(400.0, 0.0);
        let p = a.from_local(Vec2::new(3.0, -5.0));
        let mapped = map_point(&a, &b, MapConvention::Reflection, p);
        assert_eq!(b.to_local(mapped), Vec2::new(3.0, 5.0));
    }

    #[test]
    fn rotation_flips_along_and_depth() {
        let a = floor_at(0.0, 0.0);
        let b = floor_at(400.0, 0.0);
        let p = a.from_local(Vec2::new(3.0, -5.0));
        let mapped = map_point(&a, &b, MapConvention::Rotation, p);
        assert_eq!(b.to_local(mapped), Vec2::new(-3.0, 5.0));
    }

    #[test]
    fn map_point_inverts_with_endpoints_swapped() {
        for conv in [MapConvention::Reflection, MapConvention::Rotation] {
            let a = PortalFrame::fixed(Vec2::new(10.0, 20.0), RIGHT);
            let b = floor_at(300.0, 50.0);
            let p = Vec2::new(6.0, 22.0);
            let there = map_point(&a, &b, conv, p);
            let back = map_point(&b, &a, conv, there);
            assert!(
                (back - p).length() < 1e-4,
                "{conv:?}: map ∘ swapped-map must be identity, got {back:?} vs {p:?}"
            );
        }
    }

    #[test]
    fn map_vec_between_matches_the_historical_formulas() {
        // The exact op-shape the platformer math used; cardinal cases are
        // bit-identical by construction (0/±1 products).
        let v = Vec2::new(120.0, -340.0);
        let n_in = Vec2::new(0.0, -1.0);
        let n_out = Vec2::new(1.0, 0.0);
        let refl = {
            let into = -v.dot(n_in);
            let along = v.dot(tangent_of(n_in));
            into * n_out + along * tangent_of(n_out)
        };
        assert_eq!(
            map_vec_between(v, n_in, n_out, MapConvention::Reflection),
            refl
        );
        let rot = {
            let into = -v.dot(n_in);
            let along = v.dot(tangent_of(n_in));
            into * n_out - along * tangent_of(n_out)
        };
        assert_eq!(
            map_vec_between(v, n_in, n_out, MapConvention::Rotation),
            rot
        );
    }

    #[test]
    fn galilean_velocity_composition() {
        // Entry aperture moving down (+y), exit moving right: a body falling
        // WITH the entry (zero relative velocity) exits carried by the exit.
        let a = PortalFrame {
            origin: Vec2::ZERO,
            normal: UP,
            velocity: Vec2::new(0.0, 50.0),
        };
        let b = PortalFrame {
            origin: Vec2::new(500.0, 0.0),
            normal: UP,
            velocity: Vec2::new(30.0, 0.0),
        };
        let v_out = map_velocity(&a, &b, MapConvention::Reflection, Vec2::new(0.0, 50.0));
        assert_eq!(v_out, Vec2::new(30.0, 0.0));
        // Static frames: composition degenerates to the plain map.
        let sa = floor_at(0.0, 0.0);
        let sb = floor_at(500.0, 0.0);
        let v = Vec2::new(80.0, 200.0);
        assert_eq!(
            map_velocity(&sa, &sb, MapConvention::Reflection, v),
            map_vec(&sa, &sb, MapConvention::Reflection, v)
        );
    }
}
