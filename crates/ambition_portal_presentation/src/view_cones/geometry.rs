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
    pub(crate) recursion_depth: u32,
    pub(crate) include_parallax: bool,
}

/// The capture texture dims for a rig: the LONG side covers the exit's
/// along-surface (lateral) axis at the configured density up to the cap; the
/// SHORT side covers the bounded window depth. A wall exit is tall-thin, a
/// floor/ceiling exit wide-short.
pub(crate) fn capture_dims(
    budget: &EffectivePortalCaptureBudget,
    config: &PortalViewConeConfig,
    world_size: Vec2,
    exit_normal: Vec2,
    capture_frame: Option<CaptureCameraFrame>,
    screen_scale: f32,
) -> UVec2 {
    if config.capture_camera_mode == PortalCaptureCameraMode::MappedCameraSnapshot {
        let source_size = capture_frame
            .map(|frame| frame.size)
            .unwrap_or(Vec2::new(800.0, 450.0))
            .max(Vec2::splat(1.0));
        // `texels_per_world_px = 1.0` means "pixel-perfect": the main camera
        // renders each world pixel at `screen_scale` physical pixels, so the
        // capture must match that density or the window reads blurrier than
        // the world around it. `max_resolution` still caps the memory.
        let density = budget.texels_per_world_px.max(0.05) * screen_scale.max(1.0);
        let max_side = budget.max_resolution.max(256);
        let width = (source_size.x * density).round() as u32;
        let height = (source_size.y * density).round() as u32;
        let scale = (max_side as f32 / width.max(height).max(1) as f32).min(1.0);
        return UVec2::new(
            ((width as f32 * scale).round() as u32).clamp(256, max_side),
            ((height as f32 * scale).round() as u32).clamp(144, max_side),
        );
    }
    let density = budget.texels_per_world_px.max(0.05);
    let long = ((world_size.x.max(world_size.y) * density) as u32)
        .clamp(256, budget.max_resolution.max(256));
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
    /// Entry-side world polygon after clipping to the active portal view rect.
    pub(crate) entry_poly_world: Vec<Vec2>,
    /// Source-side world vertices after mapping `entry_poly_world` through the portal pair.
    pub(crate) mapped_source_vertices: Vec<Vec2>,
    pub(crate) source_min: Vec2,
    pub(crate) source_max: Vec2,
    pub(crate) source_size: Vec2,
}

/// Destination-side camera frame for snapshot-driven capture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CaptureCameraFrame {
    pub(crate) center: Vec2,
    pub(crate) size: Vec2,
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
    pub(crate) debug: ConePlanDebug,
}

/// Derived per-frame geometry diagnostics for a [`ConePlan`]. These values are
/// mirrored into the F8/debug-dump text using the same field names so that UI,
/// dump, and code stay grep-compatible during portal tuning.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ConePlanDebug {
    pub(crate) edge_distance_to_aperture: Option<f32>,
    pub(crate) half_plane_preview_alpha: f32,
    pub(crate) finite_depth: Option<f32>,
    pub(crate) half_plane_depth: Option<f32>,
    pub(crate) finite_lateral_limit: Option<f32>,
    pub(crate) half_plane_lateral_limit: Option<f32>,
    pub(crate) finite_wedge_source_size: Option<Vec2>,
    pub(crate) half_plane_wedge_source_size: Option<Vec2>,
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
    let fraction = aperture_visibility_fraction(
        candidate.los_origin,
        &candidate.los_frame,
        occluders,
        quality,
    );
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

fn preview_proximity_alpha(edge_distance: f32, config: &PortalViewConeConfig) -> f32 {
    let full = config.half_plane_preview_full_distance.max(0.0);
    let blend = config.half_plane_preview_blend_distance.max(0.0);
    let start = full + blend;
    if start <= f32::EPSILON {
        return if edge_distance <= full { 1.0 } else { 0.0 };
    }
    let raw = if edge_distance <= full {
        1.0
    } else if blend <= f32::EPSILON {
        0.0
    } else {
        ((start - edge_distance) / blend).clamp(0.0, 1.0)
    };
    smooth01(raw)
}

