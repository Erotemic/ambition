//! Pure through-portal **view** geometry — what a viewer looking into one
//! portal sees of the world at its partner.
//!
//! Two display models, sharing the same source region (the world in FRONT of
//! the exit portal):
//!
//! - **Window** ([`ViewCone`] / [`view_cone`] — what the default renderer
//!   ships): the view recedes INTO the entry's host surface, like glass set in
//!   the wall — you see "through the portal a little bit." A window's display
//!   map is the BODY map ([`map_point`]) — the same map transit uses for
//!   positions and velocities — so the window agrees with where bodies
//!   actually emerge. **The 2D parity fact:** the body map is always det −1
//!   (a reflection), so in 2D you cannot have both "sprites never mirror" and
//!   "lateral position/velocity preserved" for every pair orientation (3D
//!   escapes via a rotation about the portal normal; 2D has no third axis).
//!   The game keeps the tangent-preserving (mirror) transit; sprites realize
//!   it exactly via [`copy_roll`] + an unconditional `flip_x` (any reflection
//!   = rotation ∘ flip_x), so window, copy, and transit are ONE map.
//! - **Projection** ([`PortalViewMap`] / [`view_point`]): the view protrudes
//!   into the room in front of the entry, hologram-style. Its map is the body
//!   map composed with a reflection across the entry plane, which yields a
//!   small theorem: the body map always sends the orientation −1 frame
//!   `(-n_in, t_in)` onto the orientation +1 frame `(n_out, t_out)` (det −1,
//!   always a reflection), so the PROJECTION map is always a PROPER rotation
//!   (det +1) — a host drawing this model can orient a camera by
//!   [`PortalViewMap::angle`] with no flip case, pinned for every axis-aligned
//!   pair below.
//!
//! Like [`pieces`], this module is pure and allocation-light: no ECS, no
//! render types, no RNG. The renderer (`ambition_portal_presentation`) builds
//! its capture cameras and window UVs from [`view_cone`]; a roll-your-own
//! host consumes the same functions.

use ambition_engine_core as ae;
use bevy::math::Vec2;

use crate::pieces::{map_point, portal_map_vec, PortalFrame};

/// The proper rigid map of the VIEW through a portal pair: rotation `(cos,
/// sin)` about the entry portal's center, then translation onto the exit's.
/// Built by [`PortalViewMap::between`]; always orientation-preserving (see the
/// module docs — that is the theorem this type encodes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalViewMap {
    /// Entry portal center (the rotation pivot).
    pub enter_pos: Vec2,
    /// Exit portal center (the pivot's image).
    pub exit_pos: Vec2,
    /// Rotation cosine of the linear part.
    pub cos: f32,
    /// Rotation sine of the linear part.
    pub sin: f32,
}

impl PortalViewMap {
    /// The view map for a linked pair: body map ∘ reflection across the entry
    /// plane. The linear part is recovered from its action on the basis and
    /// debug-asserted to be a proper rotation.
    pub fn between(enter: &PortalFrame, exit: &PortalFrame) -> Self {
        let lin = |v: Vec2| {
            // Reflect across the entry plane (linear part: across the surface
            // direction), then push through the body map.
            let reflected = v - 2.0 * v.dot(enter.normal) * enter.normal;
            portal_map_vec(reflected, enter.normal, exit.normal)
        };
        let col_x = lin(Vec2::X);
        let col_y = lin(Vec2::Y);
        // Proper rotation: orthonormal columns with det +1 — col_y is col_x
        // rotated +90°. Holds for ALL normals by the module-docs argument.
        debug_assert!(
            (col_x.x * col_y.y - col_x.y * col_y.x - 1.0).abs() < 1e-4,
            "view map must be a proper rotation, got cols {col_x:?} {col_y:?}"
        );
        Self {
            enter_pos: enter.pos,
            exit_pos: exit.pos,
            cos: col_x.x,
            sin: col_x.y,
        }
    }

    /// The exit-side world point whose light "comes through" the portal to the
    /// entry-side point `p`.
    pub fn apply(&self, p: Vec2) -> Vec2 {
        let v = p - self.enter_pos;
        self.exit_pos
            + Vec2::new(
                v.x * self.cos - v.y * self.sin,
                v.x * self.sin + v.y * self.cos,
            )
    }

