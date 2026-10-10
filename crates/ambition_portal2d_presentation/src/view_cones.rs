//! Live through-portal view windows.
//!
//! Each paired portal owns an offscreen capture camera and a window mesh. In
//! viewer mode, the visible wedge is derived from the viewer's body corners and
//! partner mapping, with line-of-sight sampling controlling visibility; static
//! and off modes bypass that calculation. Rigs update mesh/camera state in place
//! and are recreated only when pair topology or capture dimensions change.
//! Dedicated render layers prevent a capture from sampling its own window;
//! positive recursion depth permits one-frame-lag views of other windows.

#![allow(unused_imports)]
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ImageRenderTarget, RenderTarget, ScalingMode};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::sprite_render::AlphaMode2d;
use std::fmt::Write as _;

use ambition_platformer2d_core::cast::{raycast_solids, SolidWorldQuery};
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_portal2d::pieces::PortalAperture;
use ambition_portal2d::view::{aperture_wedge_multi, blend_cones, view_cone, window_eye, ViewCone};
use ambition_portal2d::{find_portal, PlacedPortal, PortalChannel};

use ambition_platformer2d_shared_tangle::gameplay_presentation::ResolvedGameplayPresentation;

use crate::{PortalCameraContinuityHostView, PortalObserverViews, PortalWorldFrame};

/// Clear color of an offscreen capture: a dark tone shows through wherever the
/// exit room has no geometry (rare — parallax usually fills it). Opaque windows
/// draw it directly, so keep it unobtrusive.
const CAPTURE_CLEAR: Color = Color::srgb(0.03, 0.04, 0.05);

const WORLD_RENDER_LAYER: usize = 0;
/// Dedicated layer for portal view-window meshes. The constant is with the
/// other render-layer reservations, because the pass that keeps each camera
/// to its own live room moves a stamped window off this layer.
pub use ambition_platformer2d_shared_tangle::camera_layers::PORTAL_WINDOW_RENDER_LAYER;
const PORTAL_CAPTURE_PARALLAX_LAYER_BASE: usize = 32;
/// Base of the per-portal window layers. Every window mesh carries the shared
/// [`PORTAL_WINDOW_RENDER_LAYER`] (rendered by the main camera) and its own
/// `base + slot` layer. A capture camera can then see every other portal's
/// window (recursion). It sees its own only when the pair looks at itself
/// (`pair_looks_at_itself`): on a thin-wall pair a self-capture shows as a
/// nested window with one frame of lag, and it is not correct optics.
/// Base 512 keeps clear of the parallax layers (32 + slot, slot ≤ ~300).
const PORTAL_WINDOW_SELF_LAYER_BASE: usize = 512;

fn portal_capture_parallax_layer(channel: PortalChannel) -> usize {
    PORTAL_CAPTURE_PARALLAX_LAYER_BASE + portal_channel_render_slot(channel)
}

/// The layers a portal's window mesh draws on.
///
/// The debug dump uses this too, to report whether a pane and an actor share
/// a layer. A second spelling could make the report disagree with the renderer.
pub(crate) fn portal_window_render_layers(channel: PortalChannel) -> RenderLayers {
    // The shared layer is what the main camera renders. The per-portal layer
    // lets other rigs' captures include this window, but never its own capture.
    // While two rooms are live, the room pass of the renderer moves the shared
    // layer of a stamped window to the window layer of its room.
    RenderLayers::layer(PORTAL_WINDOW_RENDER_LAYER).with(portal_window_self_layer(channel))
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
            use ambition_portal2d::PortalChannelColor::*;
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
    room_band: Option<usize>,
) -> RenderLayers {
    let mut layers = RenderLayers::layer(WORLD_RENDER_LAYER);
    // While two rooms are live, the viewer room's world is on its room band
    // and not on the world layer, so the capture must add that band to see
    // it. The window mesh is not on a room band, so this adds no feedback.
    if let Some(band) = room_band {
        layers = layers.with(band);
    }
    if include_parallax {
        layers = layers.with(parallax_layer);
    }
    // Recursion sees other portals' windows through their per-portal layers.
    // The shared window layer would include this rig's own mesh and feed the
    // capture back into itself.
    if recursion_depth > 0 {
        for &layer in other_windows {
            layers = layers.with(layer);
        }
    }
    layers
}

