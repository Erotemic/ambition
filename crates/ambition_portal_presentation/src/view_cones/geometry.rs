//! Portal view-cone geometry pipeline: the rebuild key, aperture line-of-sight
//! + occlusion, the clipped cone polygon, and its UV/vertex computation.
//!
//! Split out of the former 1098-line `view_cones.rs` (2026-06-15).

use super::*;

/// What forces a full rig rebuild (vs. a cheap per-frame geometry update): the
/// world-space render transform (size) and the capture texture dims (derived
/// from world extent × density and the exit portal's surface axis).
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct RebuildKey {
    pub(crate) world_size: Vec2,
    pub(crate) tex: UVec2,
}

/// The capture texture dims for a rig: the LONG side covers the exit's
/// along-surface (lateral) axis at the configured density up to the cap; the
/// SHORT side covers the bounded window depth. A wall exit is tall-thin, a
/// floor/ceiling exit wide-short.
pub(crate) fn capture_dims(
    config: &PortalViewConeConfig,
    world_size: Vec2,
    exit_normal: Vec2,
) -> UVec2 {
    let density = config.texels_per_world_px.max(0.05);
    let long = ((world_size.x.max(world_size.y) * density) as u32)
        .clamp(256, config.max_resolution.max(256));
    let short = (((config.depth_close.max(config.min_depth) * 2.0 * density) as u32)
        .next_power_of_two())
    .clamp(64, 512);
    if exit_normal.x.abs() > 0.5 {
        UVec2::new(short, long) // wall exit: lateral runs vertically
    } else {
        UVec2::new(long, short) // floor/ceiling exit: lateral runs horizontally
    }
}

/// Per-frame render data for the (world-clipped) window polygon: fan-mesh
/// positions + UVs + indices, the mesh world translation, and the capture
/// camera's center + framed size.
pub(crate) struct ConeRender {
    pub(crate) positions: Vec<[f32; 3]>,
    pub(crate) uvs: Vec<[f32; 2]>,
    pub(crate) indices: Vec<u32>,
    pub(crate) centroid: Vec3,
    pub(crate) cam_center: Vec3,
    pub(crate) source_size: Vec2,
}

/// Sutherland–Hodgman clip of a convex polygon to an axis-aligned rect. The
/// wedge legitimately reaches the half-plane limit; the WORLD bounds are its
/// only honest clip (no arbitrary lateral clamp), and clipping before building
/// the mesh keeps the capture rect — and therefore the texel density — tight.
pub(crate) fn clip_polygon_to_rect(poly: &[Vec2], min: Vec2, max: Vec2) -> Vec<Vec2> {
    // (axis, bound, keep_less_than)
    let planes = [
        (0usize, min.x, false),
        (0usize, max.x, true),
        (1usize, min.y, false),
        (1usize, max.y, true),
    ];
    let mut current: Vec<Vec2> = poly.to_vec();
    for (axis, bound, keep_lt) in planes {
        if current.is_empty() {
            break;
        }
        let inside = |p: Vec2| {
            let c = if axis == 0 { p.x } else { p.y };
            if keep_lt {
                c <= bound
            } else {
                c >= bound
            }
        };
        let cross = |a: Vec2, b: Vec2| {
            let (ca, cb) = if axis == 0 { (a.x, b.x) } else { (a.y, b.y) };
            let t = (bound - ca) / (cb - ca);
            a + (b - a) * t
        };
        let mut next = Vec::with_capacity(current.len() + 2);
        for i in 0..current.len() {
            let a = current[i];
            let b = current[(i + 1) % current.len()];
            match (inside(a), inside(b)) {
                (true, true) => next.push(b),
                (true, false) => next.push(cross(a, b)),
                (false, true) => {
                    next.push(cross(a, b));
                    next.push(b);
                }
                (false, false) => {}
            }
        }
        current = next;
    }
    current
}

/// A `&[Aabb]` as a [`SolidWorldQuery`] so the LOS raycast can reuse
/// `raycast_solids` over the host-supplied occluder snapshot.
pub(crate) struct SliceSolids<'a>(&'a [ae::Aabb]);
impl SolidWorldQuery for SliceSolids<'_> {
    fn for_each_solid_aabb(&self, _include_one_way: bool, visit: &mut dyn FnMut(ae::Aabb)) {
        for a in self.0 {
            visit(*a);
        }
    }
}

