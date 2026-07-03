//! Player-following camera with smooth zoom in/out around encounter
//! transitions and an overview-camera dev mode.

use ambition_engine_core as ae;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::primitives::PlayerVisual;
use ambition_gameplay_core::camera_snapshot::{
    resolve_follow_camera_snapshot, CameraBlinkInput, CameraFocus2d, CameraSnapshot2d,
    CameraSnapshotResolveInput, CameraSnapshotResolveMode,
};
use ambition_gameplay_core::rooms::RoomSet;

/// Live camera diagnostics and feel-lab data.
///
/// Updated by [`camera_follow`] after the camera target and orthographic scale
/// are resolved. HUD/debug overlays read this so they can show the *actual*
/// gameplay view, not a recomputed approximation that may drift when aspect or
/// encounter policy changes.
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

#[derive(SystemParam)]
pub struct CameraFollowResources<'w> {
    world: Res<'w, ambition_engine_core::RoomGeometry>,
    room_set: Res<'w, RoomSet>,
    time: Res<'w, Time>,
    developer_tools: Res<'w, ambition_gameplay_core::dev::dev_tools::DeveloperTools>,
    encounter_registry: Res<'w, ambition_gameplay_core::encounter::EncounterRegistry>,
    user_settings: Res<'w, ambition_gameplay_core::persistence::settings::UserSettings>,
    camera_state: ResMut<'w, ambition_gameplay_core::CameraEaseState>,
    view_state: ResMut<'w, CameraViewState>,
    ease_tuning: Res<'w, ambition_gameplay_core::CameraEaseTuning>,
    shake: Res<'w, ambition_gameplay_core::time::camera_ease::CameraShakeState>,
}

#[cfg(feature = "portal_render")]
#[derive(SystemParam)]
pub struct PortalCameraContinuityParams<'w> {
    selection: Option<Res<'w, ambition_portal_presentation::PortalCameraContinuitySelection>>,
    state: Option<ResMut<'w, ambition_portal_presentation::PortalCameraContinuityState>>,
    host_view: Option<ResMut<'w, ambition_portal_presentation::PortalCameraContinuityHostView>>,
}

