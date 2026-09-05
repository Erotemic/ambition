//! Portal view-cone diagnostics — the debug overlay/inspector and text/PNG
//! dump machinery for [`super::sync_portal_view_cones`]. None of this runs in a
//! normal frame's render path; it is the "why does this wedge look like that"
//! toolbox: `debug_portal_view_zones` (gizmo overlay), the
//! `*_debug_dump*` chain (developer-action text/capture-texture dumps), and the
//! `fmt_*` formatting helpers they share.
use super::*;

pub fn handle_portal_view_cone_dump_hotkey(
    mut actions: MessageReader<
        ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction,
    >,
    mut request: ResMut<PortalViewConeDebugDumpRequest>,
) {
    if actions.read().any(|action| {
        *action
            == ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction::DumpPortalViewCones
    }) {
        request.request("Shift+F8");
    }
}

/// Flush one pending portal view-cone dump to stderr and, on native targets, to
/// `target/ambition-debug/portal-view-cones/`.
#[allow(clippy::too_many_arguments)]
pub fn flush_portal_view_cone_debug_dump(
    mut request: ResMut<PortalViewConeDebugDumpRequest>,
    selection: Res<crate::PortalEffectSelection>,
    config: Res<PortalViewConeConfig>,
    quality: Res<PortalCaptureQualityBudget>,
    viewer: Option<Res<PortalViewer>>,
    frame: Res<PortalWorldFrame>,
    host_view: Option<Res<PortalCameraContinuityHostView>>,
    portals: Query<&PlacedPortal>,
    candidates: Query<
        (
            Option<&Name>,
            &crate::PortalCompositingCandidate,
            Option<&GlobalTransform>,
            Option<&RenderLayers>,
        ),
    >,
    rigs: Query<(
        &PortalViewRig,
        &Camera,
        &Projection,
        Option<&GlobalTransform>,
    )>,
    cone_visibility: Query<(&Visibility, Option<&GlobalTransform>), With<PortalConeMesh>>,
    screen_density: GameplayScreenDensity,
    // The session's portal map convention, from the resource that owns it.
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
) {
    if !request.pending {
        return;
    }
    let convention = tuning
        .as_deref()
        .map(|tuning| tuning.convention.map_convention())
        .unwrap_or_default();
    let reason = if request.reason.is_empty() {
        "manual".to_string()
    } else {
        request.reason.clone()
    };
    request.pending = false;
    request.reason.clear();

    // ⭐ The host-tagged drawables, as (name, pose, drawn z). Collected here so
    // the text builder stays a pure function of plain data and can be tested
    // without a `World`.
    let drawables: Vec<(String, crate::PortalCompositingCandidate, f32, Option<RenderLayers>)> = candidates
        .iter()
        .map(|(name, view, transform, layers)| {
            (
                name.map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| "<unnamed>".to_string()),
                *view,
                transform.map(|t| t.translation().z).unwrap_or(f32::NAN),
                layers.cloned(),
            )
        })
        .collect();

    let dump = portal_view_cone_debug_dump_text(
        &reason,
        &selection,
        &config,
        &quality,
        viewer.as_deref(),
        &frame,
        host_view.as_deref(),
        &portals,
        &rigs,
        &cone_visibility,
        &drawables,
        screen_density.texels_per_world(host_view.as_deref()),
        convention,
    );

    #[cfg(not(target_arch = "wasm32"))]
    match write_portal_view_cone_debug_dump(&dump) {
        Ok(path) => {
            eprintln!("portal view-cone dump written: {}", path.display());
        }
        Err(err) => {
            eprintln!("portal view-cone dump write failed: {err}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    eprintln!("portal view-cone dump: file output skipped on wasm32");

    eprintln!("{dump}");
}

#[cfg(not(target_arch = "wasm32"))]
fn write_portal_view_cone_debug_dump(text: &str) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let dir = std::path::PathBuf::from("target/ambition-debug/portal-view-cones");
    std::fs::create_dir_all(&dir)?;
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("portal-view-cones-{millis}.txt"));
    let mut file = std::fs::File::create(&path)?;
    file.write_all(text.as_bytes())?;
    Ok(path)
}

