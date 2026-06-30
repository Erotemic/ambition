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
//! window). If LOS admits no viewer sample, the live window is hidden; the rim
//! stays visible. Set [`PortalViewConeConfig::mode`] to [`PortalViewConeMode::Static`] to use
//! the static, always-on `view_cone`, or [`PortalViewConeMode::Off`] to hide
//! view windows entirely.
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
use std::fmt::Write as _;

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
const PORTAL_CAPTURE_PARALLAX_LAYER_BASE: usize = 32;

fn portal_capture_parallax_layer(channel: PortalChannel) -> usize {
    PORTAL_CAPTURE_PARALLAX_LAYER_BASE + portal_channel_render_slot(channel)
}

fn portal_channel_render_slot(channel: PortalChannel) -> usize {
    match channel {
        PortalChannel::Gun(color) => color.slot as usize,
        PortalChannel::Authored(color) => {
            use ambition_portal::PortalChannelColor::*;
            8 + match color {
                Purple => 0,
                Yellow => 1,
                Teal => 2,
                Red => 3,
                Green => 4,
                Magenta => 5,
                Cyan => 6,
                Rose => 7,
                Indexed(n) => 8 + n as usize,
            }
        }
    }
}

fn capture_render_layers(config: &PortalViewConeConfig, parallax_layer: usize) -> RenderLayers {
    let layers = RenderLayers::layer(WORLD_RENDER_LAYER).with(parallax_layer);
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
/// frame; dynamic mode closes the view window, while static mode ignores the
/// viewer seam.
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

/// Request latch for a one-shot portal view-cone debug dump.
///
/// The app-side inspector and the F8 hotkey both set this resource; the
/// presentation system clears it after writing/printing one snapshot of portal
/// configuration, route evidence, rig state, and render rectangles.
#[derive(Resource, Clone, Debug)]
pub struct PortalViewConeDebugDumpRequest {
    pub pending: bool,
    pub reason: String,
}

impl Default for PortalViewConeDebugDumpRequest {
    fn default() -> Self {
        Self {
            pending: false,
            reason: String::new(),
        }
    }
}

impl PortalViewConeDebugDumpRequest {
    pub fn request(&mut self, reason: impl Into<String>) {
        self.pending = true;
        self.reason = reason.into();
    }
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
        Self::Low
    }
}

/// High-level mode for portal view windows.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum PortalViewConeMode {
    /// No portal view window is drawn or captured. Portal rims/body pieces still render.
    Off,
    /// Always draw the authored/static `view_cone`; no viewer LOS is required.
    Static,
    /// Draw a viewer-dependent window when dynamic visibility admits the portal.
    Dynamic,
}

impl PortalViewConeMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Static, Self::Dynamic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Static => "Static",
            Self::Dynamic => "Dynamic",
        }
    }
}

impl Default for PortalViewConeMode {
    fn default() -> Self {
        Self::Dynamic
    }
}

/// Policy for which dynamic visibility routes can open and shape a view cone.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum PortalViewConeVisibilityMode {
    /// Only direct LOS from the viewer to this portal face can admit and shape the cone.
    FaceLosOnly,
    /// Face LOS admits the cone; entry-side doorway continuity may also admit near crossing.
    FaceLosWithContinuity,
    /// Direct face, through-portal, or exit-side routes may independently admit the cone.
    AnyPortalRoute,
}

impl PortalViewConeVisibilityMode {
    pub const ALL: [Self; 3] = [
        Self::FaceLosOnly,
        Self::FaceLosWithContinuity,
        Self::AnyPortalRoute,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::FaceLosOnly => "FaceLosOnly",
            Self::FaceLosWithContinuity => "FaceLosWithContinuity",
            Self::AnyPortalRoute => "AnyPortalRoute",
        }
    }

    pub(crate) fn admit_through_portal(self, face_los_fraction: f32, via_partner: bool) -> bool {
        match self {
            Self::FaceLosOnly => false,
            Self::FaceLosWithContinuity => !via_partner || face_los_fraction > 0.0,
            Self::AnyPortalRoute => true,
        }
    }

    pub(crate) fn admit_exit_side(self, _face_los_fraction: f32) -> bool {
        match self {
            Self::FaceLosOnly => false,
            Self::FaceLosWithContinuity | Self::AnyPortalRoute => true,
        }
    }
}

impl Default for PortalViewConeVisibilityMode {
    fn default() -> Self {
        Self::FaceLosWithContinuity
    }
}

