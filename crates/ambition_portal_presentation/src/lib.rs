//! The reusable **default renderer** for the [`ambition_portal`] mechanic.
//!
//! `ambition_portal` is deliberately headless — "rendering is the host's
//! adapter". This crate IS that adapter, packaged so a host doesn't have to
//! write one to get portals on screen: placed-portal quads + channel labels,
//! the in-flight shot streak, the held / pickup gun sprite, the mid-transit
//! body-piece decomposition ("feet in, feet out"), the disorientation
//! indicator, and the through-portal **view windows** (each portal shows the
//! world in front of its partner, receding into its host surface;
//! render-to-texture with 1-frame-lag recursion for facing portals).
//!
//! ## How a host uses it
//! 1. `app.add_plugins(PortalPresentationPlugin::default())`, then place
//!    [`PortalPresentationSet`] in its schedule (typically after the system
//!    that mirrors sim state into sprites) — same wiring pattern as
//!    `PortalPlugin` + the host's `wire_portal_schedule`.
//! 2. Bridge the seams, all crate-owned so the crate never names a host type:
//!    - sync [`PortalWorldFrame`] from the host's world each frame (the
//!      world-size half of the centered y-flip render transform);
//!    - tag [`PortalSceneBody`] on the visual entity whose sprite should
//!      decompose mid-transit (the player's, in Ambition);
//!    - load [`PortalGunArt`] (asset *paths* are content — the host owns them);
//!    - write [`PortalAimHint`] from its input layer (else the held gun falls
//!      back to facing).
//!
//! ## How a host extends or replaces it
//! Every visual is a separately registered **public system** behind a
//! [`PortalPresentationPlugin`] flag. Disable a flag, register your own system
//! in [`PortalPresentationSet`], and keep the rest. A roll-your-own host skips
//! this crate entirely and consumes the geometry from `ambition_portal`
//! (`pieces`, the portal map, the view cone) — the hard-won math lives there,
//! not here.
//!
//! Depends ONLY on `bevy` + `ambition_engine_core` + `ambition_platformer_runtime`
//! + `ambition_portal` — never on a host crate. Read-only over the portal sim.

use bevy::prelude::*;

use ambition_engine_core as ae;

mod effects;
mod plugin;
#[cfg(feature = "effect_view_cones")]
mod view_cones;
mod visuals;

pub use effects::{PortalEffectSelection, PortalVisualEffect};
pub use plugin::{PortalPresentationPlugin, PortalPresentationSet};
#[cfg(feature = "effect_view_cones")]
pub use view_cones::{
    debug_portal_view_zones, sync_portal_view_cones, PortalConeMesh, PortalDebugOverlay,
    PortalViewConeConfig, PortalViewRig, PortalViewer, PORTAL_WINDOW_RENDER_LAYER,
};
pub use visuals::{
    sync_portal_body_pieces, sync_portal_disorientation_indicator, sync_portal_mode_indicator,
    sync_portal_visuals, PortalBodyPiece, PortalDisorientIndicator, PortalModeIndicator,
    PortalVisual,
};

/// The host-world half of the render transform: the world's size, copied from
/// the host each frame. Engine coordinates are top-left-origin y-down; Bevy's
/// 2D camera is centered y-up; [`Self::to_render`] is the one adapter between
/// them (delegating to `ambition_engine_core::config::world_size_to_bevy` so
/// the math is defined exactly once).
///
/// Host seam: keep `size` synced (e.g. from Ambition's `GameWorld`). A zero
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
