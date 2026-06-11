//! Through-portal **view windows**: each placed portal shows a slice of the
//! world in front of its partner, set into its host surface — you look "through
//! the portal a little bit" — rendered live by an offscreen capture camera.
//!
//! ## Viewer-dependent visibility
//! By default the window is the wedge of the surface the **controlled
//! character** can actually see through the aperture (its sightline frustum
//! through the slit), via `ambition_portal::view::visible_cone` from a
//! host-supplied [`PortalViewer`]: the wedge skews with the viewer's angle to
//! the surface, widens as they near the portal, and vanishes when a wall
//! occludes the line of sight (a short raycast over [`PortalViewer::occluders`])
//! or the viewer steps behind the surface. Set
//! [`PortalViewConeConfig::viewer_gated`] to `false` (or leave the viewer
//! unset) to fall back to the static, always-on `view_cone` — the
//! "render unconditionally" mode.
//!
//! ## How a rig works
//! Per placed portal with a placed partner, a **rig**: one offscreen image, a
//! capture `Camera2d` framing the partner-side source rect, and a window
//! `Mesh2d` set into the entry's surface. The display map is
//! `view::display_point` — the transit sprite copy's map — so the window and
//! the copy read as one continuous image; its rotation/mirror lives entirely
//! in the **UV mapping** (`cone_uvs`, pinned below), and the capture camera
//! stays axis-aligned. The capture
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
//! Window meshes live on the default render layer, so capture cameras see other
//! portals' windows. Two portals facing each other within window depth show one
//! frame of through-portal recursion per level, Portal-style, for free — and no
//! camera ever samples the image it writes (P's window shows the capture made
//! near P's partner; cross-sampling only, by construction).

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite_render::AlphaMode2d;

use ambition_engine_core::{self as ae, AabbExt};
use ambition_platformer_runtime::gravity::GravityField;
use ambition_platformer_runtime::world_query::{raycast_solids, SolidWorldQuery};
use ambition_portal::pieces::PortalFrame;
use ambition_portal::view::{aperture_wedge, blend_cones, view_cone, window_eye, ViewCone};
use ambition_portal::{find_portal, PlacedPortal, PortalChannel};

use crate::PortalWorldFrame;

/// Clear color of an offscreen capture: a dark tone shows through wherever the
/// exit room has no geometry (rare — parallax usually fills it). Opaque windows
/// draw it directly, so keep it unobtrusive.
const CAPTURE_CLEAR: Color = Color::srgb(0.03, 0.04, 0.05);

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
    /// The controlled character's eye position, world space.
    pub eye: Vec2,
    /// Solid AABBs for the line-of-sight test — a portal whose aperture is
    /// occluded from `eye` renders no window. The host syncs these from its
    /// collision world (only the blocks that block sight).
    pub occluders: Vec<ae::Aabb>,
}

/// Tuning for the view windows. A host overwrites the resource to retune; set
/// [`PortalPresentationPlugin::view_cones`](crate::PortalPresentationPlugin)
/// to `false` to drop the feature (and its capture passes) entirely.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PortalViewConeConfig {
    /// When true (default), each window opens to the controlled character's
    /// visible wedge through the aperture (from [`PortalViewer`]), blended up
    /// from the minimum cone. When false, windows render the static, always-on
    /// `view_cone` — the "always show this much" mode that needs no viewer.
    pub viewer_gated: bool,
    /// How far the **viewer-dependent** wedge may extend behind the surface
    /// (world px). Demo: ~1 tile so the window only peeks just past the wall.
    pub max_depth: f32,
    /// How far the wedge's far edge may extend laterally from the aperture
    /// center (world px). This bounds the half-plane limit (eye at the plane ⇒
    /// the wedge saturates to a `±max_half_width × max_depth` strip) so the
    /// capture rect stays frameable by a fixed-size texture.
    pub max_half_width: f32,
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
    /// Offscreen capture size in texels (square; non-square source rects are
    /// stored stretched and un-stretched by the UVs).
    pub resolution: u32,
    /// Render z of the window mesh. Just BEHIND the portal rim (9.0) so the
    /// doorway stays crisp over its own view, above world blocks (0) and below
    /// actors (10+).
    pub z: f32,
    /// Tint multiplied over the capture. Opaque by default — the window draws
    /// over whatever it is in front of (see [`AlphaMode2d::Opaque`] below).
    pub tint: Color,
    /// Debug: draw gizmo outlines of each portal's EXIT sample zone (the
    /// `ViewCone::source` rect, in the portal's channel color, in front of its
    /// partner) and the entry window. Driven host-side (in Ambition, off the
    /// standard `F1` debug overlay). Off by default.
    pub debug_outline: bool,
}

