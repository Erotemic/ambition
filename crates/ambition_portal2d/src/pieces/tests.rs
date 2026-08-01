//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use std::f32::consts::{FRAC_PI_2, PI};

fn floor(pos: Vec2) -> PortalAperture {
    // Floor portal: normal points up (y-down world → up = -y).
    PortalAperture {
        frame: PortalFrame::fixed(pos, Vec2::new(0.0, -1.0)),
        half_length: 46.0,
    }
}
fn right_wall(pos: Vec2) -> PortalAperture {
    // Right wall: normal points left.
    PortalAperture {
        frame: PortalFrame::fixed(pos, Vec2::new(-1.0, 0.0)),
        half_length: 46.0,
    }
}

#[test]
fn map_point_turns_depth_into_emergence() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    // A point sunk 10px below the floor plane (into the wall, +y) emerges
    // 10px out in front of the right wall (left of it, -x).
    let p = map_point(Vec2::new(100.0, 310.0), &enter.frame, &exit.frame);
    assert!(
        (p.x - 390.0).abs() < 1e-3 && (p.y - 200.0).abs() < 1e-3,
        "got {p:?}"
    );
    // The portal centers map onto each other.
    let c = map_point(enter.frame.origin, &enter.frame, &exit.frame);
    assert!(
        (c - exit.frame.origin).length() < 1e-3,
        "centers map together, got {c:?}"
    );
}

#[test]
fn map_aabb_swaps_halves_on_ninety_degree_turn() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let b = ae::Aabb::new(Vec2::new(100.0, 305.0), Vec2::new(12.0, 6.0));
    let m = map_aabb(b, &enter.frame, &exit.frame);
    // 90° turn → width/height swap.
    assert!(
        (m.half_size().x - 6.0).abs() < 1e-3,
        "half x {:?}",
        m.half_size()
    );
    assert!(
        (m.half_size().y - 12.0).abs() < 1e-3,
        "half y {:?}",
        m.half_size()
    );
}

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
fn straddle_requires_plane_crossing_and_aperture_overlap() {
    let f = floor(Vec2::new(100.0, 300.0));
    // Body sitting ON the floor, feet just dipping below the plane, within
    // the 46px opening → straddles.
    let dipping = ae::Aabb::new(Vec2::new(100.0, 285.0), Vec2::new(12.0, 20.0));
    assert!(straddles(dipping, &f));
    // Body fully above the plane → no straddle.
    let above = ae::Aabb::new(Vec2::new(100.0, 260.0), Vec2::new(12.0, 20.0));
    assert!(!straddles(above, &f));
    // Body crossing the plane but laterally off the opening → no straddle.
    let off = ae::Aabb::new(Vec2::new(300.0, 300.0), Vec2::new(12.0, 20.0));
    assert!(!straddles(off, &f));
}

#[test]
fn feet_in_feet_out_decomposition() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    // Body centered just above the floor with its lower 10px sunk in.
    let body = ae::Aabb::new(Vec2::new(100.0, 290.0), Vec2::new(12.0, 20.0));
    let pieces = compute_body_pieces(body, Some((enter, exit)));
    // `here` is the part above the floor plane (y <= 300).
    assert!(
        pieces.here.max.y <= 300.0 + 1e-3,
        "here below plane: {:?}",
        pieces.here
    );
    // A through-piece exists, emerging from the exit (x < 400).
    let through = pieces.through.expect("feet should poke through");
    assert!(
        through.aabb.max.x <= 400.0 + 1e-3,
        "through in front of exit: {:?}",
        through.aabb
    );
    // The 90° turn maps the crossed DEPTH (10px below the floor) onto the
    // emergence depth out of the wall (10px along the exit normal, x), and
    // the body WIDTH (24px) onto the lateral extent (y).
    assert!(
        (through.aabb.max.x - through.aabb.min.x - 10.0).abs() < 1e-2,
        "depth {:?}",
        through.aabb
    );
    assert!(
        (through.aabb.max.y - through.aabb.min.y - 24.0).abs() < 1e-2,
        "lateral {:?}",
        through.aabb
    );
}