/// Skip the line-of-sight test when the real eye is within this distance of
/// the faced aperture: at/in the doorway the ray would only graze the host
/// surface's own (uncarved) blocks and false-positive.
pub(crate) const LOS_NEAR_SKIP: f32 = 70.0;
/// Pull LOS sample points slightly inward from the body corners so a corner
/// that is flush against a wall does not sneak through as "clear" by exact
/// point geometry.
const LOS_SAMPLE_INSET: f32 = 2.0;
/// Lift LOS aperture samples away from the portal host face so rays do not land
/// exactly on the uncarved wall/floor geometry.
const APERTURE_LOS_SURFACE_LIFT: f32 = 12.0;
/// Stop LOS casts a little before the lifted target, preserving the old center
/// ray behavior and avoiding false hits on the host face near the aperture.
const APERTURE_LOS_TARGET_BACKOFF: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ApertureLosRay {
    pub(crate) origin: Vec2,
    pub(crate) target: Vec2,
    pub(crate) hit: Option<Vec2>,
}

fn aperture_tangent(enter: &PortalFrame) -> Vec2 {
    let tangent = Vec2::new(-enter.normal.y, enter.normal.x);
    if tangent.length_squared() > f32::EPSILON {
        tangent.normalize()
    } else {
        Vec2::X
    }
}

fn aperture_half_width(enter: &PortalFrame) -> f32 {
    // Support radius of the axis-aligned portal AABB along the portal surface.
    // For the current cardinal portal frames this picks X for floors/ceilings
    // and Y for walls, while staying correct if a future frame stores a rotated
    // unit normal.
    let t = aperture_tangent(enter);
    enter.half_extent.dot(Vec2::new(t.x.abs(), t.y.abs()))
}

pub(crate) struct ApertureLosTargets {
    points: [Vec2; 3],
    len: usize,
}

impl ApertureLosTargets {
    pub(crate) fn as_slice(&self) -> &[Vec2] {
        &self.points[..self.len]
    }
}

pub(crate) fn aperture_los_targets(
    enter: &PortalFrame,
    quality: PortalApertureLosQuality,
) -> ApertureLosTargets {
    let center = enter.pos + enter.normal * APERTURE_LOS_SURFACE_LIFT;
    match quality {
        PortalApertureLosQuality::Low => ApertureLosTargets {
            points: [center, center, center],
            len: 1,
        },
        PortalApertureLosQuality::Medium => {
            let tangent = aperture_tangent(enter);
            let half_width = aperture_half_width(enter);
            ApertureLosTargets {
                points: [
                    center - tangent * half_width,
                    center,
                    center + tangent * half_width,
                ],
                len: 3,
            }
        }
    }
}

pub(crate) fn aperture_los_ray_to(
    eye: Vec2,
    target: Vec2,
    occluders: &[ae::Aabb],
) -> ApertureLosRay {
    let d = target - eye;
    let dist = d.length();
    if dist < 2.0 {
        return ApertureLosRay {
            origin: eye,
            target,
            hit: None,
        };
    }
    let hit = raycast_solids(
        &SliceSolids(occluders),
        eye,
        d,
        (dist - APERTURE_LOS_TARGET_BACKOFF).max(0.0),
        false,
    )
    .map(|(hit, _)| hit);
    ApertureLosRay {
        origin: eye,
        target,
        hit,
    }
}

/// Original low-quality LOS ray helper: one ray from `eye` to the lifted center
/// of the finite aperture. Kept for tests and debug callers that want the
/// previous center-only behavior.
pub(crate) fn aperture_los_ray(
    eye: Vec2,
    enter: &PortalFrame,
    occluders: &[ae::Aabb],
) -> ApertureLosRay {
    let target = aperture_los_targets(enter, PortalApertureLosQuality::Low).as_slice()[0];
    aperture_los_ray_to(eye, target, occluders)
}

