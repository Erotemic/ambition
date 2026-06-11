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
//! a 4-corner raycast fraction (partial cover ⇒ partial window). Set
//! [`PortalViewConeConfig::viewer_gated`] to `false` (or leave the viewer
//! unset) to fall back to the static, always-on `view_cone`.
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

use ambition_engine_core as ae;
use ambition_platformer_runtime::world_query::{raycast_solids, SolidWorldQuery};
use ambition_portal::pieces::PortalFrame;
use ambition_portal::view::{aperture_wedge_multi, blend_cones, view_cone, ViewCone};
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
            depth_close: 520.0,
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
            z: 8.55,
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

/// What forces a full rig rebuild (vs. a cheap per-frame geometry update): the
/// world-space render transform (size) and the capture texture dims (derived
/// from world extent × density and the exit portal's surface axis).
#[derive(Clone, Copy, PartialEq)]
struct RebuildKey {
    world_size: Vec2,
    tex: UVec2,
}

/// The capture texture dims for a rig: the LONG side covers the exit's
/// along-surface (lateral) axis at the configured density up to the cap; the
/// SHORT side covers the bounded window depth. A wall exit is tall-thin, a
/// floor/ceiling exit wide-short.
fn capture_dims(config: &PortalViewConeConfig, world_size: Vec2, exit_normal: Vec2) -> UVec2 {
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
struct ConeRender {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
    centroid: Vec3,
    cam_center: Vec3,
    source_size: Vec2,
}

/// Sutherland–Hodgman clip of a convex polygon to an axis-aligned rect. The
/// wedge legitimately reaches the half-plane limit; the WORLD bounds are its
/// only honest clip (no arbitrary lateral clamp), and clipping before building
/// the mesh keeps the capture rect — and therefore the texel density — tight.
fn clip_polygon_to_rect(poly: &[Vec2], min: Vec2, max: Vec2) -> Vec<Vec2> {
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

/// One frame's window plan for a pair: the minimum cone, the (full) visible
/// wedge, and the target blend between them — the fraction of the viewer's
/// body corners with clear sight to the faced aperture. The renderer
/// approaches `target` temporally and blends per-corner, so partial cover and
/// approach/retreat all read as a smooth opening, not a pop.
struct ConePlan {
    min: ViewCone,
    wedge: ViewCone,
    target: f32,
}

/// The window plan for one portal pair this frame. Every portal always shows
/// at least the minimum cone; the wedge opens (smoothly, via `target`) when
/// the character is in front of (or in the doorway of) EITHER end of the pair
/// — the wormhole: being "in" one end is being in the other — in proportion
/// to how many of its body corners have clear sight to the faced aperture.
/// The wedge itself runs to the half-plane limit; the WORLD bounds are its
/// only clip (renderer-side). `viewer_gated == false` ⇒ the static always-on
/// window at full blend.
fn compute_cone(
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
    let corners = [
        v.eye + Vec2::new(-h.x, -h.y),
        v.eye + Vec2::new(h.x, -h.y),
        v.eye + Vec2::new(h.x, h.y),
        v.eye + Vec2::new(-h.x, h.y),
    ];
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
    // Visibility fraction: real corners ray-test to the nearer aperture
    // (skipped at the doorway — straddling, where the rays would graze the
    // host blocks and sight is trivially clear).
    let faced = if v.eye.distance(enter.pos) <= v.eye.distance(exit.pos) {
        &enter
    } else {
        &exit
    };
    let target = if v.eye.distance(faced.pos) <= LOS_NEAR_SKIP {
        1.0
    } else {
        let clear = corners
            .iter()
            .filter(|c| !aperture_occluded(**c, faced, &v.occluders))
            .count();
        clear as f32 / corners.len() as f32
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
fn vertex_uv(s: Vec2, source_min: Vec2, source_size: Vec2) -> [f32; 2] {
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
fn cone_render(
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
    let centroid =
        render_poly.iter().copied().sum::<Vec2>() / render_poly.len() as f32;
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

fn make_mesh(render: &ConeRender) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    apply_mesh(&mut mesh, render);
    mesh
}

fn apply_mesh(mesh: &mut Mesh, render: &ConeRender) {
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, render.uvs.clone());
    mesh.insert_indices(Indices::U32(render.indices.clone()));
}

/// A hidden-rig placeholder mesh (degenerate; the rig is invisible until its
/// first visible frame fills it in).
fn placeholder_mesh() -> Mesh {
    make_mesh(&ConeRender {
        positions: vec![[0.0; 3]; 3],
        uvs: vec![[0.0; 2]; 3],
        indices: vec![0, 1, 2],
        centroid: Vec3::ZERO,
        cam_center: Vec3::ZERO,
        source_size: Vec2::ONE,
    })
}

/// Smoothstep shaping for the temporal blend — the "squished logit" feel:
/// flat near both ends, fast through the middle.
fn smooth01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Render z for a portal's window biased by viewer proximity, so the portal
/// you are closest to draws ON TOP of the others (inverse-distance, bounded by
/// `span` and kept under the rim gap). No viewer ⇒ base z.
fn proximity_z(config: &PortalViewConeConfig, viewer: Option<&PortalViewer>, portal_pos: Vec2) -> f32 {
    let dist = viewer
        .filter(|v| v.present)
        .map_or(f32::INFINITY, |v| v.eye.distance(portal_pos));
    config.z + config.z_proximity_span / (1.0 + dist / 200.0)
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
    frame: Res<PortalWorldFrame>,
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

    // First pass: update each live rig in place, or despawn it if its pair is
    // gone / it needs a full rebuild.
    let mut served: Vec<PortalChannel> = Vec::new();
    for (entity, mut rig, mut cam_tf, mut proj, mut cam) in &mut rigs {
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

        let (enter, exit) = (portal.frame(), partner.frame());
        let plan = compute_cone(&portal, &partner, &config, viewer, frame.size);
        // Temporal approach to the visibility fraction, smoothstep-shaped.
        let step = (config.blend_rate * time.delta_secs()).clamp(0.0, 1.0);
        rig.blend += (plan.target - rig.blend) * step;
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(rig.blend), &enter, &exit);
        let z = proximity_z(&config, viewer, portal.pos);
        let render = cone_render(&cone, &enter, &exit, &frame, z);
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
        let render = cone_render(&cone, &enter, &exit, &frame, z);
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
                blend: plan.target,
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
    let to_render = |p: Vec2| frame.to_render(p, 0.0).truncate();
    for portal in &all {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        let (enter, exit) = (portal.frame(), partner.frame());
        let plan = compute_cone(portal, &partner, &config, viewer, frame.size);
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
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