/// One viewpoint: an eye in one live room. The host publishes one for each
/// live room that a local view frames ([`PortalViewers`]).
#[derive(Clone, Debug, Default)]
pub struct PortalViewer {
    /// Whether a controlled-character eye is available this frame.
    pub present: bool,
    /// The observer this eye is for: the entity the host records that
    /// observer's camera sample by ([`PortalObserverViews`]). The windows of
    /// the eye's room are clipped to that camera and captured for it. `None`
    /// has no camera sample: a window is then clipped to its room.
    pub observer: Option<Entity>,
    /// The controlled character's eye position (body center), world space.
    pub eye: Vec2,
    /// The live room the eye is in. Near and far are relative to an eye, so a
    /// window or a far-side composite is made only in this room; `None` is a
    /// room that cannot be told.
    pub room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
    /// The character's body half-size: line-of-sight is tested from all four
    /// body corners, so partial cover yields a partial (smoothly blended)
    /// window instead of a binary popping one.
    pub half_size: Vec2,
    /// Solid AABBs for the line-of-sight test — a portal whose aperture is
    /// occluded from `eye` renders no window. The host syncs these from its
    /// collision world (only the blocks that block sight).
    pub occluders: Vec<ae::Aabb>,
}

/// Host seam: the viewpoints of this frame, one for each live room that a
/// local view frames.
///
/// A window is what one eye sees through a portal of its own live room, so
/// each live room has one eye at most. Two observers of ONE room share the
/// windows of the first of them: a window for each observer needs a rig and
/// a capture for each, which this crate does not make.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortalViewers {
    viewers: Vec<PortalViewer>,
}

impl PortalViewers {
    /// One viewpoint: the whole seam of a session with one observer.
    pub fn one(viewer: PortalViewer) -> Self {
        let mut viewers = Self::default();
        viewers.publish(viewer);
        viewers
    }

    /// Remove each viewpoint. The host calls this first in a frame.
    pub fn clear(&mut self) {
        self.viewers.clear();
    }

    /// Add a viewpoint. A second eye in a room that has one is refused: the
    /// first observer of a room, in the order the host publishes, is the eye
    /// of that room.
    pub fn publish(&mut self, viewer: PortalViewer) -> bool {
        if viewer.room.is_some() && self.viewers.iter().any(|held| held.room == viewer.room) {
            return false;
        }
        self.viewers.push(viewer);
        true
    }

    /// The eye of live room `room`. A room that cannot be told has no eye.
    pub fn in_room(&self, room: ambition_portal2d::PortalRoom) -> Option<&PortalViewer> {
        let room = room?;
        self.viewers.iter().find(|viewer| viewer.room == Some(room))
    }

    /// Each viewpoint, in the order the host published them.
    pub fn iter(&self) -> impl Iterator<Item = &PortalViewer> {
        self.viewers.iter()
    }

    /// The first viewpoint: the one a diagnostic of one eye describes.
    pub fn first(&self) -> Option<&PortalViewer> {
        self.viewers.first()
    }
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

/// `Dynamic` by default: the engine does not choose a game's look. A game
/// that wants the small authored cone states `Static` for its own rooms, as a
/// declared rule (`PortalViewConeRule`); Sanic and Smash do. A default is
/// read by every game that composes this crate, so a choice for one game
/// made here changed Ambition's cones too.
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
        // The capture is what the main camera would draw from the far side:
        // the host view mapped through the pair, at the screen's own density.
        // Each texel of it is one pixel of the pane, so the far half of a
        // body that crosses joins its near half at the seam with no step. The
        // cone rect is drawn into a texture of a fixed size, at a scale that
        // is not the screen's on either axis: the far half was resampled and
        // sat up to half a texel off (about a pixel on screen).
        Self::MappedCameraSnapshot
    }
}