pub(crate) fn aperture_los_rays(
    eye: Vec2,
    enter: &PortalFrame,
    occluders: &[ae::Aabb],
    quality: PortalApertureLosQuality,
) -> Vec<ApertureLosRay> {
    aperture_los_targets(enter, quality)
        .as_slice()
        .iter()
        .copied()
        .map(|target| aperture_los_ray_to(eye, target, occluders))
        .collect()
}

pub(crate) fn aperture_visibility_fraction(
    eye: Vec2,
    enter: &PortalFrame,
    occluders: &[ae::Aabb],
    quality: PortalApertureLosQuality,
) -> f32 {
    let targets = aperture_los_targets(enter, quality);
    let samples = targets.as_slice();
    if samples.is_empty() {
        return 0.0;
    }
    let clear = samples
        .iter()
        .copied()
        .filter(|target| aperture_los_ray_to(eye, *target, occluders).hit.is_none())
        .count();
    clear as f32 / samples.len() as f32
}

pub(crate) fn inset_viewer_corners(eye: Vec2, half_size: Vec2) -> [Vec2; 4] {
    let inset = Vec2::splat(LOS_SAMPLE_INSET).min(half_size.max(Vec2::ZERO));
    let h = (half_size - inset).max(Vec2::ZERO);
    [
        eye + Vec2::new(-h.x, -h.y),
        eye + Vec2::new(h.x, -h.y),
        eye + Vec2::new(h.x, h.y),
        eye + Vec2::new(-h.x, h.y),
    ]
}

/// Is the line of sight from `eye` to the aperture blocked by a solid? The
/// target is lifted a little OFF the surface (along the normal) so the ray
/// never has to land exactly on the host face — a grazing ray along a shared
/// floor line would otherwise clip the (uncarved) host blocks themselves —
/// and the cast still stops short of the lifted point. Uses the original
/// low-quality center ray for compatibility with existing tests/callers.
pub(crate) fn aperture_occluded(eye: Vec2, enter: &PortalFrame, occluders: &[ae::Aabb]) -> bool {
    aperture_visibility_fraction(eye, enter, occluders, PortalApertureLosQuality::Low) <= 0.0
}

/// One frame's window plan for a pair: the minimum cone, the (full) visible
/// wedge, and the target blend between them — the fraction of the viewer's
/// body corners with clear sight to the faced aperture. The renderer
/// approaches `target` temporally and blends per-corner, so partial cover and
/// approach/retreat all read as a smooth opening, not a pop.
pub(crate) struct ConePlan {
    pub(crate) min: ViewCone,
    pub(crate) wedge: ViewCone,
    pub(crate) target: f32,
}

