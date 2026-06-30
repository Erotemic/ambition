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
//!   actually emerge. The body map is orthogonal: under the rotation convention
//!   it is det +1 and factors as a rotation; under the reflection convention it
//!   is det −1 and factors as a rotation plus one texture flip. Sprites realize
//!   that exact factorization via [`copy_transform`], so window, copy, and
//!   transit are ONE map.
//! - **Projection** ([`PortalViewMap`] / [`view_point`]): the view protrudes
//!   into the room in front of the entry, hologram-style. Its map is the body
//!   map composed with a reflection across the entry plane, so its parity is the
//!   opposite of the body map. [`PortalViewMap`] stores the same rotation/flip
//!   factorization and applies it exactly.
//!
//! Like [`pieces`], this module is pure and allocation-light: no ECS, no
//! render types, no RNG. The renderer (`ambition_portal_presentation`) builds
//! its capture cameras and window UVs from [`view_cone`]; a roll-your-own
//! host consumes the same functions.

use ambition_engine_core as ae;
use bevy::math::Vec2;

use crate::pieces::{
    map_point, portal_map_vec, portal_map_vec_reflection, portal_map_vec_rotation, PortalFrame,
};

/// A 2D orthogonal transform factored the way Bevy sprites can draw it:
/// optional `flip_x`, then rotation by `(cos, sin)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OrthogonalFactor {
    cos: f32,
    sin: f32,
    flip_x: bool,
}

fn factor_orthogonal(col_x: Vec2, col_y: Vec2) -> OrthogonalFactor {
    let det = col_x.x * col_y.y - col_x.y * col_y.x;
    let flip_x = det < 0.0;
    let basis_x = if flip_x { -col_x } else { col_x };
    let angle = basis_x.y.atan2(basis_x.x);
    OrthogonalFactor {
        cos: angle.cos(),
        sin: angle.sin(),
        flip_x,
    }
}

/// The rigid/reflected map of the VIEW through a portal pair: optional `flip_x`,
/// then rotation `(cos, sin)` about the entry portal's center, then translation
/// onto the exit's. The flip is false under the reflection body convention and
/// true under the rotation body convention.
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
    /// Whether the map applies a local x-reflection before rotation.
    pub flip_x: bool,
}

impl PortalViewMap {
    fn between_with_map(
        enter: &PortalFrame,
        exit: &PortalFrame,
        map_vec: fn(Vec2, Vec2, Vec2) -> Vec2,
    ) -> Self {
        let lin = |v: Vec2| {
            // Reflect across the entry plane (linear part: across the surface
            // direction), then push through the body map.
            let reflected = v - 2.0 * v.dot(enter.normal) * enter.normal;
            map_vec(reflected, enter.normal, exit.normal)
        };
        let col_x = lin(Vec2::X);
        let col_y = lin(Vec2::Y);
        let factor = factor_orthogonal(col_x, col_y);
        Self {
            enter_pos: enter.pos,
            exit_pos: exit.pos,
            cos: factor.cos,
            sin: factor.sin,
            flip_x: factor.flip_x,
        }
    }

    /// The view map for a linked pair under the active game-wide convention:
    /// body map ∘ reflection across the entry plane.
    pub fn between(enter: &PortalFrame, exit: &PortalFrame) -> Self {
        Self::between_with_map(enter, exit, portal_map_vec)
    }

    /// Pure variant used by tests and convention-specific tools.
    pub fn between_for_convention(
        enter: &PortalFrame,
        exit: &PortalFrame,
        rotation_convention: bool,
    ) -> Self {
        let map_vec = if rotation_convention {
            portal_map_vec_rotation
        } else {
            portal_map_vec_reflection
        };
        Self::between_with_map(enter, exit, map_vec)
    }

    /// The exit-side world point whose light "comes through" the portal to the
    /// entry-side point `p`.
    pub fn apply(&self, p: Vec2) -> Vec2 {
        let mut v = p - self.enter_pos;
        if self.flip_x {
            v.x = -v.x;
        }
        self.exit_pos
            + Vec2::new(
                v.x * self.cos - v.y * self.sin,
                v.x * self.sin + v.y * self.cos,
            )
    }

    /// The rotation angle (radians) of the factored linear part.
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

/// A camera/viewpoint frame in portal world coordinates.
///
/// `rotation` is the 2D z-rotation in the same world-space convention as the
/// caller uses for the view basis. The helper below composes it with the shared
/// portal VIEW map so camera continuity, view windows, and body/copy math do
/// not grow separate angle-difference shortcuts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalViewpointFrame {
    pub pos: Vec2,
    pub rotation: f32,
}