/// How much solid host material sits behind `enter`'s face — delegated to the
/// shared core measurement ([`ambition_portal::measure_host_depth`]) so the
/// window depth clip, the transit rescue, and the carve all agree on where a
/// wall ends.
pub(crate) fn host_depth_limit(
    enter: &PortalFrame,
    occluders: &[ae::Aabb],
    probe_depth: f32,
) -> f32 {
    ambition_portal::measure_host_depth(occluders, enter, probe_depth)
}

/// Effectively-unclamped lateral bound for the wedge rays. The wedge's far
/// corners must stay ON the true sight rays through the aperture endpoints —
/// clamping them to a "reasonable" lateral limit bends the rays inward and
/// turns a near-half-plane view into an arbitrary ~45° trapezoid. The render
/// stage clips the polygon to the active viewport before building the mesh
/// and the capture rect, so the viewport is the only honest lateral bound;
/// this constant exists purely to keep f32 arithmetic comfortable.
const RAY_LATERAL_CLAMP: f32 = 1.0e5;

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
            debug: ConePlanDebug::default(),
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
                debug: ConePlanDebug::default(),
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
        debug: ConePlanDebug::default(),
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
    let los_target = coverage / corners.len() as f32;
    if los_target <= 0.0 || eyes.is_empty() {
        return closed(min);
    }

    // Clip the window's depth to the host wall's measured material so a thin
    // door wall never lets the mesh punch through into the room behind it (see
    // [`host_depth_limit`]). The half-plane takeover below stays unclipped for
    // genuinely disjoint pairs — crossing a teleport, the whole view becomes
    // the exit chart — but NOT for a DOORWAY pair (opposed faces across a
    // thin slab): its two charts are the same visual space, so a takeover
    // pane photographs a region that is ALSO directly on screen and
    // double-images everything in it (the world shimmered at a parallax
    // offset, the far frame showed twice, the transiting body doubled — the
    // c136/c137 artifact family). A doorway is a HOLE, not a wormhole: its
    // pane covers only the slab (the wall material is the one thing to
    // hide), and inside the slab the mapped capture reconstructs exactly the
    // occluded middle of whatever straddles the wall — while the body's
    // clipped pieces and the far side draw direct, crisp, exactly once.
    let doorway_pair = enter.normal.dot(exit.normal) < -0.9
        && enter.pos.distance(exit.pos) <= config.doorway_pair_max_gap;
    let host_limit = host_depth_limit(
        &enter,
        &v.occluders,
        config.dynamic_depth_close.max(config.min_depth).max(1.0),
    );
    let min = view_cone(
        &enter,
        &exit,
        config.min_depth.min(host_limit),
        config.min_spread,
    );

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
    let proximity_alpha = preview_proximity_alpha(edge_distance, config);
    let target = los_target * proximity_alpha;
    if target <= 0.0 {
        return closed(min);
    }
    let dt = ((edge_distance - config.dynamic_dist_close)
        / (config.dynamic_dist_far - config.dynamic_dist_close).max(1.0))
    .clamp(0.0, 1.0);
    let finite_depth = (config.dynamic_depth_close
        + (config.dynamic_depth_far - config.dynamic_depth_close) * smooth01(dt))
    .min(host_limit);
    // The half-plane preview is the aperture-limit view. It is ALWAYS the
    // aperture-anchored wedge — near edge pinned to the opening, far corners
    // on the true rays through the aperture endpoints — so approaching the
    // portal reads as a view CONE fanning open toward the half-plane, never a
    // laterally-growing strip whose near edge leaves the face. The viewport
    // clip in the render stage bounds it; explicit positive max-lateral
    // values keep the old bounded tuning path for diagnostics.
    let full_half_plane = config.half_plane_preview_max_lateral <= 0.0;
    let half_depth = if full_half_plane {
        world_size.x.max(world_size.y).max(finite_depth)
    } else {
        finite_depth
    };
    // Doorway pairs never take over: the pane is the slab (see above).
    let half_depth = if doorway_pair {
        half_depth.min(host_limit)
    } else {
        half_depth
    };
    let half_plane_lateral_limit = if full_half_plane {
        RAY_LATERAL_CLAMP
    } else {
        config
            .half_plane_preview_max_lateral
            .max(aperture_half_width(&enter) + 1.0)
    };
    let finite_lateral_limit = RAY_LATERAL_CLAMP;
    let finite_wedge =
        aperture_wedge_multi(&enter, &exit, &eyes, finite_depth, finite_lateral_limit);
    let half_plane_alpha =
        preview_half_plane_alpha(edge_distance, config) * los_target.clamp(0.0, 1.0);
    let mut half_plane_eyes = eyes.clone();
    if half_plane_alpha > 0.0 {
        half_plane_eyes.push(enter.pos + enter.normal * 0.5);
    }
    let half_wedge = if half_plane_alpha > 0.0 {
        aperture_wedge_multi(
            &enter,
            &exit,
            &half_plane_eyes,
            half_depth,
            half_plane_lateral_limit,
        )
    } else {
        None
    };
    let debug = ConePlanDebug {
        edge_distance_to_aperture: Some(edge_distance),
        half_plane_preview_alpha: half_plane_alpha,
        finite_depth: Some(finite_depth),
        half_plane_depth: Some(half_depth),
        finite_lateral_limit: Some(finite_lateral_limit),
        half_plane_lateral_limit: Some(half_plane_lateral_limit),
        finite_wedge_source_size: finite_wedge.map(|w| w.source.max - w.source.min),
        half_plane_wedge_source_size: half_wedge.map(|w| w.source.max - w.source.min),
    };
    let wedge = match (finite_wedge, half_wedge) {
        (Some(finite), Some(half)) => blend_cones(&finite, &half, half_plane_alpha, &enter, &exit),
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
        debug,
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
    config: &PortalViewConeConfig,
    clip_min: Vec2,
    clip_max: Vec2,
    z: f32,
    capture_frame: Option<CaptureCameraFrame>,
) -> Option<ConeRender> {
    let poly = match config.source_clip_policy {
        PortalViewConeSourceClipPolicy::AllowClip => cone.entry_quad.to_vec(),
        PortalViewConeSourceClipPolicy::ClampToFrame
        | PortalViewConeSourceClipPolicy::FitToFrame => {
            clip_polygon_to_rect(&cone.entry_quad, clip_min, clip_max)
        }
    };
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
    if let Some(capture_frame) = capture_frame {
        let half = capture_frame.size.max(Vec2::splat(1.0)) * 0.5;
        smin = capture_frame.center - half;
        smax = capture_frame.center + half;
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
        entry_poly_world: poly,
        mapped_source_vertices: mapped,
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

    fn poly_span_x(poly: &[Vec2]) -> f32 {
        let min_x = poly.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = poly.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        max_x - min_x
    }

    fn rendered_span_x(plan: &ConePlan, enter: &PlacedPortal, exit: &PlacedPortal) -> f32 {
        let enter = enter.frame();
        let exit = exit.frame();
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
        span_x(&cone)
    }

    #[test]
    fn view_cone_defaults_to_low_aperture_los() {
        assert_eq!(
            PortalViewConeConfig::default().aperture_los_quality,
            PortalApertureLosQuality::Low
        );
    }

    #[test]
    fn view_cone_defaults_to_cone_rect_and_full_half_plane() {
        let config = PortalViewConeConfig::default();
        assert_eq!(
            config.capture_camera_mode,
            PortalCaptureCameraMode::ConeRect
        );
        assert_eq!(config.half_plane_preview_max_lateral, 0.0);
        assert!(config.half_plane_preview_full_distance > 0.0);
        assert!(config.half_plane_preview_full_distance <= 1.0);
    }

    #[test]
    fn full_half_plane_render_clips_to_the_full_active_frame_at_the_aperture() {
        let world = Vec2::new(3488.0, 1056.0);
        let enter = placed(
            PortalGunColor::BLUE.channel(),
            Vec2::new(300.0, 900.0),
            Vec2::new(0.0, -1.0),
        );
        let exit = placed(
            PortalGunColor::ORANGE.channel(),
            Vec2::new(600.0, 900.0),
            Vec2::new(0.0, -1.0),
        );
        let config = PortalViewConeConfig::default();
        let viewer = PortalViewer {
            present: true,
            eye: Vec2::new(303.45, 875.5),
            half_size: Vec2::new(15.0, 24.0),
            occluders: Vec::new(),
        };
        let clip_min = Vec2::new(-96.55, 626.10);
        let clip_max = Vec2::new(703.45, 1076.10);

        let plan = compute_cone(&enter, &exit, &config, Some(&viewer), world);
        assert_eq!(plan.target, 1.0);
        assert_eq!(plan.debug.half_plane_preview_alpha, 1.0);
        let enter_frame = enter.frame();
        let exit_frame = exit.frame();
        let cone = blend_cones(
            &plan.min,
            &plan.wedge,
            smooth01(plan.target),
            &enter_frame,
            &exit_frame,
        );
        let render = cone_render(
            &cone,
            &enter_frame,
            &exit_frame,
            &PortalWorldFrame { size: world },
            &config,
            clip_min,
            clip_max,
            config.z,
            None,
        )
        .expect("full half-plane should render after viewport clipping");

        let clip_width = clip_max.x - clip_min.x;
        let render_width = poly_span_x(&render.entry_poly_world);
        assert!(
            render_width >= clip_width * 0.999,
            "full half-plane should reach both sides of the active frame after clipping: render_width={render_width}, clip_width={clip_width}, poly={:?}",
            render.entry_poly_world
        );
        // Tolerance 0.1px: the fan's pre-clip corners sit at the ray clamp, so
        // the Sutherland intersection carries a little float error.
        assert!(
            render
                .entry_poly_world
                .iter()
                .any(|p| (p.x - clip_min.x).abs() < 0.1)
                && render
                    .entry_poly_world
                    .iter()
                    .any(|p| (p.x - clip_max.x).abs() < 0.1),
            "full half-plane polygon should touch both clip edges: {:?}",
            render.entry_poly_world
        );
        // The view is a CONE anchored at the aperture: every vertex ON the
        // face (y = 900) stays within the opening — the near edge never grows
        // laterally beyond the portal.
        let aperture = 46.0;
        for p in render
            .entry_poly_world
            .iter()
            .filter(|p| (p.y - 900.0).abs() < 1e-3)
        {
            assert!(
                (p.x - 300.0).abs() <= aperture + 1e-3,
                "near edge pinned to the aperture, got {p:?}"
            );
        }
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
        let mut config = PortalViewConeConfig::default();
        config.aperture_los_quality = PortalApertureLosQuality::Medium;
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
        let far_span = max_x - min_x;
        assert!(
            far_span > world.x * 0.9,
            "at the doorway the default half-plane should be full-view width, x span {min_x}..{max_x}",
        );
        let far_depth = plan.wedge.entry_quad[2].y - enter.pos.y;
        assert!(
            far_span > far_depth,
            "bounded half-plane preview should still be wider than a 45-degree cone: far_span={far_span}, far_depth={far_depth}",
        );
    }

    #[test]
    fn near_doorway_view_cone_opens_only_inside_the_proximity_band() {
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
            (
                span_x(&plan.wedge),
                plan.target,
                plan.immediate,
                plan.debug.half_plane_preview_alpha,
            )
        };

        let (far_span, far_target, far_immediate, far_half) = span_at(start_dist + 1.0);
        let (mid_span, mid_target, mid_immediate, mid_half) =
            span_at((start_dist + full_dist) * 0.5);
        let (full_span, full_target, full_immediate, full_half) = span_at(full_dist * 0.5);

        assert!(!far_immediate && !mid_immediate && !full_immediate);
        assert_eq!(far_target, 0.0);
        assert_eq!(far_half, 0.0);
        assert!(
            mid_target > far_target && mid_target < full_target,
            "proximity target should open smoothly: far={far_target}, mid={mid_target}, full={full_target}"
        );
        assert!(
            mid_half > far_half && mid_half < full_half,
            "half-plane shape should ease only near contact: far={far_half}, mid={mid_half}, full={full_half}"
        );
        assert!(
            full_span > mid_span,
            "doorway span should finish wider than the mid-ease cone: mid={mid_span}, full={full_span}"
        );
        assert_eq!(
            far_span,
            span_x(&view_cone(
                &enter.frame(),
                &exit.frame(),
                config.min_depth,
                config.min_spread
            ))
        );
        assert_eq!(full_target, 1.0);
        assert_eq!(full_half, 1.0);
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
    fn exact_mode_uses_los_geometry_without_bounded_preview() {
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
        let mut default_config = PortalViewConeConfig::default();
        default_config.aperture_los_quality = PortalApertureLosQuality::Medium;
        let mut exact_config = default_config.clone();
        exact_config.half_plane_preview_full_distance = 0.0;
        // Mid proximity: close enough that the preview assist is active, far
        // enough that exact LOS is a finite wedge (AT the plane, exact LOS
        // legitimately becomes the half-plane fan and the two coincide).
        let viewer = PortalViewer {
            present: true,
            eye: enter.pos + enter.normal * 60.0,
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let exact = compute_cone(&enter, &exit, &exact_config, Some(&viewer), world);
        let preview = compute_cone(&enter, &exit, &default_config, Some(&viewer), world);
        let exact_span = span_x(&exact.wedge);
        let preview_span = span_x(&preview.wedge);

        assert!(exact.target > 0.0);
        assert_eq!(exact.debug.half_plane_preview_alpha, 0.0);
        assert!(preview.debug.half_plane_preview_alpha > 0.0);
        assert!(
            preview_span > exact_span,
            "default preview should expand beyond exact LOS geometry: exact={exact_span}, preview={preview_span}",
        );
    }

    #[test]
    fn positive_half_plane_max_lateral_keeps_bounded_diagnostic_mode() {
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
        config.half_plane_preview_max_lateral = 360.0;
        config.aperture_los_quality = PortalApertureLosQuality::Medium;
        let viewer = PortalViewer {
            present: true,
            eye: enter.pos + enter.normal * (config.half_plane_preview_full_distance * 0.5),
            half_size: Vec2::ZERO,
            occluders: Vec::new(),
        };

        let plan = compute_cone(&enter, &exit, &config, Some(&viewer), world);
        let span = span_x(&plan.wedge);
        assert!(
            span <= config.half_plane_preview_max_lateral * 2.0 + 1e-3,
            "positive half_plane_preview_max_lateral should keep bounded mode: span={span}",
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
        let mut config = PortalViewConeConfig::default();
        config.aperture_los_quality = PortalApertureLosQuality::Medium;
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

        assert_eq!(off_axis.target, 0.0);
        assert!(centered.debug.half_plane_preview_alpha > 0.0);
        assert_eq!(off_axis.debug.half_plane_preview_alpha, 0.0);
        assert!(
            centered_span > off_axis_span,
            "centered near-plane preview should expand beyond off-axis LOS geometry: centered={centered_span}, off_axis={off_axis_span}",
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
        config.half_plane_preview_blend_distance = 240.0;
        let eye = enter.pos + enter.normal * 60.0;
        let left_blocker = ae::Aabb::new(Vec2::new(871.0, 790.0), Vec2::new(5.0, 8.0));
        let right_blocker = ae::Aabb::new(Vec2::new(929.0, 790.0), Vec2::new(5.0, 8.0));
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

    /// Thin-wall pair (portals back-to-back on opposite faces of one wall):
    /// the far-side portal's window must stay CLOSED for a viewer on the near
    /// side — there is no line of sight to its face, and the wormhole route is
    /// admission-gated on that face LOS under the default visibility mode.
    #[test]
    fn thin_wall_far_side_portal_stays_closed_for_a_near_side_viewer() {
        let world = Vec2::new(1600.0, 900.0);
        let wall = ae::Aabb::new(Vec2::new(512.0, 450.0), Vec2::new(12.0, 450.0));
        let near = placed(
            PortalChannelColor::Purple.channel(),
            Vec2::new(500.0, 450.0),
            Vec2::new(-1.0, 0.0),
        );
        let far = placed(
            PortalChannelColor::Yellow.channel(),
            Vec2::new(524.0, 450.0),
            Vec2::new(1.0, 0.0),
        );
        let config = PortalViewConeConfig::default();
        let viewer = PortalViewer {
            present: true,
            eye: Vec2::new(400.0, 450.0),
            half_size: Vec2::new(15.0, 24.0),
            occluders: vec![wall],
        };
        // The far portal's own cone: enter = far, exit = near.
        let plan = compute_cone(&far, &near, &config, Some(&viewer), world);
        assert_eq!(
            plan.target, 0.0,
            "no LOS to the far face through the thin wall — its window must stay closed",
        );
        // The near portal's cone opens normally for the same viewer... when in
        // proximity range; at 100px the band is open.
        let plan = compute_cone(&near, &far, &config, Some(&viewer), world);
        assert!(
            plan.target > 0.0,
            "the near portal's window opens for the near-side viewer",
        );
    }

    /// The window is glass set INTO the host wall: on a thin wall the finite
    /// wedge must not punch through into the room behind (where it would be
    /// visible with no LOS and would sit inside its own capture's source
    /// region, feeding back as a nested window).
    #[test]
    fn window_depth_clips_to_the_host_wall_thickness() {
        let world = Vec2::new(1600.0, 900.0);
        let wall = ae::Aabb::new(Vec2::new(512.0, 450.0), Vec2::new(12.0, 450.0));
        let near = placed(
            PortalChannelColor::Purple.channel(),
            Vec2::new(500.0, 450.0),
            Vec2::new(-1.0, 0.0),
        );
        let far = placed(
            PortalChannelColor::Yellow.channel(),
            Vec2::new(524.0, 450.0),
            Vec2::new(1.0, 0.0),
        );
        // Exact mode: no half-plane assist, so the wedge is the finite
        // LOS geometry alone (the doorway takeover is deliberately unclipped).
        let mut config = PortalViewConeConfig::default();
        config.half_plane_preview_full_distance = 0.0;
        let viewer = PortalViewer {
            present: true,
            eye: Vec2::new(400.0, 450.0),
            half_size: Vec2::ZERO,
            occluders: vec![wall],
        };
        let plan = compute_cone(&near, &far, &config, Some(&viewer), world);
        assert!(plan.target > 0.0, "the near window is open");
        let wall_back = 524.0;
        for p in plan
            .wedge
            .entry_quad
            .iter()
            .chain(plan.min.entry_quad.iter())
        {
            assert!(
                p.x <= wall_back + 0.6,
                "window geometry must stay inside the 24px host wall, got {p:?}",
            );
        }
        assert!(
            plan.debug.finite_depth.unwrap_or(0.0) <= 24.0 + 0.6,
            "finite depth clips to the wall thickness, got {:?}",
            plan.debug.finite_depth,
        );
    }

    /// A DOORWAY pair (opposed faces across a thin slab) never takes over the
    /// half-plane, even standing AT the aperture: its two charts are the same
    /// visual space, so a takeover pane would photograph a region that is
    /// also directly on screen and double-image everything in it (frames,
    /// the transiting body, the world at a parallax offset). The pane is the
    /// slab. Disjoint pairs keep the takeover
    /// (`doorway_view_cone_reaches_half_plane_without_immediate_snap` is the
    /// control at 640px separation).
    #[test]
    fn thin_wall_doorway_pane_stays_inside_the_slab_at_the_aperture() {
        let world = Vec2::new(1600.0, 900.0);
        let wall = ae::Aabb::new(Vec2::new(512.0, 450.0), Vec2::new(12.0, 450.0));
        let near = placed(
            PortalChannelColor::Purple.channel(),
            Vec2::new(500.0, 450.0),
            Vec2::new(-1.0, 0.0),
        );
        let far = placed(
            PortalChannelColor::Yellow.channel(),
            Vec2::new(524.0, 450.0),
            Vec2::new(1.0, 0.0),
        );
        // DEFAULT config: half-plane takeover enabled — the doorway rule
        // itself must suppress it, not a tuning knob.
        let config = PortalViewConeConfig::default();
        let viewer = PortalViewer {
            present: true,
            eye: near.pos + near.normal * 0.5, // standing in the aperture
            half_size: Vec2::ZERO,
            occluders: vec![wall],
        };
        let plan = compute_cone(&near, &far, &config, Some(&viewer), world);
        assert!(plan.target > 0.0, "the doorway window is open");
        let wall_back = 524.0;
        for p in plan
            .wedge
            .entry_quad
            .iter()
            .chain(plan.min.entry_quad.iter())
        {
            assert!(
                p.x <= wall_back + 0.6,
                "a doorway pane must stay inside the wall slab even at the \
                 aperture (no takeover), got {p:?}",
            );
        }
    }

    #[test]
    fn host_depth_limit_measures_merged_material_and_stops_at_gaps() {
        let face = PortalFrame {
            pos: Vec2::new(500.0, 450.0),
            normal: Vec2::new(-1.0, 0.0),
            half_extent: Vec2::new(9.0, 46.0),
        };
        let wall = |x0: f32, x1: f32| {
            ae::Aabb::new(
                Vec2::new((x0 + x1) * 0.5, 450.0),
                Vec2::new((x1 - x0) * 0.5, 450.0),
            )
        };
        // A single 24px wall.
        assert_eq!(host_depth_limit(&face, &[wall(500.0, 524.0)], 280.0), 24.0);
        // Two exactly-adjacent merged tiles extend the material.
        assert_eq!(
            host_depth_limit(&face, &[wall(500.0, 524.0), wall(524.0, 600.0)], 280.0),
            100.0
        );
        // A real gap behind the wall ends it.
        assert_eq!(
            host_depth_limit(&face, &[wall(500.0, 524.0), wall(540.0, 600.0)], 280.0),
            24.0
        );
        // Deep wall clips to the probe.
        assert_eq!(host_depth_limit(&face, &[wall(500.0, 900.0)], 280.0), 280.0);
        // No measurable host (one-way platform host, empty snapshot) → unclipped.
        assert_eq!(host_depth_limit(&face, &[], 280.0), 280.0);
        assert_eq!(host_depth_limit(&face, &[wall(700.0, 800.0)], 280.0), 280.0);
    }

    /// Capture cameras see every OTHER portal's window layer (true recursion)
    /// but never their own, and no window layers at all when recursion is off.
    #[test]
    fn capture_layers_exclude_the_rigs_own_window() {
        let a = placed(
            PortalChannelColor::Purple.channel(),
            Vec2::new(500.0, 450.0),
            Vec2::new(-1.0, 0.0),
        );
        let b = placed(
            PortalChannelColor::Yellow.channel(),
            Vec2::new(524.0, 450.0),
            Vec2::new(1.0, 0.0),
        );
        let all = [a, b];
        let own = portal_window_self_layer(a.channel);
        let other = portal_window_self_layer(b.channel);
        assert_ne!(own, other);
        // `RenderLayers::layer` is a const constructor limited to the small
        // built-in buffer; large per-portal layers need the growable `with`.
        let probe = |n: usize| RenderLayers::none().with(n);

        let layers = capture_render_layers(1, false, 0, &other_window_layers(&all, a.channel));
        assert!(
            layers.intersects(&probe(other)),
            "recursion includes the partner's window layer",
        );
        assert!(
            !layers.intersects(&probe(own)),
            "a capture must never see its own window",
        );
        assert!(
            !layers.intersects(&probe(PORTAL_WINDOW_RENDER_LAYER)),
            "captures never use the shared main-camera window layer",
        );

        let flat = capture_render_layers(0, false, 0, &other_window_layers(&all, a.channel));
        assert!(
            !flat.intersects(&probe(other)),
            "recursion depth 0 sees no window layers at all",
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
        let mut config = PortalViewConeConfig::default();
        config.aperture_los_quality = PortalApertureLosQuality::Medium;
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
        let far_span = max_y - min_y;
        assert!(
            far_span > world.y * 0.9,
            "just-behind doorway cone should get the default full half-plane, y span {min_y}..{max_y}",
        );
        let far_depth = enter.pos.x - plan.wedge.entry_quad[2].x;
        assert!(
            far_span > far_depth,
            "bounded just-behind doorway preview should still be wider than a 45-degree cone: far_span={far_span}, far_depth={far_depth}",
        );
    }
}
