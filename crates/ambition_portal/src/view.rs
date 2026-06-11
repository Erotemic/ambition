//! Pure through-portal **view** geometry — what a viewer looking into one
//! portal sees of the world at its partner.
//!
//! Two display models, sharing the same source region (the world in FRONT of
//! the exit portal):
//!
//! - **Window** ([`ViewCone`] / [`view_cone`] — what the default renderer
//!   ships): the view recedes INTO the entry's host surface, like glass set in
//!   the wall — you see "through the portal a little bit." A window's display
//!   map is the plain BODY map ([`map_point`]): depth `d` into the entry wall
//!   shows depth `d` out in front of the exit. Sight lines and transiting
//!   bodies share ONE map, so the window image and an emerging body agree at
//!   the face by construction.
//! - **Projection** ([`PortalViewMap`] / [`view_point`]): the view protrudes
//!   into the room in front of the entry, hologram-style. Its map is the body
//!   map composed with a reflection across the entry plane, which yields a
//!   small theorem: the body map always sends the orientation −1 frame
//!   `(-n_in, t_in)` onto the orientation +1 frame `(n_out, t_out)` (det −1,
//!   always a reflection), so the PROJECTION map is always a PROPER rotation
//!   (det +1) — a host drawing this model can orient a camera by
//!   [`PortalViewMap::angle`] with no flip case, pinned for every axis-aligned
//!   pair below. (For the window model the mirror lives harmlessly in UV
//!   space, so the theorem is not needed there.)
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
/// little way), displaying the world in front of the exit. The display map for
/// a window is the plain BODY map ([`map_point`]): depth `d` into the entry
/// wall shows depth `d` out in front of the exit — sight lines and transiting
/// bodies share one map, so the view and an emerging body can never disagree.
/// (The body map is orientation-reversing; for a textured mesh that is just a
/// UV-space mirror, costing nothing. [`PortalViewMap`] above remains the
/// camera-orientation tool for hosts that want a protruding-projection look.)
///
/// Corner order is `[near_a, near_b, far_b, far_a]` — near edge ON the face
/// (lateral ∓ aperture), far edge `depth` INTO the wall (lateral widened by
/// `spread * depth` per side) — so `(0,1,2) (0,2,3)` triangulates it with
/// consistent winding.
#[derive(Clone, Copy, Debug)]
pub struct ViewCone {
    /// Trapezoid corners at the ENTRY portal (face + into-the-wall), world space.
    pub entry_quad: [Vec2; 4],
    /// The same corners pushed through the body map: the exit-side world quad
    /// the window displays. `source_quad[i]` is what `entry_quad[i]` shows — a
    /// renderer derives per-vertex UVs by normalizing these inside [`Self::source`].
    pub source_quad: [Vec2; 4],
    /// Axis-aligned bounds of `source_quad`: the world rect (in FRONT of the
    /// exit) a capture camera must frame. Axis-aligned exactly (not just
    /// bounding) for axis-aligned portals, since the body map's linear part is
    /// then axis-aligned.
    pub source: ae::Aabb,
}

/// Build a [`ViewCone`] from its four entry-side corners: the source quad is
/// the corners through the body [`map_point`], the source rect their bounds.
/// One place that defines the display map, shared by every cone constructor.
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

/// The effective eye for looking into `enter`, given the controlled
/// character's real `eye`. A portal pair glues two surfaces into one window, so
/// the character can look into `enter` two ways:
///
/// - **directly**, when in front of `enter` (`front > 0`) — returns the real
///   eye, `wormhole = false`;
/// - **through the pair**, when in front of the partner `exit`: standing in
///   front of `exit` is, topologically, standing in front of `enter`, so the
///   eye's image is `map_point(eye, exit, enter)` (front-preserving) — returns
///   that, `wormhole = true`.
///
/// This is why a portal shows a cone even when you stand at its *partner*: above
/// `purple` you are equally "in" `yellow`, so `yellow`'s window opens. `None`
/// only when the character is behind BOTH ends. The `wormhole` flag tells the
/// caller which aperture to run line-of-sight against (the one actually faced).
pub fn effective_eye(enter: &PortalFrame, exit: &PortalFrame, eye: Vec2) -> Option<(Vec2, bool)> {
    if (eye - enter.pos).dot(enter.normal) > 1.0 {
        return Some((eye, false));
    }
    // In front of the partner? The eye's image through the pair appears in
    // front of `enter`. Use the front-preserving VIEW map (`view_point`), not
    // the body `map_point` (which sends front↔back) — standing in front of the
    // partner must put the image in FRONT of this end, not inside its wall.
    if (eye - exit.pos).dot(exit.normal) > 1.0 {
        return Some((view_point(eye, exit, enter), true));
    }
    None
}

