//! Player-following camera with smooth zoom in/out around encounter
//! transitions and an overview-camera dev mode.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::primitives::PlayerVisual;
use crate::config::world_to_bevy;
use crate::persistence::settings::CameraAspectPolicy;
use crate::rooms::{CameraClampMode, CameraZoneSpec, RoomSet};

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
        Self {
            base_view: ae::Vec2::new(800.0, 450.0),
            requested_view: ae::Vec2::new(800.0, 450.0),
            visible_view: ae::Vec2::new(800.0, 450.0),
            zoom_multiplier: 1.0,
            orthographic_scale: 1.0,
            target_world: ae::Vec2::ZERO,
            center_world: ae::Vec2::ZERO,
            active_camera_zones: 0,
            active_camera_zone: None,
        }
    }
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
/// `With<crate::player::PrimaryPlayer>` once a second player can
/// exist. See [`crate::player::queries::PrimaryPlayerOnly`].
pub fn camera_follow(
    world: Res<crate::GameWorld>,
    room_set: Res<RoomSet>,
    time: Res<Time>,
    developer_tools: Res<crate::dev::dev_tools::DeveloperTools>,
    encounter_registry: Res<crate::encounter::EncounterRegistry>,
    user_settings: Res<crate::persistence::settings::UserSettings>,
    mut camera_state: ResMut<crate::CameraEaseState>,
    mut view_state: ResMut<CameraViewState>,
    ease_tuning: Res<crate::CameraEaseTuning>,
    mut last_camera_room: Local<Option<String>>,
    player: Query<
        (
            &crate::player::PlayerBody,
            &crate::player::PlayerBlinkCameraState,
        ),
        crate::player::PrimaryPlayerOnly,
    >,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut Transform, &mut Projection), (With<Camera>, Without<PlayerVisual>)>,
) {
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let base_view = ae::Vec2::new(base_view_w, base_view_h);

    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    let encounter_scale = encounter_registry.active_camera_zoom().max(1.0);
    let Ok((player_body, blink_cam)) = player.single().map(|(b, bc)| (*b, *bc)) else {
        return;
    };
    let player_aabb = player_body.aabb();
    let active_spec = room_set.active_spec();
    let mut active_camera_zones = 0usize;
    let active_zone = active_spec
        .camera_zones
        .iter()
        .filter(|zone| player_aabb.strict_intersects(zone.aabb))
        .inspect(|_| active_camera_zones += 1)
        .max_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| zone_area(a).total_cmp(&zone_area(b)))
        });
    let camera_zone_scale = active_zone
        .map(CameraZoneSpec::effective_zoom)
        .unwrap_or(1.0);

    let target_scale = if developer_tools.overview_camera {
        overview_scale
    } else {
        encounter_scale.max(camera_zone_scale)
    };

    let room_changed = last_camera_room.as_deref() != Some(active_spec.id.as_str());
    if room_changed {
        *last_camera_room = Some(active_spec.id.clone());
        // Room transitions can connect LDtk areas that are spatially disjoint.
        // Reset the presentation-only camera target immediately so target easing
        // does not interpolate through unrelated world coordinates.
        camera_state.target_initialized = false;
        camera_state.live_scale = target_scale;
    }
    let snap_camera = blink_cam.camera_snap_timer > 0.0 || room_changed;

    // Ease the live scale toward the target. Different rates for
    // zoom-in (encounter starts; tighter, faster — players want
    // immediate "you're in it") vs. zoom-out (encounter ends;
    // slower, breathy "you survived"). Overview camera snaps because
    // it's a debug tool.
    let dt = time.delta_secs().max(0.0);
    let camera_scale = if developer_tools.overview_camera || snap_camera {
        camera_state.live_scale = target_scale;
        target_scale
    } else {
        let rate = if target_scale > camera_state.live_scale {
            ease_tuning.zoom_out_rate
        } else {
            ease_tuning.zoom_in_rate
        };
        let delta = (target_scale - camera_state.live_scale).abs();
        let step = (rate * dt).min(delta);
        camera_state.live_scale = if target_scale > camera_state.live_scale {
            camera_state.live_scale + step
        } else {
            camera_state.live_scale - step
        };
        if (camera_state.live_scale - target_scale).abs() < ease_tuning.snap_epsilon {
            camera_state.live_scale = target_scale;
        }
        camera_state.live_scale.max(1.0)
    };

    let target_view_w = base_view_w * camera_scale;
    let target_view_h = base_view_h * camera_scale;

    let (window_w, window_h) = windows
        .single()
        .map(|w| (w.width().max(1.0), w.height().max(1.0)))
        .unwrap_or((
            crate::config::WINDOW_W as f32,
            crate::config::WINDOW_H as f32,
        ));

    let scale_by_height = target_view_h / window_h;
    let scale_by_width = target_view_w / window_w;
    let orthographic_scale = match user_settings.video.camera_aspect {
        CameraAspectPolicy::FitDesign => scale_by_height.max(scale_by_width),
        CameraAspectPolicy::FixedHeight => scale_by_height,
        CameraAspectPolicy::FixedWidth => scale_by_width,
    };
    let half_view_w = window_w * orthographic_scale * 0.5;
    let half_view_h = window_h * orthographic_scale * 0.5;
    let visible_view = ae::Vec2::new(half_view_w * 2.0, half_view_h * 2.0);

    let (target, target_world) = if developer_tools.overview_camera {
        // Overview still follows the player — the zoom handles the wider view.
        // Previously locked to world center, making large rooms unnavigable in F5 mode.
        let resize_offset = (player_body.base_size.y - player_body.size.y) * 0.5;
        let target_world = ae::Vec2::new(player_body.pos.x, player_body.pos.y - resize_offset);
        camera_state.live_target_world = target_world;
        camera_state.target_initialized = true;
        (world_to_bevy(&world.0, target_world, 0.0), target_world)
    } else {
        // AMBITION_REVIEW(spatial): camera follows a stable "standing-pose center"
        // that doesn't pop when the body resizes. `try_change_body_mode` keeps
        // feet planted by adjusting `pos.y` (+Y down) by half the height delta,
        // so on crouch/morph/slide entry the player's *center* shifts down by
        // `(base_size.y - size.y) * 0.5`. Cancelling that offset here gives the
        // camera a fixed virtual point — entering a slide mid-dash no longer
        // produces a 10px vertical pop.
        let resize_offset = (player_body.base_size.y - player_body.size.y) * 0.5;
        let mut desired_target_world =
            ae::Vec2::new(player_body.pos.x, player_body.pos.y - resize_offset);
        let (bias_x, bias_y) = user_settings.video.camera_framing.target_offset(
            target_view_w,
            target_view_h,
            player_body.facing,
        );
        desired_target_world.x += bias_x;
        desired_target_world.y += bias_y;

        if let Some(zone) = active_zone {
            if zone.cinematic_lock {
                desired_target_world = zone.aabb.center();
            }
            desired_target_world += zone.target_offset;
        }

        if blink_cam.blink_in_timer > 0.0 && blink_cam.blink_in_duration > 0.0 {
            let raw_t =
                1.0 - (blink_cam.blink_in_timer / blink_cam.blink_in_duration).clamp(0.0, 1.0);
            let t = raw_t * raw_t * (3.0 - 2.0 * raw_t);
            desired_target_world = blink_cam.blink_camera_from
                + (desired_target_world - blink_cam.blink_camera_from) * t;
        }

        // Smooth the target itself, not just the zoom. Phase 4 introduced
        // look-ahead framing; without a target ease, flipping facing in open
        // space teleports the camera target by 10-30% of the viewport width.
        // Room-boundary clamping hides that near walls, which made the snap
        // feel inconsistent. Keep this state presentation-only so physics and
        // hit tests remain frame-exact.
        let target_world = if snap_camera || !camera_state.target_initialized {
            camera_state.target_initialized = true;
            camera_state.live_target_world = desired_target_world;
            desired_target_world
        } else {
            let target_ease_hz = active_zone
                .and_then(|zone| zone.easing_hz)
                .unwrap_or(8.0)
                .max(0.0);
            let alpha = (1.0 - (-target_ease_hz * dt).exp()).clamp(0.0, 1.0);
            let previous_target_world = camera_state.live_target_world;
            let eased_target_world =
                previous_target_world + (desired_target_world - previous_target_world) * alpha;
            camera_state.live_target_world = eased_target_world;
            eased_target_world
        };
        (world_to_bevy(&world.0, target_world, 0.0), target_world)
    };

    let bounds = active_zone
        .map(|zone| zone.clamp_mode)
        .unwrap_or(CameraClampMode::RoomBounds);
    let (x, y) = clamp_camera_target(
        &world.0,
        target,
        half_view_w,
        half_view_h,
        bounds,
        active_zone,
    );
    let center_world = ae::Vec2::new(x + world.0.size.x * 0.5, world.0.size.y * 0.5 - y);

    *view_state = CameraViewState {
        base_view,
        requested_view: ae::Vec2::new(target_view_w, target_view_h),
        visible_view,
        zoom_multiplier: camera_scale,
        orthographic_scale,
        target_world,
        center_world,
        active_camera_zones,
        active_camera_zone: active_zone.map(|zone| zone.id.clone()),
    };

    for (mut transform, mut projection) in &mut query {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = orthographic_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
    }
}