/// Map a camera/viewpoint frame through a portal pair using the portal VIEW map.
///
/// This is intentionally tiny: it does not decide when a host camera should use
/// continuity, how long to blend, or what entity the camera follows. It only
/// exposes the same map that recursive windows use as a reusable pure helper for
/// presentation layers.
pub fn map_viewpoint_frame(
    frame: PortalViewpointFrame,
    enter: &PortalFrame,
    exit: &PortalFrame,
) -> PortalViewpointFrame {
    let map = PortalViewMap::between(enter, exit);
    PortalViewpointFrame {
        pos: map.apply(frame.pos),
        rotation: frame.rotation + map.angle(),
    }
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

/// Sprite transform for a portal body copy. Bevy applies `flip_x` in texture
/// space and then the transform rotation, so this factors the active BODY map
/// as exactly that pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalCopyTransform {
    /// Render-space z-rotation to add to the copied sprite.
    pub roll: f32,
    /// Whether the copied sprite should invert `Sprite::flip_x`.
    pub flip_x: bool,
}

fn copy_transform_with_map(
    enter: &PortalFrame,
    exit: &PortalFrame,
    map_vec: fn(Vec2, Vec2, Vec2) -> Vec2,
) -> PortalCopyTransform {
    let col_x = map_vec(Vec2::X, enter.normal, exit.normal);
    let col_y = map_vec(Vec2::Y, enter.normal, exit.normal);
    let factor = factor_orthogonal(col_x, col_y);
    PortalCopyTransform {
        roll: -factor.sin.atan2(factor.cos),
        flip_x: factor.flip_x,
    }
}

/// Pure variant used by tests and convention-specific tools.
pub fn copy_transform_for_convention(
    enter: &PortalFrame,
    exit: &PortalFrame,
    rotation_convention: bool,
) -> PortalCopyTransform {
    let map_vec = if rotation_convention {
        portal_map_vec_rotation
    } else {
        portal_map_vec_reflection
    };
    copy_transform_with_map(enter, exit, map_vec)
}

/// Sprite transform for a portal body copy under the active game-wide map
/// convention.
pub fn copy_transform(enter: &PortalFrame, exit: &PortalFrame) -> PortalCopyTransform {
    copy_transform_with_map(enter, exit, portal_map_vec)
}

/// Backward-compatible shorthand for callers that only need the roll.
pub fn copy_roll(enter: &PortalFrame, exit: &PortalFrame) -> f32 {
    copy_transform(enter, exit).roll
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
/// As `front → 0` that diverges. If the eye is laterally inside the aperture
/// span, the limit shape is the full half-plane strip of depth `max_depth`.
/// If the eye is off to the side, the limit is a one-sided grazing cone, not a
/// full strip. So the near-plane branch only switches to the half-plane for
/// eyes inside the finite aperture; other eyes use the projective formula with
/// a minimum denominator and clamp the lateral offset to ±`max_lateral`. The
/// wedge therefore grows smoothly into the bounded half-plane only as the
/// viewer reaches the portal opening — no blow-up, no NaN, and a capture rect a
/// fixed-size texture can actually frame.
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
    aperture_wedge_multi(enter, exit, &[eye], max_depth, max_lateral)
}

