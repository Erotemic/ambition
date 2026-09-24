//! Pure through-portal view geometry.
//!
//! Window rendering uses the same body map as transit, preserving its parity and
//! transform. Projection view composes that map with reflection across the entry
//! plane. `PortalViewMap` stores the resulting rotation/flip factorization for
//! presentation consumers.

use ambition_platformer2d_core as ae;
use bevy::math::Vec2;

use crate::pieces::{
    map_point, portal_map_vec, portal_map_vec_reflection, portal_map_vec_rotation, MapConvention,
    PortalAperture, PortalFrame,
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

/// The rigid or reflected map of the view through a portal pair: optional `flip_x`,
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
        map_vec: impl Fn(Vec2, Vec2, Vec2) -> Vec2,
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

    /// The view map for a linked pair under a stated convention: body map ∘
    /// reflection across the entry plane.
    pub fn between(enter: &PortalFrame, exit: &PortalFrame, convention: MapConvention) -> Self {
        Self::between_with_map(enter, exit, move |v, n_in, n_out| {
            portal_map_vec(v, n_in, n_out, convention)
        })
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
/// Shorthand for [`PortalViewMap::between`] then `apply`; equals
/// `map_point(reflect(p))`.
pub fn view_point(
    p: Vec2,
    enter: &PortalFrame,
    exit: &PortalFrame,
    convention: MapConvention,
) -> Vec2 {
    PortalViewMap::between(enter, exit, convention).apply(p)
}

/// A camera/viewpoint frame in portal world coordinates.
///
/// `rotation` is the 2D z-rotation, in the caller's world-space convention.
/// `map_viewpoint_frame` composes it with the shared portal view map, so
/// cameras, view windows, and body copies use one map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalViewpointFrame {
    pub pos: Vec2,
    pub rotation: f32,
}

/// Map a camera or viewpoint frame through a portal pair with the portal view
/// map (the same map recursive windows use). Camera policy (when to use it,
/// blending, follow target) belongs to the host.
pub fn map_viewpoint_frame(
    frame: PortalViewpointFrame,
    enter: &PortalFrame,
    exit: &PortalFrame,
    convention: MapConvention,
) -> PortalViewpointFrame {
    let map = PortalViewMap::between(enter, exit, convention);
    PortalViewpointFrame {
        pos: map.apply(frame.pos),
        rotation: frame.rotation + map.angle(),
    }
}

/// The view cone of one portal: a trapezoid from the entry face into the host
/// surface, showing the world in front of the exit through the body
/// [`map_point`]. This is the transit map, so the window agrees with where
/// bodies emerge; the sprite copy uses the same map through [`copy_roll`].
///
/// Corner order is `[near_a, near_b, far_b, far_a]`: the near edge is on the
/// face, the far edge is `depth` into the wall and wider by `spread * depth`
/// per side. `(0,1,2) (0,2,3)` triangulates it with consistent winding.
#[derive(Clone, Copy, Debug)]
pub struct ViewCone {
    /// Trapezoid corners at the ENTRY portal (face + into-the-wall), world space.
    pub entry_quad: [Vec2; 4],
    /// The same corners through the body [`map_point`]: the exit-side quad the
    /// window shows. `source_quad[i]` is what `entry_quad[i]` shows; a renderer
    /// gets per-vertex UVs by normalizing these inside [`Self::source`].
    pub source_quad: [Vec2; 4],
    /// Axis-aligned bounds of `source_quad`: the world rect in front of the
    /// exit that a capture camera must frame. Exact for axis-aligned portals.
    pub source: ae::Aabb,
}

/// Sprite transform for a portal body copy. Bevy applies `flip_x` in texture
/// space and then the rotation, so this factors the body map into that pair.
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
    map_vec: impl Fn(Vec2, Vec2, Vec2) -> Vec2,
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

/// Sprite transform for a portal body copy under a stated map convention.
pub fn copy_transform(
    enter: &PortalFrame,
    exit: &PortalFrame,
    convention: MapConvention,
) -> PortalCopyTransform {
    copy_transform_with_map(enter, exit, move |v, n_in, n_out| {
        portal_map_vec(v, n_in, n_out, convention)
    })
}

/// Backward-compatible shorthand for callers that only need the roll.
pub fn copy_roll(enter: &PortalFrame, exit: &PortalFrame, convention: MapConvention) -> f32 {
    copy_transform(enter, exit, convention).roll
}

/// Build a [`ViewCone`] from its four entry-side corners. Every cone
/// constructor uses this, so the display map is defined in one place.
fn from_entry_quad(
    entry_quad: [Vec2; 4],
    enter: &PortalAperture,
    exit: &PortalAperture,
    convention: MapConvention,
) -> ViewCone {
    let source_quad = entry_quad.map(|p| map_point(p, &enter.frame, &exit.frame, convention));
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

/// Viewer-independent: the minimum cone that always shows (see
/// [`blend_cones`]).
pub fn view_cone(
    enter: &PortalAperture,
    exit: &PortalAperture,
    depth: f32,
    spread: f32,
    convention: MapConvention,
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
        convention,
    )
}

/// Smallest front distance treated as "cleanly in front" of a surface.
const MIN_FRONT: f32 = 1.0;
/// In-doorway grace, lateral: how far outside the aperture span the eye may sit
/// and still count as "in the doorway" of that end.
const DOORWAY_LATERAL_GRACE: f32 = 26.0;
/// In-doorway grace, depth: how far the eye may go behind the surface while
/// transiting and still count as in the doorway. The centroid transfer fires
/// soon after the plane crossing, so a small value is enough.
const DOORWAY_DEPTH_GRACE: f32 = 24.0;

/// Half-width (px, in end-distance difference) of the band around a pair's
/// equidistance midpoint where [`window_eye`] crossfades the two ends' eyes
/// instead of switching. Sized like the doorway grace: wide enough that no
/// frame jumps, narrow enough that the blend is quick.
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
        // In the doorway: lift to just in front. The wedge's limit case
        // turns this into the half-plane.
        MIN_FRONT * 0.5
    } else {
        return None;
    };
    Some(end.frame.origin + n * front + t * lat)
}