#[test]
fn no_straddle_returns_whole_body() {
    let enter = floor(Vec2::new(100.0, 300.0));
    let exit = right_wall(Vec2::new(400.0, 200.0));
    let body = ae::Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(12.0, 20.0));
    let pieces = compute_body_pieces(body, Some((enter, exit)));
    assert!(pieces.through.is_none());
    assert!((pieces.here.center() - body.center()).length() < 1e-3);
}

#[test]
fn front_distance_signs() {
    let f = floor(Vec2::new(100.0, 300.0));
    assert!(
        front_distance(Vec2::new(100.0, 280.0), &f.frame) > 0.0,
        "above floor = front"
    );
    assert!(
        front_distance(Vec2::new(100.0, 320.0), &f.frame) < 0.0,
        "below floor = behind"
    );
}

#[test]
fn carve_hole_reaches_through_the_surface_grace() {
    let f = floor(Vec2::new(100.0, 300.0));
    let hole = carve_hole(&f);
    // The hole reaches a little OUTWARD of the face (y < 300 by SURFACE_GRACE)
    // so it clears any thin solid lip left by a portal authored a few px off
    // the grid-snapped surface...
    assert!(
        (hole.min.y - (300.0 - SURFACE_GRACE)).abs() < 1e-3,
        "hole reaches SURFACE_GRACE outward: {hole:?}"
    );
    // ...and mostly INWARD (CARVE_DEPTH into the wall).
    assert!(
        (hole.max.y - (300.0 + CARVE_DEPTH)).abs() < 1e-3,
        "hole goes inward: {hole:?}"
    );
    // Opening width matches the aperture (2*46).
    assert!((hole.max.x - hole.min.x - 92.0).abs() < 1e-3, "{hole:?}");
}

/// The PURE vector layer is exact for ARBITRARY (non-cardinal) normals —
/// pinned at 45° so slanted portals "just work" at this layer when
/// authoring arrives. (The AABB piece/carve layer above it is documented
/// cardinal-only; see the module docs and the review report Q8.)
#[test]
fn slanted_normals_are_exact_in_the_vector_layer() {
    let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
    // A 45° ramp face (normal up-left) paired with an ordinary floor.
    let n_in = Vec2::new(-inv_sqrt2, -inv_sqrt2);
    let n_out = Vec2::new(0.0, -1.0);
    let enter = PortalFrame::fixed(Vec2::new(100.0, 300.0), n_in);
    let exit = PortalFrame::fixed(Vec2::new(500.0, 200.0), n_out);
    for v in [Vec2::new(3.0, 7.0), Vec2::new(-120.0, 45.0), Vec2::X] {
        for map in [portal_map_vec_reflection, portal_map_vec_rotation] {
            // Isometry: speed is exactly preserved at any angle.
            let out = map(v, n_in, n_out);
            assert!(
                (out.length() - v.length()).abs() < 1e-4,
                "speed preserved at 45°: {v:?} -> {out:?}"
            );
            // Into-component becomes out-component; tangent magnitude kept.
            assert!(
                ((-v.dot(n_in)) - out.dot(n_out)).abs() < 1e-4,
                "into->out at 45°: {v:?} -> {out:?}"
            );
        }
    }
    // map_point: depth behind the slanted entry becomes depth in front of
    // the exit, and mapping back through the swapped pair is the identity.
    for p in [
        enter.origin - n_in * 12.0,
        enter.origin + Vec2::new(10.0, -4.0),
        enter.origin + n_in * 3.0,
    ] {
        let depth_behind = -(p - enter.origin).dot(n_in);
        let mapped = map_point(p, &enter, &exit);
        assert!(
            (front_distance(mapped, &exit) - depth_behind).abs() < 1e-3,
            "depth->front at 45°: {p:?} -> {mapped:?}"
        );
        let back = map_point(mapped, &exit, &enter);
        assert!(
            (back - p).length() < 1e-3,
            "the 45° map inverts exactly: {p:?} -> {mapped:?} -> {back:?}"
        );
    }
}

#[test]
fn transit_roll_angles() {
    // Sanity: rotation magnitude for floor↔floor is 180°, floor↔wall 90°.
    let (c, s) = portal_rotation(Vec2::new(0.0, -1.0), Vec2::new(0.0, -1.0));
    assert!((s.atan2(c).abs() - PI).abs() < 1e-4);
    let (c, s) = portal_rotation(Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0));
    assert!((s.atan2(c).abs() - FRAC_PI_2).abs() < 1e-4);
}