/// The wedge a SET of eyes jointly sees through the aperture: the UNION of each
/// in-front eye's wedge, as one trapezoid whose far edge spans the combined
/// lateral extent. The near edge is always the aperture (exactly on the
/// surface), so the window is anchored at the portal face regardless of the
/// viewpoints.
///
/// Why a set: a body STRADDLING a portal has presence at both ends — its real
/// AABB corners AND the "shadow" corners the sprite trick maps through. Feeding
/// both makes the wedge a continuous function of position (as a corner crosses
/// the plane its real contribution hands off to its shadow), which removes the
/// abrupt flip when the viewer passes the midpoint between a pair (the eye no
/// longer hard-switches direct↔wormhole). Eyes behind the plane contribute
/// nothing; `None` only when EVERY eye is behind.
pub fn aperture_wedge_multi(
    enter: &PortalFrame,
    exit: &PortalFrame,
    eyes: &[Vec2],
    max_depth: f32,
    max_lateral: f32,
) -> Option<ViewCone> {
    let n = enter.normal;
    let t = Vec2::new(-n.y, n.x);
    let h = enter.aperture_half();
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &eye in eyes {
        let v = eye - enter.pos;
        let front = v.dot(n);
        if front <= 0.0 {
            continue;
        }
        let lat_eye = v.dot(t);
        let far_lat = |lat_a: f32| -> f32 {
            if front < MIN_FRONT && lat_eye.abs() <= h {
                // Limit continuation: on the plane, both endpoint rays are
                // parallel to the surface only when the eye is inside the
                // aperture span, so the visible set is the entire half-plane
                // behind the aperture. Give each aperture endpoint its own side
                // of the lateral clamp; the renderer clips the oversized strip
                // back to the current viewport/world rect.
                lat_a.signum() * max_lateral
            } else {
                let front = front.max(MIN_FRONT);
                (lat_a + (lat_a - lat_eye) * (max_depth / front)).clamp(-max_lateral, max_lateral)
            }
        };
        for &lat_a in &[-h, h] {
            let fl = far_lat(lat_a);
            lo = lo.min(fl);
            hi = hi.max(fl);
        }
    }
    if !lo.is_finite() {
        return None; // every eye behind the plane
    }
    let a0 = enter.pos - t * h;
    let a1 = enter.pos + t * h;
    let f0 = enter.pos + t * lo - n * max_depth;
    let f1 = enter.pos + t * hi - n * max_depth;
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

    /// For every axis-aligned (enter, exit) normal pair, the view projection
    /// factors exactly into optional `flip_x` plus a rotation. Reflection-body
    /// convention yields rotation-only projection; rotation-body convention
    /// yields a reflected projection.
    #[test]
    fn view_map_factorization_matches_each_convention() {
        let normals = [
            Vec2::new(0.0, -1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(1.0, 0.0),
        ];
        for rotation_convention in [false, true] {
            let map_vec = if rotation_convention {
                portal_map_vec_rotation
            } else {
                portal_map_vec_reflection
            };
            for n_in in normals {
                for n_out in normals {
                    let enter = frame(Vec2::new(100.0, 300.0), n_in);
                    let exit = frame(Vec2::new(700.0, 140.0), n_out);
                    let m =
                        PortalViewMap::between_for_convention(&enter, &exit, rotation_convention);
                    assert!(
                        (m.cos * m.cos + m.sin * m.sin - 1.0).abs() < 1e-4,
                        "unit factor for {n_in:?}→{n_out:?}: cos {} sin {}",
                        m.cos,
                        m.sin
                    );
                    assert_eq!(m.flip_x, rotation_convention);
                    for v in [Vec2::X, Vec2::Y, Vec2::new(3.0, -2.0)] {
                        let reflected = v - 2.0 * v.dot(enter.normal) * enter.normal;
                        let expected = map_vec(reflected, enter.normal, exit.normal);
                        let got = m.apply(enter.pos + v) - exit.pos;
                        assert!(
                            (got - expected).length() < 1e-4,
                            "projection factor mismatch convention={rotation_convention} {n_in:?}→{n_out:?}: {got:?} vs {expected:?}"
                        );
                    }
                }
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

    #[test]
    fn near_plane_eye_outside_aperture_is_not_full_half_plane() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let max_lateral = 400.0;
        let cone =
            aperture_wedge(&enter, &exit, Vec2::new(300.0, 299.5), 80.0, max_lateral).unwrap();
        let [_, _, f1, f0] = cone.entry_quad;

        assert!(
            f0.x < enter.pos.x - enter.aperture_half()
                && f1.x < enter.pos.x - enter.aperture_half(),
            "near-plane eye to the right of the aperture should see a left-skewed grazing cone, got {f0:?} {f1:?}",
        );
        assert!(
            !((f0.x - (enter.pos.x - max_lateral)).abs() < 1e-3
                && (f1.x - (enter.pos.x + max_lateral)).abs() < 1e-3),
            "off-aperture near-plane eyes must not receive the centered full half-plane",
        );
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
        let cone =
            visible_cone(&enter, &exit, Vec2::new(100.0, 300.0 - front), depth, 400.0).unwrap();
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

    /// The sprite-copy factorization equals the body map for every pair class
    /// under both map conventions.
    #[test]
    fn copy_transform_factors_the_body_map() {
        let pairs = [
            (Vec2::new(0.0, -1.0), Vec2::new(0.0, -1.0)), // floor↔floor
            (Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0)),  // ceiling↔floor
            (Vec2::new(1.0, 0.0), Vec2::new(1.0, 0.0)),   // same wall
            (Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0)),  // opposite walls
            (Vec2::new(0.0, -1.0), Vec2::new(-1.0, 0.0)), // floor→wall (90°)
        ];
        for rotation_convention in [false, true] {
            let map_vec = if rotation_convention {
                portal_map_vec_rotation
            } else {
                portal_map_vec_reflection
            };
            for (n_in, n_out) in pairs {
                let enter = frame(Vec2::new(100.0, 300.0), n_in);
                let exit = frame(Vec2::new(500.0, 200.0), n_out);
                let copy = copy_transform_for_convention(&enter, &exit, rotation_convention);
                assert_eq!(copy.flip_x, !rotation_convention);
                // World-space rotation angle is the negated render roll.
                let a = -copy.roll;
                let (s, c) = a.sin_cos();
                for v in [
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 1.0),
                    Vec2::new(3.0, -2.0),
                ] {
                    let f = if copy.flip_x { Vec2::new(-v.x, v.y) } else { v };
                    let rotated = Vec2::new(f.x * c - f.y * s, f.x * s + f.y * c);
                    let body = map_vec(v, n_in, n_out);
                    assert!(
                        (rotated - body).length() < 1e-4,
                        "{n_in:?}→{n_out:?} convention={rotation_convention}: copy {rotated:?} vs body {body:?}"
                    );
                }
            }
        }
    }

    /// The multi-eye wedge is the UNION of its eyes' wedges (far edge spans the
    /// combined lateral extent), the near edge stays exactly on the aperture,
    /// and an eye behind the plane contributes nothing.
    #[test]
    fn multi_eye_wedge_unions_and_anchors_at_the_aperture() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        let left = Vec2::new(40.0, 250.0);
        let right = Vec2::new(160.0, 250.0);
        let one_l = aperture_wedge(&enter, &exit, left, 80.0, 400.0).unwrap();
        let one_r = aperture_wedge(&enter, &exit, right, 80.0, 400.0).unwrap();
        let both = aperture_wedge_multi(&enter, &exit, &[left, right], 80.0, 400.0).unwrap();
        // Near edge unchanged (exactly the aperture, on the surface y=300).
        assert!((both.entry_quad[0].y - 300.0).abs() < 1e-3);
        assert!((both.entry_quad[1].y - 300.0).abs() < 1e-3);
        // Far edge spans the union: at least as wide as either single wedge.
        let span = |c: &ViewCone| (c.entry_quad[2].x - c.entry_quad[3].x).abs();
        assert!(span(&both) >= span(&one_l) - 1e-3 && span(&both) >= span(&one_r) - 1e-3);
        // An extra eye BEHIND the plane (below the floor) changes nothing.
        let with_behind = aperture_wedge_multi(
            &enter,
            &exit,
            &[left, right, Vec2::new(100.0, 360.0)],
            80.0,
            400.0,
        )
        .unwrap();
        assert!((span(&with_behind) - span(&both)).abs() < 1e-3);
    }

    /// Continuity across the partner plane — the reason for the eye set. As a
    /// viewpoint crosses the entry plane, swapping it for its mapped shadow on
    /// the far side leaves the far edge essentially unchanged (no abrupt flip).
    #[test]
    fn wedge_far_edge_is_continuous_through_the_plane() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = floor(Vec2::new(500.0, 300.0));
        let far_lat = |eyes: &[Vec2]| {
            let c = aperture_wedge_multi(&enter, &exit, eyes, 80.0, 1000.0).unwrap();
            (c.entry_quad[2].x + c.entry_quad[3].x) * 0.5
        };
        // Eye just in FRONT of the entry plane (y just < 300).
        let just_front = far_lat(&[Vec2::new(120.0, 299.0)]);
        // The SAME eye one tick later just BEHIND, replaced by its shadow mapped
        // from the partner (map_point(behind-entry → front-of-exit), then that
        // shadow viewed from `enter` is its partner image) — here we approximate
        // continuity by the near-plane limit being shared.
        let near_plane = far_lat(&[Vec2::new(120.0, 299.9)]);
        assert!(
            (just_front - near_plane).abs() < 60.0,
            "far edge moves smoothly near the plane: {just_front} vs {near_plane}"
        );
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
