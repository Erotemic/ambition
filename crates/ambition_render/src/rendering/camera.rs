//! Presentation half of the follow camera.
//!
//! The RESOLVE — zoom policy, camera zones, target easing, blink
//! interpolation, clamping (the `CameraEaseState` write) — is the SIM's
//! observation seam now
//! ([`ambition_gameplay_core::camera_snapshot::CameraObservationPlugin`],
//! E4-17): the sim publishes one [`ResolvedCameraSnapshot`] per tick. This
//! module only (a) publishes the physical viewport (an observer fact the
//! resolver consumes), (b) applies presentation-only deltas — portal camera
//! continuity, shake — to a COPY of the snapshot, and (c) writes the Bevy
//! camera transform/projection. Render never mutates sim camera state.

use ambition_engine_core as ae;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::primitives::PlayerVisual;
use ambition_gameplay_core::camera_snapshot::{
    CameraExtraClamp, CameraSnapshot2d, CameraViewport, ResolvedCameraSnapshot,
};

/// Live camera diagnostics and feel-lab data.
///
/// Updated by [`camera_follow`] after the presentation deltas are applied.
/// HUD/debug overlays read this so they can show the *actual* gameplay view,
/// not a recomputed approximation that may drift when aspect or encounter
/// policy changes.
#[derive(Resource, Clone, Debug)]
#[allow(dead_code)] // base_view + orthographic_scale are exposed for HUD/debug overlays.
pub struct CameraViewState {
    pub base_view: ae::Vec2,
    pub requested_view: ae::Vec2,
    pub visible_view: ae::Vec2,
    pub zoom_multiplier: f32,
    pub orthographic_scale: f32,
    pub target_world: ae::Vec2,
    pub center_world: ae::Vec2,
    pub active_camera_zones: usize,
    pub active_camera_zone: Option<String>,
}

impl Default for CameraViewState {
    fn default() -> Self {
        Self::from(&CameraSnapshot2d::default())
    }
}

impl From<&CameraSnapshot2d> for CameraViewState {
    fn from(snapshot: &CameraSnapshot2d) -> Self {
        Self {
            base_view: snapshot.base_view,
            requested_view: snapshot.requested_view,
            visible_view: snapshot.visible_view,
            zoom_multiplier: snapshot.zoom_multiplier,
            orthographic_scale: snapshot.orthographic_scale,
            target_world: snapshot.target_world,
            center_world: snapshot.center_world,
            active_camera_zones: snapshot.active_camera_zones,
            active_camera_zone: snapshot.active_camera_zone.clone(),
        }
    }
}

/// Publish the physical window size into the sim's [`CameraViewport`]
/// observer fact. Runs before the sim's `PresentationSync` resolve so the
/// snapshot uses THIS frame's viewport. Headless apps never run this — the
/// resolver keeps the design-window default.
pub fn publish_camera_viewport(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut viewport: ResMut<CameraViewport>,
) {
    if let Ok(w) = windows.single() {
        viewport.px = ae::Vec2::new(w.width().max(1.0), w.height().max(1.0));
    }
}

#[cfg(feature = "portal_render")]
#[derive(SystemParam)]
pub struct PortalCameraContinuityParams<'w> {
    selection: Option<Res<'w, ambition_portal_presentation::PortalCameraContinuitySelection>>,
    state: Option<ResMut<'w, ambition_portal_presentation::PortalCameraContinuityState>>,
    host_view: Option<ResMut<'w, ambition_portal_presentation::PortalCameraContinuityHostView>>,
}

/// Bridge the portal-continuity clamp pad into the sim resolver's generic
/// [`CameraExtraClamp`] input BEFORE this tick's resolve — same-frame, like
/// the old inline read (a post-resolve copy would lag the pad one frame and
/// visibly step the camera at transit clear).
#[cfg(feature = "portal_render")]
pub fn publish_portal_camera_clamp(
    selection: Option<Res<ambition_portal_presentation::PortalCameraContinuitySelection>>,
    state: Option<Res<ambition_portal_presentation::PortalCameraContinuityState>>,
    mut extra_clamp: ResMut<CameraExtraClamp>,
) {
    let enabled = selection.as_deref().is_some_and(|selection| {
        selection.mode == ambition_portal_presentation::PortalCameraTransitMode::Continuous
    });
    extra_clamp.0 = enabled
        .then(|| state.as_deref().and_then(|s| s.clamp_padding_center_world))
        .flatten();
}

