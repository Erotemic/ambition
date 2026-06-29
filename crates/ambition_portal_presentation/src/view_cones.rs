//! Through-portal **view windows**: each placed portal shows a slice of the
//! world in front of its partner, set into its host surface — you look "through
//! the portal a little bit" — rendered live by an offscreen capture camera.
//!
//! ## Viewer-dependent visibility
//! By default the window is the wedge the **controlled character** sees through
//! the aperture, from a host-supplied [`PortalViewer`]. The viewpoint set is
//! the character's four real AABB corners PLUS, for any corner that has crossed
//! the partner plane, its sprite-trick SHADOW (the body-map image) — so a
//! straddling viewer's presence at both ends feeds one continuous wedge
//! (`aperture_wedge_multi` unions them), with no abrupt flip at the midpoint of
//! a pair. Depth scales with proximity to the nearer aperture; line of sight is
//! a 4-corner × aperture-sample raycast fraction (partial cover ⇒ partial
//! window). Set [`PortalViewConeConfig::viewer_gated`] to `false` (or leave the
//! viewer unset) to fall back to the static, always-on `view_cone`.
//!
//! ## How a rig works
//! Per placed portal with a placed partner, a **rig**: one offscreen image, a
//! capture `Camera2d` framing the partner-side source rect, and a window
//! `Mesh2d` set into the entry's surface. The window source wedge comes from
//! [`view::view_cone`](ambition_portal::view::view_cone) (the body map, same as
//! the transit sprite copy) so the window and the copy read as one continuous
//! image; its rotation/mirror lives entirely in the per-vertex **UV mapping**
//! (computed inline below), and the capture camera stays axis-aligned. The
//! capture
//! renders into a fixed **square** texture; non-square source rects are stored
//! stretched and un-stretched by the UVs (mesh geometry is world-space), so the
//! texture never needs resizing as the viewer-dependent rect changes shape.
//!
//! Because the cone follows a moving viewer, rigs are **updated in place every
//! frame** (mesh attributes + camera transform/projection + visibility) and
//! only spawned/despawned when the set of portal pairs — or the world size /
//! capture resolution — changes. No per-frame texture or entity churn.
//!
//! ## 1-frame-lag recursion
//! Window meshes live on a dedicated render layer. The main camera sees that
//! layer; capture cameras include it only when
//! [`PortalViewConeConfig::recursion_depth`] is positive. That makes `0` a clean
//! "world only" capture and positive values keep the existing one-frame-lag
//! recursive feedback path.

#![allow(unused_imports)]
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite_render::AlphaMode2d;

use ambition_engine_core as ae;
use ambition_platformer_primitives::world_query::{raycast_solids, SolidWorldQuery};
use ambition_portal::pieces::PortalFrame;
use ambition_portal::view::{aperture_wedge_multi, blend_cones, view_cone, window_eye, ViewCone};
use ambition_portal::{find_portal, PlacedPortal, PortalChannel};

use crate::{PortalCameraContinuityHostView, PortalWorldFrame};

/// Clear color of an offscreen capture: a dark tone shows through wherever the
/// exit room has no geometry (rare — parallax usually fills it). Opaque windows
/// draw it directly, so keep it unobtrusive.
const CAPTURE_CLEAR: Color = Color::srgb(0.03, 0.04, 0.05);

const WORLD_RENDER_LAYER: usize = 0;
/// Dedicated layer for portal view-window meshes.
pub const PORTAL_WINDOW_RENDER_LAYER: usize = 5;

fn capture_render_layers(config: &PortalViewConeConfig) -> RenderLayers {
    let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
    if config.recursion_depth == 0 {
        layers
    } else {
        layers.with(PORTAL_WINDOW_RENDER_LAYER)
    }
}

/// Host seam: the controlled character's eye + the world's solid occluders,
/// used to compute the viewer-dependent visible wedge through each aperture.
/// The host (in Ambition: `crate::portal::sync_portal_viewer`) sets `eye` from
/// the possessed actor or the primary player and fills `occluders` from its
/// collision world each frame. `present == false` ⇒ no controlled viewer this
/// frame; the renderer then falls back to the static window if
/// [`PortalViewConeConfig::viewer_gated`] is on.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalViewer {
    /// Whether a controlled-character eye is available this frame.
    pub present: bool,
    /// The controlled character's eye position (body center), world space.
    pub eye: Vec2,
    /// The character's body half-size: line-of-sight is tested from all four
    /// body corners, so partial cover yields a partial (smoothly blended)
    /// window instead of a binary popping one.
    pub half_size: Vec2,
    /// Solid AABBs for the line-of-sight test — a portal whose aperture is
    /// occluded from `eye` renders no window. The host syncs these from its
    /// collision world (only the blocks that block sight).
    pub occluders: Vec<ae::Aabb>,
}