#[allow(clippy::too_many_arguments)]
fn portal_view_cone_debug_dump_text(
    reason: &str,
    selection: &crate::PortalEffectSelection,
    config: &PortalViewConeConfig,
    quality: &PortalCaptureQualityBudget,
    viewer: Option<&PortalViewer>,
    frame: &PortalWorldFrame,
    host_view: Option<&PortalCameraContinuityHostView>,
    portals: &Query<&PlacedPortal>,
    rigs: &Query<(
        &PortalViewRig,
        &Camera,
        &Projection,
        Option<&GlobalTransform>,
    )>,
    cone_visibility: &Query<(&Visibility, Option<&GlobalTransform>), With<PortalConeMesh>>,
    drawables: &[(String, crate::PortalCompositingCandidate, f32, Option<RenderLayers>)],
    screen_scale: f32,
    convention: ambition_portal2d::pieces::MapConvention,
) -> String {
    let mut out = String::new();
    let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    let (clip_min, clip_max) = portal_window_clip_rect(frame, host_view);
    let effective = effective_portal_capture_budget(config, quality);

    let _ = writeln!(out, "Portal view-cone debug dump");
    let _ = writeln!(out, "reason: {reason}");
    let _ = writeln!(out, "selection.active: {:?}", selection.active);
    let _ = writeln!(out, "frame.size: {}", fmt_vec2(frame.size));
    let _ = writeln!(
        out,
        "clip_rect: {} -> {}",
        fmt_vec2(clip_min),
        fmt_vec2(clip_max)
    );
    let _ = writeln!(out, "portal_count: {}", all.len());
    let _ = writeln!(out);
    let _ = writeln!(out, "config:");
    let _ = writeln!(out, "  mode: {:?}", config.mode);
    let _ = writeln!(out, "  visibility_mode: {:?}", config.visibility_mode);
    let _ = writeln!(
        out,
        "  aperture_los_quality: {:?}",
        config.aperture_los_quality
    );
    let _ = writeln!(out, "  source_clip_policy: {:?}", config.source_clip_policy);
    let _ = writeln!(
        out,
        "  capture_camera_mode: {:?}",
        config.capture_camera_mode
    );
    let _ = writeln!(
        out,
        "  dynamic_depth_close: {:.3}",
        config.dynamic_depth_close
    );
    let _ = writeln!(out, "  dynamic_depth_far: {:.3}", config.dynamic_depth_far);
    let _ = writeln!(
        out,
        "  dynamic_dist_close: {:.3}",
        config.dynamic_dist_close
    );
    let _ = writeln!(out, "  dynamic_dist_far: {:.3}", config.dynamic_dist_far);
    let _ = writeln!(
        out,
        "  half_plane_preview_full_distance: {:.3}",
        config.half_plane_preview_full_distance
    );
    let _ = writeln!(
        out,
        "  half_plane_preview_blend_distance: {:.3}",
        config.half_plane_preview_blend_distance
    );
    let _ = writeln!(
        out,
        "  half_plane_preview_max_lateral: {:.3}",
        config.half_plane_preview_max_lateral
    );
    let _ = writeln!(out, "  min_depth: {:.3}", config.min_depth);
    let _ = writeln!(out, "  min_spread: {:.3}", config.min_spread);
    let _ = writeln!(out, "  viewer_blend: {:.3}", config.viewer_blend);
    let _ = writeln!(out, "  static_depth: {:.3}", config.static_depth);
    let _ = writeln!(out, "  static_spread: {:.3}", config.static_spread);
    let _ = writeln!(
        out,
        "  texels_per_world_px: {:.3}",
        config.texels_per_world_px
    );
    let _ = writeln!(out, "  max_resolution: {}", config.max_resolution);
    let _ = writeln!(out, "  recursion_depth: {}", config.recursion_depth);
    let _ = writeln!(
        out,
        "  recursion_includes_portal_windows: {}",
        config.recursion_depth > 0
    );
    let _ = writeln!(out, "  z: {:.3}", config.z);
    let _ = writeln!(out, "  z_proximity_span: {:.3}", config.z_proximity_span);
    let _ = writeln!(out, "  blend_rate: {:.3}", config.blend_rate);
    let tint = config.tint.to_srgba();
    let _ = writeln!(
        out,
        "  tint_srgba: ({:.3}, {:.3}, {:.3}, {:.3})",
        tint.red, tint.green, tint.blue, tint.alpha
    );
    let _ = writeln!(out, "  debug_outline: {}", config.debug_outline);
    let _ = writeln!(out, "  debug_los_rays: {}", config.debug_los_rays);
    let _ = writeln!(out, "  debug_dump_portal: {:?}", config.debug_dump_portal);
    let _ = writeln!(out);
    let _ = writeln!(out, "effective_quality_budget:");
    let _ = writeln!(out, "  portal.max_resolution: {}", quality.max_resolution);
    let _ = writeln!(
        out,
        "  portal.texels_per_world_px: {:.3}",
        quality.texels_per_world_px
    );
    let _ = writeln!(out, "  portal.recursion_depth: {}", quality.recursion_depth);
    let _ = writeln!(
        out,
        "  portal.max_active_captures: {}",
        quality.max_active_captures
    );
    let _ = writeln!(
        out,
        "  portal.max_updates_per_frame: {}",
        quality.max_updates_per_frame
    );
    let _ = writeln!(
        out,
        "  portal.min_refresh_interval_s: {:.3}",
        quality.min_refresh_interval_s
    );
    let _ = writeln!(
        out,
        "  portal.include_parallax: {}",
        quality.include_parallax
    );
    let _ = writeln!(
        out,
        "  effective_max_resolution: {}",
        effective.max_resolution
    );
    let _ = writeln!(
        out,
        "  effective_texels_per_world_px: {:.3}",
        effective.texels_per_world_px
    );
    let _ = writeln!(
        out,
        "  effective_recursion_depth: {}",
        effective.recursion_depth
    );
    let _ = writeln!(
        out,
        "  effective_include_parallax: {}",
        effective.include_parallax
    );
    let _ = writeln!(out);

    match viewer {
        Some(viewer) => {
            let _ = writeln!(out, "viewer:");
            let _ = writeln!(out, "  present: {}", viewer.present);
            let _ = writeln!(out, "  eye: {}", fmt_vec2(viewer.eye));
            let _ = writeln!(out, "  player_position_estimate: {}", fmt_vec2(viewer.eye));
            let _ = writeln!(out, "  half_size: {}", fmt_vec2(viewer.half_size));
            let _ = writeln!(
                out,
                "  body_aabb: {} -> {}",
                fmt_vec2(viewer.eye - viewer.half_size),
                fmt_vec2(viewer.eye + viewer.half_size)
            );
            let _ = writeln!(
                out,
                "  inset_corners: {}",
                fmt_points(&inset_viewer_corners(viewer.eye, viewer.half_size))
            );
            let _ = writeln!(out, "  occluder_count: {}", viewer.occluders.len());
        }
        None => {
            let _ = writeln!(out, "viewer: <resource absent>");
        }
    }
    let _ = writeln!(out);

    let selected = selected_portals_for_dump(&all, &config.debug_dump_portal);
    let filter = config.debug_dump_portal.trim();
    if filter.is_empty() {
        let _ = writeln!(out, "debug_dump.filter: <all>");
    } else if selected.is_empty() {
        let _ = writeln!(out, "debug_dump.filter: {:?}", filter);
        let _ = writeln!(out, "debug_dump.resolved_pair: <no match>");
        let _ = writeln!(
            out,
            "debug_dump.available_portals: {}",
            fmt_portal_names(&all)
        );
        return out;
    } else {
        let pair = selected
            .iter()
            .map(|p| p.channel.name())
            .collect::<Vec<_>>()
            .join(" <-> ");
        let _ = writeln!(out, "debug_dump.filter: {:?}", filter);
        let _ = writeln!(out, "debug_dump.resolved_pair: {pair}");
    }
    let _ = writeln!(out, "debug_dump.printed_portal_count: {}", selected.len());
    let _ = writeln!(out);

    for portal in &selected {
        let _ = writeln!(out, "portal {}", portal.channel.name());
        let _ = writeln!(out, "  channel: {:?}", portal.channel);
        let _ = writeln!(out, "  pos: {}", fmt_vec2(portal.pos));
        let _ = writeln!(out, "  normal: {}", fmt_vec2(portal.normal));
        let _ = writeln!(out, "  half_extent: {}", fmt_vec2(portal.half_extent));
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            let _ = writeln!(out, "  partner: <missing>");
            let _ = writeln!(out);
            continue;
        };
        let _ = writeln!(out, "  partner: {}", partner.channel.name());
        let _ = writeln!(out, "  partner_pos: {}", fmt_vec2(partner.pos));
        let _ = writeln!(out, "  partner_normal: {}", fmt_vec2(partner.normal));

        let enter = portal.aperture();
        let exit = partner.aperture();
        let capture_frame =
            portal_capture_camera_frame(config, host_view, &enter, &exit, convention);
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(
                &effective,
                config,
                frame.size,
                partner.normal,
                capture_frame,
                screen_scale,
            ),
            recursion_depth: effective.recursion_depth,
            include_parallax: effective.include_parallax,
        };
        let route = visibility_route_summary(portal, &partner, config, viewer, convention);
        let _ = writeln!(
            out,
            "  route.face_los_fraction: {:.3} eyes={}",
            route.face_los_fraction, route.face_eye_count
        );
        let _ = writeln!(
            out,
            "  route.through_portal_los_fraction: {:.3} eyes={}",
            route.through_portal_los_fraction, route.through_portal_eye_count
        );
        let _ = writeln!(
            out,
            "  route.exit_side_los_fraction: {:.3} eyes={}",
            route.exit_side_los_fraction, route.exit_side_eye_count
        );
        let _ = writeln!(out, "  route.any_admitted: {}", route.admitted());

        let plan = compute_cone(portal, &partner, config, viewer, frame.size, convention);
        let _ = writeln!(out, "  plan.target: {:.3}", plan.target);
        let _ = writeln!(out, "  plan.immediate: {}", plan.immediate);
        let _ = writeln!(
            out,
            "  plan.min.entry_quad: {}",
            fmt_quad(plan.min.entry_quad)
        );
        let _ = writeln!(
            out,
            "  plan.wedge.entry_quad: {}",
            fmt_quad(plan.wedge.entry_quad)
        );
        let _ = writeln!(
            out,
            "  plan.wedge.source: {} -> {}",
            fmt_vec2(plan.wedge.source.min),
            fmt_vec2(plan.wedge.source.max)
        );
        let _ = writeln!(
            out,
            "  plan.wedge.source_size: {}",
            fmt_vec2(plan.wedge.source.max - plan.wedge.source.min)
        );
        let _ = writeln!(
            out,
            "  plan.debug.edge_distance_to_aperture: {}",
            fmt_option_f32(plan.debug.edge_distance_to_aperture)
        );
        let _ = writeln!(
            out,
            "  plan.debug.half_plane_preview_alpha: {:.3}",
            plan.debug.half_plane_preview_alpha
        );
        let _ = writeln!(
            out,
            "  plan.debug.finite_depth: {}",
            fmt_option_f32(plan.debug.finite_depth)
        );
        let _ = writeln!(
            out,
            "  plan.debug.half_plane_depth: {}",
            fmt_option_f32(plan.debug.half_plane_depth)
        );
        let _ = writeln!(
            out,
            "  plan.debug.finite_lateral_limit: {}",
            fmt_option_f32(plan.debug.finite_lateral_limit)
        );
        let _ = writeln!(
            out,
            "  plan.debug.half_plane_lateral_limit: {}",
            fmt_option_f32(plan.debug.half_plane_lateral_limit)
        );
        let _ = writeln!(
            out,
            "  plan.debug.finite_wedge.source_size: {}",
            fmt_option_vec2(plan.debug.finite_wedge_source_size)
        );
        let _ = writeln!(
            out,
            "  plan.debug.half_plane_wedge.source_size: {}",
            fmt_option_vec2(plan.debug.half_plane_wedge_source_size)
        );
        let _ = writeln!(out, "  rebuild.tex: {}x{}", rebuild.tex.x, rebuild.tex.y);
        write_capture_texture_debug(&mut out, config, frame.size, partner.normal);

        let rig_state = rigs
            .iter()
            .find(|(rig, _, _, _)| rig.channel == portal.channel);
        match rig_state {
            Some((rig, cam, proj, cam_global)) => {
                let _ = writeln!(out, "  rig.present: true");
                let _ = writeln!(out, "  rig.blend: {:.3}", rig.blend);
                let _ = writeln!(out, "  rig.parallax_layer: {}", rig.parallax_layer);
                let _ = writeln!(
                    out,
                    "  rig.rebuild.world_size: {}",
                    fmt_vec2(rig.rebuild.world_size)
                );
                let _ = writeln!(
                    out,
                    "  rig.rebuild.tex: {}x{}",
                    rig.rebuild.tex.x, rig.rebuild.tex.y
                );
                let _ = writeln!(out, "  camera.is_active: {}", cam.is_active);
                if let Some(global) = cam_global {
                    let _ = writeln!(
                        out,
                        "  camera.global_translation: {}",
                        fmt_vec3(global.translation())
                    );
                }
                if let Projection::Orthographic(o) = proj {
                    let _ = writeln!(out, "  camera.scaling_mode: {:?}", o.scaling_mode);
                }
                match cone_visibility.get(rig.cone) {
                    Ok((vis, cone_global)) => {
                        let _ = writeln!(out, "  cone.visibility: {:?}", vis);
                        if let Some(global) = cone_global {
                            let _ = writeln!(
                                out,
                                "  cone.global_translation: {}",
                                fmt_vec3(global.translation())
                            );
                        }
                    }
                    Err(_) => {
                        let _ = writeln!(out, "  cone.visibility: <missing cone entity>");
                    }
                }
            }
            None => {
                let _ = writeln!(out, "  rig.present: false");
            }
        }

        if plan.target > 0.0 {
            let blend = rig_state
                .map(|(rig, _, _, _)| rig.blend)
                .unwrap_or(plan.target);
            let cone = blend_cones(
                &plan.min,
                &plan.wedge,
                smooth01(blend),
                &enter,
                &exit,
                convention,
            );
            let capture_frame =
                portal_capture_camera_frame(config, host_view, &enter, &exit, convention);
            match cone_render(
                &cone,
                &enter,
                &exit,
                frame,
                config,
                clip_min,
                clip_max,
                pane_z(config, viewer, portal, &partner, None).0,
                capture_frame,
                convention,
            ) {
                Some(render) => {
                    let _ = writeln!(out, "  render.present: true");
                    let _ = writeln!(
                        out,
                        "  render.source_clip_policy: {:?}",
                        config.source_clip_policy
                    );
                    let clip = source_clip_debug(
                        plan.wedge.source.min,
                        plan.wedge.source.max,
                        render.source_min,
                        render.source_max,
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_rect: {} -> {}",
                        fmt_vec2(render.source_min),
                        fmt_vec2(render.source_max)
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_size: {}",
                        fmt_vec2(render.source_size)
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_clipped_by_plan: {}",
                        clip.source_clipped_by_plan
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_plan_size: {}",
                        fmt_vec2(clip.source_plan_size)
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_clip_loss_min: {}",
                        fmt_vec2(clip.source_clip_loss_min)
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_clip_loss_max: {}",
                        fmt_vec2(clip.source_clip_loss_max)
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_clip_loss_total: {}",
                        fmt_vec2(clip.source_clip_loss_total)
                    );
                    let _ = writeln!(
                        out,
                        "  render.source_clip_loss_fraction: {}",
                        fmt_vec2(clip.source_clip_loss_fraction)
                    );
                    let texels_per_world = Vec2::new(
                        rebuild.tex.x as f32 / render.source_size.x.max(1.0),
                        rebuild.tex.y as f32 / render.source_size.y.max(1.0),
                    );
                    let texture_aspect = rebuild.tex.x as f32 / (rebuild.tex.y as f32).max(1.0);
                    let source_aspect = render.source_size.x / render.source_size.y.max(1.0);
                    let _ = writeln!(out, "  render.texture_aspect: {:.3}", texture_aspect);
                    let _ = writeln!(out, "  render.source_aspect: {:.3}", source_aspect);
                    let _ = writeln!(
                        out,
                        "  render.source_to_texture_texels_per_world: {}",
                        fmt_vec2(texels_per_world)
                    );
                    let _ = writeln!(out, "  render.centroid: {}", fmt_vec3(render.centroid));
                    let _ = writeln!(out, "  render.cam_center: {}", fmt_vec3(render.cam_center));
                    let _ = writeln!(out, "  render.vertex_count: {}", render.positions.len());
                    let _ = writeln!(out, "  render.index_count: {}", render.indices.len());
                    let _ = writeln!(
                        out,
                        "  render.entry_poly_world: {}",
                        fmt_points(&render.entry_poly_world)
                    );
                    let _ = writeln!(
                        out,
                        "  render.mapped_source_vertices: {}",
                        fmt_points(&render.mapped_source_vertices)
                    );
                    let _ = writeln!(
                        out,
                        "  render.positions: {}",
                        fmt_positions(&render.positions)
                    );
                    let _ = writeln!(out, "  render.uvs: {}", fmt_uvs(&render.uvs));
                    let _ = writeln!(out, "  render.indices: {:?}", render.indices);
                }
                None => {
                    let _ = writeln!(out, "  render.present: false");
                }
            }
        } else {
            let _ = writeln!(out, "  render.present: false");
        }
        write_compositing_section(&mut out, portal, viewer.map(|v| v.eye), drawables);
        let _ = writeln!(out);
    }

    out
}

