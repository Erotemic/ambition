//! The drop-in presentation plugin + its schedule label.
//!
//! Mirrors the `PortalPlugin` seam contract: systems register `.in_set
//! (PortalPresentationSet)` with no host schedule knowledge; the HOST places
//! the set (typically after its sim→sprite mirror system) and bridges the
//! seam resources/markers (see the crate docs).

use bevy::prelude::*;

use crate::{view_cones, visuals};
use crate::{PortalAimHint, PortalWorldFrame};

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
    /// Placed-portal quads + channel labels, the in-flight shot streak, and
    /// the ground-pickup sprite ([`visuals::sync_portal_visuals`]).
    pub portal_quads: bool,
    /// Mid-transit body-piece decomposition over the host-tagged
    /// [`crate::PortalSceneBody`] ([`visuals::sync_portal_body_pieces`]).
    pub body_pieces: bool,
    /// The held portal-gun sprite aimed by [`PortalAimHint`]
    /// ([`visuals::sync_portal_mode_indicator`]).
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

        if self.portal_quads {
            app.add_systems(
                Update,
                visuals::sync_portal_visuals.in_set(PortalPresentationSet),
            );
        }
        if self.body_pieces {
            app.add_systems(
                Update,
                visuals::sync_portal_body_pieces.in_set(PortalPresentationSet),
            );
        }
        if self.gun_indicator {
            app.add_systems(
                Update,
                visuals::sync_portal_mode_indicator.in_set(PortalPresentationSet),
            );
        }
        if self.disorientation {
            app.add_systems(
                Update,
                visuals::sync_portal_disorientation_indicator.in_set(PortalPresentationSet),
            );
        }
        if self.view_cones {
            app.init_resource::<view_cones::PortalViewConeConfig>();
            app.add_systems(
                Update,
                (
                    view_cones::sync_portal_view_cones,
                    view_cones::debug_portal_view_zones,
                )
                    .in_set(PortalPresentationSet),
            );
        }
    }
}
