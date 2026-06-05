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
//! ([`subtract_aabb`]). It has no ECS, no Bevy systems, no RNG — so the headless
//! sim and the unit tests exercise the exact same geometry the game runs.
//!
//! Restricted to **axis-aligned portals** (normal is ±x or ±y) for now, per the
//! design note: floor / wall / ceiling portals compose cleanly with AABB
//! collision; arbitrary angles need clipped polygons and are deferred.

use crate::engine_core::{self as ae, AabbExt};
use bevy::math::Vec2;

/// An axis-aligned portal as the piece math sees it: where the doorway is, the
/// outward surface normal, and the oriented opening half-extent. Decoupled from
/// the ECS `Portal` component so this module stays pure and testable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalFrame {
    /// World-space center of the doorway, on the host surface.
    pub pos: Vec2,
    /// Unit outward normal (axis-aligned: ±x or ±y), pointing into the room.
    pub normal: Vec2,
    /// Oriented AABB half-extent of the doorway (opening along the surface,
    /// thin through it). See `portal::portal_half_extent`.
    pub half_extent: Vec2,
}

impl PortalFrame {
    /// Half-length of the opening *along* the surface (perpendicular to the
    /// normal): a wall portal's half-height, a floor portal's half-width.
    pub fn aperture_half(&self) -> f32 {
        let along = Vec2::new(-self.normal.y, self.normal.x);
        along.x.abs() * self.half_extent.x.abs() + along.y.abs() * self.half_extent.y.abs()
    }
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
    let t_in = portal_tangent(n_in);
    let t_out = portal_tangent(n_out);
    let into = -v.dot(n_in); // speed/offset INTO the entry → OUT of the exit
    let along = v.dot(t_in); // along-surface component, preserved
    into * n_out + along * t_out
}

/// Map a world point near `enter` to the corresponding point near `exit`: the
/// depth a point has sunk *into* the entry wall becomes the depth it emerges
/// *out* of the exit portal (so `enter.pos` maps to `exit.pos`), and its
/// along-surface offset is preserved (see [`portal_map_vec`]).
pub fn map_point(p: Vec2, enter: &PortalFrame, exit: &PortalFrame) -> Vec2 {
    exit.pos + portal_map_vec(p - enter.pos, enter.normal, exit.normal)
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

/// Build an AABB from explicit min/max edges (empty → `None`).
fn aabb_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<ae::Aabb> {
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    Some(ae::Aabb::new(
        Vec2::new((x0 + x1) * 0.5, (y0 + y1) * 0.5),
        Vec2::new((x1 - x0) * 0.5, (y1 - y0) * 0.5),
    ))
}

/// Keep the part of `b` on the side of the plane (through `point`, axis-aligned
/// outward `dir`) that `dir` points toward. `None` if `b` is entirely on the far
/// side. Used to clip a body to one side of a portal plane.
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
    aabb_mm(x0, y0, x1, y1)
}

/// Clip `b` laterally to a portal's opening span (so a body wider than the
/// aperture only shows the slice that fits through the doorway).
fn clip_to_aperture(b: ae::Aabb, frame: &PortalFrame) -> Option<ae::Aabb> {
    let along = Vec2::new(-frame.normal.y, frame.normal.x);
    let half = frame.aperture_half();
    if along.x.abs() > 0.5 {
        aabb_mm(
            b.min.x.max(frame.pos.x - half),
            b.min.y,
            b.max.x.min(frame.pos.x + half),
            b.max.y,
        )
    } else {
        aabb_mm(
            b.min.x,
            b.min.y.max(frame.pos.y - half),
            b.max.x,
            b.max.y.min(frame.pos.y + half),
        )
    }
}

/// A body's representation in one local chart (one side of a portal pair).
#[derive(Clone, Copy, Debug)]
pub struct ThroughPiece {
    /// The clipped piece on the EXIT side, in exit-local world space.
    pub aabb: ae::Aabb,
    /// The portal the body is sinking into (its plane clips `here`).
    pub enter: PortalFrame,
    /// The linked portal the piece emerges from.
    pub exit: PortalFrame,
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
        Self { here: body, through: None }
    }
}