    /// The rotation angle (radians) of the linear part — what a renderer
    /// rotates a capture by (in WORLD space; remember the host's y-flip when
    /// converting to screen space).
    pub fn angle(&self) -> f32 {
        self.sin.atan2(self.cos)
    }
}

/// What a viewer sees at entry-side point `p`: the view map applied to `p`.
/// Convenience over [`PortalViewMap::between`] + `apply` for one-off points;
/// equals `map_point(reflect(p))` by construction.
pub fn view_point(p: Vec2, enter: &PortalFrame, exit: &PortalFrame) -> Vec2 {
    PortalViewMap::between(enter, exit).apply(p)
}

/// The view cone of one portal, **window semantics**: a trapezoid receding
/// from the entry face INTO the host surface (you look "through" the portal a
/// little way), displaying the world in front of the exit via the body
/// [`map_point`] (the transit map, so the window agrees with where bodies
/// actually emerge; the sprite copy realizes the same map via [`copy_roll`]).
///
/// Corner order is `[near_a, near_b, far_b, far_a]` — near edge ON the face
/// (lateral ∓ aperture), far edge `depth` INTO the wall (lateral widened by
/// `spread * depth` per side) — so `(0,1,2) (0,2,3)` triangulates it with
/// consistent winding.
#[derive(Clone, Copy, Debug)]
pub struct ViewCone {
    /// Trapezoid corners at the ENTRY portal (face + into-the-wall), world space.
    pub entry_quad: [Vec2; 4],
    /// The same corners pushed through the body [`map_point`]: the exit-side
    /// world quad the window displays. `source_quad[i]` is what `entry_quad[i]`
    /// shows — a renderer derives per-vertex UVs by normalizing these inside
    /// [`Self::source`].
    pub source_quad: [Vec2; 4],
    /// Axis-aligned bounds of `source_quad`: the world rect (in FRONT of the
    /// exit) a capture camera must frame. Axis-aligned exactly (not just
    /// bounding) for axis-aligned portals, since the display map's linear part
    /// is then axis-aligned.
    pub source: ae::Aabb,
}

/// The render-space roll for the transit sprite copy such that
/// `R(roll) ∘ flip_x` equals the BODY map exactly. Works for every pair
/// because the body map is always a reflection (det −1) and every 2D
/// reflection factors as rotation ∘ flip_x. With the copy drawn this way, the
/// copy, the window (body-map UVs), and the actual transit all agree —
/// entering on one side of the entry shows you emerging exactly where transit
/// puts you, mirrored or not.
pub fn copy_roll(enter: &PortalFrame, exit: &PortalFrame) -> f32 {
    // World-space first column of M ∘ FlipX is M·(−e_x) = −m_x; the world
    // rotation angle is atan2 of that column, and render space negates it
    // (y-flip conjugation).
    let m_x = portal_map_vec(Vec2::X, enter.normal, exit.normal);
    let world = (-m_x.y).atan2(-m_x.x);
    -world
}

/// Build a [`ViewCone`] from its four entry-side corners: the source quad is
/// the corners through the body [`map_point`] (the transit map), the source
/// rect their bounds. One place that defines the display map, shared by every
/// cone constructor.
fn from_entry_quad(entry_quad: [Vec2; 4], enter: &PortalFrame, exit: &PortalFrame) -> ViewCone {
    let source_quad = entry_quad.map(|p| map_point(p, enter, exit));
    let (mut min, mut max) = (source_quad[0], source_quad[0]);
    for p in &source_quad[1..] {
        min = min.min(*p);
        max = max.max(*p);
    }
    ViewCone {
        entry_quad,
        source_quad,
        source: ae::Aabb::new((min + max) * 0.5, (max - min) * 0.5),
    }
}

/// Build the static [`ViewCone`] for a linked pair: a fixed symmetric trapezoid
/// receding `depth` into the entry's host surface, widening by `spread` per px
/// of depth. Viewer-independent — the "always show this much" baseline (also
/// the minimum-cone floor; see [`blend_cones`]).
pub fn view_cone(enter: &PortalFrame, exit: &PortalFrame, depth: f32, spread: f32) -> ViewCone {
    let n = enter.normal;
    let along = Vec2::new(-n.y, n.x);
    let near_half = enter.aperture_half();
    let far_half = near_half + depth * spread;
    from_entry_quad(
        [
            enter.pos - along * near_half,
            enter.pos + along * near_half,
            enter.pos + along * far_half - n * depth,
            enter.pos - along * far_half - n * depth,
        ],
        enter,
        exit,
    )
}

