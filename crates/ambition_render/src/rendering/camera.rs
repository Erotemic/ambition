//! Presentation half of the follow camera.
//!
//! The RESOLVE — zoom policy, camera zones, target easing, blink
//! interpolation, clamping (the `CameraEaseState` write) — belongs to the
//! observation seam
//! ([`ambition_sim_view::camera_snapshot::CameraObservationPlugin`], E4-17),
//! which publishes one [`ResolvedCameraSnapshot`] per rendered FRAME. This
//! module only (a) applies presentation-only deltas — portal camera
//! continuity, shake — to a COPY of the snapshot, and (b) writes the Bevy
//! camera transform/projection. Render never mutates sim camera state.
//!
//! Frame, not tick: the simulation produces authoritative world facts, and
//! where the camera looks at them is presentation. Fixed-tick and GGRS hosts
//! advance the sim off the render clock — GGRS resimulates the same ticks
//! during rollback — and none of that independently advances camera easing.
//!
//! The observer facts the resolver consumes (`CameraViewport`,
//! `CameraScreenFraming`) are published by
//! `ambition_platformer2d_host::gameplay_presentation`: they are answers about the physical
//! display and the active presentation profile, and render does not select
//! policy.

use ambition_platformer2d_core as ae;
#[cfg(feature = "portal_render")]
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::primitives::PlayerVisual;
use ambition_sim_view::camera_snapshot::{
    CameraPresentationInputs, CameraSnapshot2d, ResolvedCameraSnapshot,
};
// Only the portal publisher mints a chart transit; without that feature the
// import is dead and `-D warnings` compositions say so.
#[cfg(feature = "portal_render")]
use ambition_sim_view::camera_snapshot::CameraChartTransit;
use ambition_sim_view::LocalView;

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

#[cfg(feature = "portal_render")]
#[derive(SystemParam)]
pub struct PortalCameraContinuityParams<'w> {
    selection: Option<Res<'w, ambition_portal2d_presentation::PortalCameraContinuitySelection>>,
    state: Option<ResMut<'w, ambition_portal2d_presentation::PortalCameraContinuityState>>,
    host_view: Option<ResMut<'w, ambition_portal2d_presentation::PortalCameraContinuityHostView>>,
}

/// Bridge the portal-continuity facts the RESOLVER needs — the clamp pad and
/// the chart rotation — into its generic inputs BEFORE this tick's resolve.
///
/// Same-frame, like the old inline read: a post-resolve copy would lag the pad
/// one frame and visibly step the camera at transit clear.
///
/// ⭐ **the roll moved here from `camera_follow`, and that is the point.** It
/// used to be written onto the snapshot AFTER resolution, so the resolver
/// clamped an axis-aligned footprint for a view it did not know was rolled. A
/// rotation-aware clamp is only expressible once the roll is an input, and the
/// composition rule ([`presented_roll_radians`]) can only be applied where the
/// base observer roll is also known.
///
/// ⚠ **`observer_roll_at_entry` is latched on the RISING EDGE from the previous
/// frame's resolved roll.** That frame had no transit, so its roll is the pure
/// base — the roll this view had adopted before the crossing began, which is
/// exactly what the composition needs to avoid double-counting a frame change
/// the portal itself caused.
#[cfg(feature = "portal_render")]
pub fn publish_portal_camera_clamp(
    selection: Option<Res<ambition_portal2d_presentation::PortalCameraContinuitySelection>>,
    state: Option<Res<ambition_portal2d_presentation::PortalCameraContinuityState>>,
    // One row per local view. Ambition has one; the portal's facts are a fact
    // about the world's geometry, so every view presenting that world is told.
    mut views: Query<(&ResolvedCameraSnapshot, &mut CameraPresentationInputs), With<LocalView>>,
) {
    let enabled = selection.as_deref().is_some_and(|selection| {
        selection.mode == ambition_portal2d_presentation::PortalCameraTransitMode::Continuous
    });
    let clamp_center = enabled
        .then(|| state.as_deref().and_then(|s| s.clamp_padding_center_world))
        .flatten();
    let crossing = enabled
        .then(|| {
            state
                .as_deref()
                .filter(|s| s.active_weight() > 0.0)
                .map(|s| s.roll_radians)
        })
        .flatten();
    for (resolved, mut presentation) in &mut views {
        presentation.extra_clamp_center_world = clamp_center;
        presentation.chart_transit = crossing.map(|chart_roll_radians| CameraChartTransit {
            chart_roll_radians,
            observer_roll_at_entry: match presentation.chart_transit {
                // Already crossing: keep the roll adopted when it began.
                Some(active) => active.observer_roll_at_entry,
                // Rising edge: THIS view's last resolved roll is its base roll.
                None => resolved.snapshot.rotation_radians,
            },
        });
    }
}

/// Apply the sim-resolved camera snapshot to the main camera, layering the
/// presentation-only deltas (portal camera continuity, shake) onto a COPY.
pub fn camera_follow(
    // **THE VIEW whose snapshot this camera presents.** One local view today, so
    // the single main camera presents it; pairing N views with N cameras is M2's
    // slice and wants a link component, not a broader query here.
    view: Single<(&ResolvedCameraSnapshot, &mut CameraPresentationInputs), With<LocalView>>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    mut view_state: ResMut<CameraViewState>,
    shake: Res<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeState>,
    #[cfg(feature = "portal_render")] mut portal_continuity: PortalCameraContinuityParams,
    // `With<MainCamera>` (not the broad `With<Camera2d>`): besides the #31 cube
    // pause-menu Camera3d, the portal view-cone renderer spawns offscreen
    // capture `Camera2d`s. A broad match would drag every capture to the player
    // and overwrite its `Fixed` ortho scale with the main zoom each frame.
    mut query: Query<
        (&mut Transform, &mut Projection),
        (
            With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
            Without<PlayerVisual>,
        ),
    >,
) {
    let (resolved, mut presentation) = view.into_inner();
    // Presentation deltas apply to a COPY — the sim's resolved snapshot is
    // read-only here.
    #[cfg_attr(not(feature = "portal_render"), allow(unused_mut))]
    let mut snapshot = resolved.snapshot.clone();
    #[cfg(feature = "portal_render")]
    let follow_world = resolved.follow_world;

    #[cfg(not(feature = "portal_render"))]
    {
        // Without portal continuity nothing writes these; keep them cleared so a
        // stale pad or roll can't linger across feature configs.
        *presentation = CameraPresentationInputs::default();
    }
    #[cfg(feature = "portal_render")]
    let _ = &mut presentation; // written pre-resolve by publish_portal_camera_clamp

    #[cfg(feature = "portal_render")]
    {
        let portal_continuity_enabled =
            portal_continuity
                .selection
                .as_deref()
                .is_some_and(|selection| {
                    selection.mode
                        == ambition_portal2d_presentation::PortalCameraTransitMode::Continuous
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
                // ⛔ **the roll is NOT applied here any more.** It used to be
                // `snapshot.rotation_radians = portal_state.roll_radians`, an
                // overwrite of a value the resolver had just computed — which
                // meant the resolver clamped for an orientation it never saw,
                // and a base observer roll had nowhere to compose with this one.
                // `publish_portal_camera_clamp` now hands both facts to the
                // resolve, and `snapshot.rotation_radians` already carries the
                // composed answer by the time this runs.
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
