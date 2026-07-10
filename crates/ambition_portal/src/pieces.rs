//! Pure portal-piece geometry — the **Core invariant** of the portal system.
//!
//! > Every gameplay-relevant volume is representable as zero, one, or two
//! > portal-aware spatial pieces. Every system that asks "where is this thing?"
//! > uses those pieces instead of the raw body AABB.
//!
//! A portal pair topologically glues two parts of the world together. A body
//! straddling a portal plane is ONE logical object with TWO spatial pieces: the
//! part still on the entry side (`here`), and the part that has crossed the
//! plane, mapped through to emerge from the linked exit portal (`through`).
//!
//! This module is the pure, deterministic, allocation-light heart of that math:
//! the portal map (point / AABB / velocity), half-space clipping, the
//! piece-decomposition ([`compute_body_pieces`]), and the host-surface carve
//! HOLE ([`carve_hole`] — the rectangle set-difference it feeds is
//! `ambition_engine_core::geometry::subtract_aabb`, plain AABB algebra that lives
//! in the foundation). It has no ECS, no Bevy systems, no RNG — so the headless
//! sim and the unit tests exercise the exact same geometry the game runs.
//!
//! Current implementation note: gameplay pieces are still AABB-backed, so
//! production use is restricted to cardinal floor / wall / ceiling portals. The
//! frame math (now `ambition_engine_core::frame` — the CC5 aperture
//! vocabulary) is angle-general; arbitrary-angle portals need polygon clipping
//! and non-AABB body pieces before this can be a fully general standalone
//! portal crate (collision-and-ccd.md P3b).

use ambition_engine_core::{self as ae, AabbExt};
use bevy::math::Vec2;

// The engine-level aperture vocabulary (CC5): the frame type and pair map live
// in `ambition_engine_core::frame`; this module builds the AABB piece/carve
// geometry ON them.
pub use ambition_engine_core::frame::{PortalAperture, PortalFrame};

// The pure portal-map vector math (orientation-between-two-normals transforms)
// lives in the content-free `ambition_platformer_primitives` crate (delegating
// to `engine_core::frame`), including the game-wide convention dispatch.
// Re-export it here so portal_pieces' AABB/piece geometry and every other
// in-sandbox user (world_overlay, debug_overlay, portal/*) keep referencing
// `crate::pieces::{portal_rotation, rotate, portal_tangent,
// portal_map_vec}` unchanged.
pub use ambition_platformer_primitives::math::{
    portal_map_rotation, portal_map_vec, portal_map_vec_reflection, portal_map_vec_rotation,
    portal_rotation, portal_tangent, rotate, set_portal_map_rotation,
};

/// Map a world point near `enter` to the corresponding point near `exit`: the
/// depth a point has sunk *into* the entry wall becomes the depth it emerges
/// *out* of the exit portal (so `enter.origin` maps to `exit.origin`), and its
/// along-surface offset follows the game-wide convention (see
/// [`portal_map_vec`]).
pub fn map_point(p: Vec2, enter: &PortalFrame, exit: &PortalFrame) -> Vec2 {
    exit.origin + portal_map_vec(p - enter.origin, enter.normal, exit.normal)
}

/// Map an axis-aligned AABB through the portal pair. The map is axis-aligned for
/// axis-aligned portals (a 90° turn swaps the half-extents), so the image stays
/// an axis-aligned AABB.
pub fn map_aabb(b: ae::Aabb, enter: &PortalFrame, exit: &PortalFrame) -> ae::Aabb {
    let center = map_point(b.center(), enter, exit);
    // Transform the half-extent through the (axis-aligned) map: each output axis
    // gets |contribution| from each input axis.
    let col_x = portal_map_vec(Vec2::new(1.0, 0.0), enter.normal, exit.normal);
    let col_y = portal_map_vec(Vec2::new(0.0, 1.0), enter.normal, exit.normal);
    let h = b.half_size();
    let half = Vec2::new(
        col_x.x.abs() * h.x + col_y.x.abs() * h.y,
        col_x.y.abs() * h.x + col_y.y.abs() * h.y,
    );
    ae::Aabb::new(center, half)
}