/// Host seam: whether the F1 debug overlay is currently active. Portal debug
/// gizmos stay quiet unless this is on, even when their individual F3 toggles
/// are enabled.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalDebugOverlay {
    /// True while the host's F1 debug mode is active.
    pub enabled: bool,
}

/// Quality/performance knob for viewer-dependent portal aperture LOS.
///
/// `Low` preserves the original center-point test. `Medium` treats the aperture
/// as a short segment by sampling its left endpoint, center, and right endpoint.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum PortalApertureLosQuality {
    /// One LOS ray per viewer corner, aimed at the lifted aperture center.
    Low,
    /// Three LOS rays per viewer corner: left endpoint, center, right endpoint.
    Medium,
}

impl Default for PortalApertureLosQuality {
    fn default() -> Self {
        Self::Medium
    }
}

/// Tuning for the view windows. A host overwrites the resource to retune; set
/// [`PortalPresentationPlugin::view_cones`](crate::PortalPresentationPlugin)
/// to `false` to drop the feature (and its capture passes) entirely.
#[derive(Resource, Clone, Debug, Reflect, PartialEq)]
#[reflect(Resource)]
pub struct PortalViewConeConfig {
    /// When true (default), each window opens to the controlled character's
    /// visible wedge through the aperture (from [`PortalViewer`]), blended up
    /// from the minimum cone. When false, windows render the static, always-on
    /// `view_cone` — the "always show this much" mode that needs no viewer.
    pub viewer_gated: bool,
    /// Aperture LOS quality. `Low` is the original single center ray per viewer
    /// corner. `Medium` is the default finite-aperture heuristic: sample the
    /// left endpoint, center, and right endpoint, then average visible samples.
    pub aperture_los_quality: PortalApertureLosQuality,
    /// Max window depth behind the surface (world px), reached when the viewer
    /// is within `dist_close` of the aperture — the "large maximum." The depth
    /// is proximity-proportional: the closer you are, the deeper you see (down
    /// to `depth_far` when beyond `dist_far`). The world bounds still clip it.
    pub depth_close: f32,
    /// Min window depth behind the surface (world px), reached when the viewer
    /// is beyond `dist_far`.
    pub depth_far: f32,
    /// Viewer→aperture distance (world px) at/below which depth = `depth_close`.
    pub dist_close: f32,
    /// Viewer→aperture distance (world px) at/beyond which depth = `depth_far`.
    pub dist_far: f32,
    /// Z range over which nearer portals' windows draw ON TOP of farther ones
    /// (added to `z` by an inverse-distance bias). Kept under the rim gap.
    pub z_proximity_span: f32,
    /// How quickly the window opens/closes between the minimum cone and the
    /// visible wedge (per second, exponential approach) — the temporal half of
    /// the smooth blend; the spatial half is the 4-corner visibility fraction.
    pub blend_rate: f32,
    /// The **minimum cone** every portal always shows (depth into the surface,
    /// world px), so a portal is never blank even when the character is behind
    /// both ends or its sight line is occluded.
    pub min_depth: f32,
    /// Minimum-cone side widening per px of depth.
    pub min_spread: f32,
    /// Blend from the minimum cone (0) toward the visible wedge (1). Keep at
    /// 1.0 (default): once ANY visibility exists the window follows the real
    /// visibility wedge exactly and the minimum has no influence — the minimum
    /// only fills in when no wedge exists at all. Lower values are for tuning
    /// transitions only.
    pub viewer_blend: f32,
    /// Static-fallback window depth into the surface (world px), used only when
    /// `viewer_gated` is off. Keep near wall scale.
    pub depth: f32,
    /// Static-fallback side widening per px of depth (0 = straight corridor).
    pub spread: f32,
    /// Capture sharpness target: texels per world pixel along the window's
    /// long (lateral) axis. The wedge runs to the half-plane (clipped only by
    /// the world bounds), so the texture's long side is sized from the WORLD
    /// extent × this density, capped by `max_resolution`. The short side
    /// covers the window depth. 1.0 ⇒ pixel-perfect up to the cap.
    pub texels_per_world_px: f32,
    /// Hard cap on the capture texture's long side (GPU memory guard; a
    /// 2048×256 RGBA capture is ~2 MB per portal).
    pub max_resolution: u32,
    /// Portal-window capture recursion. `0` makes capture cameras see only the
    /// world layer; positive values include other portal windows and preserve
    /// the current one-frame-lag recursive feedback. Exact multi-pass finite
    /// depth can later refine this field without changing the dev UI.
    pub recursion_depth: u32,
    /// Render z of the window mesh. Just BEHIND the portal rim (9.0) so the
    /// doorway stays crisp over its own view, above world blocks (0) and below
    /// actors (10+).
    pub z: f32,
    /// Tint multiplied over the capture (opaque — the window draws over what it
    /// is in front of). This is ALSO the **recursion attenuator**: a capture
    /// sees other portals' windows, so two facing/door portals recurse with one
    /// frame of lag. A tint slightly below white makes each nested level
    /// `tint × tint × …` → the infinite recursion CONVERGES to dark (a fading
    /// tunnel) instead of a full-brightness chaotic fractal. 1.0 = no
    /// attenuation (chaos); ~0.8 = a calm fade.
    pub tint: Color,
    /// Debug: draw gizmo outlines of each portal's EXIT sample zone (the
    /// `ViewCone::source` rect, in the portal's channel color, in front of its
    /// partner) and the entry window.
    pub debug_outline: bool,
    /// Debug: draw the line-of-sight rays used to decide whether the viewer can
    /// see into the portal. In low quality this is four rays; in medium quality
    /// it is four viewer corners times three aperture samples. Rays that reach
    /// the aperture are drawn brightly; blocked rays are truncated at the blocker.
    pub debug_los_rays: bool,
}

