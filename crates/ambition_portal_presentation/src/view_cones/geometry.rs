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
    let max_depth = config
        .dynamic_depth_close
        .max(config.static_depth)
        .max(config.min_depth);
    let short = (((max_depth * 2.0 * density) as u32).next_power_of_two()).clamp(64, 512);
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
    pub(crate) source_min: Vec2,
    pub(crate) source_max: Vec2,
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
/// of the finite aperture. Kept for tests that pin the center-only behavior.
#[cfg(test)]
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
/// low-quality center ray for compatibility with existing tests.
#[cfg(test)]
pub(crate) fn aperture_occluded(eye: Vec2, enter: &PortalFrame, occluders: &[ae::Aabb]) -> bool {
    aperture_visibility_fraction(eye, enter, occluders, PortalApertureLosQuality::Low) <= 0.0
}

/// One frame's window plan for a pair: the minimum cone, the visible wedge, and
/// the target blend between them. `target == 0` means no viewer sample has LOS
/// into either relevant aperture chart, so the renderer hides the capture
/// window outright. The minimum cone is only a visible lower bound once LOS has
/// admitted the window.
pub(crate) struct ConePlan {
    pub(crate) min: ViewCone,
    pub(crate) wedge: ViewCone,
    pub(crate) target: f32,
    pub(crate) immediate: bool,
}

#[derive(Clone, Copy)]
struct VisibleEyeCandidate {
    wedge_eye: Vec2,
    los_origin: Vec2,
    los_frame: PortalFrame,
}

/// Per-portal, per-frame visibility-route evidence used by the debug dump.
/// Fractions are averaged over the same inset body corners that feed
/// [`compute_cone`]. They are diagnostics, not a second source of truth.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct VisibilityRouteSummary {
    pub(crate) face_los_fraction: f32,
    pub(crate) through_portal_los_fraction: f32,
    pub(crate) exit_side_los_fraction: f32,
    pub(crate) face_eye_count: usize,
    pub(crate) through_portal_eye_count: usize,
    pub(crate) exit_side_eye_count: usize,
}

impl VisibilityRouteSummary {
    pub(crate) fn admitted(self) -> bool {
        self.face_los_fraction > 0.0
            || self.through_portal_los_fraction > 0.0
            || self.exit_side_los_fraction > 0.0
    }
}

fn push_unique_eye(eyes: &mut Vec<Vec2>, eye: Vec2) {
    const EPS2: f32 = 1.0e-4;
    if !eyes.iter().any(|e| e.distance_squared(eye) <= EPS2) {
        eyes.push(eye);
    }
}

fn visible_candidate_fraction(
    candidate: VisibleEyeCandidate,
    enter: &PortalFrame,
    occluders: &[ae::Aabb],
    quality: PortalApertureLosQuality,
) -> Option<f32> {
    if (candidate.wedge_eye - enter.pos).dot(enter.normal) <= 0.0 {
        return None;
    }
    let fraction =
        aperture_visibility_fraction(candidate.los_origin, &candidate.los_frame, occluders, quality);
    if fraction > 0.0 {
        Some(fraction)
    } else {
        None
    }
}

fn body_edge_distance_to_aperture(viewer: &PortalViewer, frame: &PortalFrame) -> f32 {
    let center_front = (viewer.eye - frame.pos).dot(frame.normal);
    let tangent = aperture_tangent(frame);
    let center_lateral = (viewer.eye - frame.pos).dot(tangent).abs();
    let normal_radius = viewer
        .half_size
        .dot(Vec2::new(frame.normal.x.abs(), frame.normal.y.abs()));
    let lateral_radius = viewer
        .half_size
        .dot(Vec2::new(tangent.x.abs(), tangent.y.abs()));
    let front_gap = (center_front - normal_radius).max(0.0);
    let lateral_gap = (center_lateral - aperture_half_width(frame) - lateral_radius).max(0.0);
    Vec2::new(front_gap, lateral_gap).length()
}