/// Keep the part of `b` on the side of the plane (through `point`, axis-aligned
/// outward `dir`) that `dir` points toward. `None` if `b` is entirely on the far
/// side. Used to clip a body to one side of a portal plane.
///
/// (Cardinal-only, like the whole AABB piece layer — P3b generalizes.)
pub fn clip_halfspace(b: ae::Aabb, point: Vec2, dir: Vec2) -> Option<ae::Aabb> {
    let (mut x0, mut y0, mut x1, mut y1) = (b.min.x, b.min.y, b.max.x, b.max.y);
    if dir.x > 0.5 {
        x0 = x0.max(point.x);
    } else if dir.x < -0.5 {
        x1 = x1.min(point.x);
    } else if dir.y > 0.5 {
        y0 = y0.max(point.y);
    } else if dir.y < -0.5 {
        y1 = y1.min(point.y);
    }
    ae::geometry::aabb_from_min_max(x0, y0, x1, y1)
}

/// Clip `b` laterally to a portal's opening span (so a body wider than the
/// aperture only shows the slice that fits through the doorway).
fn clip_to_aperture(b: ae::Aabb, ap: &PortalAperture) -> Option<ae::Aabb> {
    let along = ap.frame.tangent();
    let half = ap.half_length;
    if along.x.abs() > 0.5 {
        ae::geometry::aabb_from_min_max(
            b.min.x.max(ap.frame.origin.x - half),
            b.min.y,
            b.max.x.min(ap.frame.origin.x + half),
            b.max.y,
        )
    } else {
        ae::geometry::aabb_from_min_max(
            b.min.x,
            b.min.y.max(ap.frame.origin.y - half),
            b.max.x,
            b.max.y.min(ap.frame.origin.y + half),
        )
    }
}

/// A body's representation in one local chart (one side of a portal pair).
#[derive(Clone, Copy, Debug)]
pub struct ThroughPiece {
    /// The clipped piece on the EXIT side, in exit-local world space.
    pub aabb: ae::Aabb,
    /// The portal the body is sinking into (its plane clips `here`).
    pub enter: PortalAperture,
    /// The linked portal the piece emerges from.
    pub exit: PortalAperture,
}

/// The portal-aware decomposition of one body: always a `here` piece on the
/// body's current side, plus an optional `through` piece on the far side of a
/// straddled portal. Their union reconstructs the whole body across two charts.
#[derive(Clone, Copy, Debug)]
pub struct BodyPieces {
    /// The piece on the body's authoritative side — clipped to the front of the
    /// straddled portal when mid-transit, else the whole body.
    pub here: ae::Aabb,
    /// The piece that has crossed the portal plane, mapped to the exit side.
    /// `None` when the body straddles no portal.
    pub through: Option<ThroughPiece>,
}

impl BodyPieces {
    /// A body that straddles no portal: a single, whole piece.
    pub fn whole(body: ae::Aabb) -> Self {
        Self {
            here: body,
            through: None,
        }
    }
}

/// Does `body` straddle `ap`'s plane within the opening? True when the plane
/// passes through the body's extent AND the body overlaps the aperture span.
pub fn straddles(body: ae::Aabb, ap: &PortalAperture) -> bool {
    let half = ap.half_length;
    let origin = ap.frame.origin;
    if ap.frame.normal.x.abs() > 0.5 {
        // Vertical-plane (wall) portal: plane is a vertical line at origin.x.
        body.min.x < origin.x
            && body.max.x > origin.x
            && body.max.y > origin.y - half
            && body.min.y < origin.y + half
    } else {
        // Horizontal-plane (floor / ceiling) portal: plane is a horizontal line.
        body.min.y < origin.y
            && body.max.y > origin.y
            && body.max.x > origin.x - half
            && body.min.x < origin.x + half
    }
}

