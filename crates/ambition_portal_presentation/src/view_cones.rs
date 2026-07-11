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

use ambition_engine_core::cast::{raycast_solids, SolidWorldQuery};
use ambition_engine_core::{self as ae, AabbExt};
use ambition_portal::pieces::PortalAperture;
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
/// Base of the per-portal window layers. Every window mesh carries the shared
/// [`PORTAL_WINDOW_RENDER_LAYER`] (what the MAIN camera renders) PLUS its own
/// `base + slot` layer, so a capture camera can include every OTHER portal's
/// window (true recursion) while excluding its own — a window photographing
/// itself is never correct optics, and on a thin-wall pair the self-capture
/// fed back as a spurious nested window with one frame of lag. Base 512 keeps
/// clear of the parallax layers (32 + slot, slot ≤ ~300).
const PORTAL_WINDOW_SELF_LAYER_BASE: usize = 512;

fn portal_capture_parallax_layer(channel: PortalChannel) -> usize {
    PORTAL_CAPTURE_PARALLAX_LAYER_BASE + portal_channel_render_slot(channel)
}

fn portal_window_self_layer(channel: PortalChannel) -> usize {
    PORTAL_WINDOW_SELF_LAYER_BASE + portal_channel_render_slot(channel)
}

/// The per-portal window layers of every placed portal EXCEPT `own` — the set
/// a capture camera may see when recursion is on.
fn other_window_layers(all: &[PlacedPortal], own: PortalChannel) -> Vec<usize> {
    all.iter()
        .filter(|p| p.channel != own)
        .map(|p| portal_window_self_layer(p.channel))
        .collect()
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

fn capture_render_layers(
    recursion_depth: u32,
    include_parallax: bool,
    parallax_layer: usize,
    other_windows: &[usize],
) -> RenderLayers {
    let mut layers = RenderLayers::layer(WORLD_RENDER_LAYER);
    if include_parallax {
        layers = layers.with(parallax_layer);
    }
    // Recursion sees the OTHER portals' windows via their per-portal layers —
    // never the shared window layer, which would include this rig's OWN mesh
    // and feed the capture back into itself (the thin-wall nested-window bug).
    if recursion_depth > 0 {
        for &layer in other_windows {
            layers = layers.with(layer);
        }
    }
    layers
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

/// Host-supplied quality budget for portal capture rigs.
///
/// This resource is deliberately profile-free. The Ambition host resolves
/// Low/Medium/High into concrete fields once, then copies the portal slice here.
#[derive(Resource, Clone, Debug, Reflect, PartialEq)]
#[reflect(Resource)]
pub struct PortalCaptureQualityBudget {
    pub max_resolution: u32,
    pub texels_per_world_px: f32,
    pub recursion_depth: u32,
    pub max_active_captures: u32,
    pub max_updates_per_frame: u32,
    pub min_refresh_interval_s: f32,
    pub include_parallax: bool,
}

impl Default for PortalCaptureQualityBudget {
    fn default() -> Self {
        Self {
            max_resolution: 1024,
            texels_per_world_px: 1.0,
            recursion_depth: 1,
            max_active_captures: 2,
            max_updates_per_frame: 2,
            min_refresh_interval_s: 0.0,
            include_parallax: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectivePortalCaptureBudget {
    pub max_resolution: u32,
    pub texels_per_world_px: f32,
    pub recursion_depth: u32,
    pub max_active_captures: u32,
    pub max_updates_per_frame: u32,
    pub min_refresh_interval_s: f32,
    pub include_parallax: bool,
}

pub fn effective_portal_capture_budget(
    config: &PortalViewConeConfig,
    quality: &PortalCaptureQualityBudget,
) -> EffectivePortalCaptureBudget {
    EffectivePortalCaptureBudget {
        max_resolution: config.max_resolution.min(quality.max_resolution),
        texels_per_world_px: config.texels_per_world_px.min(quality.texels_per_world_px),
        recursion_depth: config.recursion_depth.min(quality.recursion_depth),
        max_active_captures: quality.max_active_captures.max(1),
        max_updates_per_frame: quality.max_updates_per_frame.max(1),
        min_refresh_interval_s: quality.min_refresh_interval_s.max(0.0),
        include_parallax: quality.include_parallax,
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
        // ConeRect: the capture frames the tight source rect, so the fixed
        // texture's density is spent entirely on what the window shows. Its
        // historical parallax problem (background evaluated at the framing
        // center) is gone — parallax copies anchor at the rig's
        // `parallax_anchor` (the mapped host camera), independent of framing.
        Self::ConeRect
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
    /// Maximum face separation (world px) for an opposed-normal pair to count
    /// as a DOORWAY — a hole through a thin shared wall rather than a
    /// wormhole between disjoint places. A doorway's window is clipped to the
    /// wall slab and never takes over the half-plane: its two charts are the
    /// same visual space, so a takeover pane would photograph a region that
    /// is also directly on screen and double-image it. The kin criterion for
    /// the camera is `PortalCameraContinuityConfig::min_anchor_camera_cut`.
    pub doorway_pair_max_gap: f32,
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
    /// Render z of the window mesh. Defaults to [`crate::PORTAL_WINDOW_Z`]:
    /// OVER the portal rims/labels (9.0–9.2) and the exit body copy — the
    /// window is a captured composite of the far side, so it draws as the
    /// single seamless source, not underneath the far portal's frame or a
    /// doubled sprite — while staying BELOW actors (20) so a near-side actor
    /// still occludes it. Above world blocks (0).
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
            capture_camera_mode: PortalCaptureCameraMode::ConeRect,
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
            doorway_pair_max_gap: 64.0,
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
            z: crate::PORTAL_WINDOW_Z,
            // Pure white: the window is a SEAMLESS view — no tint, so what
            // you see through a portal is exactly the exit chart. The field
            // stays a knob: a below-white tint makes nested recursion levels
            // converge to dark (each level multiplies the tint) if a game
            // wants a fading tunnel instead of full-brightness recursion.
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
    parallax_anchor: Vec2,
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
    last_capture_update_s: f32,
    /// Sticky pairwise pane-dominance winner (see [`mesh::pane_z`]): keeps the
    /// two overlapping panes of a thin-wall pair from swapping draw order with
    /// sub-pixel eye jitter around the material midpoint.
    pane_dominant: bool,
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

    /// Sticky pairwise pane-dominance winner (see `mesh::pane_z`): true when
    /// this portal's pane — and its identifying frame — draw on top of the
    /// partner's. The portal frame overlay reuses this so the frame you are
    /// in front of stays whole while the far frame hides behind the glass,
    /// with the same hysteresis (no flicker at the material midpoint).
    pub fn pane_dominant(&self) -> bool {
        self.pane_dominant
    }

    /// Render-space viewpoint the rig's parallax copies should be anchored to:
    /// the HOST camera's center mapped through the portal pair — the position
    /// a viewer looking through this window effectively sees from. Anchoring
    /// parallax to the capture camera's own transform instead evaluates the
    /// background at whatever point the framing policy happens to center
    /// (wrong for a tight cone-rect frame — the "fundamental parallax issue").
    pub fn parallax_anchor(&self) -> Vec2 {
        self.parallax_anchor
    }
}

/// Physical screen pixels the main camera spends per world pixel — the density
/// a "pixel-perfect" capture must match, or the window reads blurrier than the
/// world around it. Falls back to 1.0 when the window or host view is
/// unavailable (headless, first frame).
fn screen_texels_per_world(
    window: Option<&Window>,
    host_view: Option<&PortalCameraContinuityHostView>,
) -> f32 {
    let (Some(window), Some(view)) = (
        window,
        host_view.filter(|v| v.initialized && v.visible_view.x >= 1.0 && v.visible_view.y >= 1.0),
    ) else {
        return 1.0;
    };
    let sx = window.physical_width() as f32 / view.visible_view.x;
    let sy = window.physical_height() as f32 / view.visible_view.y;
    sx.max(sy).clamp(1.0, 4.0)
}

/// How far past the viewer's own extent a portal still counts as "at the seam"
/// for capture priority (world px).
const PORTAL_SEAM_REACH: f32 = 64.0;

/// A portal is "at the seam" when the viewer is essentially ON it — within its
/// own reach of the aperture. Such a portal (and, on a thin wall, its partner
/// right beside it) ALWAYS refreshes its capture: a stale window at the exact
/// opening you are crossing is the most visible place for capture throttling to
/// flicker, so the crossed pair bypasses the slot cap + refresh interval
/// regardless of quality tier. Away from any portal, the ordinary budget
/// applies, so this costs nothing except at the moment it matters.
fn portal_at_seam(viewer: Option<&PortalViewer>, portal_pos: Vec2) -> bool {
    viewer
        .filter(|v| v.present)
        .is_some_and(|v| v.eye.distance(portal_pos) <= v.half_size.length() + PORTAL_SEAM_REACH)
}

/// World-space viewpoint the rig's parallax should be evaluated at: the host
/// camera center mapped through the pair. `None` when no host view exists.
fn portal_parallax_anchor_world(
    host_view: Option<&PortalCameraContinuityHostView>,
    enter: &PortalAperture,
    exit: &PortalAperture,
) -> Option<Vec2> {
    host_view.filter(|v| v.initialized).map(|v| {
        ambition_portal::pieces::map_point(v.current_center_world, &enter.frame, &exit.frame)
    })
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
    enter: &PortalAperture,
    exit: &PortalAperture,
) -> Option<geometry::CaptureCameraFrame> {
    if config.capture_camera_mode != PortalCaptureCameraMode::MappedCameraSnapshot {
        return None;
    }
    let host_view = host_view.filter(|view| {
        view.initialized && view.visible_view.x >= 1.0 && view.visible_view.y >= 1.0
    })?;
    // Map the WHOLE host-view rect through the body map, not just its center:
    // a 90° pair rotates the viewport, so its image swaps width/height. Using
    // the unrotated size framed the wrong region for floor↔wall pairs — mapped
    // cone vertices fell outside the capture rect, their UVs clamped at the
    // edge, and the window rendered smeared/warped. `map_aabb` is exact for
    // cardinal portals, and the mesh's entry polygon is clipped to this same
    // host rect, so every mapped vertex now lands inside the capture frame.
    let rect = ae::Aabb::new(host_view.current_center_world, host_view.visible_view * 0.5);
    let mapped = ambition_portal::pieces::map_aabb(rect, &enter.frame, &exit.frame);
    Some(geometry::CaptureCameraFrame {
        center: mapped.center(),
        size: mapped.half_size() * 2.0,
    })
}

mod geometry;
mod mesh;

// D-B split: the debug-overlay + text/PNG dump diagnostics live in `debug.rs`.
// Re-exported so `view_cones::<item>` paths (lib.rs re-exports, plugin.rs system
// registration) are unchanged by the relocation.
mod debug;
pub use debug::*;
use geometry::{
    aperture_los_rays, aperture_visibility_fraction, capture_dims, compute_cone, cone_render,
    inset_viewer_corners, visibility_route_summary, ApertureLosRay, ConeRender, RebuildKey,
};
pub(crate) use mesh::pane_dominance;
use mesh::{apply_mesh, make_mesh, pane_z, placeholder_mesh, smooth01};

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
    quality: Res<PortalCaptureQualityBudget>,
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
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
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
    let all: Vec<PlacedPortal> = portals.iter().cloned().collect();
    let viewer = viewer.as_deref();
    let (clip_min, clip_max) = portal_window_clip_rect(&frame, host_view.as_deref());
    let effective = effective_portal_capture_budget(&config, &quality);
    let screen_scale = screen_texels_per_world(windows.single().ok(), host_view.as_deref());
    let now_s = time.elapsed_secs();
    let mut active_captures = 0u32;
    let mut updates_this_frame = 0u32;

    // First pass: update each live rig in place, or despawn it if its pair is
    // gone / it needs a full rebuild.
    let mut served: Vec<PortalChannel> = Vec::new();
    for (entity, mut rig, mut cam_tf, mut proj, mut cam, mut layers) in &mut rigs {
        let portal = all.iter().find(|p| p.channel == rig.channel).cloned();
        let partner = portal
            .as_ref()
            .and_then(|p| find_portal(&all, p.channel.partner()));
        let (Some(portal), Some(partner)) = (portal, partner) else {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        };
        let (enter, exit) = (portal.aperture(), partner.aperture());
        let capture_frame =
            portal_capture_camera_frame(&config, host_view.as_deref(), &enter, &exit);
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(
                &effective,
                &config,
                frame.size,
                partner.normal,
                capture_frame,
                screen_scale,
            ),
            recursion_depth: effective.recursion_depth,
            include_parallax: effective.include_parallax,
        };
        if rig.rebuild != rebuild {
            commands.entity(entity).despawn();
            commands.entity(rig.cone).despawn();
            continue;
        }
        served.push(rig.channel);
        *layers = capture_render_layers(
            effective.recursion_depth,
            effective.include_parallax,
            rig.parallax_layer,
            &other_window_layers(&all, rig.channel),
        );
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
        let (z, pane_dominant) =
            pane_z(&config, viewer, &portal, &partner, Some(rig.pane_dominant));
        rig.pane_dominant = pane_dominant;
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
                rig.parallax_anchor = frame
                    .to_render(
                        portal_parallax_anchor_world(host_view.as_deref(), &enter, &exit)
                            .unwrap_or_else(|| (r.source_min + r.source_max) * 0.5),
                        0.0,
                    )
                    .truncate();
                if let Projection::Orthographic(o) = &mut *proj {
                    o.scaling_mode = ScalingMode::Fixed {
                        width: r.source_size.x,
                        height: r.source_size.y,
                    };
                }
                // The portal (or its partner) you are crossing always refreshes,
                // bypassing the slot cap + refresh interval, so the seam never
                // shows a stale window mid-crossing on a throttled tier.
                let at_seam =
                    portal_at_seam(viewer, portal.pos) || portal_at_seam(viewer, partner.pos);
                let refresh_due = at_seam
                    || now_s - rig.last_capture_update_s >= effective.min_refresh_interval_s;
                let has_active_slot = active_captures < effective.max_active_captures;
                let has_update_slot = updates_this_frame < effective.max_updates_per_frame;
                cam.is_active = refresh_due && (at_seam || (has_active_slot && has_update_slot));
                if cam.is_active {
                    active_captures += 1;
                    updates_this_frame += 1;
                    rig.last_capture_update_s = now_s;
                }
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
    for portal in all.iter() {
        let Some(partner) = find_portal(&all, portal.channel.partner()) else {
            continue;
        };
        if served.contains(&portal.channel) {
            continue;
        }
        let (enter, exit) = (portal.aperture(), partner.aperture());
        let capture_frame =
            portal_capture_camera_frame(&config, host_view.as_deref(), &enter, &exit);
        let rebuild = RebuildKey {
            world_size: frame.size,
            tex: capture_dims(
                &effective,
                &config,
                frame.size,
                partner.normal,
                capture_frame,
                screen_scale,
            ),
            recursion_depth: effective.recursion_depth,
            include_parallax: effective.include_parallax,
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
        let (z, pane_dominant) = pane_z(&config, viewer, portal, &partner, None);
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
                // Shared layer = what the MAIN camera renders; the per-portal
                // layer lets other rigs' captures include this window without
                // any capture ever seeing its OWN window.
                RenderLayers::layer(PORTAL_WINDOW_RENDER_LAYER)
                    .with(portal_window_self_layer(portal.channel)),
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
        let (cam_tf, requested_active, scaling) = match &render {
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
        // Same seam priority as the update pass: a freshly-spawned rig for the
        // pair you are crossing captures immediately, never waiting on a slot.
        let at_seam = portal_at_seam(viewer, portal.pos) || portal_at_seam(viewer, partner.pos);
        let active = requested_active
            && (at_seam
                || (active_captures < effective.max_active_captures
                    && updates_this_frame < effective.max_updates_per_frame));
        if active {
            active_captures += 1;
            updates_this_frame += 1;
        }
        commands.spawn((
            Camera2d,
            Camera {
                // Derived from the channel's stable render slot — NOT the
                // portal query index, which is not stable across frames and
                // would shuffle capture ordering (visible as recursion
                // shimmer when multiple pairs are live).
                order: -8 - portal_channel_render_slot(portal.channel) as isize,
                is_active: active,
                clear_color: ClearColorConfig::Custom(CAPTURE_CLEAR),
                ..default()
            },
            // Single-sampled target needs a single-sampled camera (see commit
            // history): a default 4×-MSAA camera renders nothing into it.
            Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image.clone())),
            capture_render_layers(
                effective.recursion_depth,
                effective.include_parallax,
                portal_capture_parallax_layer(portal.channel),
                &other_window_layers(&all, portal.channel),
            ),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: scaling,
                ..OrthographicProjection::default_2d()
            }),
            cam_tf,
            PortalViewRig {
                channel: portal.channel,
                parallax_layer: portal_capture_parallax_layer(portal.channel),
                parallax_anchor: frame
                    .to_render(
                        portal_parallax_anchor_world(host_view.as_deref(), &enter, &exit)
                            .or_else(|| {
                                render.as_ref().map(|r| (r.source_min + r.source_max) * 0.5)
                            })
                            .unwrap_or(exit.frame.origin),
                        0.0,
                    )
                    .truncate(),
                rebuild,
                blend: if plan.target > 0.0 { plan.target } else { 0.0 },
                _image: image,
                mesh,
                cone: cone_entity,
                last_capture_update_s: if active { now_s } else { f32::NEG_INFINITY },
                pane_dominant,
            },
            Name::new(format!("Portal view capture ({})", portal.channel.name())),
        ));
    }
}

/// F8 requests a portal view-cone dump. This intentionally shares the existing
/// trace-dump hotkey: for portal rendering bugs, the gameplay trace and the
/// portal presentation snapshot are most useful as a pair.

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
