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
use core::sync::atomic::{AtomicBool, Ordering};

/// Game-wide portal map **convention**, switchable at runtime so a host can A/B
/// the feel (and a game can pick the one it wants). It is a global because the
/// map is a property of the WORLD's portal physics, not of any one pair, and
/// every consumer — transit position, transit velocity, the collision pieces,
/// and the view cone — must agree. Default `false` = the historical convention.
///
/// - `false` — **tangent-reflection** (det −1). The along-surface component is
///   PRESERVED, so falling right-and-down through two floor portals comes out
///   right-and-up (horizontal direction kept). Two portals facing each other /
///   on opposite faces of a wall VERTICALLY FLIP what passes through (the
///   tangent maps onto the partner's oppositely-oriented tangent).
/// - `true` — **rotation** (det +1). The bare rotation taking `−n_in` onto
///   `n_out`; the along-surface component is FLIPPED relative to reflection. Two
///   facing/opposite-wall portals become a clean straight-through (no flip),
///   but floor↔floor now reverses horizontal direction (a true 180° turn).
///
/// The two differ by exactly the sign of the along-surface (tangent) term.
static PORTAL_MAP_ROTATION: AtomicBool = AtomicBool::new(false);

/// Set the game-wide portal map convention: `true` = rotation (det +1, no flip
/// for facing/opposite-wall pairs), `false` = tangent-reflection (det −1, the
/// default). See [`PORTAL_MAP_ROTATION`].
pub fn set_portal_map_rotation(rotation: bool) {
    PORTAL_MAP_ROTATION.store(rotation, Ordering::Relaxed);
}

/// Whether the rotation convention is currently active (see
/// [`set_portal_map_rotation`]).
pub fn portal_map_rotation() -> bool {
    PORTAL_MAP_ROTATION.load(Ordering::Relaxed)
}

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
    // Dispatch on the game-wide convention; the two differ ONLY by the sign of
    // the along-surface term (det −1 vs det +1).
    if portal_map_rotation() {
        portal_map_vec_rotation(v, n_in, n_out)
    } else {
        portal_map_vec_reflection(v, n_in, n_out)
    }
}

/// Tangent-**reflection** map (det −1, the default): along-surface component
/// PRESERVED. Floor↔floor keeps horizontal direction; opposite-wall / thin-wall
/// pairs vertically FLIP. Pure — does not read the global convention.
pub fn portal_map_vec_reflection(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    let into = -v.dot(n_in);
    let along = v.dot(portal_tangent(n_in));
    into * n_out + along * portal_tangent(n_out)
}

/// **Rotation** map (det +1): the bare rotation taking `−n_in` onto `n_out`;
/// along-surface component FLIPPED vs reflection. Opposite-wall / thin-wall
/// pairs become the IDENTITY (a door that looks "almost normal" — the far side
/// just shifted by the portals' displacement = the wall thickness); floor↔floor
/// reverses horizontal (a true 180° turn). Pure — does not read the global.
pub fn portal_map_vec_rotation(v: Vec2, n_in: Vec2, n_out: Vec2) -> Vec2 {
    let into = -v.dot(n_in);
    let along = v.dot(portal_tangent(n_in));
    into * n_out - along * portal_tangent(n_out)
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
        // The live dispatch defaults to reflection (global untouched).
        assert!(!portal_map_rotation());
        assert!((portal_map_vec(v, left, right) - Vec2::new(3.0, -7.0)).length() < 1e-4);
    }
}