/// Why a drawable appeared above or below THIS pane.
///
/// ⛔⛔ THE EXISTING DUMP CAN PROVE THE POLYGON IS RIGHT AND MISS THE BUG
/// ENTIRELY. It records the entry polygon, the source rect, the capture camera,
/// blend and LOS — everything about GEOMETRY — and says nothing about ordering.
/// So it reports a geometrically perfect pane while an actor at `z = 11.0`
/// necessarily punches through it at `z = 9.5`. This section is the missing
/// half: for each candidate whose drawn bounds meet the pane, the relation it
/// SHOULD have and whether today's z gives it.
///
/// ⚠ THE CANDIDATE COUNT IS PRINTED EVEN WHEN IT IS ZERO, deliberately. A host
/// that has tagged no [`crate::PortalCompositingCandidate`] gets an empty report,
/// and an empty report beside a screenshot showing an overlap means THE HOST HAS
/// NOT TAGGED, not that the scene is clean. Saying `candidates: 0` distinguishes
/// those two; printing nothing would not.
fn write_compositing_section(
    out: &mut String,
    portal: &PlacedPortal,
    viewer_pos: Option<Vec2>,
    drawables: &[(String, crate::PortalCompositingCandidate, f32, Option<RenderLayers>)],
) {
    let _ = writeln!(out, "  compositing.pane_z: {:.3}", crate::PORTAL_WINDOW_Z);
    // ⛔ NEAR AND FAR ARE RELATIVE TO A VIEWPOINT. With no `PortalViewer` there
    // is no near side, so this reports that it cannot classify rather than
    // picking a default and printing confident nonsense.
    let Some(viewer_pos) = viewer_pos else {
        let _ = writeln!(out, "  compositing.viewer: ABSENT — cannot classify");
        return;
    };
    let _ = writeln!(out, "  compositing.viewer: {}", fmt_vec2(viewer_pos));
    let _ = writeln!(out, "  compositing.candidates: {}", drawables.len());
    let frame = portal.frame();
    let _ = writeln!(
        out,
        "  compositing.viewer_front: {:.3}",
        ambition_portal2d::pieces::front_distance(viewer_pos, &frame)
    );
    let mut violations = 0usize;
    for (name, view, z, layers) in drawables {
        // ⚠ DRAWN bounds, not the collision box: the question is which pixels a
        // pane should cover, and a sprite routinely overhangs the box.
        let (min, max) = (view.drawn_centre - view.drawn_half, view.drawn_centre + view.drawn_half);
        let relation = crate::pane_relation(portal, viewer_pos, min, max, false);
        if matches!(relation, crate::PaneRelation::Disjoint) {
            continue;
        }
        let correct = crate::current_z_policy_is_correct_for(relation, *z, crate::PORTAL_WINDOW_Z);
        if !correct {
            violations += 1;
        }
        let _ = writeln!(out, "  compositing.drawable {name}");
        let _ = writeln!(out, "    bounds_world: {} .. {}", fmt_vec2(min), fmt_vec2(max));
        let _ = writeln!(out, "    render_z: {z:.3}");
        let _ = writeln!(
            out,
            "    actor_front: {:.3}",
            ambition_portal2d::pieces::front_distance(view.drawn_centre, &frame)
        );
        let _ = writeln!(out, "    expected_relation: {relation:?}");
        let _ = writeln!(
            out,
            "    render_layers: {}",
            match layers {
                Some(l) => format!("{l:?}"),
                // ⚠ Bevy's default is layer 0, the world layer — the same one the
                // capture reads. Saying "default(0)" rather than "none" stops a
                // reader concluding the actor is on no layer at all.
                None => "default(0)".to_string(),
            }
        );
        // ⛔⛔ A Z COMPARISON ONLY SETTLES ORDERING IF BOTH DRAW UNDER THE SAME
        // CAMERA ON A SHARED LAYER, and this now ANSWERS that rather than
        // warning about it. The window mesh carries the shared window layer plus
        // a per-portal one; an actor sprite carries none, i.e. Bevy's default
        // layer 0. If those masks do not intersect, the two are never compared
        // by any depth test and a `depth_says` verdict is not weak evidence --
        // it is NO evidence.
        // ⚠ THE PANE'S LAYERS, which the report used to tell the reader to
        // "compare" without ever printing. Both masks are here now.
        //
        // ⛔⛔ AND THEY DO NOT SETTLE IT, WHICH IS WHY THE HEDGE STAYS.
        // `RenderLayers` gates which CAMERA sees an entity; it does not group
        // depth. Two entities on different layers rendered by the SAME camera
        // are still depth-tested against each other, so "disjoint masks" is not
        // "never compared" — an earlier version of this line said exactly that
        // and was wrong. Settling it needs the camera stack (which cameras, in
        // what order, with which masks), and that is the host's to publish.
        let _ = writeln!(
            out,
            "    pane_render_layers: {:?}",
            crate::view_cones::portal_window_render_layers(portal.channel)
        );
        let _ = writeln!(
            out,
            "    depth_says: {} (a camera rendering BOTH masks compares them; \
             the camera stack is not published here)",
            if *z > crate::PORTAL_WINDOW_Z { "ABOVE_PANE" } else { "BELOW_PANE" }
        );
        let _ = writeln!(out, "    COMPOSITE_VIOLATION: {}", !correct);
    }
    let _ = writeln!(out, "  compositing.violations: {violations}");
}