/// Policy for reconciling the planned portal source rect with the final source
/// rect the mesh/UV/capture camera can sample this frame.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum PortalViewConeSourceClipPolicy {
    /// Build the mesh from the planned entry quad, even if it reaches outside
    /// the active view rect. Useful only as a diagnostic escape hatch.
    AllowClip,
    /// Clip the entry polygon to the active frame before mapping to source
    /// space, then build mesh, UVs, and camera framing from that same final
    /// source rect.
    ClampToFrame,
    /// Preserve the same coherent final-source path as [`Self::ClampToFrame`].
    /// Kept as an explicit tuning label for future aspect-preserving fitting.
    FitToFrame,
}

impl PortalViewConeSourceClipPolicy {
    pub const ALL: [Self; 3] = [Self::AllowClip, Self::ClampToFrame, Self::FitToFrame];

    pub fn label(self) -> &'static str {
        match self {
            Self::AllowClip => "AllowClip",
            Self::ClampToFrame => "ClampToFrame",
            Self::FitToFrame => "FitToFrame",
        }
    }
}

impl Default for PortalViewConeSourceClipPolicy {
    fn default() -> Self {
        Self::ClampToFrame
    }
}

/// Camera model used by portal view-window capture rigs.
#[derive(Clone, Copy, Debug, Reflect, PartialEq, Eq)]
pub enum PortalCaptureCameraMode {
    /// Frame the exact cone source rect computed from viewer visibility.
    ConeRect,
    /// Frame a destination-side camera snapshot by mapping the host view
    /// through the portal pair. The cone mesh still controls admission and
    /// shape; the capture texture is sampled from the mapped camera frame.
    MappedCameraSnapshot,
}

impl PortalCaptureCameraMode {
    pub const ALL: [Self; 2] = [Self::ConeRect, Self::MappedCameraSnapshot];

    pub fn label(self) -> &'static str {
        match self {
            Self::ConeRect => "ConeRect",
            Self::MappedCameraSnapshot => "MappedCameraSnapshot",
        }
    }
}

impl Default for PortalCaptureCameraMode {
    fn default() -> Self {
        Self::MappedCameraSnapshot
    }
}