impl Default for PortalViewConeConfig {
    fn default() -> Self {
        Self {
            viewer_gated: true,
            aperture_los_quality: PortalApertureLosQuality::Medium,
            // Large but not so deep it punches through thin "door" walls into
            // the far room (which is what drives the heaviest recursion); also
            // keeps the near-face↔deep-content parallax modest.
            depth_close: 280.0,
            depth_far: 44.0,
            dist_close: 70.0,
            dist_far: 900.0,
            z_proximity_span: 0.35,
            blend_rate: 10.0,
            min_depth: 22.0,
            min_spread: 0.12,
            viewer_blend: 1.0,
            depth: 90.0,
            spread: 0.20,
            texels_per_world_px: 1.0,
            max_resolution: 2048,
            recursion_depth: 1,
            z: 8.55,
            // Slightly below white: opaque, but each recursion level multiplies
            // the tint so facing/door portals fade into a tunnel rather than a
            // full-brightness chaotic fractal (see the field docs). 1.0 brings
            // back the chaos; lower fades faster.
            tint: Color::srgb(0.8, 0.8, 0.8),
            debug_outline: true,
            debug_los_rays: false,
        }
    }
}

/// Marks a window mesh entity (the `Mesh2d` set into a portal's surface),
/// disjoint from the capture-camera entity that carries [`PortalViewRig`].
#[derive(Component)]
pub struct PortalConeMesh;

/// One rig, carried by the capture camera entity: which portal channel it
/// serves, the rebuild key it was built for, the live min↔wedge blend state,
/// and handles to its image + mesh + window-mesh entity. Geometry is updated
/// in place each frame; the rig is only respawned when `rebuild` drifts
/// (world size / texture dims) or the pair disappears.
#[derive(Component)]
pub struct PortalViewRig {
    channel: PortalChannel,
    rebuild: RebuildKey,
    /// Temporal blend state, 0 = minimum cone, 1 = full visible wedge;
    /// approaches the 4-corner visibility fraction at `blend_rate`/s and is
    /// shaped by a smoothstep before use, so opening/closing feels smooth.
    blend: f32,
    /// Keep-alive for the offscreen target (also referenced by the camera's
    /// `RenderTarget` and the window material; held here so the rig owns its
    /// asset lifetime explicitly).
    _image: Handle<Image>,
    mesh: Handle<Mesh>,
    cone: Entity,
}

fn portal_window_clip_rect(
    frame: &PortalWorldFrame,
    host_view: Option<&PortalCameraContinuityHostView>,
) -> (Vec2, Vec2) {
    if let Some(host_view) = host_view
        .filter(|view| view.initialized && view.visible_view.x > 0.0 && view.visible_view.y > 0.0)
    {
        let half = host_view.visible_view * 0.5;
        (
            host_view.current_center_world - half,
            host_view.current_center_world + half,
        )
    } else {
        (Vec2::ZERO, frame.size)
    }
}