/// What the active room's game says about the view windows' mode, over the
/// host's own [`PortalViewConeConfig`]. `None` means the host's mode stands.
///
/// The host projects it every frame from the rule the active room's game
/// declared, so a game states its mode for its own rooms without writing the
/// host's configuration, which a developer may have changed. Nothing else
/// writes it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PortalViewConeRule(pub Option<PortalViewConeMode>);

impl From<Option<PortalViewConeMode>> for PortalViewConeRule {
    fn from(mode: Option<PortalViewConeMode>) -> Self {
        Self(mode)
    }
}

impl PortalViewConeRule {
    /// The configuration in force: the host's, with the rule's mode when the
    /// rule states one. Every other field is the host's.
    pub fn over(self, config: &PortalViewConeConfig) -> std::borrow::Cow<'_, PortalViewConeConfig> {
        match self.0.filter(|mode| *mode != config.mode) {
            Some(mode) => std::borrow::Cow::Owned(PortalViewConeConfig {
                mode,
                ..config.clone()
            }),
            None => std::borrow::Cow::Borrowed(config),
        }
    }
}

/// The view-window configuration in force: the active room's rule over the
/// host's [`PortalViewConeConfig`]. Every reader of the live configuration
/// reads it here, so the windows and their debug views cannot disagree.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PortalViewCones<'w> {
    config: Res<'w, PortalViewConeConfig>,
    rule: Option<Res<'w, PortalViewConeRule>>,
}