/// Smallest front distance treated as "cleanly in front" of a surface.
const MIN_FRONT: f32 = 1.0;
/// In-doorway grace, lateral: how far outside the aperture span the eye may sit
/// and still count as "in the doorway" of that end.
const DOORWAY_LATERAL_GRACE: f32 = 26.0;
/// In-doorway grace, depth: how far the eye may dip BEHIND the surface while
/// transiting and still count as in the doorway. The centroid transfer fires
/// shortly after the plane crossing, so a small slab suffices; anything deeper
/// is genuinely "behind the wall."
const DOORWAY_DEPTH_GRACE: f32 = 24.0;

/// The effective eye for looking into `enter`, given the controlled
/// character's real `eye`. A portal pair glues two surfaces into ONE window,
/// so the character can look into `enter` from in front of EITHER end —
/// directly, or through the pair (standing in front of the partner IS standing
/// in front of this end; the eye's image is the front-preserving
/// [`view_point`], never the front-flipping body map).
///
/// Ends are tried **nearest first** (Euclidean distance to the aperture
/// center). This matters when the eye is in front of both ends — e.g. two
/// floor portals share one plane, so a viewer above the partner is "in front
/// of" this end too, but 250px to the side: the honest window comes from the
/// partner-side image right above the aperture, not from the grazing direct
/// ray. Nearest-first picks it.
///
/// **In-doorway grace:** while transiting, the eye dips just BEHIND the plane
/// of the end it is passing through; visually the character is *in* the
/// window, which should read as a (near) half-plane, not vanish. An eye within
/// the aperture span (+[`DOORWAY_LATERAL_GRACE`]) and within
/// [`DOORWAY_DEPTH_GRACE`] of the plane is lifted to just in front of it —
/// [`aperture_wedge`]'s small-front continuation then yields the half-plane
/// limit. `None` only when the eye is behind both ends and in neither doorway.
pub fn window_eye(enter: &PortalFrame, exit: &PortalFrame, eye: Vec2) -> Option<(Vec2, bool)> {
    let mut ends = [(enter, false), (exit, true)];
    if eye.distance(exit.pos) < eye.distance(enter.pos) {
        ends.swap(0, 1);
    }
    for (end, via_partner) in ends {
        let n = end.normal;
        let t = Vec2::new(-n.y, n.x);
        let v = eye - end.pos;
        let (front, lat) = (v.dot(n), v.dot(t));
        let in_doorway = lat.abs() <= end.aperture_half() + DOORWAY_LATERAL_GRACE
            && front.abs() <= DOORWAY_DEPTH_GRACE;
        let front = if front >= MIN_FRONT {
            front
        } else if in_doorway {
            // At/inside the doorway: lift to a hair in front — the wedge's
            // limit continuation turns this into the half-plane.
            MIN_FRONT * 0.5
        } else {
            continue;
        };
        let resolved = end.pos + n * front + t * lat;
        return Some(if via_partner {
            (view_point(resolved, exit, enter), true)
        } else {
            (resolved, false)
        });
    }
    None
}

/// The viewer-dependent wedge through the aperture, given an `eye` already in
/// front of `enter` (use [`window_eye`] to resolve it). Treat the aperture as
/// a slit: a point behind the surface is visible iff the sight line `eye → P`
/// crosses it, so the region is the wedge bounded by the rays from `eye`
/// through the aperture endpoints, clipped to `max_depth` deep.
///
/// In the (normal, tangent) frame each far corner sits at depth exactly
/// `max_depth` with lateral offset `lat_A + (lat_A − lat_eye)·(max_depth/front)`.
/// As `front → 0` that diverges — but the LIMIT shape is well-defined: the
/// full half-plane strip of depth `max_depth`. So instead of bounding the
/// denominator, the lateral offset is **clamped to ±`max_lateral`** and the
/// near-plane case (`front < 1`) switches to the limit directly: far corners
/// at the clamp, away from the eye. The wedge therefore grows smoothly into
/// the (bounded) half-plane as the viewer reaches the portal — no blow-up, no
/// NaN, and a capture rect a fixed-size texture can actually frame.
///
/// `None` if `eye` is behind the plane. Pure geometry — line-of-sight
/// occlusion is the caller's check.
pub fn aperture_wedge(
    enter: &PortalFrame,
    exit: &PortalFrame,
    eye: Vec2,
    max_depth: f32,
    max_lateral: f32,
) -> Option<ViewCone> {
    let n = enter.normal;
    let t = Vec2::new(-n.y, n.x);
    let v = eye - enter.pos;
    let front = v.dot(n);
    if front <= 0.0 {
        return None;
    }
    let lat_eye = v.dot(t);
    let h = enter.aperture_half();
    let far_lat = |lat_a: f32| -> f32 {
        if front < MIN_FRONT {
            // Limit continuation: essentially on the plane, the ray through
            // the endpoint is parallel to the surface — the strip extends to
            // the clamp, away from the eye (or outward if dead-centered).
            if (lat_a - lat_eye).abs() < 1e-3 {
                lat_a.signum() * max_lateral
            } else {
                (lat_a - lat_eye).signum() * max_lateral
            }
        } else {
            (lat_a + (lat_a - lat_eye) * (max_depth / front)).clamp(-max_lateral, max_lateral)
        }
    };
    let a0 = enter.pos - t * h;
    let a1 = enter.pos + t * h;
    let f0 = enter.pos + t * far_lat(-h) - n * max_depth;
    let f1 = enter.pos + t * far_lat(h) - n * max_depth;
    Some(from_entry_quad([a0, a1, f1, f0], enter, exit))
}