/// The window plan for one portal pair this frame. Every portal always shows
/// at least the minimum cone; the wedge opens (smoothly, via `target`) when
/// the character is in front of (or in the doorway of) EITHER end of the pair
/// — the wormhole: being "in" one end is being in the other — in proportion
/// to how many of its body corners have clear sight to the faced aperture.
/// The wedge itself runs to the half-plane limit; the WORLD bounds are its
/// only clip (renderer-side). `viewer_gated == false` ⇒ the static always-on
/// window at full blend.
pub(crate) fn compute_cone(
    portal: &PlacedPortal,
    partner: &PlacedPortal,
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
    world_size: Vec2,
) -> ConePlan {
    let enter = portal.frame();
    let exit = partner.frame();
    if !config.viewer_gated {
        let c = view_cone(&enter, &exit, config.depth, config.spread);
        return ConePlan {
            min: c,
            wedge: c,
            target: 1.0,
        };
    }
    // The minimum cone, always shown when nothing better exists.
    let min = view_cone(&enter, &exit, config.min_depth, config.min_spread);
    let closed = |min: ViewCone| ConePlan {
        min,
        wedge: min,
        target: 0.0,
    };
    let Some(v) = viewer.filter(|v| v.present) else {
        return closed(min);
    };
    let h = v.half_size;
    let corners = inset_viewer_corners(v.eye, h);
    // Eye set for this end's wedge: the viewer's REAL AABB corners (those in
    // front of `enter` contribute) PLUS, for any corner that has crossed the
    // partner plane, its sprite-trick SHADOW (the body-map image, which emerges
    // in front of `enter`). A straddling viewer has presence at both ends, so
    // both sets feed the wedge — and as a corner crosses, its real contribution
    // hands off to its shadow continuously, removing the abrupt flip at the
    // midpoint between a pair (no hard direct↔wormhole eye switch).
    let mut eyes: Vec<Vec2> = corners.to_vec();
    for &c in &corners {
        if (c - exit.pos).dot(exit.normal) < 0.0 {
            eyes.push(ambition_portal::pieces::map_point(c, &exit, &enter));
        }
    }
    // Proximity-proportional depth from the NEAREST aperture — the distance is
    // continuous across the midpoint even as which-is-nearer flips.
    let dist = v.eye.distance(enter.pos).min(v.eye.distance(exit.pos));
    let dt = ((dist - config.dist_close) / (config.dist_far - config.dist_close).max(1.0))
        .clamp(0.0, 1.0);
    let depth = config.depth_close + (config.depth_far - config.depth_close) * smooth01(dt);
    // Visibility fraction: real corners ray-test to the nearer finite aperture
    // using the configured quality. The doorway case is still skipped because
    // rays there would graze the host blocks and sight is trivially clear.
    let faced = if v.eye.distance(enter.pos) <= v.eye.distance(exit.pos) {
        &enter
    } else {
        &exit
    };
    let target = if v.eye.distance(faced.pos) <= LOS_NEAR_SKIP {
        1.0
    } else {
        corners
            .iter()
            .map(|c| {
                aperture_visibility_fraction(*c, faced, &v.occluders, config.aperture_los_quality)
            })
            .sum::<f32>()
            / corners.len() as f32
    };
    if target <= 0.0 {
        return closed(min);
    }
    // The wedge runs to the half-plane: far extent past every world bound (the
    // renderer clips to the world rect afterwards).
    let far_extent = world_size.x + world_size.y;
    let Some(wedge) = aperture_wedge_multi(&enter, &exit, &eyes, depth, far_extent) else {
        return closed(min);
    };
    ConePlan {
        min,
        wedge,
        target: target * config.viewer_blend.clamp(0.0, 1.0),
    }
}

/// Per-vertex UVs for the window mesh: each mapped source vertex normalized
/// inside the source rect. World y-down and texture v-down agree (the render
/// y-flip cancels between camera and capture), so this is flip-free.
pub(crate) fn vertex_uv(s: Vec2, source_min: Vec2, source_size: Vec2) -> [f32; 2] {
    [
        ((s.x - source_min.x) / source_size.x.max(1e-6)).clamp(0.0, 1.0),
        ((s.y - source_min.y) / source_size.y.max(1e-6)).clamp(0.0, 1.0),
    ]
}