impl PortalViewCones<'_> {
    pub fn config(&self) -> std::borrow::Cow<'_, PortalViewConeConfig> {
        let rule = self.rule.as_deref().copied().unwrap_or_default();
        rule.over(&self.config)
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
    /// The minimum cone shown once LOS admits the window (depth into the
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
    /// covers the window depth. 1.0  pixel-perfect up to the cap.
    pub texels_per_world_px: f32,
    /// Hard cap on the capture texture's long side (GPU memory guard; a
    /// 2048×256 RGBA capture is ~2 MB per portal).
    pub max_resolution: u32,
    /// Portal-window capture recursion. `0` makes capture cameras see only the
    /// world layer; positive values include other portal windows and preserve
    /// the current one-frame-lag recursive feedback. Exact multi-pass finite
    /// depth can later refine this field without changing the dev UI.
    pub recursion_depth: u32,
    /// Render z of the window mesh. Defaults to [`crate::PORTAL_WINDOW_Z`]: above
    /// the portal rims and labels (9.0–9.2) and the exit body copy, so the captured
    /// far side draws as one seamless source. Below actors (20), so a near-side
    /// actor still occludes it. Above world blocks (0).
    pub z: f32,
    /// Tint multiplied over the capture (opaque). It also attenuates recursion:
    /// a capture sees other portals' windows, so facing portals recurse with one
    /// frame of lag. Below white, each nested level multiplies the tint and the
    /// recursion fades to dark. 1.0 = no attenuation; ~0.8 = a calm fade.
    pub tint: Color,
    /// Debug: draw gizmo outlines of each portal's EXIT sample zone (the
    /// `ViewCone::source` rect, in the portal's channel color, in front of its
    /// partner) and the entry window.
    pub debug_outline: bool,
    pub debug_los_rays: bool,
    /// Debug dump portal filter. Empty means dump all portals. A name like
    /// `c136` or `c137` resolves to that portal and its paired portal, so the
    /// text dump stays small enough to copy/paste while debugging one pair.
    pub debug_dump_portal: String,
}
impl Default for PortalViewConeConfig {
    fn default() -> Self {
        Self {
            mode: PortalViewConeMode::default(),
            visibility_mode: PortalViewConeVisibilityMode::FaceLosWithContinuity,
            aperture_los_quality: PortalApertureLosQuality::Low,
            source_clip_policy: PortalViewConeSourceClipPolicy::ClampToFrame,
            capture_camera_mode: PortalCaptureCameraMode::default(),
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
            z: crate::PORTAL_WINDOW_Z,
            // Pure white: the view through a portal is exactly the exit chart.
            // A below-white tint gives a fading recursion tunnel instead.
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

/// Retire a view-cone rig: despawn every entity it is made of.
///
/// A rig is two entities: the root and `rig.cone`. This is the one teardown
/// for all four sites in `sync_portal_view_cones`. If a rig gets a third
/// entity, add it here, or an offscreen camera and its render target leak.
fn retire_rig(commands: &mut Commands, entity: Entity, rig: &PortalViewRig) {
    commands.entity(entity).despawn();
    commands.entity(rig.cone).despawn();
}

/// One rig, carried by the capture camera entity: which portal channel it
/// serves, the rebuild key it was built for, the live min↔wedge blend state,
/// and handles to its image + mesh + window-mesh entity. Geometry is updated
/// in place each frame; the rig is only respawned when `rebuild` drifts
/// (world size / texture dims) or the pair disappears.
#[derive(Component)]
pub struct PortalViewRig {
    /// The live room of the portal. A channel is not an identity: two live
    /// rooms can each hold a portal of one channel.
    room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
    channel: PortalChannel,
    parallax_layer: usize,
    parallax_anchor: Vec2,
    rebuild: RebuildKey,
    /// Temporal blend state, 0 = minimum cone, 1 = full visible wedge;
    /// approaches the 4-corner visibility fraction at `blend_rate`/s and is
    /// shaped by a smoothstep before use, so opening/closing feels smooth.
    blend: f32,
    /// The cones of the last plan that was open (its minimum and its wedge):
    /// what a window that eases shut is drawn from, after its plan is closed.
    last_open: Option<(ViewCone, ViewCone)>,
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

    /// The live room of the portal this rig serves.
    pub fn room(&self) -> ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance {
        self.room
    }

    /// Whether this rig serves `portal` of live room `room`.
    pub fn serves(
        &self,
        room: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
        channel: PortalChannel,
    ) -> bool {
        self.room == room && self.channel == channel
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

    /// Render-space viewpoint for the rig's parallax copies: the host camera
    /// centre mapped through the portal pair. The capture camera's own transform
    /// is wrong here, because the framing policy chooses its centre (for example
    /// a tight cone-rect frame).
    pub fn parallax_anchor(&self) -> Vec2 {
        self.parallax_anchor
    }
}

/// Physical screen pixels the main camera spends per world pixel — the density
/// a "pixel-perfect" capture must match, or the window reads blurrier than the
/// world around it. With no window (a host that draws to a texture: a capture
/// tool, a stream) it is the size of the main camera's own target. Falls back
/// to 1.0 when neither is known, or the host view is not (headless, first
/// frame).
#[derive(bevy::ecs::system::SystemParam)]
pub struct GameplayScreenDensity<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<bevy::window::PrimaryWindow>>,
    presentation: Option<Res<'w, ResolvedGameplayPresentation>>,
    cameras: Query<
        'w,
        's,
        &'static Camera,
        (
            With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
            // Disjoint from the capture rigs' cameras, which the same system
            // writes.
            Without<PortalViewRig>,
        ),
    >,
}

impl GameplayScreenDensity<'_, '_> {
    fn texels_per_world(&self, host_view: Option<&PortalCameraContinuityHostView>) -> f32 {
        let Some(view) =
            host_view.filter(|v| v.initialized && v.visible_view.x >= 1.0 && v.visible_view.y >= 1.0)
        else {
            return 1.0;
        };
        let physical = match self.windows.single() {
            Ok(window) => {
                let logical = self
                    .presentation
                    .as_deref()
                    .map(|presentation| presentation.gameplay_rect.size())
                    .unwrap_or_else(|| Vec2::new(window.width(), window.height()));
                logical * window.scale_factor().max(f32::EPSILON)
            }
            // No window: the pixels of the target the main camera draws to.
            Err(_) => match self.cameras.iter().find_map(|camera| camera.physical_viewport_size()) {
                Some(size) => size.as_vec2(),
                None => return 1.0,
            },
        };
        let sx = physical.x / view.visible_view.x;
        let sy = physical.y / view.visible_view.y;
        sx.max(sy).clamp(1.0, 4.0)
    }
}

/// How far past the viewer's own extent a portal still counts as "at the seam"
/// for capture priority (world px).
const PORTAL_SEAM_REACH: f32 = 64.0;

/// A portal is "at the seam" when the viewer is within its reach of the
/// aperture. Such a portal (and, on a thin wall, its partner) always refreshes
/// its capture, bypassing the slot cap and refresh interval on every quality
/// tier. A stale window at the opening being crossed is the most visible
/// flicker. Away from portals the ordinary budget applies.
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
    convention: ambition_portal2d::pieces::MapConvention,
) -> Option<Vec2> {
    host_view.filter(|v| v.initialized).map(|v| {
        ambition_portal2d::pieces::map_point(
            v.current_center_world,
            &enter.frame,
            &exit.frame,
            convention,
        )
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
    convention: ambition_portal2d::pieces::MapConvention,
) -> Option<geometry::CaptureCameraFrame> {
    if config.capture_camera_mode != PortalCaptureCameraMode::MappedCameraSnapshot {
        return None;
    }
    let host_view = host_view.filter(|view| {
        view.initialized && view.visible_view.x >= 1.0 && view.visible_view.y >= 1.0
    })?;
    // Map the whole host-view rect, not only its centre: a 90° pair rotates
    // the viewport and swaps width and height. With the unrotated size, mapped
    // cone vertices fell outside the capture rect and the window smeared.
    // `map_aabb` is exact for cardinal portals, and the mesh's entry polygon is
    // clipped to this same host rect, so every mapped vertex lands inside.
    let rect = ae::Aabb::new(host_view.current_center_world, host_view.visible_view * 0.5);
    let mapped = ambition_portal2d::pieces::map_aabb(rect, &enter.frame, &exit.frame, convention);
    Some(geometry::CaptureCameraFrame {
        center: mapped.center(),
        size: mapped.half_size() * 2.0,
    })
}

mod geometry;
mod mesh;

// The debug overlay and text/PNG dump live in `debug.rs`, re-exported so
// `view_cones::<item>` paths stay valid.
mod debug;
pub use debug::*;
use geometry::{came_through, eased_blend, pair_looks_at_itself, 
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
/// The asset stores the cone rig writes, grouped to stay inside Bevy's
/// sixteen-parameter limit.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ConeRigAssets<'w> {
    images: ResMut<'w, Assets<Image>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<ColorMaterial>>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn sync_portal_view_cones(
    mut commands: Commands,
    selection: Res<crate::PortalEffectSelection>,
    view_cones: PortalViewCones,
    quality: Res<PortalCaptureQualityBudget>,
    viewers: Option<Res<PortalViewers>>,
    frames: crate::PortalFrames,
    observer_views: Option<Res<PortalObserverViews>>,
    mut assets: ConeRigAssets,
    time: Res<Time>,
    portals: Query<(Entity, &PlacedPortal)>,
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
    screen_density: GameplayScreenDensity,
    // The session's portal map convention, from the resource that owns it.
    tuning: Option<Res<ambition_portal2d::PortalTuning>>,
) {
    let (images, meshes, materials) = (
        &mut *assets.images,
        &mut *assets.meshes,
        &mut *assets.materials,
    );
    let convention = tuning
        .as_deref()
        .map(|tuning| tuning.convention.map_convention())
        .unwrap_or_default();
    let config = view_cones.config();
    let config: &PortalViewConeConfig = &config;
    if selection.active != crate::PortalVisualEffect::ViewCones {
        for (entity, rig, ..) in &rigs {
            retire_rig(&mut commands, entity, rig);
        }
        return;
    }
    if config.mode == PortalViewConeMode::Off {
        for (entity, rig, ..) in &rigs {
            retire_rig(&mut commands, entity, rig);
        }
        return;
    }
    // A window is what an eye sees through a portal of its own live room, so
    // each live room that has an eye has its rigs, in that room's frame, and
    // a live room with no eye has none.
    let by_room = frames.portals_by_room(portals.iter());
    let effective = effective_portal_capture_budget(&config, &quality);
    let mut eyes: Vec<RoomEye> = Vec::new();
    for viewer in viewers.iter().flat_map(|viewers| viewers.iter()) {
        let Some(placement) = frames.in_room(viewer.room) else {
            continue;
        };
        let host_view = observer_views.as_deref().and_then(|views| views.of(viewer.observer));
        let (clip_min, clip_max) = portal_window_clip_rect(&placement.frame, host_view);
        eyes.push(RoomEye {
            placement,
            room_band: frames.band(placement.room),
            all: by_room.in_room(Some(placement.room)).to_vec(),
            viewer,
            host_view,
            clip_min,
            clip_max,
            screen_scale: screen_density.texels_per_world(host_view),
        });
    }
    if eyes.is_empty() {
        for (entity, rig, ..) in &rigs {
            retire_rig(&mut commands, entity, rig);
        }
        return;
    }
    let now_s = time.elapsed_secs();
    let mut active_captures = 0u32;
    let mut updates_this_frame = 0u32;

    // The blend each window had, before this frame changes it: a window that
    // takes its viewer from its partner goes on from the partner's blend, and
    // a rig that is built again goes on from its own.
    let before: Vec<(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, PortalChannel, f32)> =
        rigs.iter().map(|(_, rig, ..)| (rig.room, rig.channel, rig.blend)).collect();
    let blend_before = |room, channel: PortalChannel| {
        before.iter().find(|(r, c, _)| *r == room && *c == channel).map_or(0.0, |(_, _, blend)| *blend)
    };
    let step = (config.blend_rate * time.delta_secs()).clamp(0.0, 1.0);

    // First pass: update each live rig in place, or despawn it if its pair is
    // gone / it needs a full rebuild.
    let mut served: Vec<(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance, PortalChannel)> =
        Vec::new();
    for (entity, mut rig, mut cam_tf, mut proj, mut cam, mut layers) in &mut rigs {
        let Some(eye) = eyes.iter().find(|eye| eye.placement.room == rig.room) else {
            retire_rig(&mut commands, entity, &rig);
            continue;
        };
        let RoomEye { placement: _, room_band, all, viewer, host_view, clip_min, clip_max, screen_scale } = eye;
        let (room_band, host_view, clip_min, clip_max, screen_scale) =
            (*room_band, *host_view, *clip_min, *clip_max, *screen_scale);
        let viewer = Some(*viewer);
        let frame = eye.placement.frame;
        // A room with no size yet: leave its rigs as they are.
        if frame.size == Vec2::ZERO {
            served.push((rig.room, rig.channel));
            continue;
        }
        let portal = all.iter().find(|p| p.channel == rig.channel).cloned();
        let partner = portal
            .as_ref()
            .and_then(|p| find_portal(all, p.channel.partner()));
        let (Some(portal), Some(partner)) = (portal, partner) else {
            retire_rig(&mut commands, entity, &rig);
            continue;
        };
        let (enter, exit) = (portal.aperture(), partner.aperture());
        let capture_frame =
            portal_capture_camera_frame(&config, host_view, &enter, &exit, convention);
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
            retire_rig(&mut commands, entity, &rig);
            continue;
        }
        served.push((rig.room, rig.channel));
        *layers = capture_render_layers(
            effective.recursion_depth,
            effective.include_parallax,
            rig.parallax_layer,
            &windows_a_capture_may_see(all, &portal, &partner, room_band),
            room_band,
        );
        sync_cone_material_tint(&cone_materials, materials, rig.cone, config.tint);

        let plan = compute_cone(&portal, &partner, &config, viewer, frame.size, convention);
        // Which pane of the pair the viewer is in front of, before the window
        // can close: a closed window is still the near one or the far one, and
        // the portal's frame is drawn over or under the glass by this answer.
        // A window that closed when the viewer crossed kept its old answer,
        // and its frame was drawn over its partner's glass.
        let (z, pane_dominant) =
            pane_z(&config, viewer, &portal, &partner, Some(rig.pane_dominant));
        rig.pane_dominant = pane_dominant;
        // Temporal approach to the visibility fraction, smoothstep-shaped: a
        // window opens with an ease and closes with one.
        // A window that takes its viewer from its partner goes on from the
        // partner's blend.
        let taken_over = if viewer.is_some_and(|viewer| came_through(viewer.eye, &partner)) {
            blend_before(rig.room, rig.channel.partner())
        } else {
            0.0
        };
        rig.blend = eased_blend(rig.blend, &plan, taken_over, step);
        if plan.target > 0.0 {
            rig.last_open = Some((plan.min, plan.wedge));
        }
        // A window whose plan is closed and that is not shut yet eases shut
        // on the cones it had.
        let Some((min, wedge)) = rig.last_open.filter(|_| rig.blend > 0.0) else {
            rig.blend = 0.0;
            rig.last_open = None;
            cam.is_active = false;
            if let Ok((_, mut vis)) = cones.get_mut(rig.cone) {
                *vis = Visibility::Hidden;
            }
            continue;
        };
        let cone = blend_cones(
            &min,
            &wedge,
            smooth01(rig.blend),
            &enter,
            &exit,
            convention,
        );
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
            convention,
        );
        match render {
            Some(r) => {
                if let Some(mut mesh) = meshes.get_mut(&rig.mesh) {
                    apply_mesh(&mut mesh, &r);
                }
                cam_tf.translation = r.cam_center;
                rig.parallax_anchor = frame
                    .to_render(
                        portal_parallax_anchor_world(
                            host_view,
                            &enter,
                            &exit,
                            convention,
                        )
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

    // Second pass: spawn rigs for desired pairs not yet served, room by room.
    for eye in &eyes {
    let RoomEye { placement, room_band, all, viewer, host_view, clip_min, clip_max, screen_scale } = eye;
    let (room_band, host_view, clip_min, clip_max, screen_scale) =
        (*room_band, *host_view, *clip_min, *clip_max, *screen_scale);
    let viewer = Some(*viewer);
    let frame = placement.frame;
    if frame.size == Vec2::ZERO {
        continue;
    }
    for portal in all.iter() {
        let Some(partner) = find_portal(all, portal.channel.partner()) else {
            continue;
        };
        if served.contains(&(placement.room, portal.channel)) {
            continue;
        }
        let (enter, exit) = (portal.aperture(), partner.aperture());
        let capture_frame =
            portal_capture_camera_frame(&config, host_view, &enter, &exit, convention);
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
        let plan = compute_cone(portal, &partner, &config, viewer, frame.size, convention);
        // A rig that is built again goes on from the blend it had. A window
        // that is new opens with the ease, from nothing or from its partner.
        let taken_over = if viewer.is_some_and(|viewer| came_through(viewer.eye, &partner)) {
            blend_before(placement.room, portal.channel.partner())
        } else {
            0.0
        };
        let blend = eased_blend(blend_before(placement.room, portal.channel), &plan, taken_over, step);
        let cone = if plan.target > 0.0 && blend > 0.0 {
            Some(blend_cones(
                &plan.min,
                &plan.wedge,
                smooth01(blend),
                &enter,
                &exit,
                convention,
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
                convention,
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
                portal_window_render_layers(portal.channel),
                // The window is of this live room: only a camera that frames
                // the room draws it while two rooms are live.
                placement.stamp(),
                // The vertices are rewritten each frame, but Bevy computes a mesh
                // entity's culling Aabb only once. A stale Aabb (possibly the
                // degenerate placeholder) would cull a correct window. Never cull it.
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
                // From the channel's stable render slot, not the query index,
                // which is not stable and would shuffle capture order.
                order: -8 - portal_channel_render_slot(portal.channel) as isize,
                is_active: active,
                clear_color: ClearColorConfig::Custom(CAPTURE_CLEAR),
                ..default()
            },
            // A single-sampled target needs a single-sampled camera: a default
            // 4×-MSAA camera renders nothing into it.
            Msaa::Off,
            RenderTarget::Image(ImageRenderTarget::from(image.clone())),
            capture_render_layers(
                effective.recursion_depth,
                effective.include_parallax,
                portal_capture_parallax_layer(portal.channel),
                &windows_a_capture_may_see(all, portal, &partner, room_band),
                room_band,
            ),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: scaling,
                ..OrthographicProjection::default_2d()
            }),
            cam_tf,
            PortalViewRig {
                room: placement.room,
                channel: portal.channel,
                parallax_layer: portal_capture_parallax_layer(portal.channel),
                parallax_anchor: frame
                    .to_render(
                        portal_parallax_anchor_world(
                            host_view,
                            &enter,
                            &exit,
                            convention,
                        )
                        .or_else(|| render.as_ref().map(|r| (r.source_min + r.source_max) * 0.5))
                        .unwrap_or(exit.frame.origin),
                        0.0,
                    )
                    .truncate(),
                rebuild,
                blend,
                last_open: (plan.target > 0.0).then_some((plan.min, plan.wedge)),
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
}

/// One live room that has an eye, and what its windows are made from.
struct RoomEye<'a> {
    placement: crate::PortalPlacement,
    /// The render band of the room while two or more rooms are live.
    room_band: Option<usize>,
    /// The placed portals of the room.
    all: Vec<PlacedPortal>,
    viewer: &'a PortalViewer,
    /// The camera sample of the observer the eye is for.
    host_view: Option<&'a PortalCameraContinuityHostView>,
    clip_min: Vec2,
    clip_max: Vec2,
    screen_scale: f32,
}

/// The per-portal window layers a capture of `own` may see when recursion is
/// on: each other window of its room, while one room is live, and its own
/// window when it and its partner look at each other
/// ([`pair_looks_at_itself`]).
///
/// While two or more rooms are live it sees none. A per-portal layer is the
/// layer of a channel, and two live rooms can each hold a portal of one
/// channel at the same coordinates, so a capture would draw a window of the
/// other room.
fn windows_a_capture_may_see(
    all: &[PlacedPortal],
    own: &PlacedPortal,
    partner: &PlacedPortal,
    room_band: Option<usize>,
) -> Vec<usize> {
    match room_band {
        Some(_) => Vec::new(),
        None => {
            let mut layers = other_window_layers(all, own.channel);
            if pair_looks_at_itself(own, partner) {
                layers.push(portal_window_self_layer(own.channel));
            }
            layers
        }
    }
}

/// `Shift+F8` requests a portal view-cone dump. Plain F8 remains the gameplay
/// trace dump, so the two expensive diagnostics are explicit and independent.

fn sync_cone_material_tint(
    cone_materials: &Query<&MeshMaterial2d<ColorMaterial>, With<PortalConeMesh>>,
    materials: &mut Assets<ColorMaterial>,
    cone: Entity,
    tint: Color,
) {
    let Ok(material_handle) = cone_materials.get(cone) else {
        return;
    };
    let Some(mut material) = materials.get_mut(&material_handle.0) else {
        return;
    };
    material.color = tint;
}

#[cfg(test)]
mod rule_tests {
    use super::*;

    /// The room's rule changes only the mode. Every other field is the host's,
    /// and the host's configuration is never written.
    #[test]
    fn a_rooms_cone_rule_changes_only_the_mode_of_the_host_configuration() {
        use bevy::ecs::system::RunSystemOnce as _;

        fn live(world: &mut World) -> PortalViewConeConfig {
            world
                .run_system_once(|cones: PortalViewCones| cones.config().into_owned())
                .expect("the reader runs")
        }

        let hosts = PortalViewConeConfig {
            mode: PortalViewConeMode::Dynamic,
            dynamic_depth_close: 999.0,
            ..Default::default()
        };
        let mut world = World::new();
        world.insert_resource(hosts.clone());
        assert_eq!(live(&mut world), hosts, "with no rule the host's configuration stands");

        world.insert_resource(PortalViewConeRule(Some(PortalViewConeMode::Static)));
        assert_eq!(
            live(&mut world),
            PortalViewConeConfig {
                mode: PortalViewConeMode::Static,
                ..hosts.clone()
            },
            "the room's rule must change the mode and keep every other host field"
        );
        assert_eq!(
            world.resource::<PortalViewConeConfig>(),
            &hosts,
            "reading the rule wrote the host's configuration"
        );
    }
}

#[cfg(test)]
mod rig_tests;