#[derive(Clone, Copy, Debug)]
struct SourceClipDebug {
    source_clipped_by_plan: bool,
    source_plan_size: Vec2,
    source_clip_loss_min: Vec2,
    source_clip_loss_max: Vec2,
    source_clip_loss_total: Vec2,
    source_clip_loss_fraction: Vec2,
}

fn source_clip_debug(
    plan_min: Vec2,
    plan_max: Vec2,
    render_min: Vec2,
    render_max: Vec2,
) -> SourceClipDebug {
    let source_plan_size = (plan_max - plan_min).max(Vec2::ZERO);
    let source_clip_loss_min = (render_min - plan_min).max(Vec2::ZERO);
    let source_clip_loss_max = (plan_max - render_max).max(Vec2::ZERO);
    let source_clip_loss_total = source_clip_loss_min + source_clip_loss_max;
    let source_clip_loss_fraction = Vec2::new(
        source_clip_loss_total.x / source_plan_size.x.max(1.0),
        source_clip_loss_total.y / source_plan_size.y.max(1.0),
    );
    let source_clipped_by_plan = source_clip_loss_total.x > 0.01 || source_clip_loss_total.y > 0.01;
    SourceClipDebug {
        source_clipped_by_plan,
        source_plan_size,
        source_clip_loss_min,
        source_clip_loss_max,
        source_clip_loss_total,
        source_clip_loss_fraction,
    }
}