mod geometry;
mod mesh;
use geometry::{
    aperture_los_rays, aperture_los_targets, capture_dims, compute_cone, cone_render,
    inset_viewer_corners, ApertureLosRay, ConeRender, RebuildKey, LOS_NEAR_SKIP,
};
use mesh::{apply_mesh, make_mesh, placeholder_mesh, proximity_z, smooth01};

/// Maintain + update one rig per placed portal with a placed partner: spawn
/// missing, despawn stale, and update every live rig's geometry in place each
/// frame (the viewer moves, so the cone changes continuously).
///
/// When [`crate::PortalEffectSelection`] is not on `ViewCones`, every rig is
/// DESPAWNED (cameras included) rather than hidden, so an A/B profile against
/// the other effects measures the true cost of the capture passes.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_portal_view_cones(
    mut commands: Commands,
    selection: Res<crate::PortalEffectSelection>,
    config: Res<PortalViewConeConfig>,
    viewer: Option<Res<PortalViewer>>,
    frame: Res<PortalWorldFrame>,
    host_view: Option<Res<PortalCameraContinuityHostView>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    portals: Query<&PlacedPortal>,
    mut rigs: Query<(
        Entity,
        &mut PortalViewRig,
        &mut Transform,
        &mut Projection,
        &mut Camera,
        &mut RenderLayers,
    )>,
    mut cones: Query<
        (&mut Transform, &mut Visibility),
        (With<PortalConeMesh>, Without<PortalViewRig>),
    >,
) {
    if selection.active != crate::PortalVisualEffect::ViewCones {
        for (entity, rig, ..) in &rigs {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
        }
        return;
    }
    if frame.size == Vec2::ZERO {
        return;
    }
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let viewer = viewer.as_deref();
    let (clip_min, clip_max) = portal_window_clip_rect(&frame, host_view.as_deref());

    // First pass: update each live rig in place, or despawn it if its pair is
    // gone / it needs a full rebuild.
    let mut served: Vec<PortalChannel> = Vec::new();
    let capture_layers = capture_render_layers(&config);
    for (entity, mut rig, mut cam_tf, mut proj, mut cam, mut layers) in &mut rigs {
        let portal = all.iter().find(|p| p.channel == rig.channel).copied();
        let partner = portal.and_then(|p| find_portal(&all, p.channel.partner()));
        let (Some(portal), Some(partner)) = (portal, partner) else {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        };
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(&config, frame.size, partner.normal),
        };
        if rig.rebuild != rebuild {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        }
        served.push(rig.channel);
        *layers = capture_layers.clone();

        let (enter, exit) = (portal.frame(), partner.frame());
        let plan = compute_cone(&portal, &partner, &config, viewer, frame.size);
        // Temporal approach to the visibility fraction, smoothstep-shaped.
        if plan.immediate {
            rig.blend = plan.target;
        } else {
            let step = (config.blend_rate * time.delta_secs()).clamp(0.0, 1.0);
            rig.blend += (plan.target - rig.blend) * step;
        }
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(rig.blend), &enter, &exit);
        let z = proximity_z(&config, viewer, portal.pos);
        let render = cone_render(&cone, &enter, &exit, &frame, clip_min, clip_max, z);
        match render {
            Some(r) => {
                if let Some(mesh) = meshes.get_mut(&rig.mesh) {
                    apply_mesh(mesh, &r);
                }
                cam_tf.translation = r.cam_center;
                if let Projection::Orthographic(o) = &mut *proj {
                    o.scaling_mode = ScalingMode::Fixed {
                        width: r.source_size.x,
                        height: r.source_size.y,
                    };
                }
                cam.is_active = true;
                if let Ok((mut ctf, mut vis)) = cones.get_mut(rig.cone) {
                    ctf.translation = r.centroid;
                    *vis = Visibility::Inherited;
                }
            }
            None => {
                // Occluded / behind the surface: stop capturing and hide.
                cam.is_active = false;
                if let Ok((_, mut vis)) = cones.get_mut(rig.cone) {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }

    // Second pass: spawn rigs for desired pairs not yet served.
    for (i, portal) in all.iter().enumerate() {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        if served.contains(&portal.channel) {
            continue;
        }
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(&config, frame.size, partner.normal),
        };
        let image = images.add(Image::new_target_texture(
            rebuild.tex.x,
            rebuild.tex.y,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        let (enter, exit) = (portal.frame(), partner.frame());
        let plan = compute_cone(portal, &partner, &config, viewer, frame.size);
        // Spawn at the target blend (no opening animation on appear).
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
        let z = proximity_z(&config, viewer, portal.pos);
        let render = cone_render(&cone, &enter, &exit, &frame, clip_min, clip_max, z);
        let mesh = meshes.add(match &render {
            Some(r) => make_mesh(r),
            None => placeholder_mesh(),
        });
        let material = materials.add(ColorMaterial {
            color: config.tint,
            // Opaque: the window draws over whatever it is in front of, rather
            // than ghosting it through.
            alpha_mode: AlphaMode2d::Opaque,
            texture: Some(image.clone()),
            ..default()
        });
        let (cone_tf, cone_vis) = match &render {
            Some(r) => (
                Transform::from_translation(r.centroid),
                Visibility::Inherited,
            ),
            None => (
                Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                Visibility::Hidden,
            ),
        };
        let cone_entity = commands
            .spawn((
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material),
                cone_tf,
                cone_vis,
                PortalConeMesh,
                RenderLayers::layer(PORTAL_WINDOW_RENDER_LAYER),
                // The mesh's vertices are rewritten in place every frame as the
                // viewer moves, but Bevy computes a mesh entity's culling Aabb
                // ONCE (calculate_bounds only fills in missing Aabbs; mutating
                // the asset never refreshes it). A stale Aabb from the spawn
                // shape — possibly the degenerate hidden placeholder — gets the
                // window frustum-culled into nothing even though its geometry
                // is correct. One quad: just never cull it.
                bevy::camera::visibility::NoFrustumCulling,
                Name::new(format!("Portal view window ({})", portal.channel.name())),
            ))
            .id();
        let (cam_tf, active, scaling) = match &render {
            Some(r) => (
                Transform::from_translation(r.cam_center),
                true,
                ScalingMode::Fixed {
                    width: r.source_size.x,
                    height: r.source_size.y,
                },
            ),
            None => (
                Transform::default(),
                false,
                ScalingMode::Fixed {
                    width: 1.0,
                    height: 1.0,
                },
            ),
        };
        commands.spawn((
            Camera2d,
            Camera {
                order: -8 - i as isize,
                is_active: active,
                clear_color: ClearColorConfig::Custom(CAPTURE_CLEAR),
                ..default()
            },
            // Single-sampled target needs a single-sampled camera (see commit
            // history): a default 4×-MSAA camera renders nothing into it.
            Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image.clone())),
            capture_render_layers(&config),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: scaling,
                ..OrthographicProjection::default_2d()
            }),
            cam_tf,
            PortalViewRig {
                channel: portal.channel,
                rebuild,
                blend: plan.target,
                _image: image,
                mesh,
                cone: cone_entity,
            },
            Name::new(format!("Portal view capture ({})", portal.channel.name())),
        ));
    }
}

