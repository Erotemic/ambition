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
    map_point, portal_map_vec, portal_map_vec_reflection, portal_map_vec_rotation, PortalAperture,
    PortalFrame,
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
            enter_pos: enter.origin,
            exit_pos: exit.origin,
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
fn from_entry_quad(
    entry_quad: [Vec2; 4],
    enter: &PortalAperture,
    exit: &PortalAperture,
) -> ViewCone {
    let source_quad = entry_quad.map(|p| map_point(p, &enter.frame, &exit.frame));
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
pub fn view_cone(
    enter: &PortalAperture,
    exit: &PortalAperture,
    depth: f32,
    spread: f32,
) -> ViewCone {
    let n = enter.frame.normal;
    let along = enter.frame.tangent();
    let near_half = enter.half_length;
    let far_half = near_half + depth * spread;
    let o = enter.frame.origin;
    from_entry_quad(
        [
            o - along * near_half,
            o + along * near_half,
            o + along * far_half - n * depth,
            o - along * far_half - n * depth,
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

/// Half-width (px, in end-distance difference) of the handoff band around a
/// pair's equidistance midpoint, over which [`window_eye`] CROSSFADES the two
/// ends' resolved eyes instead of hard-switching to the nearer end. Sized to
/// the doorway grace: the crossfade completes over a body-scale walk, quick
/// enough that the transitional wedge shapes barely register, wide enough that
/// no single frame jumps (the Q10.2 crossing pop).
const EYE_HANDOFF_BAND: f32 = 24.0;

/// Resolve `eye` against ONE portal end, in that end's own chart: the eye
/// itself when cleanly in front, the just-in-front lift when dipped into the
/// doorway (see [`window_eye`]'s in-doorway grace), `None` when genuinely
/// behind the surface.
fn resolve_end_front(end: &PortalAperture, eye: Vec2) -> Option<Vec2> {
    let n = end.frame.normal;
    let t = end.frame.tangent();
    let v = eye - end.frame.origin;
    let (front, lat) = (v.dot(n), v.dot(t));
    let in_doorway =
        lat.abs() <= end.half_length + DOORWAY_LATERAL_GRACE && front.abs() <= DOORWAY_DEPTH_GRACE;
    let front = if front >= MIN_FRONT {
        front
    } else if in_doorway {
        // At/inside the doorway: lift to a hair in front — the wedge's
        // limit continuation turns this into the half-plane.
        MIN_FRONT * 0.5
    } else {
        return None;
    };
    Some(end.frame.origin + n * front + t * lat)
}

/// The effective eye for looking into `enter`, given the controlled
/// character's real `eye`. A portal pair glues two surfaces into ONE window,
/// so the character can look into `enter` from in front of EITHER end —
/// directly, or through the pair (standing in front of the partner IS standing
/// in front of this end; the eye's image is the front-preserving
/// [`view_point`], never the front-flipping body map).
///
/// When only one end resolves, it wins outright. When the eye is in front of
/// BOTH ends — e.g. two floor portals share one plane, so a viewer above the
/// partner is "in front of" this end too, but 250px to the side — the ends'
/// resolutions are combined **nearest-weighted**: outside the
/// [`EYE_HANDOFF_BAND`] around the equidistance midpoint that is exactly the
/// nearer end (the honest window comes from the partner-side image right above
/// the aperture, not from the grazing direct ray), and inside the band the two
/// resolved eyes crossfade. A hard nearest-pick jumped discontinuously the
/// frame the nearer end flipped (thin-wall crossing, walking between a
/// same-plane pair) and the whole wedge popped with it; the face-continuity of
/// the doorway lift means the two resolutions nearly coincide at a thin wall's
/// midpoint, so the crossfade removes the pop (review Q10.2). The reported
/// wormhole flag stays the discrete nearest end.
///
/// **In-doorway grace:** while transiting, the eye dips just BEHIND the plane
/// of the end it is passing through; visually the character is *in* the
/// window, which should read as a (near) half-plane, not vanish. An eye within
/// the aperture span (+[`DOORWAY_LATERAL_GRACE`]) and within
/// [`DOORWAY_DEPTH_GRACE`] of the plane is lifted to just in front of it —
/// [`aperture_wedge`]'s small-front continuation then yields the half-plane
/// limit. `None` only when the eye is behind both ends and in neither doorway.
pub fn window_eye(
    enter: &PortalAperture,
    exit: &PortalAperture,
    eye: Vec2,
) -> Option<(Vec2, bool)> {
    let direct = resolve_end_front(enter, eye);
    let via = resolve_end_front(exit, eye).map(|r| view_point(r, &exit.frame, &enter.frame));
    match (direct, via) {
        (None, None) => None,
        (Some(d), None) => Some((d, false)),
        (None, Some(v)) => Some((v, true)),
        (Some(d), Some(v)) => {
            // 0 = all-direct, 1 = all-via-partner, 0.5 at equidistance. Both
            // inputs have front ≥ MIN_FRONT/2 of `enter`, and the front
            // coordinate is affine, so every blend stays cleanly in front.
            let gap = eye.distance(enter.frame.origin) - eye.distance(exit.frame.origin);
            let t = (gap / EYE_HANDOFF_BAND * 0.5 + 0.5).clamp(0.0, 1.0);
            Some((d.lerp(v, t), t > 0.5))
        }
    }
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
    enter: &PortalAperture,
    exit: &PortalAperture,
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
    enter: &PortalAperture,
    exit: &PortalAperture,
    eyes: &[Vec2],
    max_depth: f32,
    max_lateral: f32,
) -> Option<ViewCone> {
    let n = enter.frame.normal;
    let t = enter.frame.tangent();
    let h = enter.half_length;
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &eye in eyes {
        let v = eye - enter.frame.origin;
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
    let o = enter.frame.origin;
    let a0 = o - t * h;
    let a1 = o + t * h;
    let f0 = o + t * lo - n * max_depth;
    let f1 = o + t * hi - n * max_depth;
    Some(from_entry_quad([a0, a1, f1, f0], enter, exit))
}

/// Convenience: [`window_eye`] (so it works from either end of the pair, with
/// the in-doorway grace) then [`aperture_wedge`]. `None` only when the viewer
/// is behind both ends and in neither doorway.
pub fn visible_cone(
    enter: &PortalAperture,
    exit: &PortalAperture,
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
    enter: &PortalAperture,
    exit: &PortalAperture,
) -> ViewCone {
    let t = t.clamp(0.0, 1.0);
    let entry_quad = std::array::from_fn(|i| a.entry_quad[i].lerp(b.entry_quad[i], t));
    from_entry_quad(entry_quad, enter, exit)
}

#[cfg(test)]
mod tests;
