//! Player-following camera with smooth zoom in/out around encounter
//! transitions and an overview-camera dev mode.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::primitives::PlayerVisual;
use crate::config::world_to_bevy;
use crate::settings::CameraAspectPolicy;

/// Live camera diagnostics and feel-lab data.
///
/// Updated by [`camera_follow`] after the camera target and orthographic scale
/// are resolved. HUD/debug overlays read this so they can show the *actual*
/// gameplay view, not a recomputed approximation that may drift when aspect or
/// encounter policy changes.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CameraViewState {
    pub base_view: ae::Vec2,
    pub requested_view: ae::Vec2,
    pub visible_view: ae::Vec2,
    pub zoom_multiplier: f32,
    pub orthographic_scale: f32,
    pub target_world: ae::Vec2,
    pub center_world: ae::Vec2,
    pub active_camera_zones: usize,
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
pub fn camera_follow(
    world: Res<crate::GameWorld>,
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    developer_tools: Res<crate::dev_tools::DeveloperTools>,
    encounter_registry: Res<crate::encounter::EncounterRegistry>,
    ldtk_spine_index: Res<crate::ldtk_world::LdtkRuntimeSpineIndex>,
    user_settings: Res<crate::settings::UserSettings>,
    mut camera_state: ResMut<crate::CameraEaseState>,
    mut view_state: ResMut<CameraViewState>,
    ease_tuning: Res<crate::CameraEaseTuning>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut Transform, &mut Projection), (With<Camera>, Without<PlayerVisual>)>,
) {
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let base_view = ae::Vec2::new(base_view_w, base_view_h);

    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    let encounter_scale = encounter_registry.active_camera_zoom().max(1.0);
    let player_body = runtime.player.aabb();
    let active_camera_zones = ldtk_spine_index
        .entities
        .iter()
        .filter(|entity| entity.role == crate::ldtk_world::LdtkRuntimeRole::CameraZone)
        .filter(|entity| player_body.strict_intersects(entity.aabb()))
        .count();
    // CameraZone support is intentionally conservative in this first pass:
    // without authored fields in the runtime spine, a CameraZone just asks for
    // a modest arena-style breath-out. EncounterTrigger.camera_zoom still owns
    // encounter/boss-specific framing when present.
    let camera_zone_scale = if active_camera_zones > 0 { 1.15 } else { 1.0 };

    let target_scale = if developer_tools.overview_camera {
        overview_scale
    } else {
        encounter_scale.max(camera_zone_scale)
    };

    // Ease the live scale toward the target. Different rates for
    // zoom-in (encounter starts; tighter, faster — players want
    // immediate "you're in it") vs. zoom-out (encounter ends;
    // slower, breathy "you survived"). Overview camera snaps because
    // it's a debug tool.
    let dt = time.delta_secs().max(0.0);
    let camera_scale = if developer_tools.overview_camera {
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
        camera_state.target_initialized = false;
        let target_world = world.0.size * 0.5;
        (world_to_bevy(&world.0, target_world, 0.0), target_world)
    } else {
        // AMBITION_REVIEW(spatial): camera follows a stable "standing-pose center"
        // that doesn't pop when the body resizes. `try_change_body_mode` keeps
        // feet planted by adjusting `pos.y` (+Y down) by half the height delta,
        // so on crouch/morph/slide entry the player's *center* shifts down by
        // `(base_size.y - size.y) * 0.5`. Cancelling that offset here gives the
        // camera a fixed virtual point — entering a slide mid-dash no longer
        // produces a 10px vertical pop.
        let resize_offset = (runtime.player.base_size.y - runtime.player.size.y) * 0.5;
        let mut desired_target_world =
            ae::Vec2::new(runtime.player.pos.x, runtime.player.pos.y - resize_offset);
        let (bias_x, bias_y) = user_settings.video.camera_framing.target_offset(
            target_view_w,
            target_view_h,
            runtime.player.facing,
        );
        desired_target_world.x += bias_x;
        desired_target_world.y += bias_y;

        // Smooth the target itself, not just the zoom. Phase 4 introduced
        // look-ahead framing; without a target ease, flipping facing in open
        // space teleports the camera target by 10-30% of the viewport width.
        // Room-boundary clamping hides that near walls, which made the snap
        // feel inconsistent. Keep this state presentation-only so physics and
        // hit tests remain frame-exact.
        let target_world = if !camera_state.target_initialized {
            camera_state.target_initialized = true;
            camera_state.live_target_world = desired_target_world;
            desired_target_world
        } else {
            let target_ease_hz = 8.0;
            let alpha = (1.0 - (-target_ease_hz * dt).exp()).clamp(0.0, 1.0);
            let previous_target_world = camera_state.live_target_world;
            let eased_target_world =
                previous_target_world + (desired_target_world - previous_target_world) * alpha;
            camera_state.live_target_world = eased_target_world;
            eased_target_world
        };
        (world_to_bevy(&world.0, target_world, 0.0), target_world)
    };

    let min_x = -world.0.size.x * 0.5 + half_view_w;
    let max_x = world.0.size.x * 0.5 - half_view_w;
    let min_y = -world.0.size.y * 0.5 + half_view_h;
    let max_y = world.0.size.y * 0.5 - half_view_h;

    let x = if min_x <= max_x {
        target.x.clamp(min_x, max_x)
    } else {
        0.0
    };
    let y = if min_y <= max_y {
        target.y.clamp(min_y, max_y)
    } else {
        0.0
    };
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
    };

    for (mut transform, mut projection) in &mut query {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = orthographic_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
    }
}