impl Default for PortalViewConeConfig {
    fn default() -> Self {
        Self {
            viewer_gated: true,
            max_depth: 80.0,
            max_half_width: 480.0,
            min_depth: 22.0,
            min_spread: 0.12,
            viewer_blend: 1.0,
            depth: 90.0,
            spread: 0.20,
            resolution: 384,
            z: 8.9,
            // Opaque white: the window draws over what it is in front of.
            tint: Color::WHITE,
            debug_outline: false,
        }
    }
}

/// Marks a window mesh entity (the `Mesh2d` set into a portal's surface),
/// disjoint from the capture-camera entity that carries [`PortalViewRig`].
#[derive(Component)]
pub struct PortalConeMesh;

/// One rig, carried by the capture camera entity: which portal channel it
/// serves, the rebuild key it was built for, and handles to its image + mesh +
/// window-mesh entity. Geometry is updated in place each frame; the rig is only
/// respawned when `rebuild` drifts (world size / resolution) or the pair
/// disappears.
#[derive(Component)]
pub struct PortalViewRig {
    channel: PortalChannel,
    rebuild: RebuildKey,
    /// Keep-alive for the offscreen target (also referenced by the camera's
    /// `RenderTarget` and the window material; held here so the rig owns its
    /// asset lifetime explicitly).
    _image: Handle<Image>,
    mesh: Handle<Mesh>,
    cone: Entity,
}

/// What forces a full rig rebuild (vs. a cheap per-frame geometry update): the
/// world-space render transform (size) and the capture texture size.
#[derive(Clone, Copy, PartialEq)]
struct RebuildKey {
    world_size: Vec2,
    resolution: u32,
}

/// Per-frame render data derived from a [`ViewCone`]: the window mesh's
/// centroid-relative corner positions + UVs, its world translation, and the
/// capture camera's center + framed size.
struct ConeRender {
    positions: [[f32; 3]; 4],
    uvs: [[f32; 2]; 4],
    centroid: Vec3,
    cam_center: Vec3,
    source_size: Vec2,
}

/// A `&[Aabb]` as a [`SolidWorldQuery`] so the LOS raycast can reuse
/// `raycast_solids` over the host-supplied occluder snapshot.
struct SliceSolids<'a>(&'a [ae::Aabb]);
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
const LOS_NEAR_SKIP: f32 = 70.0;

/// Is the line of sight from `eye` to the aperture blocked by a solid? The
/// target is lifted a little OFF the surface (along the normal) so the ray
/// never has to land exactly on the host face — a grazing ray along a shared
/// floor line would otherwise clip the (uncarved) host blocks themselves —
/// and the cast still stops short of the lifted point.
fn aperture_occluded(eye: Vec2, enter: &PortalFrame, occluders: &[ae::Aabb]) -> bool {
    let target = enter.pos + enter.normal * 12.0;
    let d = target - eye;
    let dist = d.length();
    if dist < 2.0 {
        return false;
    }
    raycast_solids(&SliceSolids(occluders), eye, d, (dist - 4.0).max(0.0), false).is_some()
}