/// Decompose `body` against a linked portal pair. Finds the portal whose plane
/// the body straddles (within its opening), keeps the front slice as `here`, and
/// maps the crossed slice through to the linked exit as `through`. If the body
/// straddles neither portal, returns the whole body.
///
/// Direction-agnostic: it works the same before the centroid crosses (body on
/// the entry side, trailing nothing) and after (body on the exit side, its
/// trailing slice mapped back to the entry) — whichever plane it currently
/// straddles is the "entry" for the decomposition.
pub fn compute_body_pieces(
    body: ae::Aabb,
    pair: Option<(PortalAperture, PortalAperture)>,
) -> BodyPieces {
    let Some((a, b)) = pair else {
        return BodyPieces::whole(body);
    };
    for (enter, exit) in [(a, b), (b, a)] {
        if !straddles(body, &enter) {
            continue;
        }
        // Front slice stays here (clipped at the plane so it never shows inside
        // the wall); the back slice is what has crossed.
        let here = clip_halfspace(body, enter.frame.origin, enter.frame.normal).unwrap_or(body);
        let through = clip_halfspace(body, enter.frame.origin, -enter.frame.normal)
            .map(|back| map_aabb(back, &enter.frame, &exit.frame))
            // The emerged piece shows only what is in front of the exit and
            // within its opening.
            .and_then(|mapped| clip_halfspace(mapped, exit.frame.origin, exit.frame.normal))
            .and_then(|mapped| clip_to_aperture(mapped, &exit))
            .map(|aabb| ThroughPiece { aabb, enter, exit });
        return BodyPieces { here, through };
    }
    BodyPieces::whole(body)
}

/// Signed distance of `point` from `frame`'s plane along its outward normal:
/// positive = in front (room side), negative = behind (into the wall). The
/// centroid transfer fires when this changes sign. (This IS
/// `frame.to_local(point).y` — kept as the named domain verb.)
pub fn front_distance(point: Vec2, frame: &PortalFrame) -> f32 {
    (point - frame.origin).dot(frame.normal)
}

// ---------------------------------------------------------------------------
// Host-surface carve: the floor / wall must become non-solid in the opening.
//
// The rectangle set-difference itself is `ae::geometry::subtract_aabb` — plain
// AABB algebra, so it lives in the foundation (refactor-chain R3). It moved so
// `ambition_world` could composite a carved collision world without depending on
// this crate. The portal-specific part — how deep and how wide the hole is — is
// below.

/// How deep (px) into the host surface a portal carves its doorway. Just past a
/// body's half-depth so the leading slice can sink in up to the centroid before
/// the transfer fires; small enough that it never punches through to far-side
/// geometry on a thick wall.
pub const CARVE_DEPTH: f32 = 60.0;

/// How far OUTWARD of the portal face (px) the carve also reaches. A portal
/// authored on a surface can land a few px off the grid-snapped collision edge
/// (e.g. a floor whose IntGrid top is y=896 but the portal face is y=900); the
/// carve must reach back across that gap or a thin solid LIP survives in the
/// opening and the body rests on it instead of sinking in. One grid cell covers
/// any realistic snap; for a floor it only removes the lip (open room is above).
pub const SURFACE_GRACE: f32 = 16.0;

/// The carve hole for a portal: the opening width along the surface, cut from a
/// little OUTWARD of the face ([`SURFACE_GRACE`], to clear any grid-snap lip)
/// through [`CARVE_DEPTH`] inward.
pub fn carve_hole(ap: &PortalAperture) -> ae::Aabb {
    carve_hole_with_depth(ap, f32::INFINITY)
}

/// [`carve_hole`] bounded by the MEASURED host material depth: the aperture
/// volume ends where the wall does. On a thin wall a full-depth hole would
/// reach into the open room behind it, and anything keying off "inside the
/// hole" (the transit rescue, the carve engagement) would wrongly engage a
/// body standing behind the wall.
pub fn carve_hole_with_depth(ap: &PortalAperture, host_depth: f32) -> ae::Aabb {
    let depth = CARVE_DEPTH.min(host_depth.max(0.0));
    let along = ap.frame.tangent();
    let open = ap.half_length;
    let n = ap.frame.normal;
    // Span from `+SURFACE_GRACE` outward of the face to `depth` inward.
    let through = (SURFACE_GRACE + depth) * 0.5;
    let center = ap.frame.origin + n * (SURFACE_GRACE * 0.5) - n * (depth * 0.5);
    let half = Vec2::new(
        along.x.abs() * open + n.x.abs() * through,
        along.y.abs() * open + n.y.abs() * through,
    );
    ae::Aabb::new(center, half)
}

#[cfg(test)]
mod tests {
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
}