/// Follow the player in rooms larger than the window.
///
/// The simulation uses top-left world coordinates, while Bevy renders around a
/// centered camera. We convert the player to Bevy coordinates, then clamp the
/// camera center so the player can scroll through large rooms without showing
/// outside the generated level bounds. Small rooms remain centered.
///
/// Smoothly eases between camera scales when an encounter starts /
/// ends. A snap was distracting; the eased path preserves "I crossed
/// a threshold and the world breathed out" pacing without making
/// the player wait for the camera.
///
/// **Multiplayer caveat (primary-player-only):** the camera follows
/// the lone player today. A future co-op build needs to follow the
/// player with `PrimaryPlayer` (or compute a midpoint between local
/// players); the query should switch to
/// `With<ambition_platformer_primitives::markers::PrimaryPlayer>` once a second player can
/// exist. See [`ambition_gameplay_core::player::queries::PrimaryPlayerOnly`].
pub fn camera_follow(
    resources: CameraFollowResources,
    #[cfg(feature = "portal_render")] mut portal_continuity: PortalCameraContinuityParams,
    mut last_camera_room: Local<Option<String>>,
    player: Query<
        (
            &ambition_platformer_primitives::body::BodyKinematics,
            &ambition_engine_core::BodyBaseSize,
            &ambition_gameplay_core::player::PlayerBlinkCameraState,
        ),
        ambition_platformer_primitives::markers::PrimaryPlayerOnly,
    >,
    // The camera follows the CONTROLLED SUBJECT — the body carrying
    // `Brain::Player(PRIMARY)`. That's the home avatar normally, or the possessed
    // actor while possessing (so the view follows the body you're driving). Both
    // carry the shared `BodyKinematics`, so one read query serves either.
    controlled: Res<ambition_gameplay_core::abilities::traversal::possession::ControlledSubject>,
    body_kinematics: Query<&ambition_platformer_primitives::body::BodyKinematics>,
    windows: Query<&Window, With<PrimaryWindow>>,
    // `With<MainCamera>` (not the broad `With<Camera2d>`): besides the #31 cube
    // pause-menu Camera3d, the portal view-cone renderer spawns offscreen
    // capture `Camera2d`s. A broad match would drag every capture to the player
    // and overwrite its `Fixed` ortho scale with the main zoom each frame — so
    // each portal window would show "the player area at the current zoom"
    // instead of a fixed slice of its exit. Pinning to the single main game
    // camera keeps follow/zoom off the captures.
    mut query: Query<
        (&mut Transform, &mut Projection),
        (
            With<ambition_gameplay_core::session::camera_layers::MainCamera>,
            Without<PlayerVisual>,
        ),
    >,
) {
    let CameraFollowResources {
        world,
        room_set,
        time,
        developer_tools,
        encounter_registry,
        user_settings,
        mut camera_state,
        mut view_state,
        ease_tuning,
        shake,
    } = resources;

    // DeveloperTools can temporarily replace the authored/default camera view.
    let (base_view_w, base_view_h) = if developer_tools.camera_view_override_enabled {
        (
            developer_tools.camera_view_w.max(64.0),
            developer_tools.camera_view_h.max(64.0),
        )
    } else {
        user_settings.video.camera_zoom.base_view()
    };
    let base_view = ae::Vec2::new(base_view_w, base_view_h);

    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    let encounter_scale = encounter_registry.active_camera_zoom().max(1.0);
    let Ok((mut player_body, player_base_size, blink_cam)) =
        player.single().map(|(b, bs, bc)| (*b, *bs, *bc))
    else {
        return;
    };
    // Follow the controlled subject's body position. Zoom + blink easing stay on
    // the home avatar's presentation state (`player_base_size`/`blink_cam`),
    // which is fine for framing; only the follow point tracks the driven body.
    if let Some(subject) = controlled.0 {
        if let Ok(kin) = body_kinematics.get(subject) {
            player_body.pos = kin.pos;
        }
    }

    let active_spec = room_set.active_spec();
    let room_changed = last_camera_room.as_deref() != Some(active_spec.id.as_str());
    if room_changed {
        *last_camera_room = Some(active_spec.id.clone());
        // Room transitions can connect LDtk areas that are spatially disjoint.
        // Reset the presentation-only camera target immediately so target easing
        // does not interpolate through unrelated world coordinates.
        camera_state.target_initialized = false;
    }
    let snap_camera = blink_cam.camera_snap_timer > 0.0 || room_changed;

    let (window_w, window_h) = windows
        .single()
        .map(|w| (w.width().max(1.0), w.height().max(1.0)))
        .unwrap_or((
            ambition_engine_core::config::WINDOW_W as f32,
            ambition_engine_core::config::WINDOW_H as f32,
        ));

    #[cfg(feature = "portal_render")]
    let (portal_continuity_enabled, portal_clamp_padding_center_world) = {
        let enabled = portal_continuity
            .selection
            .as_deref()
            .is_some_and(|selection| {
                selection.mode == ambition_portal_presentation::PortalCameraTransitMode::Continuous
            });
        let padding = enabled
            .then(|| {
                portal_continuity
                    .state
                    .as_deref()
                    .and_then(|state| state.clamp_padding_center_world)
            })
            .flatten();
        (enabled, padding)
    };
    #[cfg(not(feature = "portal_render"))]
    let portal_clamp_padding_center_world = None;

    let focus = CameraFocus2d {
        center_world: player_body.pos,
        size: player_body.size,
        base_size: player_base_size.base_size,
        facing: player_body.facing,
    };
    let blink = CameraBlinkInput {
        blink_in_timer: blink_cam.blink_in_timer,
        blink_in_duration: blink_cam.blink_in_duration,
        blink_camera_from: blink_cam.blink_camera_from,
    };
    let mut snapshot = resolve_follow_camera_snapshot(
        CameraSnapshotResolveInput {
            world: &world.0,
            camera_zones: &active_spec.camera_zones,
            focus,
            base_view,
            viewport_px: ae::Vec2::new(window_w, window_h),
            aspect_policy: user_settings.video.camera_aspect,
            framing: user_settings.video.camera_framing,
            overview_scale,
            encounter_scale,
            overview_camera: developer_tools.overview_camera,
            snap_camera,
            blink: Some(blink),
            dt: time.delta_secs(),
            mode: CameraSnapshotResolveMode::Eased,
            extra_clamp_center_world: portal_clamp_padding_center_world,
            ease_tuning: *ease_tuning,
        },
        Some(&mut *camera_state),
    );

    #[cfg(feature = "portal_render")]
    let ordinary_center_world = snapshot.center_world;
    #[cfg(feature = "portal_render")]
    let portal_clamp_padding_still_needed =
        (ordinary_center_world - snapshot.unpadded_center_world).length() > 0.5;

    #[cfg(feature = "portal_render")]
    {
        if let Some(portal_state) = portal_continuity.state.as_deref_mut() {
            if portal_continuity_enabled {
                let weight = portal_state.active_weight();
                if weight > 0.0 {
                    let screen_offset = portal_state.body_screen_offset_world.unwrap_or(Vec2::ZERO);
                    snapshot.center_world = player_body.pos - screen_offset;
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