/// The window geometry for one portal pair this frame. ALWAYS returns a cone:
/// every portal shows at least the minimum cone; once any real visibility
/// exists the window follows the visibility wedge exactly (the minimum has no
/// influence — see `viewer_blend`). The wedge opens when the character is in
/// front of (or in the doorway of) EITHER end of the pair — the wormhole:
/// being "in" one end is being in the other — and its sight line to the end it
/// actually faces is clear. `viewer_gated == false` ⇒ the static always-on
/// window.
fn compute_cone(
    portal: &PlacedPortal,
    partner: &PlacedPortal,
    config: &PortalViewConeConfig,
    viewer: Option<&PortalViewer>,
    gravity_dir: Vec2,
) -> ViewCone {
    let enter = portal.frame();
    let exit = partner.frame();
    if !config.viewer_gated {
        return view_cone(&enter, &exit, config.depth, config.spread, gravity_dir);
    }
    // The minimum cone, always shown when nothing better exists.
    let min = view_cone(&enter, &exit, config.min_depth, config.min_spread, gravity_dir);
    let Some(v) = viewer.filter(|v| v.present) else {
        return min;
    };
    // Resolve which end the character actually looks through (nearest-first,
    // with the in-doorway transit grace) — it decides both the wedge eye and
    // which aperture the LOS runs against.
    let Some((eye, via_partner)) = window_eye(&enter, &exit, v.eye) else {
        return min; // behind both ends — minimum only
    };
    let faced = if via_partner { &exit } else { &enter };
    // LOS from the REAL eye to the faced aperture; skipped when basically at
    // the doorway (straddling/transiting — sight is trivially clear).
    if v.eye.distance(faced.pos) > LOS_NEAR_SKIP && aperture_occluded(v.eye, faced, &v.occluders)
    {
        return min; // sight line blocked — minimum only
    }
    let Some(wedge) = aperture_wedge(&enter, &exit, eye, config.max_depth, config.max_half_width, gravity_dir)
    else {
        return min;
    };
    // viewer_blend = 1.0 (default): the wedge verbatim — the real visibility
    // map, unmodified by the minimum.
    blend_cones(&min, &wedge, config.viewer_blend, &enter, &exit, gravity_dir)
}

/// Per-vertex UVs for the window mesh: each source-quad corner normalized
/// inside the source rect. World y-down and texture v-down agree (the render
/// y-flip cancels between camera and capture), so this is flip-free.
fn cone_uvs(source_quad: &[Vec2; 4], source: ae::Aabb) -> [[f32; 2]; 4] {
    let size = source.half_size() * 2.0;
    source_quad.map(|s| {
        [
            ((s.x - source.min.x) / size.x.max(1e-6)).clamp(0.0, 1.0),
            ((s.y - source.min.y) / size.y.max(1e-6)).clamp(0.0, 1.0),
        ]
    })
}

/// Resolve a [`ViewCone`] into renderable data, or `None` for a degenerate rect.
fn cone_render(cone: &ViewCone, frame: &PortalWorldFrame, z: f32) -> Option<ConeRender> {
    let source_size = cone.source.half_size() * 2.0;
    if source_size.x < 1.0 || source_size.y < 1.0 {
        return None;
    }
    let render_quad: [Vec2; 4] =
        std::array::from_fn(|i| frame.to_render(cone.entry_quad[i], 0.0).truncate());
    let centroid = (render_quad[0] + render_quad[1] + render_quad[2] + render_quad[3]) * 0.25;
    let positions: [[f32; 3]; 4] =
        std::array::from_fn(|i| [render_quad[i].x - centroid.x, render_quad[i].y - centroid.y, 0.0]);
    Some(ConeRender {
        positions,
        uvs: cone_uvs(&cone.source_quad, cone.source),
        centroid: centroid.extend(z),
        cam_center: frame.to_render(cone.source.center(), 0.0),
        source_size,
    })
}

fn make_mesh(render: &ConeRender) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    apply_mesh(&mut mesh, render);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

fn apply_mesh(mesh: &mut Mesh, render: &ConeRender) {
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render.positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, render.uvs.to_vec());
}