fn preview_half_plane_alpha(edge_distance: f32, config: &PortalViewConeConfig) -> f32 {
    let full = config.half_plane_preview_full_distance.max(0.0);
    if full <= f32::EPSILON {
        return 0.0;
    }
    let blend = config.half_plane_preview_blend_distance.max(0.0);
    let raw = if edge_distance <= full {
        1.0
    } else if blend <= f32::EPSILON {
        0.0
    } else {
        ((full + blend - edge_distance) / blend).clamp(0.0, 1.0)
    };
    let eased = smooth01(raw);
    eased * eased
}

pub(crate) fn visibility_route_summary(
    portal: &PlacedPortal,
    partner: &PlacedPortal,
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
) -> VisibilityRouteSummary {
    let Some(v) = viewer.filter(|v| v.present) else {
        return VisibilityRouteSummary::default();
    };
    let enter = portal.frame();
    let exit = partner.frame();
    let corners = inset_viewer_corners(v.eye, v.half_size);
    let mut summary = VisibilityRouteSummary::default();
    for &corner in &corners {
        let direct_candidate = VisibleEyeCandidate {
            wedge_eye: corner,
            los_origin: corner,
            los_frame: enter,
        };
        let direct_fraction = visible_candidate_fraction(
            direct_candidate,
            &enter,
            &v.occluders,
            config.aperture_los_quality,
        )
        .unwrap_or(0.0);
        summary.face_los_fraction += direct_fraction;
        if direct_fraction > 0.0 {
            summary.face_eye_count += 1;
        }

        if let Some((resolved, via_partner)) = window_eye(&enter, &exit, corner) {
            if config
                .visibility_mode
                .admit_through_portal(direct_fraction, via_partner)
            {
                let candidate = VisibleEyeCandidate {
                    wedge_eye: resolved,
                    los_origin: corner,
                    los_frame: if via_partner { exit } else { enter },
                };
                let fraction = visible_candidate_fraction(
                    candidate,
                    &enter,
                    &v.occluders,
                    config.aperture_los_quality,
                )
                .unwrap_or(0.0);
                summary.through_portal_los_fraction += fraction;
                if fraction > 0.0 {
                    summary.through_portal_eye_count += 1;
                }
            }
        }

        if config.visibility_mode.admit_exit_side(direct_fraction)
            && (corner - exit.pos).dot(exit.normal) < 0.0
        {
            let candidate = VisibleEyeCandidate {
                wedge_eye: ambition_portal::pieces::map_point(corner, &exit, &enter),
                los_origin: corner,
                los_frame: exit,
            };
            let fraction = visible_candidate_fraction(
                candidate,
                &enter,
                &v.occluders,
                config.aperture_los_quality,
            )
            .unwrap_or(0.0);
            summary.exit_side_los_fraction += fraction;
            if fraction > 0.0 {
                summary.exit_side_eye_count += 1;
            }
        }
    }

    let denom = corners.len() as f32;
    summary.face_los_fraction /= denom;
    summary.through_portal_los_fraction /= denom;
    summary.exit_side_los_fraction /= denom;
    summary
}