fn zone_area(zone: &CameraZoneSpec) -> f32 {
    let half = zone.aabb.half_size();
    (half.x * 2.0).max(0.0) * (half.y * 2.0).max(0.0)
}

fn clamp_camera_target(
    world: &ae::World,
    target: Vec3,
    half_view_w: f32,
    half_view_h: f32,
    mode: CameraClampMode,
    zone: Option<&CameraZoneSpec>,
) -> (f32, f32) {
    match mode {
        CameraClampMode::None => (target.x, target.y),
        CameraClampMode::ZoneBounds => {
            let Some(zone) = zone else {
                return clamp_to_world_bounds(world, target, half_view_w, half_view_h);
            };
            let min_x = zone.aabb.left() + half_view_w - world.size.x * 0.5;
            let max_x = zone.aabb.right() - half_view_w - world.size.x * 0.5;
            let min_y = world.size.y * 0.5 - (zone.aabb.bottom() - half_view_h);
            let max_y = world.size.y * 0.5 - (zone.aabb.top() + half_view_h);
            (
                clamp_or_center(target.x, min_x, max_x),
                clamp_or_center(target.y, min_y, max_y),
            )
        }
        CameraClampMode::RoomBounds => {
            clamp_to_world_bounds(world, target, half_view_w, half_view_h)
        }
    }
}

fn clamp_to_world_bounds(
    world: &ae::World,
    target: Vec3,
    half_view_w: f32,
    half_view_h: f32,
) -> (f32, f32) {
    let min_x = -world.size.x * 0.5 + half_view_w;
    let max_x = world.size.x * 0.5 - half_view_w;
    let min_y = -world.size.y * 0.5 + half_view_h;
    let max_y = world.size.y * 0.5 - half_view_h;
    (
        clamp_or_center(target.x, min_x, max_x),
        clamp_or_center(target.y, min_y, max_y),
    )
}

fn clamp_or_center(value: f32, min: f32, max: f32) -> f32 {
    if min <= max {
        value.clamp(min, max)
    } else {
        (min + max) * 0.5
    }
}