/// A hidden-rig placeholder mesh (degenerate; the rig is invisible until its
/// first visible frame fills it in).
fn placeholder_mesh() -> Mesh {
    make_mesh(&ConeRender {
        positions: [[0.0; 3]; 4],
        uvs: [[0.0; 2]; 4],
        centroid: Vec3::ZERO,
        cam_center: Vec3::ZERO,
        source_size: Vec2::ONE,
    })
}

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
    gravity: Option<Res<GravityField>>,
    frame: Res<PortalWorldFrame>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    portals: Query<&PlacedPortal>,
    mut rigs: Query<(
        Entity,
        &PortalViewRig,
        &mut Transform,
        &mut Projection,
        &mut Camera,
    )>,
    mut cones: Query<(&mut Transform, &mut Visibility), (With<PortalConeMesh>, Without<PortalViewRig>)>,
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
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);
    let rebuild = RebuildKey {
        world_size: frame.size,
        resolution: config.resolution.max(8),
    };

    // First pass: update each live rig in place, or despawn it if its pair is
    // gone / it needs a full rebuild.
    let mut served: Vec<PortalChannel> = Vec::new();
    for (entity, rig, mut cam_tf, mut proj, mut cam) in &mut rigs {
        let portal = all.iter().find(|p| p.channel == rig.channel).copied();
        let partner = portal.and_then(|p| find_portal(&all, p.channel.partner()));
        let (Some(portal), Some(partner)) = (portal, partner) else {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        };
        if rig.rebuild != rebuild {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        }
        served.push(rig.channel);

        let cone = compute_cone(&portal, &partner, &config, viewer, gravity_dir);
        let render = cone_render(&cone, &frame, config.z);
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
        let image = images.add(Image::new_target_texture(
            rebuild.resolution,
            rebuild.resolution,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        let cone = compute_cone(portal, &partner, &config, viewer, gravity_dir);
        let render = cone_render(&cone, &frame, config.z);
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
            Some(r) => (Transform::from_translation(r.centroid), Visibility::Inherited),
            None => (
                Transform::from_translation(Vec3::new(0.0, 0.0, config.z)),
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
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: scaling,
                ..OrthographicProjection::default_2d()
            }),
            cam_tf,
            PortalViewRig {
                channel: portal.channel,
                rebuild,
                _image: image,
                mesh,
                cone: cone_entity,
            },
            Name::new(format!("Portal view capture ({})", portal.channel.name())),
        ));
    }
}

/// Debug overlay (gated by [`PortalViewConeConfig::debug_outline`]): for every
/// portal with a placed partner, outline the **exit sample zone** (the world
/// rect `ViewCone::source` in front of the partner, in the portal's channel
/// color) and the entry window. Uses the SAME `compute_cone` as the renderer,
/// so the gizmo reflects the live viewer-dependent wedge (or nothing, when the
/// aperture is occluded). The sample zone shows where the capture samples from;
/// the entry window shows where it is displayed.
pub fn debug_portal_view_zones(
    selection: Res<crate::PortalEffectSelection>,
    config: Res<PortalViewConeConfig>,
    viewer: Option<Res<PortalViewer>>,
    gravity: Option<Res<GravityField>>,
    frame: Res<PortalWorldFrame>,
    portals: Query<&PlacedPortal>,
    mut gizmos: Gizmos,
) {
    if selection.active != crate::PortalVisualEffect::ViewCones
        || !config.debug_outline
        || frame.size == Vec2::ZERO
    {
        return;
    }
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let viewer = viewer.as_deref();
    let gravity_dir = gravity.map_or(Vec2::new(0.0, 1.0), |g| g.dir);
    let to_render = |p: Vec2| frame.to_render(p, 0.0).truncate();
    for portal in &all {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        let cone = compute_cone(portal, &partner, &config, viewer, gravity_dir);
        let (_, core) = portal.channel.display();

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
        let uvs = cone_uvs(&quad, source);
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
        let cone = view_cone(&enter, &exit, 120.0, 0.25, Vec2::new(0.0, 1.0));
        let uvs = cone_uvs(&cone.source_quad, cone.source);
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
}
