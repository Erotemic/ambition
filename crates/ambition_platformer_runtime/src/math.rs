//! Pure portal-map vector math for platformer mechanics.
//!
//! These are the orientation-between-two-normals transforms at the heart of the
//! portal system, factored out as plain [`Vec2`] math with no ECS, no Ambition
//! content, and no AABB types. They are portal-shaped in origin but are reusable
//! reflect/rotate-between-two-normals primitives, so the sandbox's
//! `portal_pieces` (AABB / piece geometry) and `transit` (velocity transit) both
//! build on them while this crate stays content-free.
//!
//! Restricted to **axis-aligned portals** (normal is ±x or ±y) in practice, per
//! the portal design note, though the math here is general.

use bevy::math::Vec2;

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

/// The canonical along-surface **tangent** for a portal normal — the "second
/// normal" that fixes which way is "along" the doorway: the normal rotated +90°.
/// (floor → +x, ceiling → -x, right-wall → -y, left-wall → +y.) The portal map
/// preserves the tangent component, so it does NOT mirror your along-surface
/// direction the way a bare rotation would.
pub fn portal_tangent(normal: Vec2) -> Vec2 {
    Vec2::new(-normal.y, normal.x)
}

/// The IDEAL portal map for a free vector (velocity / spatial offset), given a
/// consistent [`portal_tangent`]: the component going INTO the entry emerges OUT
/// of the exit, and the along-surface (tangent) component is carried straight
/// over. So falling right-and-down through two floor portals comes out
/// right-and-up — you keep your horizontal direction — instead of the bare
/// rotation's left-and-up mirror. This is one orthogonal map shared by velocity,
/// position, AABB, input, and rays so they always agree.
pub fn portal_map_vec(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    let t_in = portal_tangent(n_in);
    let t_out = portal_tangent(n_out);
    let into = -v.dot(n_in); // speed/offset INTO the entry → OUT of the exit
    let along = v.dot(t_in); // along-surface component, preserved
    into * n_out + along * t_out
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
}
