//! The drop-in presentation plugin + its schedule label.
//!
//! Mirrors the `PortalPlugin` seam contract: systems register `.in_set
//! (PortalPresentationSet)` with no host schedule knowledge; the HOST places
//! the set (typically after its sim→sprite mirror system) and bridges the
//! seam resources/markers (see the crate docs).

use bevy::prelude::*;

use crate::far_side;
use crate::gun_visuals;
use crate::source_visibility;
#[cfg(feature = "effect_view_cones")]
use crate::view_cones;
use crate::visuals;
#[cfg(feature = "effect_view_cones")]
use crate::PortalDebugOverlay;
use crate::{
    PortalAimHint, PortalCameraContinuityConfig, PortalCameraContinuityHostView,
    PortalCameraContinuitySelection, PortalEffectSelection,
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
    /// Each placed portal's line of light ([`crate::sync_portal_glows`]) and
    /// its channel label. For compatibility, the label system still calls
    /// sequestered gun helpers for in-flight shot and pickup markers; split
    /// that flag after behavior is stable.
    pub portal_quads: bool,
    /// What a pair carries ([`crate::carry`]): each drawable that goes through
    /// a pair (a body in transit, and any drawable that reaches past an entry
    /// from its front) is drawn in front of the entry and out of the exit.
    pub body_pieces: bool,
    /// Far-side compositing ([`far_side::composite_far_side_bodies`]): a body
    /// standing behind an aperture is redrawn as the part the pane leaves
    /// visible, so the covered pixels are never submitted and no depth ordering
    /// can bring them back.
    ///
    /// With this off, far-side bodies draw over panes: a pane draws at
    /// [`crate::PORTAL_WINDOW_Z`] (`9.5`) and every actor above it.
    pub far_side_compositing: bool,
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
            far_side_compositing: true,
            gun_indicator: true,
            disorientation: true,
            view_cones: true,
        }
    }
}