/// Convenience: [`window_eye`] (so it works from either end of the pair, with
/// the in-doorway grace) then [`aperture_wedge`]. `None` only when the viewer
/// is behind both ends and in neither doorway.
pub fn visible_cone(
    enter: &PortalFrame,
    exit: &PortalFrame,
    eye: Vec2,
    max_depth: f32,
    max_lateral: f32,
) -> Option<ViewCone> {
    let (eye, _) = window_eye(enter, exit, eye)?;
    aperture_wedge(enter, exit, eye, max_depth, max_lateral)
}

/// Per-corner linear blend `a → b` by `t ∈ [0,1]`. With `a` the minimum cone
/// and `b` the viewer wedge, `t = 0` shows the always-on minimum and `t = 1`
/// the full visible wedge — the two share the near (aperture) edge, so the
/// blend just opens the far edge from the floor out to what the viewer sees.
pub fn blend_cones(
    a: &ViewCone,
    b: &ViewCone,
    t: f32,
    enter: &PortalFrame,
    exit: &PortalFrame,
) -> ViewCone {
    let t = t.clamp(0.0, 1.0);
    let entry_quad = std::array::from_fn(|i| a.entry_quad[i].lerp(b.entry_quad[i], t));
    from_entry_quad(entry_quad, enter, exit)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::pieces::{front_distance, map_point};
    use ambition_engine_core::AabbExt;

    fn frame(pos: Vec2, normal: Vec2) -> PortalFrame {
        PortalFrame {
            pos,
            normal,
            half_extent: crate::portal_half_extent(normal),
        }
    }
    fn floor(pos: Vec2) -> PortalFrame {
        frame(pos, Vec2::new(0.0, -1.0))
    }
    fn right_wall(pos: Vec2) -> PortalFrame {
        frame(pos, Vec2::new(-1.0, 0.0))
    }
    fn size(b: ae::Aabb) -> Vec2 {
        b.half_size() * 2.0
    }

    /// The theorem: for every axis-aligned (enter, exit) normal pair, the view
    /// map is a PROPER rotation — orthonormal linear part, det +1. This is what
    /// lets a renderer draw the view without ever mirroring a capture.
    #[test]
    fn view_map_is_always_a_proper_rotation() {
        let normals = [
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
        ];
        for n_in in normals {
            for n_out in normals {
                let enter = frame(Vec2::new(100.0, 300.0), n_in);
                let exit = frame(Vec2::new(700.0, 140.0), n_out);
                let m = PortalViewMap::between(&enter, &exit);
                assert!(
                    (m.cos * m.cos + m.sin * m.sin - 1.0).abs() < 1e-4,
                    "unit rotation for {n_in:?}→{n_out:?}: cos {} sin {}",
                    m.cos,
                    m.sin
                );
            }
        }
    }

    /// On the portal face the reflection is the identity, so the view map and
    /// the body map agree — an emerging body lines up with its cone image.
    #[test]
    fn view_agrees_with_body_map_on_the_face() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        for s in [-30.0_f32, 0.0, 18.5, 46.0] {
            let on_face = enter.pos + Vec2::new(s, 0.0); // floor face runs along x
            let via_view = view_point(on_face, &enter, &exit);
            let via_body = map_point(on_face, &enter, &exit);
            assert!(
                (via_view - via_body).length() < 1e-3,
                "face continuity at s={s}: view {via_view:?} body {via_body:?}"
            );
        }
    }

    /// Projection model: depth in front of the entry becomes depth in front
    /// of the exit — the projection shows the exit's room, never the inside
    /// of its wall.
    #[test]
    fn view_preserves_front_depth() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        for d in [0.0_f32, 5.0, 60.0, 240.0] {
            let p = enter.pos + enter.normal * d;
            let seen = view_point(p, &enter, &exit);
            assert!(
                (front_distance(seen, &exit) - d).abs() < 1e-3,
                "depth {d} maps to front depth {}",
                front_distance(seen, &exit)
            );
        }
    }

    /// Pin the floor→right-wall map numerically: 10px above the floor portal
    /// at lateral +s shows the point 10px left of the wall portal at lateral
    /// -s along the wall's tangent (t_out = (0,-1) ⇒ world offset (0,-s)).
    #[test]
    fn floor_to_wall_view_pinned() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        // y-down world: 10px in FRONT of a floor portal is y=290.
        let seen = view_point(Vec2::new(120.0, 290.0), &enter, &exit);
        assert!(
            (seen - Vec2::new(390.0, 180.0)).length() < 1e-3,
            "got {seen:?}"
        );
        // The rotation angle is -90° (cos 0, sin -1) for this pair.
        let m = PortalViewMap::between(&enter, &exit);
        assert!((m.cos).abs() < 1e-4 && (m.sin + 1.0).abs() < 1e-4, "{m:?}");
    }

    /// Window semantics: the trapezoid recedes INTO the entry's host surface,
    /// while its source rect sits fully in FRONT of the exit (it images the
    /// exit's room), swapping extents across a 90° pair: window depth becomes
    /// the rect's x-extent, window width its y-extent.
    #[test]
    fn view_cone_source_geometry() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let depth = 120.0;
        let spread = 0.25;
        let cone = view_cone(&enter, &exit, depth, spread);
        // Entry quad: near edge on the face, far edge `depth` INTO the floor
        // (y-down world: into a floor = +y).
        assert!((cone.entry_quad[0].y - 300.0).abs() < 1e-3);
        assert!((cone.entry_quad[2].y - 420.0).abs() < 1e-3);
        // Source rect: x spans the wall's front depth, y the widened lateral.
        assert!(
            (size(cone.source).x - depth).abs() < 1e-3,
            "depth extent {:?}",
            size(cone.source)
        );
        let far_half = enter.aperture_half() + depth * spread;
        assert!(
            (size(cone.source).y - 2.0 * far_half).abs() < 1e-3,
            "lateral extent {:?}",
            size(cone.source)
        );
        // Fully in front of the exit wall (x <= 400), touching the face.
        assert!(cone.source.max.x <= 400.0 + 1e-3, "{:?}", cone.source);
        assert!((cone.source.max.x - 400.0).abs() < 1e-3);
        // Every source corner is the BODY-map image of its entry corner (the
        // window's display map IS the body map — one map for sight and transit).
        for (e, s) in cone.entry_quad.iter().zip(cone.source_quad.iter()) {
            assert!((map_point(*e, &enter, &exit) - *s).length() < 1e-3);
        }
    }

    /// Behind BOTH ends, in neither doorway ⇒ `None` (two floors, eye well
    /// below both planes).
    #[test]
    fn visible_cone_none_behind_both_ends() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = floor(Vec2::new(500.0, 300.0));
        // Floors face up (−y); 60px below is past the doorway depth grace.
        assert!(visible_cone(&enter, &exit, Vec2::new(100.0, 360.0), 80.0, 400.0).is_none());
    }

    /// The wormhole: standing in front of the PARTNER opens this end's window
    /// even though the eye is behind this surface (above purple ⇒ yellow shows).
    #[test]
    fn visible_cone_opens_from_the_partner_side() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        // Eye is BEHIND the floor (y > 300, past the doorway grace) but in
        // FRONT of the wall partner (x < 400) — only the wormhole opens it.
        let eye = Vec2::new(100.0, 360.0);
        let (resolved, wormhole) = window_eye(&enter, &exit, eye).expect("in front of partner");
        assert!(wormhole, "resolved via the partner side");
        // The image is in front of `enter` (above the floor, y < 300).
        assert!(resolved.y < 300.0, "image in front of enter: {resolved:?}");
        assert!(visible_cone(&enter, &exit, eye, 80.0, 400.0).is_some());
    }

    /// Same-plane pair (two floor portals): the eye above the PARTNER is in
    /// front of BOTH ends, but the window must resolve from the nearer one —
    /// the partner-side image right above this aperture — not from the grazing
    /// 400px-away direct ray. This is the straddle case: standing on purple,
    /// yellow's window opens as if you stood on yellow.
    #[test]
    fn window_eye_prefers_the_nearer_end() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = floor(Vec2::new(500.0, 300.0));
        // Eye 20px above the EXIT (the partner end).
        let eye = Vec2::new(500.0, 280.0);
        let (resolved, wormhole) = window_eye(&enter, &exit, eye).expect("in front of both");
        assert!(wormhole, "nearer end is the partner");
        // The image sits 20px above THIS aperture (floor↔floor: x preserved
        // relative to centers, front preserved).
        assert!(
            (resolved - Vec2::new(100.0, 280.0)).length() < 1e-3,
            "partner image above this end, got {resolved:?}"
        );
    }

    /// In-doorway grace: an eye dipped just BEHIND the plane mid-transit
    /// (within the aperture span) still opens the window — as the half-plane
    /// limit, not a sliver and not `None`.
    #[test]
    fn window_survives_the_transit_dip_as_a_half_plane() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = floor(Vec2::new(500.0, 300.0));
        let max_lateral = 400.0;
        // Eye 10px BELOW the entry plane, laterally centered (mid-transit).
        let eye = Vec2::new(100.0, 310.0);
        let cone =
            visible_cone(&enter, &exit, eye, 80.0, max_lateral).expect("doorway grace holds");
        let [_, _, f1, f0] = cone.entry_quad;
        // The limit continuation: far corners at the lateral clamp.
        assert!(
            (f0.x - (100.0 - max_lateral)).abs() < 1e-3
                && (f1.x - (100.0 + max_lateral)).abs() < 1e-3,
            "half-plane strip, got {f0:?} {f1:?}"
        );
        // Depth still exact.
        assert!((f0.y - 380.0).abs() < 1e-3 && (f1.y - 380.0).abs() < 1e-3);
    }

    /// The small-front continuation is finite and clamped — no blow-up as the
    /// eye approaches the plane, and the wedge saturates smoothly to the strip.
    #[test]
    fn wedge_is_stable_near_the_plane_and_under_extreme_skew() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let max_lateral = 400.0;
        for eye in [
            Vec2::new(100.0, 299.5),  // 0.5px in front (limit branch)
            Vec2::new(100.0, 298.0),  // 2px in front (projective, clamped)
            Vec2::new(1000.0, 295.0), // extreme grazing skew from the right
        ] {
            let cone = aperture_wedge(&enter, &exit, eye, 80.0, max_lateral).unwrap();
            for p in cone.entry_quad.iter().chain(cone.source_quad.iter()) {
                assert!(p.x.is_finite() && p.y.is_finite(), "finite corners");
                assert!(
                    (p.x - enter.pos.x).abs() <= max_lateral + 1e-2
                        || (*p - exit.pos).length() <= max_lateral + 80.0 + 1e-2,
                    "within the clamp envelope: {p:?}"
                );
            }
            // Far corners always land exactly max_depth behind the entry.
            assert!((cone.entry_quad[2].y - 380.0).abs() < 1e-3);
            assert!((cone.entry_quad[3].y - 380.0).abs() < 1e-3);
        }
    }

    /// The far edge sits exactly `max_depth` behind the surface, and a head-on
    /// viewer yields a laterally-centered, symmetric wedge wider than the
    /// aperture (perspective through the slit).
    #[test]
    fn visible_cone_head_on_is_symmetric_and_depth_clamped() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let depth = 80.0;
        // Eye 80px in front of the floor portal (−y), directly above center.
        let front = 80.0;
        let cone = visible_cone(&enter, &exit, Vec2::new(100.0, 300.0 - front), depth, 400.0).unwrap();
        let [a0, a1, f1, f0] = cone.entry_quad;
        // Near edge is the aperture (on the surface, y = 300).
        assert!((a0.y - 300.0).abs() < 1e-3 && (a1.y - 300.0).abs() < 1e-3);
        // Far corners sit exactly `depth` behind (into the floor, +y).
        assert!((f0.y - (300.0 + depth)).abs() < 1e-3, "{f0:?}");
        assert!((f1.y - (300.0 + depth)).abs() < 1e-3, "{f1:?}");
        // Head-on ⇒ far edge centered on the aperture center (x=100) and wider
        // than the aperture by (1 + depth/front).
        let h = enter.aperture_half();
        assert!(((f0.x + f1.x) * 0.5 - 100.0).abs() < 1e-3, "centered");
        let far_half = (f1.x - f0.x).abs() * 0.5;
        assert!(
            (far_half - h * (1.0 + depth / front)).abs() < 1e-3,
            "far_half {far_half} vs {}",
            h * (1.0 + depth / front)
        );
    }

    /// An off-axis viewer skews the wedge: through a slit you see the FAR side,
    /// away from you (looking from the left, the visible far edge shifts
    /// right). Pinned for BOTH a floor portal AND a ceiling portal (the
    /// magenta case) so the skew direction is identical regardless of which
    /// way the surface faces — a ceiling never inverts.
    #[test]
    fn visible_cone_skews_away_from_viewer_floor_and_ceiling() {
        // Floor (normal up): eye up-and-LEFT ⇒ far edge to the RIGHT.
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let cone = visible_cone(&enter, &exit, Vec2::new(40.0, 220.0), 80.0, 400.0).unwrap();
        let [_, _, f1, f0] = cone.entry_quad;
        assert!(
            (f0.x + f1.x) * 0.5 > 100.0,
            "floor: off-left viewer ⇒ far edge right, got {}",
            (f0.x + f1.x) * 0.5
        );
        // Ceiling (normal DOWN, +y): eye BELOW and to the LEFT ⇒ far edge still
        // to the RIGHT (consistent — no ceiling-specific inversion).
        let ceil = frame(Vec2::new(100.0, 300.0), Vec2::new(0.0, 1.0));
        let cone = visible_cone(&ceil, &exit, Vec2::new(40.0, 380.0), 80.0, 400.0).unwrap();
        let [_, _, f1, f0] = cone.entry_quad;
        assert!(
            (f0.x + f1.x) * 0.5 > 100.0,
            "ceiling: off-left viewer ⇒ far edge right, got {}",
            (f0.x + f1.x) * 0.5
        );
    }

    /// `R(copy_roll) ∘ flip_x` equals the body map for every pair class — the
    /// factorization that lets the sprite copy realize the transit map exactly
    /// (det −1) with only a rotation and a texture flip.
    #[test]
    fn copy_roll_flip_x_factors_the_body_map() {
        let pairs = [
            (Vec2::new(0.0, -1.0), Vec2::new(0.0, -1.0)), // floor↔floor
            (Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0)),  // ceiling↔floor
            (Vec2::new(1.0, 0.0), Vec2::new(1.0, 0.0)),   // same wall
            (Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0)),  // opposite walls
            (Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0)), // floor→wall (90°)
        ];
        for (n_in, n_out) in pairs {
            let enter = frame(Vec2::new(100.0, 300.0), n_in);
            let exit = frame(Vec2::new(500.0, 200.0), n_out);
            // World-space rotation angle is the negated render roll.
            let a = -copy_roll(&enter, &exit);
            let (s, c) = a.sin_cos();
            for v in [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(3.0, -2.0)] {
                // flip_x first, then rotate (render applies rotation ∘ flip).
                let f = Vec2::new(-v.x, v.y);
                let rotated = Vec2::new(f.x * c - f.y * s, f.x * s + f.y * c);
                let body = portal_map_vec(v, n_in, n_out);
                assert!(
                    (rotated - body).length() < 1e-4,
                    "{n_in:?}→{n_out:?}: R∘flip_x {rotated:?} vs body {body:?}"
                );
            }
        }
    }

    /// Zero spread degenerates to a straight corridor: source rect lateral
    /// extent equals the aperture.
    #[test]
    fn view_cone_zero_spread_is_a_corridor() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = floor(Vec2::new(500.0, 300.0));
        let cone = view_cone(&enter, &exit, 90.0, 0.0);
        assert!(
            (size(cone.source).x - 2.0 * enter.aperture_half()).abs() < 1e-3,
            "{:?}",
            size(cone.source)
        );
        assert!((size(cone.source).y - 90.0).abs() < 1e-3);
    }
}
