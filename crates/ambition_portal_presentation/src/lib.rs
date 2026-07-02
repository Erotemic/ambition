//! Default renderer for the headless [`ambition_portal`] mechanic.
//!
//! Provides placed-portal visuals, mid-transit body pieces, disorientation
//! indicators, through-portal view windows, and a sequestered compatibility
//! module for Ambition's portal-gun sprites. Hosts sync the
//! crate-owned seams ([`PortalWorldFrame`], [`PortalSceneBody`],
//! [`PortalGunArt`], [`PortalAimHint`]) and may replace any visual by disabling
//! that [`PortalPresentationPlugin`] flag and registering an alternative system.
//!
//! Depends only on `bevy`, `ambition_engine_core`,
//! `ambition_platformer_primitives`, and `ambition_portal`; it never names a host
//! crate.

use bevy::prelude::*;

use ambition_engine_core as ae;

mod camera_continuity;
mod clip_material;
mod effects;
mod gun_visuals;
mod plugin;
#[cfg(feature = "effect_view_cones")]
mod view_cones;
mod visuals;

pub use camera_continuity::{
    camera_roll_for_portal_transit, PortalCameraContinuityCamera, PortalCameraContinuityConfig,
    PortalCameraContinuityFocus, PortalCameraContinuityHostView, PortalCameraContinuitySelection,
    PortalCameraContinuityState, PortalCameraTransitMode,
};
pub use clip_material::{
    clip_piece_transform, clip_plane_render, sprite_frame_basis, PortalClipMaterial,
    SpriteFrameBasis, CLIP_PLANE_OFF,
};
pub use effects::{PortalEffectSelection, PortalVisualEffect};
pub use gun_visuals::{sync_portal_mode_indicator, PortalModeIndicator};
pub use plugin::{PortalPresentationPlugin, PortalPresentationSet};
#[cfg(feature = "effect_view_cones")]
pub use view_cones::{
    debug_portal_view_zones, flush_portal_view_cone_debug_dump,
    handle_portal_view_cone_dump_hotkey, selected_portal_view_cone_debug_rows,
    sync_portal_view_cones, PortalApertureLosQuality, PortalCaptureCameraMode,
    PortalCaptureQualityBudget, PortalConeMesh, PortalDebugOverlay, PortalViewConeConfig,
    PortalViewConeDebugDumpRequest, PortalViewConeDebugRow, PortalViewConeMode,
    PortalViewConeSourceClipPolicy, PortalViewConeVisibilityMode, PortalViewRig, PortalViewer,
    PORTAL_WINDOW_RENDER_LAYER,
};
pub use visuals::{
    sync_portal_body_pieces, sync_portal_disorientation_indicator, sync_portal_visuals,
    PortalBodyPiece, PortalDisorientIndicator, PortalVisual,
};

/// Portal composite z band — the ONE place the seam's front-to-back order is
/// declared. A through-portal window shows a captured composite of the FAR
/// side, so it must draw OVER the portal rims/labels (9.0–9.2) AND over the
/// exit body copy, which then reads as the single seamless source of the far
/// side instead of a second sprite laid on top. It stays BELOW actors
/// (`WORLD_Z_PLAYER` = 20) so a near-side actor standing in front of the
/// aperture still correctly occludes the window.
///
/// The transiting body itself draws as texture-clipped PIECES (see
/// [`sync_portal_body_pieces`]): the `here` slice replaces the real sprite in
/// the actor band (a body in front of the entry surface draws over walls,
/// rims, and windows), while the emerged `through` slice — like the fallback
/// unclipped exit copy — sits at [`PORTAL_EXIT_COPY_Z`], just BELOW the
/// window: an open window (especially the doorway-takeover glass while
/// crossing) captures it on the far side, so the glass stays the single
/// source of the far-side image; drawing the slice on top would paint a
/// second, parallax-offset copy over the glass and cover the exit portal's
/// front rim half. A closed window (LOS blocked / windows off) still shows
/// the slice over the rim as the emerging-body visual.
///
/// NOTE — thin-wall pairs whose two windows overlap in screen space share this
/// band and sort only by viewer proximity; a fully unambiguous composite there
/// needs per-window stenciling (see the review report, Q9).
pub const PORTAL_WINDOW_Z: f32 = 9.5;
/// The exit-side body slice z (just below [`PORTAL_WINDOW_Z`]).
pub const PORTAL_EXIT_COPY_Z: f32 = 9.4;

/// The host-world half of the render transform: the world's size, copied from
/// the host each frame. Engine coordinates are top-left-origin y-down; Bevy's
/// 2D camera is centered y-up; [`Self::to_render`] is the one adapter between
/// them (delegating to `ambition_engine_core::config::world_size_to_bevy` so
/// the math is defined exactly once).
///
/// Host seam: keep `size` synced (e.g. from Ambition's `RoomGeometry`). A zero
/// size just centers everything on the camera origin for a frame — wrong but
/// harmless until the first sync runs.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PortalWorldFrame {
    /// World size in engine units (the world's bottom-right corner).
    pub size: Vec2,
}

impl PortalWorldFrame {
    /// Engine world position → Bevy render translation at layer `z`.
    pub fn to_render(&self, p: Vec2, z: f32) -> Vec3 {
        ae::config::world_size_to_bevy(self.size, p, z)
    }
}

/// Host seam: marks the visual entity whose sprite the mid-transit body-piece
/// decomposition draws (in Ambition, the player's sprite entity). The entity
/// must also carry the runtime `BodyKinematics` plus `Sprite` + `Visibility`;
/// `PortalTransit` / `ActorRoll` are read when present.
#[derive(Component)]
pub struct PortalSceneBody;

/// Host seam: the loaded portal-gun art (blue / orange mode sprites). The
/// crate defines the resource; the HOST loads it — asset paths are content.
/// Absent resource → the held gun doesn't draw and the ground pickup falls
/// back to a marker quad.
#[derive(Resource)]
pub struct PortalGunArt {
    pub blue: Handle<Image>,
    pub orange: Handle<Image>,
}

/// Host seam: content-agnostic aim hint for the held-gun presentation — the
/// resolved world-space direction the barrel should point (the same aim the
/// host's input adapter resolves for `FirePortalGun`). The host writes it each
/// frame; [`sync_portal_mode_indicator`] reads it, so portal presentation
/// never imports a host input type. Zero / unset aim falls back to facing.
///
/// Initialised by [`PortalPresentationPlugin`] (it is render-only state, so it
/// lives here rather than in the headless mechanic's plugin).
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct PortalAimHint {
    /// Resolved aim direction (need not be normalized; zero falls back to facing).
    pub aim: Vec2,
}
