//! Pure portal-map vector math for platformer mechanics.
//!
//! These are the orientation-between-two-normals transforms at the heart of the
//! portal system, factored out as plain [`Vec2`] math with no ECS, no Ambition
//! content, and no AABB types. They are portal-shaped in origin but are reusable
//! reflect/rotate-between-two-normals primitives, so the sandbox's
//! `portal_pieces` (AABB / piece geometry) and `transit` (velocity transit) both
//! build on them while this crate stays content-free.
//!
//! Restricted to axis-aligned portals (normal is ±x or ±y) in practice, per
//! the portal design note, though the math here is general.

use bevy::math::Vec2;

pub use ambition_platformer2d_core::frame::MapConvention;

/// The rotation `(cos, sin)` that maps the "into the entry portal" direction
/// (`-n_in`) onto the "out of the exit portal" direction (`n_out`). This is the
/// single rotation every portal transform (velocity, point, AABB) shares, so
/// position and momentum always turn through the pair consistently.
pub fn portal_rotation(n_in: Vec2, n_out: Vec2) -> (f32, f32) {
    let u = -n_in;
    let cos = u.dot(n_out);
    let sin = u.x * n_out.y - u.y * n_out.x; // 2D cross (z component)
    (cos, sin)
}

/// Apply a `(cos, sin)` rotation to a vector.
pub fn rotate(v: Vec2, cs: (f32, f32)) -> Vec2 {
    let (c, s) = cs;
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// The canonical along-surface tangent for a portal normal — the "second
/// normal" that fixes which way is "along" the doorway: the normal rotated +90°.
/// (floor → +x, ceiling → -x, right-wall → -y, left-wall → +y.) The portal map
/// preserves the tangent component, so it does NOT mirror your along-surface
/// direction the way a bare rotation would. Delegates to the ONE handedness
/// definition, [`ambition_platformer2d_core::frame::tangent_of`] (CC5).
pub fn portal_tangent(normal: Vec2) -> Vec2 {
    ambition_platformer2d_core::frame::tangent_of(normal)
}

/// The portal map for a free vector (velocity / spatial offset), under an
/// explicitly stated `convention`: the component going INTO the entry emerges
/// OUT of the exit, and the along-surface (tangent) component is carried over
/// (reflection) or flipped (rotation). One orthogonal map shared by velocity,
/// position, AABB, input and rays, so they always agree.
///
/// ⛔⛔ THE CONVENTION USED TO BE A `static AtomicBool`, and a process global is
/// not a world's physics — it is every world's physics. Two providers in one
/// process could not disagree, load order decided who won, and a rollback could
/// not rewind a convention the inspector changed mid-session because a static is
/// not rollback state. It is a parameter now, resolved from `PortalTuning` at
/// the system boundary and threaded, so the value travels with the thing it
/// governs.
pub fn portal_map_vec(v: Vec2, n_in: Vec2, n_out: Vec2, convention: MapConvention) -> Vec2 {
    ambition_platformer2d_core::frame::map_vec_between(v, n_in, n_out, convention)
}

/// Tangent-reflection map (det −1, the default): along-surface component
/// PRESERVED. Floor↔floor keeps horizontal direction; opposite-wall / thin-wall
/// pairs vertically FLIP. Pure — does not read the global convention.
/// Delegates to the ONE implementation
/// ([`ambition_platformer2d_core::frame::map_vec_between`], CC5).
pub fn portal_map_vec_reflection(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    ambition_platformer2d_core::frame::map_vec_between(
        v,
        n_in,
        n_out,
        ambition_platformer2d_core::frame::MapConvention::Reflection,
    )
}

/// Rotation map (det +1): the bare rotation taking `−n_in` onto `n_out`;
/// along-surface component FLIPPED vs reflection. Opposite-wall / thin-wall
/// pairs become the IDENTITY (a door that looks "almost normal" — the far side
/// just shifted by the portals' displacement = the wall thickness); floor↔floor
/// reverses horizontal (a true 180° turn). Pure — does not read the global.
/// Delegates to the ONE implementation
/// ([`ambition_platformer2d_core::frame::map_vec_between`], CC5).
pub fn portal_map_vec_rotation(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    ambition_platformer2d_core::frame::map_vec_between(
        v,
        n_in,
        n_out,
        ambition_platformer2d_core::frame::MapConvention::Rotation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    #[test]
    fn velocity_rotation_matches_existing_convention() {
        // Falling down (+y) into a floor portal, exit a left-facing wall → move
        // left (-x), same speed.
        let cs = portal_rotation(Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0));
        let out = rotate(Vec2::new(0.0, 100.0), cs);
        assert!(
            (out.x + 100.0).abs() < 1e-2 && out.y.abs() < 1e-2,
            "got {out:?}"
        );
    }

    #[test]
    fn transit_roll_angles() {
        // Sanity: rotation magnitude for floor↔floor is 180°, floor↔wall 90°.
        let (c, s) = portal_rotation(Vec2::new(0.0, -1.0), Vec2::new(0.0, -1.0));
        assert!((s.atan2(c).abs() - PI).abs() < 1e-4);
        let (c, s) = portal_rotation(Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0));
        assert!((s.atan2(c).abs() - FRAC_PI_2).abs() < 1e-4);
    }

    /// The two conventions differ only by the along-surface sign, and the
    /// thin-wall / facing case (opposite normals) is the IDENTITY under
    /// rotation (door "looks normal") but a vertical FLIP under reflection.
    /// Pure variants — no global, so no test-ordering race.
    #[test]
    fn reflection_vs_rotation_on_opposite_walls_and_floors() {
        let left = Vec2::new(-1.0, 0.0);
        let right = Vec2::new(1.0, 0.0);
        let v = Vec2::new(3.0, 7.0);
        // Opposite walls (thin-wall door): reflection flips y, rotation is id.
        assert!((portal_map_vec_reflection(v, left, right) - Vec2::new(3.0, -7.0)).length() < 1e-4);
        assert!((portal_map_vec_rotation(v, left, right) - Vec2::new(3.0, 7.0)).length() < 1e-4);
        // Floor↔floor: reflection keeps horizontal, rotation reverses it (180°).
        let up = Vec2::new(0.0, -1.0);
        assert!((portal_map_vec_reflection(v, up, up) - Vec2::new(3.0, -7.0)).length() < 1e-4);
        assert!((portal_map_vec_rotation(v, up, up) - Vec2::new(-3.0, -7.0)).length() < 1e-4);
        // And the dispatching entry point says which it is being asked for.
        assert!(
            (portal_map_vec(v, left, right, MapConvention::Reflection) - Vec2::new(3.0, -7.0))
                .length()
                < 1e-4
        );
        assert!(
            (portal_map_vec(v, left, right, MapConvention::Rotation) - Vec2::new(3.0, 7.0))
                .length()
                < 1e-4
        );
    }
}