/// The effective eye for looking into `enter`, given the controlled
/// character's real `eye`. A portal pair joins two surfaces into one window,
/// so the character can look into `enter` from in front of either end:
/// directly, or through the pair. The image of the eye uses the
/// front-preserving [`view_point`], not the body map.
///
/// If only one end resolves, it wins. If both do (e.g. two floor portals on
/// one plane), outside the [`EYE_HANDOFF_BAND`] the nearer end wins, and
/// inside it the two eyes crossfade.
///
/// In-doorway grace: while transiting, the eye goes just behind the plane of
/// the end it passes through, and the window should show a near half-plane. An
/// eye within the aperture span (plus [`DOORWAY_LATERAL_GRACE`]) and within
/// [`DOORWAY_DEPTH_GRACE`] of the plane is lifted to just in front of it.
/// `None` only when the eye is behind both ends and in neither doorway.
pub fn window_eye(
    enter: &PortalAperture,
    exit: &PortalAperture,
    eye: Vec2,
    convention: MapConvention,
) -> Option<(Vec2, bool)> {
    let direct = resolve_end_front(enter, eye);
    let via =
        resolve_end_front(exit, eye).map(|r| view_point(r, &exit.frame, &enter.frame, convention));
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
/// In the (normal, tangent) frame each far corner sits at depth exactly `max_depth` with
/// lateral offset `lat_A + (lat_A − lat_eye)·(max_depth/front)`. As `front → 0` that diverges.
/// If the eye is laterally inside the aperture span, the limit shape is the full half-plane
/// strip of depth `max_depth`. If the eye is off to the side, the limit is a one-sided grazing
/// cone, not a full strip. So the near-plane branch only switches to the half-plane for eyes
/// inside the finite aperture; other eyes use the projective formula with a minimum denominator
/// and clamp the lateral offset to ±`max_lateral`.
///
/// `None` if `eye` is behind the plane. Pure geometry — line-of-sight
/// occlusion is the caller's check.
#[allow(clippy::too_many_arguments)]
pub fn aperture_wedge(
    enter: &PortalAperture,
    exit: &PortalAperture,
    eye: Vec2,
    max_depth: f32,
    max_lateral: f32,
    convention: MapConvention,
) -> Option<ViewCone> {
    aperture_wedge_multi(enter, exit, &[eye], max_depth, max_lateral, convention)
}

/// The wedge a set of eyes sees through the aperture: the union of each
/// in-front eye's wedge, as one trapezoid. The near edge is always the
/// aperture on the surface.
///
/// A body that straddles a portal is present at both ends (its real AABB
/// corners and the mapped "shadow" corners). Using both keeps the wedge
/// continuous as the body moves, with no sudden flip at the pair midpoint.
/// Eyes behind the plane add nothing; `None` only when every eye is behind.
pub fn aperture_wedge_multi(
    enter: &PortalAperture,
    exit: &PortalAperture,
    eyes: &[Vec2],
    max_depth: f32,
    max_lateral: f32,
    convention: MapConvention,
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
                // Limit case: an eye on the plane inside the aperture span sees
                // the whole half-plane behind it. Give each endpoint its own
                // side of the lateral clamp; the renderer clips the strip.
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
    Some(from_entry_quad([a0, a1, f1, f0], enter, exit, convention))
}

/// Convenience: [`window_eye`] (so it works from either end of the pair, with
/// the in-doorway grace) then [`aperture_wedge`]. `None` only when the viewer
/// is behind both ends and in neither doorway.
#[allow(clippy::too_many_arguments)]
pub fn visible_cone(
    enter: &PortalAperture,
    exit: &PortalAperture,
    eye: Vec2,
    max_depth: f32,
    max_lateral: f32,
    convention: MapConvention,
) -> Option<ViewCone> {
    let (eye, _) = window_eye(enter, exit, eye, convention)?;
    aperture_wedge(enter, exit, eye, max_depth, max_lateral, convention)
}

/// Per-corner linear blend `a → b` by `t ∈ [0,1]`. With `a` the minimum cone
/// and `b` the viewer wedge, both share the near edge, so the blend only opens
/// the far edge.
pub fn blend_cones(
    a: &ViewCone,
    b: &ViewCone,
    t: f32,
    enter: &PortalAperture,
    exit: &PortalAperture,
    convention: MapConvention,
) -> ViewCone {
    let t = t.clamp(0.0, 1.0);
    let entry_quad = std::array::from_fn(|i| a.entry_quad[i].lerp(b.entry_quad[i], t));
    from_entry_quad(entry_quad, enter, exit, convention)
}

#[cfg(test)]
mod tests;