/// The viewer-dependent wedge through the aperture, given an `eye` already
/// **in front of** `enter` (use [`effective_eye`] to resolve it). Treat the
/// aperture as a slit: a point behind the surface is visible iff the sight line
/// `eye → P` crosses it, so the region is the wedge bounded by the rays from
/// `eye` through the aperture endpoints, clipped to `max_depth` deep. Each far
/// corner is its endpoint pushed away from the eye to exactly `max_depth`:
///
/// > `F = A + (max_depth / front) · (A − eye)`,  `front = (eye − pos)·n`
///
/// so the wedge **skews with the viewer's angle** and **widens as the viewer
/// nears** the portal. `None` if `eye` is not in front (`front ≤ 0`). Pure
/// geometry — line-of-sight occlusion is the caller's check.
pub fn aperture_wedge(
    enter: &PortalFrame,
    exit: &PortalFrame,
    eye: Vec2,
    max_depth: f32,
) -> Option<ViewCone> {
    let n = enter.normal;
    let front = (eye - enter.pos).dot(n);
    if front <= 1.0 {
        return None;
    }
    // Clamp the effective front off zero: as the eye nears the aperture plane
    // the wedge blows up toward a half-plane (`k → ∞`) — a huge, fuzzy source
    // rect and numerically unstable. A small floor bounds the spread.
    let front = front.max(8.0);
    let t = Vec2::new(-n.y, n.x);
    let h = enter.aperture_half();
    let a0 = enter.pos - t * h;
    let a1 = enter.pos + t * h;
    let k = max_depth / front;
    let f0 = a0 + (a0 - eye) * k;
    let f1 = a1 + (a1 - eye) * k;
    Some(from_entry_quad([a0, a1, f1, f0], enter, exit))
}

/// Convenience: [`effective_eye`] (so it works from either end of the pair)
/// then [`aperture_wedge`]. `None` only when the viewer is behind both ends.
pub fn visible_cone(
    enter: &PortalFrame,
    exit: &PortalFrame,
    eye: Vec2,
    max_depth: f32,
) -> Option<ViewCone> {
    let (eye, _) = effective_eye(enter, exit, eye)?;
    aperture_wedge(enter, exit, eye, max_depth)
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

/// Push a cone's far corners so each is at least `min_depth` behind the surface
/// — the minimum-cone floor, so even a grazing/near-aligned wedge never reads
/// as a flat sliver. Leaves the lateral (skew) of each corner intact.
pub fn floor_cone_depth(
    cone: &ViewCone,
    enter: &PortalFrame,
    exit: &PortalFrame,
    min_depth: f32,
) -> ViewCone {
    let n = enter.normal;
    let mut entry_quad = cone.entry_quad;
    for i in [2, 3] {
        let p = entry_quad[i];
        let depth = (p - enter.pos).dot(-n); // into the wall is positive
        if depth < min_depth {
            entry_quad[i] = p + (-n) * (min_depth - depth);
        }
    }
    from_entry_quad(entry_quad, enter, exit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pieces::front_distance;
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

    /// Behind BOTH ends ⇒ `None` (two floors, eye below both).
    #[test]
    fn visible_cone_none_behind_both_ends() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = floor(Vec2::new(500.0, 300.0));
        // Floors face up (−y); a viewer BELOW both (y > 300) is behind each.
        assert!(visible_cone(&enter, &exit, Vec2::new(100.0, 360.0), 80.0).is_none());
    }

    /// The wormhole: standing in front of the PARTNER opens this end's window
    /// even though the eye is behind this surface (above purple ⇒ yellow shows).
    #[test]
    fn visible_cone_opens_from_the_partner_side() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        // Eye is BEHIND the floor (y > 300) but in FRONT of the wall partner
        // (x < 400) — only the wormhole opens this end.
        let eye = Vec2::new(100.0, 360.0);
        let (resolved, wormhole) = effective_eye(&enter, &exit, eye).expect("in front of partner");
        assert!(wormhole, "resolved via the partner side");
        // The image is in front of `enter` (above the floor, y < 300).
        assert!(resolved.y < 300.0, "image in front of enter: {resolved:?}");
        assert!(visible_cone(&enter, &exit, eye, 80.0).is_some());
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
        let cone = visible_cone(&enter, &exit, Vec2::new(100.0, 300.0 - front), depth).unwrap();
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

    /// An off-axis viewer skews the wedge: looking from the left, the far edge
    /// shifts right (you see more of the far side through the slit).
    #[test]
    fn visible_cone_skews_with_viewer_angle() {
        let enter = floor(Vec2::new(100.0, 300.0));
        let exit = right_wall(Vec2::new(400.0, 200.0));
        // Eye up and to the LEFT of center.
        let cone = visible_cone(&enter, &exit, Vec2::new(40.0, 220.0), 80.0).unwrap();
        let [_, _, f1, f0] = cone.entry_quad;
        // Far edge center is pushed to the RIGHT of the aperture center (x=100).
        assert!(
            (f0.x + f1.x) * 0.5 > 100.0,
            "off-left viewer pushes the far edge right, got {}",
            (f0.x + f1.x) * 0.5
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