/// The window plan for one portal pair this frame. LOS is the hard admission
/// gate in dynamic mode, but [`PortalViewConeVisibilityMode`] controls which
/// evidence routes can admit and shape the visible wedge. Static mode bypasses
/// viewer LOS and uses the authored static cone; off mode closes the window.
/// The optional half-plane preview is an art-directed assist layered on top of
/// visible LOS geometry; setting
/// [`PortalViewConeConfig::half_plane_preview_full_distance`] to `0.0` leaves
/// only exact LOS-derived geometry at the configured ray fidelity.
pub(crate) fn compute_cone(
    portal: &PlacedPortal,
    partner: &PlacedPortal,
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
    world_size: Vec2,
) -> ConePlan {
    let enter = portal.frame();
    let exit = partner.frame();
    let closed = || {
        let min = view_cone(&enter, &exit, config.min_depth, config.min_spread);
        ConePlan {
            min,
            wedge: min,
            target: 0.0,
            immediate: false,
        }
    };

    match config.mode {
        PortalViewConeMode::Off => return closed(),
        PortalViewConeMode::Static => {
            let c = view_cone(&enter, &exit, config.static_depth, config.static_spread);
            return ConePlan {
                min: c,
                wedge: c,
                target: 1.0,
                immediate: true,
            };
        }
        PortalViewConeMode::Dynamic => {}
    }

    // Lower bound for an admitted window. If LOS admits nothing, the renderer
    // hides the window rather than showing this cone through blocked space.
    let min = view_cone(&enter, &exit, config.min_depth, config.min_spread);
    let closed = |min: ViewCone| ConePlan {
        min,
        wedge: min,
        target: 0.0,
        immediate: false,
    };
    let Some(v) = viewer.filter(|v| v.present) else {
        return closed(min);
    };
    let corners = inset_viewer_corners(v.eye, v.half_size);
    let mut eyes: Vec<Vec2> = Vec::with_capacity(corners.len() * 3);
    let mut coverage: f32 = 0.0;
    for &corner in &corners {
        let mut best: f32 = 0.0;
        let direct_candidate = VisibleEyeCandidate {
            wedge_eye: corner,
            los_origin: corner,
            los_frame: enter,
        };
        let direct_fraction = visible_candidate_fraction(
            direct_candidate,
            &enter,
            &v.occluders,
            config.aperture_los_quality,
        )
        .unwrap_or(0.0);
        if direct_fraction > 0.0 {
            best = best.max(direct_fraction);
            push_unique_eye(&mut eyes, direct_candidate.wedge_eye);
        }

        if let Some((resolved, via_partner)) = window_eye(&enter, &exit, corner) {
            if config
                .visibility_mode
                .admit_through_portal(direct_fraction, via_partner)
            {
                let candidate = VisibleEyeCandidate {
                    wedge_eye: resolved,
                    los_origin: corner,
                    los_frame: if via_partner { exit } else { enter },
                };
                if let Some(fraction) = visible_candidate_fraction(
                    candidate,
                    &enter,
                    &v.occluders,
                    config.aperture_los_quality,
                ) {
                    best = best.max(fraction);
                    push_unique_eye(&mut eyes, candidate.wedge_eye);
                }
            }
        }

        if config.visibility_mode.admit_exit_side(direct_fraction)
            && (corner - exit.pos).dot(exit.normal) < 0.0
        {
            let candidate = VisibleEyeCandidate {
                wedge_eye: ambition_portal::pieces::map_point(corner, &exit, &enter),
                los_origin: corner,
                los_frame: exit,
            };
            if let Some(fraction) = visible_candidate_fraction(
                candidate,
                &enter,
                &v.occluders,
                config.aperture_los_quality,
            ) {
                best = best.max(fraction);
                push_unique_eye(&mut eyes, candidate.wedge_eye);
            }
        }

        coverage += best;
    }
    let target = coverage / corners.len() as f32;
    if target <= 0.0 || eyes.is_empty() {
        return closed(min);
    }

    // Proximity-proportional depth and preview distance use the body-edge gap
    // to the nearer finite aperture, not the aperture's infinite host plane.
    // Being close in Y while far away laterally should still read as a cone,
    // not as the doorway half-plane limit.
    let faced = if v.eye.distance(enter.pos) <= v.eye.distance(exit.pos) {
        &enter
    } else {
        &exit
    };
    let edge_distance = body_edge_distance_to_aperture(v, faced);
    let dt = ((edge_distance - config.dynamic_dist_close)
        / (config.dynamic_dist_far - config.dynamic_dist_close).max(1.0))
        .clamp(0.0, 1.0);
    let near_depth = config.dynamic_depth_close.max(world_size.x + world_size.y);
    let finite_depth = config.dynamic_depth_close
        + (config.dynamic_depth_far - config.dynamic_depth_close) * smooth01(dt);
    let half_depth = near_depth + (config.dynamic_depth_far - near_depth) * smooth01(dt);
    let far_extent = (world_size.x + world_size.y) * 64.0;
    let finite_wedge = aperture_wedge_multi(&enter, &exit, &eyes, finite_depth, far_extent);
    let half_plane_alpha = preview_half_plane_alpha(edge_distance, config) * target.clamp(0.0, 1.0);
    let mut half_plane_eyes = eyes.clone();
    if half_plane_alpha > 0.0 {
        half_plane_eyes.push(enter.pos + enter.normal * 0.5);
    }
    let half_wedge = aperture_wedge_multi(&enter, &exit, &half_plane_eyes, half_depth, far_extent);
    let wedge = match (finite_wedge, half_wedge) {
        (Some(finite), Some(half)) => {
            blend_cones(&finite, &half, half_plane_alpha, &enter, &exit)
        }
        (Some(finite), None) => finite,
        (None, Some(half)) => {
            if half_plane_alpha > 0.0 {
                blend_cones(&min, &half, half_plane_alpha, &enter, &exit)
            } else {
                return closed(min);
            }
        }
        (None, None) => return closed(min),
    };
    ConePlan {
        min,
        wedge,
        target: target * config.viewer_blend.clamp(0.0, 1.0),
        immediate: false,
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
    clip_min: Vec2,
    clip_max: Vec2,
    z: f32,
) -> Option<ConeRender> {
    let poly = clip_polygon_to_rect(&cone.entry_quad, clip_min, clip_max);
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
        source_min: smin,
        source_max: smax,
        source_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_portal::pieces::PortalFrame;
    use ambition_portal::{PortalChannelColor, PortalGunColor};

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

    /// Rect clipping: a half-plane-sized wedge clips to the world rect, the
    /// clipped polygon stays convex-fan renderable, and a fully-outside quad
    /// clips away entirely.
    #[test]
    fn wedge_clips_to_rect_bounds() {
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

    #[test]
    fn wedge_clip_rect_can_extend_outside_the_room() {
        let clip_min = Vec2::new(500.0, -122.0);
        let clip_max = Vec2::new(1300.0, 328.0);
        let quad = [
            Vec2::new(854.0, 220.0),
            Vec2::new(946.0, 220.0),
            Vec2::new(4000.0, -4000.0),
            Vec2::new(-4000.0, -4000.0),
        ];
        let poly = clip_polygon_to_rect(&quad, clip_min, clip_max);
        assert!(poly.len() >= 3, "clipped poly: {poly:?}");
        assert!(
            poly.iter().any(|p| p.y < 0.0),
            "portal half-plane should be allowed to fill the out-of-room viewport: {poly:?}"
        );
        for p in &poly {
            assert!(
                p.x >= clip_min.x - 1e-3
                    && p.x <= clip_max.x + 1e-3
                    && p.y >= clip_min.y - 1e-3
                    && p.y <= clip_max.y + 1e-3,
                "inside viewport clip: {p:?}"
            );
        }
    }

    fn placed(channel: ambition_portal::PortalChannel, pos: Vec2, normal: Vec2) -> PlacedPortal {
        PlacedPortal {
            channel,
            pos,
            normal,
            half_extent: ambition_portal::portal_half_extent(normal),
        }
    }

    fn span_x(cone: &ViewCone) -> f32 {
        let min_x = cone
            .entry_quad
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = cone
            .entry_quad
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        max_x - min_x
    }

    fn rendered_span_x(plan: &ConePlan, enter: &PlacedPortal, exit: &PlacedPortal) -> f32 {
        let enter = enter.frame();
        let exit = exit.frame();
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
        span_x(&cone)
    }

    #[test]
    fn doorway_view_cone_reaches_half_plane_without_immediate_snap() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(900.0, 820.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(900.0, 180.0),
            Vec2::new(0.0, 1.0),
        );
        let config = PortalViewConeConfig::default();
        let viewer = PortalViewer {
            present: true,
            eye: enter.pos + enter.normal * 0.5,
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let plan = compute_cone(&enter, &exit, &config, Some(&viewer), world);
        assert!(
            !plan.immediate,
            "doorway cone should now use the continuous spatial/temporal ease"
        );
        assert_eq!(plan.target, 1.0);
        let min_x = plan
            .wedge
            .entry_quad
            .iter()
            .map(|p| p.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = plan
            .wedge
            .entry_quad
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min_x <= -world.x + 1e-3 && max_x >= world.x * 2.0 - 1e-3,
            "at the doorway the eased cone should have reached the viewport-clipped half-plane, x span {min_x}..{max_x}",
        );
        let far_depth = plan.wedge.entry_quad[2].y - enter.pos.y;
        let far_span = max_x - min_x;
        assert!(
            far_span > far_depth * 32.0,
            "half-plane side rays should be nearly parallel to the surface, not a 45-degree cone: far_span={far_span}, far_depth={far_depth}",
        );
    }

    #[test]
    fn near_doorway_view_cone_eases_toward_half_plane_before_contact() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(900.0, 820.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(900.0, 180.0),
            Vec2::new(0.0, 1.0),
        );
        let config = PortalViewConeConfig::default();
        let full_dist = config.half_plane_preview_full_distance;
        let start_dist = full_dist + config.half_plane_preview_blend_distance;

        let span_at = |dist: f32| {
            let viewer = PortalViewer {
                present: true,
                eye: enter.pos + enter.normal * dist,
                half_size: Vec2::ZERO,
                occluders: Vec::new(),
            };
            let plan = compute_cone(&enter, &exit, &config, Some(&viewer), world);
            (span_x(&plan.wedge), plan.target, plan.immediate)
        };

        let (start_span, _, start_immediate) = span_at(start_dist + 1.0);
        let (mid_span, mid_target, mid_immediate) = span_at((start_dist + full_dist) * 0.5);
        let (full_span, full_target, full_immediate) = span_at(full_dist * 0.5);

        assert!(!start_immediate && !mid_immediate && !full_immediate);
        assert!(
            mid_span > start_span,
            "mid-ease span should be wider than the ordinary finite cone: start={start_span}, mid={mid_span}"
        );
        assert!(
            full_span > mid_span,
            "doorway span should finish wider than the mid-ease cone: mid={mid_span}, full={full_span}"
        );
        assert!(
            mid_target > 0.0 && mid_target <= 1.0,
            "mid-ease target should be valid, got {mid_target}"
        );
        assert_eq!(full_target, 1.0);
    }

    #[test]
    fn blocked_los_hides_near_portal_cone_inside_preview_range() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(900.0, 820.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(900.0, 180.0),
            Vec2::new(0.0, 1.0),
        );
        let config = PortalViewConeConfig::default();
        let blocker = ae::Aabb::new(Vec2::new(900.0, 780.0), Vec2::new(120.0, 3.0));
        let viewer = PortalViewer {
            present: true,
            eye: enter.pos
                + enter.normal
                    * (config.half_plane_preview_full_distance
                        + config.half_plane_preview_blend_distance * 0.5),
            half_size: Vec2::ZERO,
            occluders: vec![blocker],
        };

        let plan = compute_cone(&enter, &exit, &config, Some(&viewer), world);
        assert_eq!(
            plan.target, 0.0,
            "preview proximity must not admit a portal window when LOS is blocked",
        );
        assert_eq!(plan.wedge.entry_quad, plan.min.entry_quad);
    }

    #[test]
    fn exact_mode_uses_los_geometry_without_preview_half_plane() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(900.0, 820.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(900.0, 180.0),
            Vec2::new(0.0, 1.0),
        );
        let default_config = PortalViewConeConfig::default();
        let mut exact_config = default_config.clone();
        exact_config.half_plane_preview_full_distance = 0.0;
        let viewer = PortalViewer {
            present: true,
            eye: enter.pos + enter.normal * (default_config.half_plane_preview_full_distance * 0.5),
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let exact = compute_cone(&enter, &exit, &exact_config, Some(&viewer), world);
        let preview = compute_cone(&enter, &exit, &default_config, Some(&viewer), world);
        let exact_span = span_x(&exact.wedge);
        let preview_span = span_x(&preview.wedge);

        assert_eq!(exact.target, 1.0);
        assert!(
            preview_span > exact_span * 10.0,
            "default preview should approach the half-plane, while exact mode stays on LOS geometry: exact={exact_span}, preview={preview_span}",
        );
    }

    #[test]
    fn off_axis_near_plane_viewer_does_not_get_half_plane_preview() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(900.0, 820.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(900.0, 180.0),
            Vec2::new(0.0, 1.0),
        );
        let config = PortalViewConeConfig::default();
        let centered_viewer = PortalViewer {
            present: true,
            eye: enter.pos + enter.normal * (config.half_plane_preview_full_distance * 0.5),
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };
        let off_axis_viewer = PortalViewer {
            present: true,
            eye: enter.pos
                + Vec2::X * 500.0
                + enter.normal * (config.half_plane_preview_full_distance * 0.5),
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let centered = compute_cone(&enter, &exit, &config, Some(&centered_viewer), world);
        let off_axis = compute_cone(&enter, &exit, &config, Some(&off_axis_viewer), world);
        let centered_span = span_x(&centered.wedge);
        let off_axis_span = span_x(&off_axis.wedge);

        assert_eq!(off_axis.target, 1.0);
        assert!(
            centered_span > off_axis_span * 10.0,
            "being close to the infinite portal plane is not enough for the half-plane preview: centered={centered_span}, off_axis={off_axis_span}",
        );
    }

    #[test]
    fn partial_los_reduces_window_growth() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(900.0, 820.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(900.0, 180.0),
            Vec2::new(0.0, 1.0),
        );
        let mut config = PortalViewConeConfig::default();
        config.aperture_los_quality = PortalApertureLosQuality::Medium;
        let eye = enter.pos + enter.normal * 200.0;
        let left_blocker = ae::Aabb::new(Vec2::new(875.5, 720.0), Vec2::new(5.0, 8.0));
        let right_blocker = ae::Aabb::new(Vec2::new(924.5, 720.0), Vec2::new(5.0, 8.0));
        let partial_viewer = PortalViewer {
            present: true,
            eye,
            half_size: Vec2::ZERO,
            occluders: vec![left_blocker, right_blocker],
        };
        let clear_viewer = PortalViewer {
            present: true,
            eye,
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let partial = compute_cone(&enter, &exit, &config, Some(&partial_viewer), world);
        let clear = compute_cone(&enter, &exit, &config, Some(&clear_viewer), world);
        let partial_span = rendered_span_x(&partial, &enter, &exit);
        let clear_span = rendered_span_x(&clear, &enter, &exit);

        assert!(
            partial.target > 0.0 && partial.target < 1.0,
            "endpoint blockers should leave the aperture partially visible, target={}",
            partial.target,
        );
        assert!(
            partial_span < clear_span,
            "partial LOS should not inflate to the clear window width: partial={partial_span}, clear={clear_span}",
        );
    }

    #[test]
    fn just_behind_doorway_still_contributes_to_half_plane() {
        let world = Vec2::new(1600.0, 900.0);
        let enter = placed(
            PortalChannelColor::Indexed(140).channel(),
            Vec2::new(2552.0, 248.0),
            Vec2::new(1.0, 0.0),
        );
        let exit = placed(
            PortalChannelColor::Indexed(141).channel(),
            Vec2::new(2792.0, 248.0),
            Vec2::new(-1.0, 0.0),
        );
        let config = PortalViewConeConfig::default();
        let viewer = PortalViewer {
            present: true,
            eye: enter.pos - enter.normal * 0.5,
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let plan = compute_cone(&enter, &exit, &config, Some(&viewer), world);
        assert!(
            plan.target > 0.99,
            "a just-crossed doorway viewer should still see the full portal chart, target={}",
            plan.target,
        );
        let min_y = plan
            .wedge
            .entry_quad
            .iter()
            .map(|p| p.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = plan
            .wedge
            .entry_quad
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            min_y < -world.y && max_y > world.y * 2.0,
            "just-behind doorway cone should still expand to the vertical half-plane, y span {min_y}..{max_y}",
        );
        let far_depth = enter.pos.x - plan.wedge.entry_quad[2].x;
        let far_span = max_y - min_y;
        assert!(
            far_span > far_depth * 32.0,
            "just-behind doorway side rays should be nearly parallel to the surface, not a 45-degree cone: far_span={far_span}, far_depth={far_depth}",
        );
    }
}