/// Tuning for the view windows. A host overwrites the resource to retune; set
/// [`PortalPresentationPlugin::view_cones`](crate::PortalPresentationPlugin)
/// to `false` to drop the feature (and its capture passes) entirely.
#[derive(Resource, Clone, Debug, Reflect, PartialEq)]
#[reflect(Resource)]
pub struct PortalViewConeConfig {
    /// High-level view-window behavior: off, static authored cone, or dynamic
    /// viewer-dependent cone. Use [`PortalViewConeMode::Static`] for an
    /// always-on `view_cone` and [`PortalViewConeMode::Dynamic`] for LOS-gated
    /// windows driven by [`PortalViewer`].
    pub mode: PortalViewConeMode,
    /// Dynamic visibility policy. In dynamic mode this selects which LOS route
    /// may admit/open the cone, and whether portal-continuity routes can shape
    /// the cone after face LOS exists or while crossing this portal's doorway.
    pub visibility_mode: PortalViewConeVisibilityMode,
    /// Aperture LOS quality. `Low` is the original single center ray per viewer
    /// corner. `Medium` samples the left endpoint, center, and right endpoint,
    /// then averages visible samples.
    pub aperture_los_quality: PortalApertureLosQuality,
    /// Source clipping/fitting policy. The default clamps the final entry
    /// polygon to the active frame before deriving mesh vertices, UVs, and the
    /// capture camera source rect, so the visible window never samples one rect
    /// while the camera captures another.
    pub source_clip_policy: PortalViewConeSourceClipPolicy,
    /// Capture camera policy. `ConeRect` preserves the current tight source
    /// rectangle; `MappedCameraSnapshot` lets portal preview request a
    /// destination-side capture frame derived from the host camera snapshot.
    pub capture_camera_mode: PortalCaptureCameraMode,
    /// Max dynamic window depth behind the surface (world px), reached when the
    /// viewer is within `dynamic_dist_close` of the aperture. The world bounds
    /// still clip it.
    pub dynamic_depth_close: f32,
    /// Min dynamic window depth behind the surface (world px), reached when the
    /// viewer is beyond `dynamic_dist_far`.
    pub dynamic_depth_far: f32,
    /// Viewer→aperture distance (world px) at/below which dynamic depth equals
    /// `dynamic_depth_close`.
    pub dynamic_dist_close: f32,
    /// Viewer→aperture distance (world px) at/beyond which dynamic depth equals
    /// `dynamic_depth_far`.
    pub dynamic_dist_far: f32,
    /// Body-edge distance to the finite aperture where the art-directed
    /// half-plane preview is fully applied. Keep near zero: the full 180-degree
    /// half-plane should arrive only when the viewer is essentially touching
    /// the aperture. Set to `0.0` for exact LOS geometry with no half-plane
    /// shape assist.
    pub half_plane_preview_full_distance: f32,
    /// Extra directed distance before [`Self::half_plane_preview_full_distance`]
    /// over which the view window opens from nothing to raw LOS geometry and
    /// eases toward the half-plane preview.
    pub half_plane_preview_blend_distance: f32,
    /// Maximum lateral reach of the half-plane preview behind the portal face
    /// (world px). `0.0` asks the renderer for a full-view half-plane that is
    /// clipped by the active camera frame.
    pub half_plane_preview_max_lateral: f32,
    /// Z range over which nearer portals' windows draw ON TOP of farther ones
    /// (added to `z` by an inverse-distance bias). Kept under the rim gap.
    pub z_proximity_span: f32,
    /// How quickly the window opens/closes between the minimum cone and the
    /// visible wedge (per second, exponential approach) — the temporal half of
    /// the smooth blend; the spatial half is the 4-corner visibility fraction.
    pub blend_rate: f32,
    /// The **minimum cone** shown once LOS admits the window (depth into the
    /// surface, world px). Blocked LOS hides the capture window instead of
    /// drawing the minimum through walls.
    pub min_depth: f32,
    /// Minimum-cone side widening per px of depth.
    pub min_spread: f32,
    /// Blend from the minimum cone (0) toward the visible wedge (1). Keep at
    /// 1.0 (default): once ANY visibility exists the window follows the real
    /// visibility wedge exactly and the minimum has no influence — the minimum
    /// only fills in when no wedge exists at all. Lower values are for tuning
    /// transitions only.
    pub viewer_blend: f32,
    /// Static-mode window depth into the surface (world px). Keep near wall scale.
    pub static_depth: f32,
    /// Static-mode side widening per px of depth (0 = straight corridor).
    pub static_spread: f32,
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
    /// Debug dump portal filter. Empty means dump all portals. A name like
    /// `c136` or `c137` resolves to that portal and its paired portal, so the
    /// text dump stays small enough to copy/paste while debugging one pair.
    pub debug_dump_portal: String,
}
impl Default for PortalViewConeConfig {
    fn default() -> Self {
        Self {
            mode: PortalViewConeMode::Dynamic,
            visibility_mode: PortalViewConeVisibilityMode::FaceLosWithContinuity,
            aperture_los_quality: PortalApertureLosQuality::Low,
            source_clip_policy: PortalViewConeSourceClipPolicy::ClampToFrame,
            capture_camera_mode: PortalCaptureCameraMode::MappedCameraSnapshot,
            // Large but not so deep it punches through thin "door" walls into
            // the far room (which is what drives the heaviest recursion); also
            // keeps the near-face↔deep-content parallax modest.
            dynamic_depth_close: 280.0,
            dynamic_depth_far: 44.0,
            dynamic_dist_close: 70.0,
            dynamic_dist_far: 900.0,
            half_plane_preview_full_distance: 1.0,
            half_plane_preview_blend_distance: 120.0,
            half_plane_preview_max_lateral: 0.0,
            z_proximity_span: 0.35,
            blend_rate: 10.0,
            min_depth: 22.0,
            min_spread: 0.12,
            viewer_blend: 1.0,
            static_depth: 90.0,
            static_spread: 0.20,
            texels_per_world_px: 1.0,
            max_resolution: 4096,
            recursion_depth: 1,
            z: 8.55,
            // Slightly below white: opaque, but each recursion level multiplies
            // the tint so facing/door portals fade into a tunnel rather than a
            // full-brightness chaotic fractal (see the field docs). 1.0 brings
            // back the chaos; lower fades faster.
            tint: Color::srgb(1.0, 1.0, 1.0),
            debug_outline: true,
            debug_los_rays: false,
            debug_dump_portal: String::new(),
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
    parallax_layer: usize,
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

impl PortalViewRig {
    /// Portal channel served by this capture rig.
    pub fn channel(&self) -> PortalChannel {
        self.channel
    }

    /// Private `RenderLayers` index for parallax sprites that should render
    /// only into this rig's capture texture.
    pub fn parallax_layer(&self) -> usize {
        self.parallax_layer
    }
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

fn portal_capture_camera_frame(
    config: &PortalViewConeConfig,
    host_view: Option<&PortalCameraContinuityHostView>,
    enter: &PortalFrame,
    exit: &PortalFrame,
) -> Option<geometry::CaptureCameraFrame> {
    if config.capture_camera_mode != PortalCaptureCameraMode::MappedCameraSnapshot {
        return None;
    }
    let host_view = host_view.filter(|view| {
        view.initialized && view.visible_view.x >= 1.0 && view.visible_view.y >= 1.0
    })?;
    let center = ambition_portal::pieces::map_point(host_view.current_center_world, enter, exit);
    Some(geometry::CaptureCameraFrame {
        center,
        size: host_view.visible_view,
    })
}

mod geometry;
mod mesh;
use geometry::{
    aperture_los_rays, aperture_visibility_fraction, capture_dims, compute_cone, cone_render,
    inset_viewer_corners, visibility_route_summary, ApertureLosRay, ConeRender, RebuildKey,
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
    cone_materials: Query<&MeshMaterial2d<ColorMaterial>, With<PortalConeMesh>>,
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
    if config.mode == PortalViewConeMode::Off {
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
    for (entity, mut rig, mut cam_tf, mut proj, mut cam, mut layers) in &mut rigs {
        let portal = all.iter().find(|p| p.channel == rig.channel).copied();
        let partner = portal.and_then(|p| find_portal(&all, p.channel.partner()));
        let (Some(portal), Some(partner)) = (portal, partner) else {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        };
        let (enter, exit) = (portal.frame(), partner.frame());
        let capture_frame =
            portal_capture_camera_frame(&config, host_view.as_deref(), &enter, &exit);
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(&config, frame.size, partner.normal, capture_frame),
        };
        if rig.rebuild != rebuild {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        }
        served.push(rig.channel);
        *layers = capture_render_layers(&config, rig.parallax_layer);
        sync_cone_material_tint(&cone_materials, &mut materials, rig.cone, config.tint);

        let plan = compute_cone(&portal, &partner, &config, viewer, frame.size);
        if plan.target <= 0.0 {
            rig.blend = 0.0;
            cam.is_active = false;
            if let Ok((_, mut vis)) = cones.get_mut(rig.cone) {
                *vis = Visibility::Hidden;
            }
            continue;
        }
        // Temporal approach to the visibility fraction, smoothstep-shaped.
        if plan.immediate {
            rig.blend = plan.target;
        } else {
            let step = (config.blend_rate * time.delta_secs()).clamp(0.0, 1.0);
            rig.blend += (plan.target - rig.blend) * step;
        }
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(rig.blend), &enter, &exit);
        let z = proximity_z(&config, viewer, portal.pos);
        let render = cone_render(
            &cone,
            &enter,
            &exit,
            &frame,
            &config,
            clip_min,
            clip_max,
            z,
            capture_frame,
        );
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
        let (enter, exit) = (portal.frame(), partner.frame());
        let capture_frame =
            portal_capture_camera_frame(&config, host_view.as_deref(), &enter, &exit);
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(&config, frame.size, partner.normal, capture_frame),
        };
        let image = images.add(Image::new_target_texture(
            rebuild.tex.x,
            rebuild.tex.y,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
        let plan = compute_cone(portal, &partner, &config, viewer, frame.size);
        // Spawn at the target blend (no opening animation on appear).
        let cone = if plan.target > 0.0 {
            Some(blend_cones(
                &plan.min,
                &plan.wedge,
                smooth01(plan.target),
                &enter,
                &exit,
            ))
        } else {
            None
        };
        let z = proximity_z(&config, viewer, portal.pos);
        let render = cone.as_ref().and_then(|cone| {
            cone_render(
                cone,
                &enter,
                &exit,
                &frame,
                &config,
                clip_min,
                clip_max,
                z,
                capture_frame,
            )
        });
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
            capture_render_layers(&config, portal_capture_parallax_layer(portal.channel)),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: scaling,
                ..OrthographicProjection::default_2d()
            }),
            cam_tf,
            PortalViewRig {
                channel: portal.channel,
                parallax_layer: portal_capture_parallax_layer(portal.channel),
                rebuild,
                blend: if plan.target > 0.0 { plan.target } else { 0.0 },
                _image: image,
                mesh,
                cone: cone_entity,
            },
            Name::new(format!("Portal view capture ({})", portal.channel.name())),
        ));
    }
}

