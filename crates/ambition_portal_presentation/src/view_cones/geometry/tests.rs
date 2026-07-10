//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use ambition_portal::pieces::{PortalAperture, PortalFrame};
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
    let enter = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0)),
        half_length: 46.0,
    };
    let exit = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(400.0, 200.0), Vec2::new(-1.0, 0.0)),
        half_length: 46.0,
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
    let enter = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0)),
        half_length: 46.0,
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
    let enter = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0)),
        half_length: 46.0,
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
    let enter = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0)),
        half_length: 46.0,
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
    let enter = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0)),
        half_length: 46.0,
    };
    let eye = Vec2::new(100.0, 100.0);
    let wall = ae::Aabb::new(Vec2::new(100.0, 200.0), Vec2::new(40.0, 8.0));
    let blocked = aperture_los_ray(eye, &enter, &[wall]);
    assert_eq!(
        blocked.target,
        enter.frame.origin + enter.frame.normal * 12.0
    );
    assert!(blocked.hit.is_some());
    assert!(aperture_occluded(eye, &enter, &[wall]));

    let clear = aperture_los_ray(eye, &enter, &[]);
    assert_eq!(clear.target, enter.frame.origin + enter.frame.normal * 12.0);
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
    PlacedPortal::fixed(
        channel,
        pos,
        normal,
        ambition_portal::portal_half_extent(normal),
    )
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
    let enter = enter.aperture();
    let exit = exit.aperture();
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
    let enter_frame = enter.aperture();
    let exit_frame = exit.aperture();
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
    let (mid_span, mid_target, mid_immediate, mid_half) = span_at((start_dist + full_dist) * 0.5);
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
            &enter.aperture(),
            &exit.aperture(),
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
    let face = PortalAperture {
        frame: PortalFrame::fixed(Vec2::new(500.0, 450.0), Vec2::new(-1.0, 0.0)),
        half_length: 46.0,
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
    let all = [a.clone(), b.clone()];
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