/// One read-only row for a host debug UI. Labels intentionally use dump/Rust
/// variable paths; explanatory text belongs in `help`.
#[derive(Clone, Debug)]
pub struct PortalViewConeDebugRow {
    pub label: String,
    pub value: String,
    pub units: &'static str,
    pub help: &'static str,
}

impl PortalViewConeDebugRow {
    fn new(
        label: impl Into<String>,
        value: impl Into<String>,
        units: &'static str,
        help: &'static str,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            units,
            help,
        }
    }
}

/// Build compact selected-portal-pair diagnostics for the F3 inspector from
/// the same compute/render path used by the F8 dump.
pub fn selected_portal_view_cone_debug_rows(
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
    frame: &PortalWorldFrame,
    host_view: Option<&PortalCameraContinuityHostView>,
    portals: &[PlacedPortal],
    convention: ambition_portal2d::pieces::MapConvention,
) -> Vec<PortalViewConeDebugRow> {
    let mut rows = Vec::new();
    rows.push(PortalViewConeDebugRow::new(
        "recursion_includes_portal_windows",
        (config.recursion_depth > 0).to_string(),
        "derived",
        "Derived runtime value from recursion_depth. False means capture cameras exclude portal-window meshes.",
    ));

    let filter = config.debug_dump_portal.trim();
    if filter.is_empty() {
        rows.push(PortalViewConeDebugRow::new(
            "selected_pair.resolved_pair",
            "<debug_dump_portal empty>",
            "portal pair",
            "Set debug_dump_portal to a portal name such as c136 to show selected-pair diagnostics.",
        ));
        return rows;
    }

    let selected = selected_portals_for_dump(portals, filter);
    if selected.is_empty() {
        rows.push(PortalViewConeDebugRow::new(
            "selected_pair.resolved_pair",
            "<no match>",
            "portal pair",
            "No live portal matched debug_dump_portal.",
        ));
        return rows;
    }

    let pair = selected
        .iter()
        .map(|p| p.channel.name())
        .collect::<Vec<_>>()
        .join(" <-> ");
    rows.push(PortalViewConeDebugRow::new(
        "selected_pair.resolved_pair",
        pair,
        "portal pair",
        "Resolved portal pair for debug_dump_portal.",
    ));

    let (clip_min, clip_max) = portal_window_clip_rect(frame, host_view);
    for portal in &selected {
        let name = portal.channel.name();
        let Some(partner) = find_portal(portals, portal.channel.partner()) else {
            rows.push(PortalViewConeDebugRow::new(
                format!("selected_pair.{name}.partner"),
                "<missing>",
                "portal",
                "The selected portal has no live partner.",
            ));
            continue;
        };
        let enter = portal.aperture();
        let exit = partner.aperture();
        let plan = compute_cone(portal, &partner, config, viewer, frame.size, convention);
        rows.push(PortalViewConeDebugRow::new(
            format!("selected_pair.{name}.plan.target"),
            format!("{:.3}", plan.target),
            "0..1",
            "Current target visibility blend for this portal plan.",
        ));
        rows.push(PortalViewConeDebugRow::new(
            format!("selected_pair.{name}.plan.wedge.source_size"),
            fmt_vec2(plan.wedge.source.max - plan.wedge.source.min),
            "world px",
            "Planned source rect size before final frame/policy reconciliation.",
        ));
        if plan.target <= 0.0 {
            rows.push(PortalViewConeDebugRow::new(
                format!("selected_pair.{name}.render.present"),
                "false",
                "derived",
                "No render data is built because plan.target is zero.",
            ));
            continue;
        }
        let cone = blend_cones(
            &plan.min,
            &plan.wedge,
            smooth01(plan.target),
            &enter,
            &exit,
            convention,
        );
        let capture_frame =
            portal_capture_camera_frame(config, host_view, &enter, &exit, convention);
        match cone_render(
            &cone,
            &enter,
            &exit,
            frame,
            config,
            clip_min,
            clip_max,
            pane_z(config, viewer, portal, &partner, None).0,
            capture_frame,
            convention,
        ) {
            Some(render) => {
                let clip = source_clip_debug(
                    plan.wedge.source.min,
                    plan.wedge.source.max,
                    render.source_min,
                    render.source_max,
                );
                rows.push(PortalViewConeDebugRow::new(
                    format!("selected_pair.{name}.render.present"),
                    "true",
                    "derived",
                    "True when final mesh/camera render data exists for this portal.",
                ));
                rows.push(PortalViewConeDebugRow::new(
                    format!("selected_pair.{name}.render.source_size"),
                    fmt_vec2(render.source_size),
                    "world px",
                    "Final source rect size used by mesh UVs and capture-camera scaling.",
                ));
                rows.push(PortalViewConeDebugRow::new(
                    format!("selected_pair.{name}.render.source_clipped_by_plan"),
                    clip.source_clipped_by_plan.to_string(),
                    "derived",
                    "True when the final source rect lost area relative to plan.wedge.source.",
                ));
                rows.push(PortalViewConeDebugRow::new(
                    format!("selected_pair.{name}.render.source_clip_loss_fraction"),
                    fmt_vec2(clip.source_clip_loss_fraction),
                    "fraction",
                    "Per-axis fraction of the planned source rect lost by the final render source rect.",
                ));
            }
            None => {
                rows.push(PortalViewConeDebugRow::new(
                    format!("selected_pair.{name}.render.present"),
                    "false",
                    "derived",
                    "No render data remains after final clipping/policy reconciliation.",
                ));
            }
        }
    }

    rows
}

