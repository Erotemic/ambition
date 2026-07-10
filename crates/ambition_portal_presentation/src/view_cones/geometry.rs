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

fn aperture_tangent(enter: &PortalAperture) -> Vec2 {
    let tangent = Vec2::new(-enter.frame.normal.y, enter.frame.normal.x);
    if tangent.length_squared() > f32::EPSILON {
        tangent.normalize()
    } else {
        Vec2::X
    }
}

fn aperture_half_width(enter: &PortalAperture) -> f32 {
    // Support radius of the axis-aligned portal AABB along the portal surface.
    // For the current cardinal portal frames this picks X for floors/ceilings
    // and Y for walls, while staying correct if a future frame stores a rotated
    // unit normal.
    enter.half_length
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
    enter: &PortalAperture,
    quality: PortalApertureLosQuality,
) -> ApertureLosTargets {
    let center = enter.frame.origin + enter.frame.normal * APERTURE_LOS_SURFACE_LIFT;
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
    enter: &PortalAperture,
    occluders: &[ae::Aabb],
) -> ApertureLosRay {
    let target = aperture_los_targets(enter, PortalApertureLosQuality::Low).as_slice()[0];
    aperture_los_ray_to(eye, target, occluders)
}

pub(crate) fn aperture_los_rays(
    eye: Vec2,
    enter: &PortalAperture,
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
    enter: &PortalAperture,
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
pub(crate) fn aperture_occluded(eye: Vec2, enter: &PortalAperture, occluders: &[ae::Aabb]) -> bool {
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
    los_frame: PortalAperture,
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
    enter: &PortalAperture,
    occluders: &[ae::Aabb],
    quality: PortalApertureLosQuality,
) -> Option<f32> {
    if (candidate.wedge_eye - enter.frame.origin).dot(enter.frame.normal) <= 0.0 {
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

fn body_edge_distance_to_aperture(viewer: &PortalViewer, frame: &PortalAperture) -> f32 {
    let center_front = (viewer.eye - frame.frame.origin).dot(frame.frame.normal);
    let tangent = aperture_tangent(frame);
    let center_lateral = (viewer.eye - frame.frame.origin).dot(tangent).abs();
    let normal_radius = viewer.half_size.dot(Vec2::new(
        frame.frame.normal.x.abs(),
        frame.frame.normal.y.abs(),
    ));
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
    enter: &PortalAperture,
    occluders: &[ae::Aabb],
    probe_depth: f32,
) -> f32 {
    ambition_portal::measure_host_depth(occluders, &enter.frame, probe_depth)
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
    let enter = portal.aperture();
    let exit = partner.aperture();
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
            && (corner - exit.frame.origin).dot(exit.frame.normal) < 0.0
        {
            let candidate = VisibleEyeCandidate {
                wedge_eye: ambition_portal::pieces::map_point(corner, &exit.frame, &enter.frame),
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
    let enter = portal.aperture();
    let exit = partner.aperture();
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
            && (corner - exit.frame.origin).dot(exit.frame.normal) < 0.0
        {
            let candidate = VisibleEyeCandidate {
                wedge_eye: ambition_portal::pieces::map_point(corner, &exit.frame, &enter.frame),
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
    let doorway_pair = enter.frame.normal.dot(exit.frame.normal) < -0.9
        && enter.frame.origin.distance(exit.frame.origin) <= config.doorway_pair_max_gap;
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
    let faced = if v.eye.distance(enter.frame.origin) <= v.eye.distance(exit.frame.origin) {
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
        half_plane_eyes.push(enter.frame.origin + enter.frame.normal * 0.5);
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
    enter: &PortalAperture,
    exit: &PortalAperture,
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
        .map(|p| ambition_portal::pieces::map_point(*p, &enter.frame, &exit.frame))
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
mod tests;