/// F8 requests a portal view-cone dump. This intentionally shares the existing
/// trace-dump hotkey: for portal rendering bugs, the gameplay trace and the
/// portal presentation snapshot are most useful as a pair.
pub fn handle_portal_view_cone_dump_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    mut request: ResMut<PortalViewConeDebugDumpRequest>,
) {
    if keys.just_pressed(KeyCode::F8) {
        request.request("F8");
    }
}

/// Flush one pending portal view-cone dump to stderr and, on native targets, to
/// `target/ambition-debug/portal-view-cones/`.
#[allow(clippy::too_many_arguments)]
pub fn flush_portal_view_cone_debug_dump(
    mut request: ResMut<PortalViewConeDebugDumpRequest>,
    selection: Res<crate::PortalEffectSelection>,
    config: Res<PortalViewConeConfig>,
    viewer: Option<Res<PortalViewer>>,
    frame: Res<PortalWorldFrame>,
    host_view: Option<Res<PortalCameraContinuityHostView>>,
    portals: Query<&PlacedPortal>,
    rigs: Query<(
        &PortalViewRig,
        &Camera,
        &Projection,
        Option<&GlobalTransform>,
    )>,
    cone_visibility: Query<(&Visibility, Option<&GlobalTransform>), With<PortalConeMesh>>,
) {
    if !request.pending {
        return;
    }
    let reason = if request.reason.is_empty() {
        "manual".to_string()
    } else {
        request.reason.clone()
    };
    request.pending = false;
    request.reason.clear();

    let dump = portal_view_cone_debug_dump_text(
        &reason,
        &selection,
        &config,
        viewer.as_deref(),
        &frame,
        host_view.as_deref(),
        &portals,
        &rigs,
        &cone_visibility,
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
) -> String {
    let mut out = String::new();
    let all: Vec<PlacedPortal> = portals.iter().copied().collect();
    let (clip_min, clip_max) = portal_window_clip_rect(frame, host_view);

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

        let enter = portal.frame();
        let exit = partner.frame();
        let capture_frame = portal_capture_camera_frame(config, host_view, &enter, &exit);
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(config, frame.size, partner.normal, capture_frame),
        };
        let route = visibility_route_summary(portal, &partner, config, viewer);
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

        let plan = compute_cone(portal, &partner, config, viewer, frame.size);
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
            let cone = blend_cones(&plan.min, &plan.wedge, smooth01(blend), &enter, &exit);
            let capture_frame = portal_capture_camera_frame(config, host_view, &enter, &exit);
            match cone_render(
                &cone,
                &enter,
                &exit,
                frame,
                config,
                clip_min,
                clip_max,
                proximity_z(config, viewer, portal.pos),
                capture_frame,
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
        let _ = writeln!(out);
    }

    out
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
        let enter = portal.frame();
        let exit = partner.frame();
        let plan = compute_cone(portal, &partner, config, viewer, frame.size);
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
        let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
        let capture_frame = portal_capture_camera_frame(config, host_view, &enter, &exit);
        match cone_render(
            &cone,
            &enter,
            &exit,
            frame,
            config,
            clip_min,
            clip_max,
            proximity_z(config, viewer, portal.pos),
            capture_frame,
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
    let mut selected = vec![*portal];
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

fn sync_cone_material_tint(
    cone_materials: &Query<&MeshMaterial2d<ColorMaterial>, With<PortalConeMesh>>,
    materials: &mut Assets<ColorMaterial>,
    cone: Entity,
    tint: Color,
) {
    let Ok(material_handle) = cone_materials.get(cone) else {
        return;
    };
    let Some(material) = materials.get_mut(&material_handle.0) else {
        return;
    };
    material.color = tint;
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
        let (_, core) = portal.channel.display();

        if config.debug_outline && plan.target > 0.0 {
            let cone = blend_cones(&plan.min, &plan.wedge, smooth01(plan.target), &enter, &exit);
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
                if let Some((_, via_partner)) = window_eye(&enter, &exit, origin) {
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
                    && (origin - exit.pos).dot(exit.normal) < 0.0
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
