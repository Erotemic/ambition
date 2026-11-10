//! Aperture geometry for portal placement, transit and host carving.
//! Material partitions belong to `ambition_platformer2d_core::frame::thread`.

use ambition_platformer2d_core::{self as ae, AabbExt};
use bevy::math::Vec2;

// The frame and aperture types live in `ambition_platformer2d_core::frame`.
pub use ambition_platformer2d_core::frame::{PortalAperture, PortalFrame};

// The portal-map vector math lives in `ambition_platformer2d_shared_tangle`.
// Re-exported so users can keep referencing `crate::pieces::{portal_rotation,
// rotate, portal_tangent, portal_map_vec}`.
pub use ambition_platformer2d_shared_tangle::math::{
    portal_map_vec, portal_map_vec_reflection, portal_map_vec_rotation, portal_rotation,
    portal_tangent, rotate, MapConvention,
};

/// Map a world point near `enter` to the corresponding point near `exit`: the
/// depth a point has sunk *into* the entry wall becomes the depth it emerges
/// *out* of the exit portal (so `enter.origin` maps to `exit.origin`), and its
/// along-surface offset follows the stated `convention` (see
/// [`portal_map_vec`]).
pub fn map_point(
    p: Vec2,
    enter: &PortalFrame,
    exit: &PortalFrame,
    convention: MapConvention,
) -> Vec2 {
    exit.origin + portal_map_vec(p - enter.origin, enter.normal, exit.normal, convention)
}

/// Map an axis-aligned AABB through the portal pair. The map is axis-aligned for
/// axis-aligned portals (a 90° turn swaps the half-extents), so the image stays
/// an axis-aligned AABB.
pub fn map_aabb(
    b: ae::Aabb,
    enter: &PortalFrame,
    exit: &PortalFrame,
    convention: MapConvention,
) -> ae::Aabb {
    let center = map_point(b.center(), enter, exit, convention);
    // Transform the half-extent through the (axis-aligned) map: each output axis
    // gets |contribution| from each input axis.
    let col_x = portal_map_vec(Vec2::new(1.0, 0.0), enter.normal, exit.normal, convention);
    let col_y = portal_map_vec(Vec2::new(0.0, 1.0), enter.normal, exit.normal, convention);
    let h = b.half_size();
    let half = Vec2::new(
        col_x.x.abs() * h.x + col_y.x.abs() * h.y,
        col_x.y.abs() * h.x + col_y.y.abs() * h.y,
    );
    ae::Aabb::new(center, half)
}

/// The box that bounds the part of `b` on the `dir` side of the line through
/// `point` (`dir` is the line's normal): exact for a cardinal `dir`.
pub fn clip_halfspace(b: ae::Aabb, point: Vec2, dir: Vec2) -> Option<ae::Aabb> {
    bound_clipped(b, &[(point, dir)])
}

/// The box that bounds the part of `b` inside the half planes `(point, dir)`
/// (each keeps `(p - point) . dir >= 0`): the box's corners clipped as a
/// polygon, then bounded.
fn bound_clipped(b: ae::Aabb, planes: &[(Vec2, Vec2)]) -> Option<ae::Aabb> {
    let mut poly = vec![
        Vec2::new(b.min.x, b.min.y),
        Vec2::new(b.max.x, b.min.y),
        Vec2::new(b.max.x, b.max.y),
        Vec2::new(b.min.x, b.max.y),
    ];
    for &(point, dir) in planes {
        let side = |p: Vec2| (p - point).dot(dir);
        let mut out = Vec::with_capacity(poly.len() + 2);
        for i in 0..poly.len() {
            let (a, c) = (poly[i], poly[(i + 1) % poly.len()]);
            let (sa, sc) = (side(a), side(c));
            if sa >= 0.0 {
                out.push(a);
            }
            if (sa >= 0.0) != (sc >= 0.0) {
                out.push(a + (c - a) * (sa / (sa - sc)));
            }
        }
        poly = out;
        if poly.is_empty() {
            return None;
        }
    }
    let (min, max) = poly.iter().fold((Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    ae::geometry::aabb_from_min_max(min.x, min.y, max.x, max.y)
}

/// Does `body` straddle `ap`'s plane within the opening? True when the plane
/// passes through the body's extent AND the body overlaps the aperture span,
/// for a plane of any angle.
pub fn straddles(body: ae::Aabb, ap: &PortalAperture) -> bool {
    let (n, t) = (ap.frame.normal, ap.frame.tangent());
    let d = body.center() - ap.frame.origin;
    let h = body.half_size();
    let reach = |axis: Vec2| h.x * axis.x.abs() + h.y * axis.y.abs();
    // The plane cuts the box, and the box reaches the opening along the face.
    d.dot(n).abs() < reach(n) && d.dot(t).abs() < reach(t) + ap.half_length
}

/// Signed distance of `point` from `frame`'s plane along its outward normal:
/// positive = in front (room side), negative = behind (into the wall). The
/// centroid transfer fires when this changes sign. Same as
/// `frame.to_local(point).y`.
pub fn front_distance(point: Vec2, frame: &PortalFrame) -> f32 {
    (point - frame.origin).dot(frame.normal)
}

// ---------------------------------------------------------------------------
// Host-surface carve: the floor / wall must become non-solid in the opening.
//
// The rectangle subtraction is `ae::geometry::subtract_aabb`. The hole size
// is below.

/// How deep (px) into the host surface a portal carves its doorway. Just past
/// a body's half-depth, so the body can sink to its centroid before the
/// transfer, but not through a thick wall.
pub const CARVE_DEPTH: f32 = 60.0;

/// Maximum host-surface offset used to admit a crossing from authored data.
pub const SURFACE_GRACE: f32 = 16.0;

/// The opening width along the surface, from the face into its host.
pub fn carve_hole(ap: &PortalAperture) -> ae::Aabb {
    carve_hole_with_depth(ap, f32::INFINITY)
}

/// The box a portal carves out of the boxes of its host:
/// [`carve_face_with_depth`]'s bounds, which is that face for a cardinal
/// portal.
pub fn carve_hole_with_depth(ap: &PortalAperture, host_depth: f32) -> ae::Aabb {
    carve_face_with_depth(ap, host_depth).bounds()
}

/// Cut from the face into its host, to at most [`CARVE_DEPTH`].
/// A solid in front of the face stays solid.
pub fn carve_face_with_depth(ap: &PortalAperture, host_depth: f32) -> ae::frame::FaceRect {
    let depth = CARVE_DEPTH.min(host_depth.max(0.0));
    let n = ap.frame.normal;
    ae::frame::FaceRect {
        center: ap.frame.origin - n * (depth * 0.5),
        normal: n,
        half_along: ap.half_length,
        half_out: depth * 0.5,
    }
}

#[cfg(test)]
mod tests;