/// Resolve a blended window cone into renderable data: clip the entry quad to
/// the WORLD rect (the wedge's only honest bound — it may legitimately reach
/// the half-plane), map each clipped vertex through the body map, and frame
/// the capture on the clipped source's bounds — so the texture's density is
/// spent only on what the window can actually show. `None` when the clip
/// leaves nothing or the source degenerates.
pub(crate) fn cone_render(
    cone: &ViewCone,
    enter: &PortalFrame,
    exit: &PortalFrame,
    frame: &PortalWorldFrame,
    z: f32,
) -> Option<ConeRender> {
    let poly = clip_polygon_to_rect(&cone.entry_quad, Vec2::ZERO, frame.size);
    if poly.len() < 3 {
        return None;
    }
    // Map the clipped vertices; their bounds are the capture rect.
    let mapped: Vec<Vec2> = poly
        .iter()
        .map(|p| ambition_portal::pieces::map_point(*p, enter, exit))
        .collect();
    let (mut smin, mut smax) = (mapped[0], mapped[0]);
    for m in &mapped[1..] {
        smin = smin.min(*m);
        smax = smax.max(*m);
    }
    let source_size = smax - smin;
    if source_size.x < 1.0 || source_size.y < 1.0 {
        return None;
    }
    let render_poly: Vec<Vec2> = poly
        .iter()
        .map(|p| frame.to_render(*p, 0.0).truncate())
        .collect();
    let centroid = render_poly.iter().copied().sum::<Vec2>() / render_poly.len() as f32;
    let positions: Vec<[f32; 3]> = render_poly
        .iter()
        .map(|p| [p.x - centroid.x, p.y - centroid.y, 0.0])
        .collect();
    let uvs: Vec<[f32; 2]> = mapped
        .iter()
        .map(|m| vertex_uv(*m, smin, source_size))
        .collect();
    // Fan triangulation (the clipped polygon stays convex).
    let indices: Vec<u32> = (1..poly.len() as u32 - 1)
        .flat_map(|i| [0, i, i + 1])
        .collect();
    Some(ConeRender {
        positions,
        uvs,
        indices,
        centroid: centroid.extend(z),
        cam_center: frame.to_render((smin + smax) * 0.5, 0.0),
        source_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal::pieces::PortalFrame;

    /// Pin the flip-free UV convention: the source-rect corner with MINIMAL
    /// world coords (left, world-top) is texture (0,0); maximal is (1,1).
    #[test]
    fn cone_uvs_are_flip_free_in_world_space() {
        let source = ae::Aabb::new(Vec2::new(100.0, 50.0), Vec2::new(40.0, 20.0));
        let quad = [
            Vec2::new(60.0, 30.0),
            Vec2::new(140.0, 30.0),
            Vec2::new(140.0, 70.0),
            Vec2::new(60.0, 70.0),
        ];
        let size = source.max - source.min;
        let uvs: Vec<[f32; 2]> = quad
            .iter()
            .map(|q| vertex_uv(*q, source.min, size))
            .collect();
        assert_eq!(uvs[0], [0.0, 0.0]);
        assert_eq!(uvs[1], [1.0, 0.0]);
        assert_eq!(uvs[2], [1.0, 1.0]);
        assert_eq!(uvs[3], [0.0, 1.0]);
    }

    /// The UVs cover the unit square's bounds, rotated per the view map — pinned
    /// for a 90° pair (the entry near edge maps onto the exit face → u = 1).
    #[test]
    fn cone_uvs_rotate_with_the_view_map() {
        let enter = PortalFrame {
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: Vec2::new(46.0, 9.0),
        };
        let exit = PortalFrame {
            pos: Vec2::new(400.0, 200.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        let cone = view_cone(&enter, &exit, 120.0, 0.25);
        let size = cone.source.max - cone.source.min;
        let uvs: Vec<[f32; 2]> = cone
            .source_quad
            .iter()
            .map(|q| vertex_uv(*q, cone.source.min, size))
            .collect();
        for uv in &uvs {
            assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
        }
        let touch =
            |f: &dyn Fn(&[f32; 2]) -> f32, v: f32| uvs.iter().any(|uv| (f(uv) - v).abs() < 1e-4);
        assert!(touch(&|uv| uv[0], 0.0) && touch(&|uv| uv[0], 1.0));
        assert!(touch(&|uv| uv[1], 0.0) && touch(&|uv| uv[1], 1.0));
        assert!((uvs[0][0] - 1.0).abs() < 1e-4 && (uvs[1][0] - 1.0).abs() < 1e-4);
    }

    /// LOS: a solid AABB straddling the segment eye→aperture blocks it; none
    /// clear of the segment does not.
    #[test]
    fn aperture_occlusion_tracks_a_blocking_wall() {
        let enter = PortalFrame {
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: Vec2::new(46.0, 9.0),
        };
        let eye = Vec2::new(100.0, 100.0); // 200px above the floor portal
                                           // Wall across the sight line, well in front of the surface.
        let wall = ae::Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(40.0, 8.0));
        assert!(aperture_occluded(eye, &enter, &[wall]));
        // A wall off to the side does not block.
        let aside = ae::Aabb::new(Vec2::new(400.0, 200.0), Vec2::new(40.0, 8.0));
        assert!(!aperture_occluded(eye, &enter, &[aside]));
        // No occluders at all → clear.
        assert!(!aperture_occluded(eye, &enter, &[]));
    }

    #[test]
    fn medium_aperture_los_sees_around_center_blockers() {
        let enter = PortalFrame {
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: Vec2::new(46.0, 9.0),
        };
        let eye = Vec2::new(100.0, 100.0);
        let center_blocker = ae::Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(6.0, 8.0));

        assert_eq!(
            aperture_visibility_fraction(
                eye,
                &enter,
                &[center_blocker],
                PortalApertureLosQuality::Low,
            ),
            0.0
        );
        assert!(
            aperture_visibility_fraction(
                eye,
                &enter,
                &[center_blocker],
                PortalApertureLosQuality::Medium,
            ) > 0.0,
            "medium quality should keep the aperture partially visible when only the center ray is blocked",
        );
    }

    #[test]
    fn medium_aperture_los_does_not_require_visible_endpoints() {
        let enter = PortalFrame {
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: Vec2::new(46.0, 9.0),
        };
        let eye = Vec2::new(100.0, 100.0);
        // Block the left/right endpoint rays near y=200 while leaving the center
        // line open. Endpoint-only LOS would fail this case.
        let left_blocker = ae::Aabb::new(Vec2::new(75.5, 200.0), Vec2::new(5.0, 8.0));
        let right_blocker = ae::Aabb::new(Vec2::new(124.5, 200.0), Vec2::new(5.0, 8.0));

        let visibility = aperture_visibility_fraction(
            eye,
            &enter,
            &[left_blocker, right_blocker],
            PortalApertureLosQuality::Medium,
        );
        assert!(
            visibility > 0.0 && visibility < 1.0,
            "medium quality should report partial visibility when the center is visible but endpoints are blocked: {visibility}",
        );
    }

    /// The debug ray helper reports the same blocker that the occlusion test
    /// uses, and stays clear when nothing blocks the aperture.
    #[test]
    fn aperture_los_ray_reports_blockers_and_clear_paths() {
        let enter = PortalFrame {
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, -1.0),
            half_extent: Vec2::new(46.0, 9.0),
        };
        let eye = Vec2::new(100.0, 100.0);
        let wall = ae::Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(40.0, 8.0));
        let blocked = aperture_los_ray(eye, &enter, &[wall]);
        assert_eq!(blocked.target, enter.pos + enter.normal * 12.0);
        assert!(blocked.hit.is_some());
        assert!(aperture_occluded(eye, &enter, &[wall]));

        let clear = aperture_los_ray(eye, &enter, &[]);
        assert_eq!(clear.target, enter.pos + enter.normal * 12.0);
        assert!(clear.hit.is_none());
        assert!(!aperture_occluded(eye, &enter, &[]));
    }

    /// The inset corner sampler pulls each corner inward by the configured
    /// amount, without reordering the corners.
    #[test]
    fn inset_viewer_corners_are_conservative_and_stable() {
        let eye = Vec2::new(100.0, 100.0);
        let half_size = Vec2::new(20.0, 12.0);
        let corners = inset_viewer_corners(eye, half_size);
        assert_eq!(corners[0], Vec2::new(82.0, 90.0));
        assert_eq!(corners[1], Vec2::new(118.0, 90.0));
        assert_eq!(corners[2], Vec2::new(118.0, 110.0));
        assert_eq!(corners[3], Vec2::new(82.0, 110.0));
    }

    /// World clipping: a half-plane-sized wedge clips to the world rect, the
    /// clipped polygon stays convex-fan renderable, and a fully-outside quad
    /// clips away entirely.
    #[test]
    fn wedge_clips_to_world_bounds() {
        let world = Vec2::new(800.0, 600.0);
        // A huge trapezoid wildly exceeding the world.
        let quad = [
            Vec2::new(300.0, 400.0),
            Vec2::new(500.0, 400.0),
            Vec2::new(4000.0, 480.0),
            Vec2::new(-4000.0, 480.0),
        ];
        let poly = clip_polygon_to_rect(&quad, Vec2::ZERO, world);
        assert!(poly.len() >= 4, "clipped poly: {poly:?}");
        for p in &poly {
            assert!(
                p.x >= -1e-3 && p.x <= world.x + 1e-3 && p.y >= -1e-3 && p.y <= world.y + 1e-3,
                "inside world: {p:?}"
            );
        }
        // Fully outside → empty.
        let outside = [
            Vec2::new(-100.0, -100.0),
            Vec2::new(-50.0, -100.0),
            Vec2::new(-50.0, -50.0),
            Vec2::new(-100.0, -50.0),
        ];
        assert!(clip_polygon_to_rect(&outside, Vec2::ZERO, world).is_empty());
    }
}