/// Apply the sim-resolved camera snapshot to the main camera, layering the
/// presentation-only deltas (portal camera continuity, shake) onto a COPY.
pub fn camera_follow(
    resolved: Res<ResolvedCameraSnapshot>,
    world: Res<ambition_engine_core::RoomGeometry>,
    mut view_state: ResMut<CameraViewState>,
    shake: Res<ambition_gameplay_core::time::camera_ease::CameraShakeState>,
    mut extra_clamp: ResMut<CameraExtraClamp>,
    #[cfg(feature = "portal_render")] mut portal_continuity: PortalCameraContinuityParams,
    // `With<MainCamera>` (not the broad `With<Camera2d>`): besides the #31 cube
    // pause-menu Camera3d, the portal view-cone renderer spawns offscreen
    // capture `Camera2d`s. A broad match would drag every capture to the player
    // and overwrite its `Fixed` ortho scale with the main zoom each frame.
    mut query: Query<
        (&mut Transform, &mut Projection),
        (
            With<ambition_gameplay_core::session::camera_layers::MainCamera>,
            Without<PlayerVisual>,
        ),
    >,
) {
    // Presentation deltas apply to a COPY — the sim's resolved snapshot is
    // read-only here.
    #[cfg_attr(not(feature = "portal_render"), allow(unused_mut))]
    let mut snapshot = resolved.snapshot.clone();
    let follow_world = resolved.follow_world;

    #[cfg(not(feature = "portal_render"))]
    {
        // Without portal continuity nothing writes the extra clamp; keep it
        // cleared so a stale pad can't linger across feature configs.
        extra_clamp.0 = None;
    }
    #[cfg(feature = "portal_render")]
    let _ = &mut extra_clamp; // written pre-resolve by publish_portal_camera_clamp

    #[cfg(feature = "portal_render")]
    {
        let portal_continuity_enabled =
            portal_continuity
                .selection
                .as_deref()
                .is_some_and(|selection| {
                    selection.mode
                        == ambition_portal_presentation::PortalCameraTransitMode::Continuous
                });
        let ordinary_center_world = snapshot.center_world;
        let portal_clamp_padding_still_needed =
            (ordinary_center_world - snapshot.unpadded_center_world).length() > 0.5;

        if let Some(portal_state) = portal_continuity.state.as_deref_mut() {
            if portal_continuity_enabled {
                let weight = portal_state.active_weight();
                if weight > 0.0 {
                    let screen_offset = portal_state.body_screen_offset_world.unwrap_or(Vec2::ZERO);
                    snapshot.center_world = follow_world - screen_offset;
                    portal_state.target_camera_world = Some(snapshot.center_world);
                } else if !portal_clamp_padding_still_needed {
                    portal_state.clear_clamp_padding();
                }
                snapshot.rotation_radians = portal_state.roll_radians;
            } else {
                portal_state.clear();
            }
        }
        if let Some(mut host_view) = portal_continuity.host_view {
            host_view.capture(
                snapshot.center_world,
                ordinary_center_world,
                snapshot.target_world,
                snapshot.visible_view,
                snapshot.active_camera_zones,
                snapshot.active_camera_zone.clone(),
            );
        }
        if let Some(portal_state) = portal_continuity.state.as_deref_mut() {
            portal_state.last_host_camera_world = Some(snapshot.center_world);
        }
    }

    let x = snapshot.center_world.x - world.0.size.x * 0.5;
    let y = world.0.size.y * 0.5 - snapshot.center_world.y;

    *view_state = CameraViewState::from(&snapshot);

    let shake_offset = shake.offset();
    for (mut transform, mut projection) in &mut query {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = snapshot.orthographic_scale;
        }
        transform.translation.x = x + shake_offset.x;
        transform.translation.y = y + shake_offset.y;
        transform.rotation = Quat::from_rotation_z(snapshot.rotation_radians);
    }
}