/// Does `body` straddle `frame`'s plane within the opening? True when the plane
/// passes through the body's extent AND the body overlaps the aperture span.
pub fn straddles(body: ae::Aabb, frame: &PortalFrame) -> bool {
    let half = frame.aperture_half();
    if frame.normal.x.abs() > 0.5 {
        // Vertical-plane (wall) portal: plane is a vertical line at pos.x.
        body.min.x < frame.pos.x
            && body.max.x > frame.pos.x
            && body.max.y > frame.pos.y - half
            && body.min.y < frame.pos.y + half
    } else {
        // Horizontal-plane (floor / ceiling) portal: plane is a horizontal line.
        body.min.y < frame.pos.y
            && body.max.y > frame.pos.y
            && body.max.x > frame.pos.x - half
            && body.min.x < frame.pos.x + half
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
pub fn compute_body_pieces(body: ae::Aabb, pair: Option<(PortalFrame, PortalFrame)>) -> BodyPieces {
    let Some((a, b)) = pair else {
        return BodyPieces::whole(body);
    };
    for (enter, exit) in [(a, b), (b, a)] {
        if !straddles(body, &enter) {
            continue;
        }
        // Front slice stays here (clipped at the plane so it never shows inside
        // the wall); the back slice is what has crossed.
        let here = clip_halfspace(body, enter.pos, enter.normal).unwrap_or(body);
        let through = clip_halfspace(body, enter.pos, -enter.normal)
            .map(|back| map_aabb(back, &enter, &exit))
            // The emerged piece shows only what is in front of the exit and
            // within its opening.
            .and_then(|mapped| clip_halfspace(mapped, exit.pos, exit.normal))
            .and_then(|mapped| clip_to_aperture(mapped, &exit))
            .map(|aabb| ThroughPiece { aabb, enter, exit });
        return BodyPieces { here, through };
    }
    BodyPieces::whole(body)
}

/// Signed distance of `point` from `frame`'s plane along its outward normal:
/// positive = in front (room side), negative = behind (into the wall). The
/// centroid transfer fires when this changes sign.
pub fn front_distance(point: Vec2, frame: &PortalFrame) -> f32 {
    (point - frame.pos).dot(frame.normal)
}

// ---------------------------------------------------------------------------
// Host-surface carve: the floor / wall must become non-solid in the opening.

/// Set-difference of two axis-aligned rectangles: `block` minus `hole`, pushed
/// into `out` as up to four sub-rectangles (the frame around the hole). If they
/// don't overlap, `block` is pushed unchanged; if `hole` covers `block`, nothing
/// is pushed. This is how a portal carves a doorway out of its host surface
/// while leaving the rim and surrounding geometry solid.
pub fn subtract_aabb(block: ae::Aabb, hole: ae::Aabb, out: &mut Vec<ae::Aabb>) {
    let (bx0, by0, bx1, by1) = (block.min.x, block.min.y, block.max.x, block.max.y);
    // Clamp the hole to the block.
    let hx0 = hole.min.x.max(bx0);
    let hy0 = hole.min.y.max(by0);
    let hx1 = hole.max.x.min(bx1);
    let hy1 = hole.max.y.min(by1);
    if hx0 >= hx1 || hy0 >= hy1 {
        // No real overlap — keep the block whole.
        out.push(block);
        return;
    }
    // Up to four rectangles around the hole (below, above, left-middle,
    // right-middle). `aabb_mm` drops any that are empty.
    out.extend(aabb_mm(bx0, by0, bx1, hy0)); // below the hole
    out.extend(aabb_mm(bx0, hy1, bx1, by1)); // above the hole
    out.extend(aabb_mm(bx0, hy0, hx0, hy1)); // left of the hole
    out.extend(aabb_mm(hx1, hy0, bx1, hy1)); // right of the hole
}

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
pub fn carve_hole(frame: &PortalFrame) -> ae::Aabb {
    let along = Vec2::new(-frame.normal.y, frame.normal.x);
    let open = frame.aperture_half();
    let n = frame.normal;
    // Span from `+SURFACE_GRACE` outward of the face to `CARVE_DEPTH` inward.
    let through = (SURFACE_GRACE + CARVE_DEPTH) * 0.5;
    let center = frame.pos + n * (SURFACE_GRACE * 0.5) - n * (CARVE_DEPTH * 0.5);
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

    fn floor(pos: Vec2) -> PortalFrame {
        // Floor portal: normal points up (y-down world → up = -y).
        PortalFrame { pos, normal: Vec2::new(0.0, -1.0), half_extent: Vec2::new(46.0, 9.0) }
    }
    fn right_wall(pos: Vec2) -> PortalFrame {
        // Right wall: normal points left.
        PortalFrame { pos, normal: Vec2::new(-1.0, 0.0), half_extent: Vec2::new(9.0, 46.0) }
    }

    #[test]
    fn map_point_turns_depth_into_emergence() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        // A point sunk 10px below the floor plane (into the wall, +y) emerges
        // 10px out in front of the right wall (left of it, -x).
        let p = map_point(Vec2::new(100.0, 310.0), &enter, &exit);
        assert!((p.x - 390.0).abs() < 1e-3 && (p.y - 200.0).abs() < 1e-3, "got {p:?}");
        // The portal centers map onto each other.
        let c = map_point(enter.pos, &enter, &exit);
        assert!((c - exit.pos).length() < 1e-3, "centers map together, got {c:?}");
    }

    #[test]
    fn map_aabb_swaps_halves_on_ninety_degree_turn() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let b = ae::Aabb::new(Vec2::new(100.0, 305.0), Vec2::new(12.0, 6.0));
        let m = map_aabb(b, &enter, &exit);
        // 90° turn → width/height swap.
        assert!((m.half_size().x - 6.0).abs() < 1e-3, "half x {:?}", m.half_size());
        assert!((m.half_size().y - 12.0).abs() < 1e-3, "half y {:?}", m.half_size());
    }

    #[test]
    fn velocity_rotation_matches_existing_convention() {
        // Falling down (+y) into a floor portal, exit a left-facing wall → move
        // left (-x), same speed.
        let cs = portal_rotation(Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0));
        let out = rotate(Vec2::new(0.0, 100.0), cs);
        assert!((out.x + 100.0).abs() < 1e-2 && out.y.abs() < 1e-2, "got {out:?}");
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
        assert!(pieces.here.max.y <= 300.0 + 1e-3, "here below plane: {:?}", pieces.here);
        // A through-piece exists, emerging from the exit (x < 400).
        let through = pieces.through.expect("feet should poke through");
        assert!(through.aabb.max.x <= 400.0 + 1e-3, "through in front of exit: {:?}", through.aabb);
        // The 90° turn maps the crossed DEPTH (10px below the floor) onto the
        // emergence depth out of the wall (10px along the exit normal, x), and
        // the body WIDTH (24px) onto the lateral extent (y).
        assert!((through.aabb.max.x - through.aabb.min.x - 10.0).abs() < 1e-2, "depth {:?}", through.aabb);
        assert!((through.aabb.max.y - through.aabb.min.y - 24.0).abs() < 1e-2, "lateral {:?}", through.aabb);
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
        assert!(front_distance(Vec2::new(100.0, 280.0), &f) > 0.0, "above floor = front");
        assert!(front_distance(Vec2::new(100.0, 320.0), &f) < 0.0, "below floor = behind");
    }

    #[test]
    fn subtract_carves_a_doorway_leaving_a_frame() {
        // A wide floor block, carve a hole in the middle.
        let block = ae::Aabb::new(Vec2::new(100.0, 300.0), Vec2::new(100.0, 10.0));
        let hole = ae::Aabb::new(Vec2::new(100.0, 300.0), Vec2::new(30.0, 30.0));
        let mut out = Vec::new();
        subtract_aabb(block, hole, &mut out);
        // Left + right segments remain; the middle is open.
        assert_eq!(out.len(), 2, "left + right frame: {out:?}");
        // The opening (x in [70,130]) is not covered by any remaining piece.
        for piece in &out {
            assert!(piece.min.x >= 130.0 - 1e-3 || piece.max.x <= 70.0 + 1e-3, "piece spans hole: {piece:?}");
        }
    }

    #[test]
    fn subtract_no_overlap_keeps_block() {
        let block = ae::Aabb::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let hole = ae::Aabb::new(Vec2::new(100.0, 100.0), Vec2::new(5.0, 5.0));
        let mut out = Vec::new();
        subtract_aabb(block, hole, &mut out);
        assert_eq!(out.len(), 1);
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
        assert!((hole.max.y - (300.0 + CARVE_DEPTH)).abs() < 1e-3, "hole goes inward: {hole:?}");
        // Opening width matches the aperture (2*46).
        assert!((hole.max.x - hole.min.x - 92.0).abs() < 1e-3, "{hole:?}");
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
