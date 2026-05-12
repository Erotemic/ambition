//! Player-following camera with smooth zoom in/out around encounter
//! transitions and an overview-camera dev mode.

use ambition_engine as ae;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::primitives::PlayerVisual;
use crate::config::world_to_bevy;

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
    user_settings: Res<crate::settings::UserSettings>,
    mut camera_state: ResMut<crate::CameraEaseState>,
    ease_tuning: Res<crate::CameraEaseTuning>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut query: Query<(&mut Transform, &mut Projection), (With<Camera>, Without<PlayerVisual>)>,
) {
    let overview_scale = developer_tools.overview_camera_scale.max(1.0);
    // Encounter scale: when an encounter is in Active phase, zoom out
    // by the spec's `camera_zoom` factor. Overview camera trumps
    // encounter zoom for dev convenience.
    let encounter_scale = encounter_registry.active_camera_zoom().max(1.0);
    let target_scale = if developer_tools.overview_camera {
        overview_scale
    } else {
        encounter_scale
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
        // Snap the last sliver to avoid floating-point drift
        // accumulating into never-converges territory.
        if (camera_state.live_scale - target_scale).abs() < ease_tuning.snap_epsilon {
            camera_state.live_scale = target_scale;
        }
        camera_state.live_scale.max(1.0)
    };

    let target = if developer_tools.overview_camera {
        // AMBITION_REVIEW(spatial): overview centers the composed active area, not
        // individual LDtk chunks. If active areas become sparse, switch this from
        // bounding-box center to a validated camera overview region.
        world_to_bevy(&world.0, world.0.size * 0.5, 0.0)
    } else {
        // AMBITION_REVIEW(spatial): camera follows a stable "standing-pose center"
        // that doesn't pop when the body resizes. `try_change_body_mode` keeps
        // feet planted by adjusting `pos.y` (+Y down) by half the height delta,
        // so on crouch/morph/slide entry the player's *center* shifts down by
        // `(base_size.y - size.y) * 0.5`. Cancelling that offset here gives the
        // camera a fixed virtual point — entering a slide mid-dash no longer
        // produces a 10px vertical pop.
        let resize_offset = (runtime.player.base_size.y - runtime.player.size.y) * 0.5;
        let camera_target_world =
            ae::Vec2::new(runtime.player.pos.x, runtime.player.pos.y - resize_offset);
        world_to_bevy(&world.0, camera_target_world, 0.0)
    };

    // Fixed gameplay viewport: larger desktop windows should not reveal an
    // accidentally larger slice of the level. The user-selected viewport
    // profile defines the base world-space view; encounter zoom multiplies
    // that profile. The orthographic scale is then derived from the actual
    // window dimensions. Wider-than-design aspect ratios can still reveal
    // extra horizontal margin for now; strict letterboxing/safe-rect support is
    // a later camera policy pass.
    let (window_w, window_h) = windows
        .single()
        .map(|w| (w.width().max(1.0), w.height().max(1.0)))
        .unwrap_or((
            crate::config::WINDOW_W as f32,
            crate::config::WINDOW_H as f32,
        ));
    let (base_view_w, base_view_h) = user_settings.video.camera_zoom.base_view();
    let target_view_w = base_view_w * camera_scale;
    let target_view_h = base_view_h * camera_scale;
    let orthographic_scale = (target_view_h / window_h).max(target_view_w / window_w);
    let half_view_w = window_w * orthographic_scale * 0.5;
    let half_view_h = window_h * orthographic_scale * 0.5;
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

    for (mut transform, mut projection) in &mut query {
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scale = orthographic_scale;
        }
        transform.translation.x = x;
        transform.translation.y = y;
    }
}
