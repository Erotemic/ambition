//! The drop-in presentation plugin + its schedule label.
//!
//! Mirrors the `PortalPlugin` seam contract: systems register `.in_set
//! (PortalPresentationSet)` with no host schedule knowledge; the HOST places
//! the set (typically after its sim→sprite mirror system) and bridges the
//! seam resources/markers (see the crate docs).

use bevy::prelude::*;

use crate::gun_visuals;
#[cfg(feature = "effect_view_cones")]
use crate::view_cones;
use crate::visuals;
#[cfg(feature = "effect_view_cones")]
use crate::PortalDebugOverlay;
use crate::{
    PortalAimHint, PortalCameraContinuityConfig, PortalCameraContinuityHostView,
    PortalCameraContinuitySelection, PortalCameraContinuityState, PortalEffectSelection,
    PortalWorldFrame,
};

/// The one schedule label every portal visual runs in. Hosts order this set
/// against their own presentation systems (e.g. `.after(sync_visuals)`); the
/// crate declares no cross-set edges of its own.
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct PortalPresentationSet;

/// Registers the default portal visuals. Each flag gates one independently
/// registered public system, so a host can turn one visual off and register
/// its own replacement in [`PortalPresentationSet`] — extend by subtraction,
/// not by forking.
#[derive(Clone, Copy, Debug)]
pub struct PortalPresentationPlugin {
    /// Placed-portal quads + channel labels. For compatibility, this system
    /// still calls sequestered gun helpers for in-flight shot and pickup
    /// markers; split that flag after behavior is stable.
    pub portal_quads: bool,
    /// The mid-transit **body pieces** over the host-tagged
    /// [`crate::PortalSceneBody`] ([`visuals::sync_portal_body_pieces`]):
    /// while the body straddles a pair its sprite draws as two texture-clipped
    /// charts (here-slice at the entry, emerged slice at the exit), rebuilt
    /// from the real sprite each frame via [`crate::PortalClipMaterial`].
    pub body_pieces: bool,
    /// The held portal-gun sprite aimed by [`PortalAimHint`]
    /// ([`gun_visuals::sync_portal_mode_indicator`]).
    pub gun_indicator: bool,
    /// The input-warp disorientation glyph
    /// ([`visuals::sync_portal_disorientation_indicator`]).
    pub disorientation: bool,
    /// The through-portal view windows: each portal shows a render-to-texture
    /// capture of the world in front of its partner, receding into its host
    /// surface, with 1-frame-lag recursion when portals face each other
    /// ([`view_cones::sync_portal_view_cones`]; tune via
    /// [`crate::PortalViewConeConfig`]).
    pub view_cones: bool,
}

impl Default for PortalPresentationPlugin {
    fn default() -> Self {
        Self {
            portal_quads: true,
            body_pieces: true,
            gun_indicator: true,
            disorientation: true,
            view_cones: true,
        }
    }
}

impl Plugin for PortalPresentationPlugin {
    fn build(&self, app: &mut App) {
        // Crate-owned seam resources. `PortalAimHint` is render-only state, so
        // it is initialised HERE, not by the headless mechanic's plugin; the
        // host's input adapter writes it each frame.
        app.init_resource::<PortalWorldFrame>();
        app.init_resource::<PortalAimHint>();
        // The live effect choice (view cones / off), cycled from the host's
        // developer menu for in-session A/B profiling.
        app.init_resource::<PortalEffectSelection>();
        // Optional camera/viewpoint continuity is controlled by this resource.
        // It is the single source of truth surfaced by hosts; its resource
        // default currently enables Continuous for portal-lab debugging.
        app.init_resource::<PortalCameraContinuitySelection>();
        app.init_resource::<PortalCameraContinuityConfig>();
        app.init_resource::<PortalCameraContinuityState>();
        app.init_resource::<PortalCameraContinuityHostView>();

        if self.portal_quads {
            app.add_systems(
                Update,
                visuals::sync_portal_visuals.in_set(PortalPresentationSet),
            );
        }
        if self.body_pieces {
            // Texture-clipped transit pieces. No-op on hosts without an asset
            // registry (headless tests); the system then uses its unclipped
            // sprite-copy fallback.
            crate::clip_material::add_portal_clip_material_plugin(app);
            app.add_systems(
                Update,
                visuals::sync_portal_body_pieces.in_set(PortalPresentationSet),
            );
        }
        if self.gun_indicator {
            app.add_systems(
                Update,
                gun_visuals::sync_portal_mode_indicator.in_set(PortalPresentationSet),
            );
        }
        if self.disorientation {
            app.add_systems(
                Update,
                visuals::sync_portal_disorientation_indicator.in_set(PortalPresentationSet),
            );
        }
        #[cfg(feature = "effect_view_cones")]
        if self.view_cones {
            app.init_resource::<view_cones::PortalViewConeConfig>();
            app.init_resource::<view_cones::PortalCaptureQualityBudget>();
            app.init_resource::<view_cones::PortalViewConeDebugDumpRequest>();
            // The viewer seam (host-synced each frame); empty/absent ⇒ static
            // window fallback. Init here so the host can `ResMut` it.
            app.init_resource::<view_cones::PortalViewer>();
            app.init_resource::<PortalDebugOverlay>();
            app.add_systems(
                Update,
                (
                    view_cones::handle_portal_view_cone_dump_hotkey,
                    view_cones::sync_portal_view_cones,
                    view_cones::debug_portal_view_zones,
                    view_cones::flush_portal_view_cone_debug_dump,
                )
                    .chain()
                    .in_set(PortalPresentationSet),
            );
        }
    }
}