fn selected_portals_for_dump(all: &[PlacedPortal], filter: &str) -> Vec<PlacedPortal> {
    let filter = filter.trim();
    if filter.is_empty() {
        return all.to_vec();
    }
    let Some(portal) = all
        .iter()
        .find(|portal| portal_name_matches(portal, filter))
    else {
        return Vec::new();
    };
    let mut selected = vec![portal.clone()];
    if let Some(partner) = find_portal(all, portal.channel.partner()) {
        if partner.channel != portal.channel {
            selected.push(partner);
        }
    }
    selected
}

fn portal_name_matches(portal: &PlacedPortal, filter: &str) -> bool {
    let name = portal.channel.name();
    if name.eq_ignore_ascii_case(filter) {
        return true;
    }
    match (
        name.strip_prefix('c'),
        filter.strip_prefix('c').or(Some(filter)),
    ) {
        (Some(name_index), Some(filter_index)) => name_index == filter_index,
        _ => false,
    }
}

fn fmt_portal_names(portals: &[PlacedPortal]) -> String {
    portals
        .iter()
        .map(|portal| portal.channel.name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn write_capture_texture_debug(
    out: &mut String,
    config: &PortalViewConeConfig,
    world_size: Vec2,
    exit_normal: Vec2,
) {
    let density = config.texels_per_world_px.max(0.05);
    let long_world_extent = world_size.x.max(world_size.y);
    let long_uncapped = long_world_extent * density;
    let long_tex = ((long_uncapped) as u32).clamp(256, config.max_resolution.max(256));
    let max_depth = config
        .dynamic_depth_close
        .max(config.static_depth)
        .max(config.min_depth);
    let short_world_extent = max_depth * 2.0;
    let short_uncapped = short_world_extent * density;
    let short_tex = ((short_uncapped) as u32).next_power_of_two().clamp(64, 512);
    let orientation = if exit_normal.x.abs() > 0.5 {
        "wall_exit_lateral_y_short_x"
    } else {
        "floor_or_ceiling_exit_lateral_x_short_y"
    };
    let _ = writeln!(out, "  texture.density_texels_per_world_px: {:.3}", density);
    let _ = writeln!(out, "  texture.exit_normal: {}", fmt_vec2(exit_normal));
    let _ = writeln!(out, "  texture.orientation: {orientation}");
    let _ = writeln!(out, "  texture.long_world_extent: {:.3}", long_world_extent);
    let _ = writeln!(out, "  texture.long_texels_uncapped: {:.3}", long_uncapped);
    let _ = writeln!(out, "  texture.long_texels_final: {}", long_tex);
    let _ = writeln!(out, "  texture.max_depth_for_short_axis: {:.3}", max_depth);
    let _ = writeln!(
        out,
        "  texture.short_world_extent: {:.3}",
        short_world_extent
    );
    let _ = writeln!(
        out,
        "  texture.short_texels_uncapped: {:.3}",
        short_uncapped
    );
    let _ = writeln!(out, "  texture.short_texels_power2_final: {}", short_tex);
}

fn fmt_vec2(v: Vec2) -> String {
    format!("({:.2}, {:.2})", v.x, v.y)
}

fn fmt_option_vec2(v: Option<Vec2>) -> String {
    match v {
        Some(v) => fmt_vec2(v),
        None => "None".to_string(),
    }
}

fn fmt_option_f32(v: Option<f32>) -> String {
    match v {
        Some(v) => format!("{v:.3}"),
        None => "None".to_string(),
    }
}

fn fmt_vec3(v: Vec3) -> String {
    format!("({:.2}, {:.2}, {:.2})", v.x, v.y, v.z)
}

fn fmt_quad(quad: [Vec2; 4]) -> String {
    format!(
        "[{}, {}, {}, {}]",
        fmt_vec2(quad[0]),
        fmt_vec2(quad[1]),
        fmt_vec2(quad[2]),
        fmt_vec2(quad[3])
    )
}

fn fmt_points(points: &[Vec2]) -> String {
    format!(
        "[{}]",
        points
            .iter()
            .map(|p| fmt_vec2(*p))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn fmt_positions(positions: &[[f32; 3]]) -> String {
    format!(
        "[{}]",
        positions
            .iter()
            .map(|p| format!("({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn fmt_uvs(uvs: &[[f32; 2]]) -> String {
    format!(
        "[{}]",
        uvs.iter()
            .map(|uv| format!("({:.3}, {:.3})", uv[0], uv[1]))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Debug overlay for portal view cones: when the host's F1 debug overlay is on,
/// optional F3 toggles can draw the exit sample zone (the world rect
/// `ViewCone::source` in front of the partner, in the portal's channel color),
/// the entry window, and/or the LOS rays that decide whether the viewer can see
/// through the aperture. Uses the SAME `compute_cone` as the renderer, so the
/// gizmo reflects the live viewer-dependent wedge (or nothing, when the
/// aperture is occluded). The sample zone shows where the capture samples
/// from; the entry window shows where it is displayed.
pub fn debug_portal_view_zones(
    selection: Res<crate::PortalEffectSelection>,
    config: Res<PortalViewConeConfig>,
    debug: Res<PortalDebugOverlay>,
    viewer: Option<Res<PortalViewer>>,
    frame: Res<PortalWorldFrame>,
    portals: Query<&PlacedPortal>,
    // ⚠ THE TRANSFORM COMES WITH IT: the overlay used to colour a violation
    // without knowing the drawn z at all, which meant it was carrying the same
    // hardcoded "actors outrank panes" assumption the dump was built to expose.
    compositing: Query<(&crate::PortalCompositingCandidate, Option<&GlobalTransform>)>,
    mut gizmos: Gizmos,
    // The session's portal map convention, from the resource that owns it.
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
) {
    if selection.active != crate::PortalVisualEffect::ViewCones
        || !debug.enabled
        || frame.size == Vec2::ZERO
        || (!config.debug_outline && !config.debug_los_rays)
    {
        return;
    }
    let convention = tuning
        .as_deref()
        .map(|tuning| tuning.convention.map_convention())
        .unwrap_or_default();
    let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    let viewer = viewer.as_deref();
    let to_render = |p: Vec2| frame.to_render(p, 0.0).truncate();
    draw_compositing_relations(&mut gizmos, &all, viewer, &compositing, to_render);
    for portal in &all {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        let (enter, exit) = (portal.aperture(), partner.aperture());
        let plan = compute_cone(portal, &partner, &config, viewer, frame.size, convention);
        let (_, core) = portal.channel.display();

        if config.debug_outline && plan.target > 0.0 {
            let cone = blend_cones(
                &plan.min,
                &plan.wedge,
                smooth01(plan.target),
                &enter,
                &exit,
                convention,
            );
            // Exit sample zone: the source rect (axis-aligned in world stays
            // axis-aligned through the y-flip). Bright channel color.
            let s = cone.source;
            gizmos.linestrip_2d(
                [
                    to_render(Vec2::new(s.min.x, s.min.y)),
                    to_render(Vec2::new(s.max.x, s.min.y)),
                    to_render(Vec2::new(s.max.x, s.max.y)),
                    to_render(Vec2::new(s.min.x, s.max.y)),
                    to_render(Vec2::new(s.min.x, s.min.y)),
                ],
                core,
            );
            // Entry window, dimmer so the two never read as the same shape.
            let entry: Vec<Vec2> = cone
                .entry_quad
                .iter()
                .chain(std::iter::once(&cone.entry_quad[0]))
                .map(|p| to_render(*p))
                .collect();
            gizmos.linestrip_2d(entry, core.with_alpha(0.4));
        }

        if config.debug_los_rays && config.mode == PortalViewConeMode::Dynamic {
            let Some(viewer) = viewer.filter(|v| v.present) else {
                continue;
            };
            let corners = inset_viewer_corners(viewer.eye, viewer.half_size);
            for origin in corners {
                let mut candidate_rays: Vec<Vec<ApertureLosRay>> = Vec::new();
                candidate_rays.push(aperture_los_rays(
                    origin,
                    &enter,
                    &viewer.occluders,
                    config.aperture_los_quality,
                ));
                let direct_fraction = aperture_visibility_fraction(
                    origin,
                    &enter,
                    &viewer.occluders,
                    config.aperture_los_quality,
                );
                if let Some((_, via_partner)) = window_eye(&enter, &exit, origin, convention) {
                    if config
                        .visibility_mode
                        .admit_through_portal(direct_fraction, via_partner)
                    {
                        candidate_rays.push(aperture_los_rays(
                            origin,
                            if via_partner { &exit } else { &enter },
                            &viewer.occluders,
                            config.aperture_los_quality,
                        ));
                    }
                }
                if config.visibility_mode.admit_exit_side(direct_fraction)
                    && (origin - exit.frame.origin).dot(exit.frame.normal) < 0.0
                {
                    candidate_rays.push(aperture_los_rays(
                        origin,
                        &exit,
                        &viewer.occluders,
                        config.aperture_los_quality,
                    ));
                }
                for ray in candidate_rays.into_iter().flatten() {
                    let clear = ray.hit.is_none();
                    let end = ray.hit.unwrap_or(ray.target);
                    let color = if clear {
                        core.with_alpha(0.95)
                    } else {
                        core.with_alpha(0.30)
                    };
                    let hit_color = if clear {
                        Color::srgba(0.14, 1.00, 0.65, 0.95)
                    } else {
                        Color::srgba(1.00, 0.32, 0.28, 0.80)
                    };
                    gizmos.line_2d(to_render(ray.origin), to_render(end), color);
                    gizmos.line_2d(
                        to_render(end + Vec2::new(-3.0, -3.0)),
                        to_render(end + Vec2::new(3.0, 3.0)),
                        hit_color,
                    );
                    gizmos.line_2d(
                        to_render(end + Vec2::new(-3.0, 3.0)),
                        to_render(end + Vec2::new(3.0, -3.0)),
                        hit_color,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod compositing_report_tests {
    use super::*;
    use ambition_portal2d::{PlacedPortal, PortalChannel, PortalChannelColor};

    fn pane() -> PlacedPortal {
        PlacedPortal {
            channel: PortalChannel::Authored(PortalChannelColor::Purple),
            pos: Vec2::new(100.0, 300.0),
            normal: Vec2::new(0.0, 1.0),
            half_extent: Vec2::new(46.0, 9.0),
            host: None,
            host_lift: 0.0,
            vel: Vec2::ZERO,
            prev_pos: Vec2::new(100.0, 300.0),
        }
    }

    fn body_at(y: f32) -> crate::PortalCompositingCandidate {
        crate::PortalCompositingCandidate {
            drawn_centre: Vec2::new(100.0, y),
            drawn_half: Vec2::new(14.0, 23.0),
        }
    }

    /// ⛔⛔ THE DUMP MUST NAME THE VIOLATION, not merely describe geometry.
    ///
    /// The existing dump can prove the entry polygon is perfect while the actor
    /// at `z = 11.0` necessarily punches through a pane at `z = 9.5`. This is the
    /// arm that would have made the reported screenshot self-diagnosing.
    #[test]
    fn a_far_side_drawable_is_reported_as_a_composite_violation() {
        let mut out = String::new();
        // Viewer above the floor pane; the body BELOW it, overlapping, drawn at
        // the ordinary actor z.
        let drawables = vec![(
            "Perfect Cellular Automaton".to_string(),
            body_at(292.0),
            11.0,
            None,
        )];
        write_compositing_section(&mut out, &pane(), Some(Vec2::new(100.0, 360.0)), &drawables);
        assert!(out.contains("expected_relation: FarCovered"), "{out}");
        assert!(out.contains("depth_says: ABOVE_PANE"), "{out}");
        assert!(out.contains("COMPOSITE_VIOLATION: true"), "{out}");
        assert!(out.contains("compositing.violations: 1"), "{out}");
    }

    /// ⚠ THE CONTROL. A report that cried violation for every overlap would pass
    /// the arm above and be useless: the near-side case is what today's z gets
    /// RIGHT, and it must read as correct.
    #[test]
    fn a_near_side_drawable_is_reported_as_correct_under_todays_z() {
        let mut out = String::new();
        let drawables = vec![("Near NPC".to_string(), body_at(308.0), 11.0, None)];
        write_compositing_section(&mut out, &pane(), Some(Vec2::new(100.0, 360.0)), &drawables);
        assert!(out.contains("expected_relation: NearOccluder"), "{out}");
        assert!(out.contains("COMPOSITE_VIOLATION: false"), "{out}");
        assert!(out.contains("compositing.violations: 0"), "{out}");
    }

    /// ⛔ AN EMPTY REPORT MUST BE DISTINGUISHABLE FROM A CLEAN ONE. A host that
    /// has tagged no candidates gets `candidates: 0`, which is what tells a
    /// reader the instrument is blind rather than the scene innocent.
    #[test]
    fn an_untagged_host_reports_zero_candidates_rather_than_silence() {
        let mut out = String::new();
        write_compositing_section(&mut out, &pane(), Some(Vec2::new(100.0, 360.0)), &[]);
        assert!(out.contains("compositing.candidates: 0"), "{out}");
        assert!(out.contains("compositing.violations: 0"), "{out}");
    }

    /// ⛔ NEAR AND FAR NEED A VIEWPOINT. With no `PortalViewer` the section must
    /// say it cannot classify rather than defaulting to one side.
    #[test]
    fn no_viewer_reports_that_it_cannot_classify() {
        let mut out = String::new();
        let drawables = vec![("Somebody".to_string(), body_at(292.0), 11.0, None)];
        write_compositing_section(&mut out, &pane(), None, &drawables);
        assert!(out.contains("ABSENT — cannot classify"), "{out}");
        assert!(!out.contains("COMPOSITE_VIOLATION"), "{out}");
    }
    /// ⛔⛔ THE REPORT MUST NOT CLAIM MORE THAN A Z CAN SETTLE. A depth comparison
    /// only decides ordering when both drawables share a camera AND a layer, and
    /// the window mesh uses per-portal layers. So the line is `depth_says:` and
    /// carries its own caveat, and the layer is printed beside it.
    ///
    /// ⚠ This is the same over-claim as the z policy the section exists to
    /// expose, one level up: asserting an outcome from one of the two facts that
    /// decide it.
    #[test]
    fn the_report_qualifies_what_a_depth_comparison_can_settle() {
        let mut out = String::new();
        let drawables = vec![("Somebody".to_string(), body_at(292.0), 11.0, None)];
        write_compositing_section(&mut out, &pane(), Some(Vec2::new(100.0, 360.0)), &drawables);
        assert!(out.contains("depth_says:"), "{out}");
        // The qualifier names the CAMERA, because that is the fact that is
        // missing -- not the layers, which are both printed below.
        assert!(out.contains("camera"), "depth must stay qualified: {out}");
        assert!(
            out.contains("render_layers: default(0)"),
            "an absent RenderLayers is Bevy's layer 0, not 'no layer': {out}"
        );
        // ⚠ BOTH masks, because the old report told the reader to "compare
        // render_layers" while printing only the actor's half.
        assert!(
            out.contains("pane_render_layers:"),
            "the pane's own layers were never printed: {out}"
        );
        assert!(
            !out.contains("actual_ordering"),
            "the unqualified claim must be gone: {out}"
        );
    }
}

/// Colour every candidate's bounds by what it is to each pane.
///
/// ⛔⛔ THE SCREENSHOT THAT STARTED THIS COULD NOT DIAGNOSE ITSELF. A far-side
/// actor drawn over a seamless window looks exactly like a near-side actor
/// correctly occluding one — the difference is which side of a plane it stands
/// on, which no still image shows. ⇒ These outlines say it: a body the pane
/// SHOULD be covering is drawn RED while it is drawn on top.
///
/// ⚠ ONE OUTLINE PER (PANE, CANDIDATE) PAIR, not one per candidate. A body is
/// near one aperture and far of another in the same frame, and collapsing that
/// to a single colour would reintroduce, in the diagnostic, the exact one-answer-
/// per-actor assumption the bug is made of.
fn draw_compositing_relations(
    gizmos: &mut Gizmos,
    panes: &[PlacedPortal],
    viewer: Option<&PortalViewer>,
    candidates: &Query<(&crate::PortalCompositingCandidate, Option<&GlobalTransform>)>,
    to_render: impl Fn(Vec2) -> Vec2,
) {
    // ⛔ Near and far are relative to a viewpoint; with none there is nothing
    // honest to colour.
    let Some(viewer) = viewer else {
        return;
    };
    for pane in panes {
        for (candidate, transform) in candidates {
            let drawn_z = transform.map(|t| t.translation().z).unwrap_or(f32::NAN);
            let min = candidate.drawn_centre - candidate.drawn_half;
            let max = candidate.drawn_centre + candidate.drawn_half;
            let relation = crate::pane_relation(pane, viewer.eye, min, max, false);
            let colour = match relation {
                crate::PaneRelation::Disjoint => continue,
                // Correct under today's z: it may occlude, and it does.
                crate::PaneRelation::NearOccluder => Color::srgb(0.20, 0.85, 0.30),
                // The split presentation owns this body.
                crate::PaneRelation::Transiting => Color::srgb(0.95, 0.85, 0.15),
                // ⛔ Should be covered BY the pane, and today is drawn over it.
                crate::PaneRelation::FarCovered
                    if !crate::current_z_policy_is_correct_for(
                        relation,
                        drawn_z,
                        crate::PORTAL_WINDOW_Z,
                    ) =>
                {
                    Color::srgb(0.95, 0.15, 0.15)
                }
                crate::PaneRelation::FarCovered => Color::srgb(0.25, 0.45, 0.95),
            };
            let a = to_render(min);
            let b = to_render(max);
            gizmos.rect_2d(
                Isometry2d::from_translation((a + b) * 0.5),
                (b - a).abs(),
                colour,
            );
        }
    }
}