impl Plugin for PortalPresentationPlugin {
    fn build(&self, app: &mut App) {
        // Asleep where no live room uses portals (Mary-O's), after one frame
        // that takes the quads down and gives back the bodies they hid.
        app.configure_sets(
            Update,
            PortalPresentationSet.in_set(
                ambition_platformer2d_shared_tangle::capability::CapabilityPresentationSet(
                    ambition_platformer2d_shared_tangle::capability::Capability::Portals,
                ),
            ),
        );
        // Crate-owned seam resources. `PortalAimHint` is render-only state, so
        // it is initialised HERE, not by the headless mechanic's plugin; the
        // host's input adapter writes it each frame.
        app.init_resource::<PortalAimHint>();
        app.init_resource::<crate::PortalGlowStyle>().register_type::<crate::PortalGlowStyle>();
        // The live effect choice (view cones / off), cycled from the host's
        // developer menu for in-session A/B profiling.
        app.init_resource::<PortalEffectSelection>();
        // Optional camera/viewpoint continuity: the host's selection, and the
        // active room's rule over it (read both through `PortalCameraTransit`).
        // The selection's default enables Continuous for portal-lab debugging.
        app.init_resource::<PortalCameraContinuitySelection>();
        app.init_resource::<crate::PortalCameraTransitRule>();
        app.init_resource::<PortalCameraContinuityConfig>();
        app.init_resource::<crate::PortalParallaxCarry>();
        app.init_resource::<PortalCameraContinuityHostView>();
        app.init_resource::<crate::PortalObserverViews>();
        app.init_resource::<crate::PortalViewers>();

        if self.portal_quads {
            app.add_systems(
                Update,
                visuals::sync_portal_visuals.in_set(PortalPresentationSet),
            );
            // Each portal's line of light, with its placed and dissolved
            // effects. A host with no asset registry draws none.
            crate::glow::add_portal_glow_material_plugin(app);
            app.add_systems(
                Update,
                (
                    crate::glow::sync_portal_glows,
                    (
                        crate::glow::dress_held_light_lines,
                        crate::limb_seams::lay_light_lines_on_limb_seams,
                        crate::glow::sync_portal_light_lines,
                    )
                        .chain(),
                )
                    .in_set(PortalPresentationSet)
                    .run_if(resource_exists::<Assets<Mesh>>)
                    .run_if(resource_exists::<Assets<crate::glow::PortalGlowMaterial>>),
            );
        }
        if self.body_pieces {
            // The carry. A host with no render stack draws nothing, and
            // carries nothing.
            crate::clip_material::add_portal_clip_material_plugin(app);
            app.init_resource::<crate::PortalCarryAtlas>();
            app.init_resource::<crate::PortalCarrySelection>();
            app.init_resource::<crate::CarryQuadPool>();
            app.add_systems(First, crate::carry::give_back_carried_layers);
            // Before the assets' events, the cameras and the propagation: see
            // `carry`'s module docs.
            app.add_systems(
                PostUpdate,
                crate::carry::carry_through_pairs
                    .run_if(resource_exists::<Assets<Image>>)
                    .before(bevy::asset::AssetEventSystems)
                    .before(bevy::camera::CameraUpdateSystems)
                    .before(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate)
                    .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
            );
        }
        if self.far_side_compositing {
            // Shares the clip material with the transit pieces; the adder is
            // idempotent precisely because both flags default to on.
            crate::clip_material::add_portal_clip_material_plugin(app);
            app.add_systems(
                Update,
                far_side::composite_far_side_bodies.in_set(PortalPresentationSet),
            );
        }
        // The one writer of a portal-presented body's `Visibility`, after every
        // system that states a reason to hide it. Registered unconditionally: a
        // body whose last hide reason went away must still be given back. With
        // both flags off, the query matches nothing. `.after` on an unregistered
        // system constrains nothing, which is correct here.
        app.add_systems(
            Update,
            source_visibility::resolve_portal_source_visibility
                .in_set(PortalPresentationSet)
                .after(far_side::composite_far_side_bodies),
        );
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
            app.add_message::<ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction>();
            app.init_resource::<view_cones::PortalViewConeConfig>();
            app.init_resource::<view_cones::PortalViewConeRule>();
            app.init_resource::<view_cones::PortalCaptureQualityBudget>();
            app.init_resource::<view_cones::PortalViewConeDebugDumpRequest>();
            app.init_resource::<PortalDebugOverlay>();
            app.add_systems(
                Update,
                (
                    // The three debug passes run only on their trigger: a
                    // developer action waits, the F1 overlay is on, a dump is
                    // pending. Each returned at once otherwise.
                    view_cones::handle_portal_view_cone_dump_hotkey.run_if(
                        ambition_platformer2d_core::a_message_waits::<
                            ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction,
                        >,
                    ),
                    // Guard all three asset params: Bevy 0.19 panics on a missing
                    // system param, and headless compositions have no render stack.
                    // Per `engine/headless-verification.md`, guard with `run_if`
                    // instead of registering render assets in headless apps (same
                    // pattern as `avatar::trail.rs`). Chained `run_if`s are ANDed.
                    view_cones::sync_portal_view_cones
                        .run_if(resource_exists::<Assets<Image>>)
                        .run_if(resource_exists::<Assets<Mesh>>)
                        .run_if(resource_exists::<Assets<ColorMaterial>>),
                    // Same guard: `Gizmos` requires `GizmoConfigStore`.
                    view_cones::debug_portal_view_zones
                        .run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>)
                        .run_if(|debug: Res<view_cones::PortalDebugOverlay>| debug.enabled),
                    view_cones::flush_portal_view_cone_debug_dump
                        .run_if(|request: Res<view_cones::PortalViewConeDebugDumpRequest>| request.pending),
                )
                    .chain()
                    .in_set(PortalPresentationSet),
            );
        }
    }
}