/// Debug overlay for portal view cones: when the host's F1 debug overlay is on,
/// optional F3 toggles can draw the **exit sample zone** (the world rect
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
    mut gizmos: Gizmos,
) {
    if selection.active != crate::PortalVisualEffect::ViewCones
        || !debug.enabled
        || frame.size == Vec2::ZERO
        || (!config.debug_outline && !config.debug_los_rays)
    {
        return;
    }
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let viewer = viewer.as_deref();
    let to_render = |p: Vec2| frame.to_render(p, 0.0).truncate();
    for portal in &all {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        let (enter, exit) = (portal.frame(), partner.frame());
        let plan = compute_cone(portal, &partner, &config, viewer, frame.size);
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
        let (_, core) = portal.channel.display();

        if config.debug_outline {
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

        if config.debug_los_rays {
            let Some(viewer) = viewer.filter(|v| v.present) else {
                continue;
            };
            let corners = inset_viewer_corners(viewer.eye, viewer.half_size);
            let faced = if viewer.eye.distance(enter.pos) <= viewer.eye.distance(exit.pos) {
                &enter
            } else {
                &exit
            };
            let near = viewer.eye.distance(faced.pos) <= LOS_NEAR_SKIP;
            for origin in corners {
                let rays: Vec<ApertureLosRay> = if near {
                    aperture_los_targets(faced, config.aperture_los_quality)
                        .as_slice()
                        .iter()
                        .copied()
                        .map(|target| ApertureLosRay {
                            origin,
                            target,
                            hit: None,
                        })
                        .collect()
                } else {
                    aperture_los_rays(
                        origin,
                        faced,
                        &viewer.occluders,
                        config.aperture_los_quality,
                    )
                };
                for ray in rays {
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
