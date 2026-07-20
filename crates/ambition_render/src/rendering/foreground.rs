//! Camera-relative foreground parallax layers.
//!
//! These layers are generated PNGs from the sprite2d background generator and
//! are intentionally optional. If a generated PNG is absent, no fallback quad is
//! spawned; foreground atmosphere should never become gameplay-critical.

use ambition_engine_core as ae;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::primitives::RoomVisual;
use ambition_engine_core::config::{world_to_bevy, WINDOW_H, WINDOW_W, WORLD_Z_FX};
use ambition_platformer_primitives::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use ambition_sprite_sheet::game_assets::{
    foreground_parallax_factor, foreground_parallax_sprite_for_biome, ForegroundParallaxSprite,
    GameAssets,
};

const FOREGROUND_OVERSCAN: f32 = 1.32;
const FOREGROUND_Z: f32 = WORLD_Z_FX - 0.75;

#[derive(Component, Clone, Debug)]
pub struct ForegroundParallax {
    /// Near-camera drift factor. Values just above 1.0 drift opposite the camera
    /// a little faster than the gameplay world while remaining screen-framed.
    pub factor: f32,
    /// Bevy-space center of the active room. The sync system measures camera
    /// displacement from here so layers are stable when entering a room.
    pub room_center: Vec2,
}

/// Spawn the active room's optional generated foreground layer.
pub fn spawn_room_foreground_parallax(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    spec: &ambition_world::rooms::RoomSpec,
    assets: Option<&GameAssets>,
) {
    let Some(assets) = assets else {
        return;
    };
    let world = &spec.world;
    let metadata = &spec.metadata;
    let has_boss_spawn = !spec.boss_spawns.is_empty();
    let sprite_key = if has_boss_spawn {
        ForegroundParallaxSprite::Boss
    } else {
        foreground_parallax_sprite_for_biome(metadata.biome.as_deref())
    };
    let Some(handle) = assets.foregrounds.get(sprite_key).cloned() else {
        return;
    };

    let room_center = world_to_bevy(world, world.size * 0.5, FOREGROUND_Z).truncate();
    let mut sprite = Sprite::from_image(handle);
    // Startup fallback; the per-frame sync replaces this with the real window
    // size and camera scale. Keep it larger than the default window so the first
    // visible frame does not flash uncovered corners.
    sprite.custom_size = Some(Vec2::new(WINDOW_W as f32, WINDOW_H as f32) * FOREGROUND_OVERSCAN);

    commands.spawn_session_scoped(
        session_scope,
        (
            sprite,
            Transform::from_xyz(room_center.x, room_center.y, FOREGROUND_Z),
            ForegroundParallax {
                factor: foreground_parallax_factor(sprite_key),
                room_center,
            },
            RoomVisual,
            Name::new(format!("Foreground parallax: {:?}", sprite_key)),
        ),
    );
}

/// Keep foreground layers viewport-sized and apply a subtle near-camera drift.
///
/// The texture stays centered on the camera, but its screen-space offset is
/// `(1 - factor) * camera_delta_from_room_center`. For a factor of 1.10, a
/// 1000px camera pan moves the foreground edge art about 100px across the
/// screen: enough to imply depth without creating a readable gameplay layer.
pub fn sync_foreground_parallax(
    // The camera's OWN visible world extent, not the window: under a
    // fixed-aspect presentation profile the main camera covers the gameplay
    // rectangle only, and sizing this art from the window would overscan it by
    // the pillarbox ratio.
    view_state: Res<super::camera::CameraViewState>,
    // `With<MainCamera>`: ignore the #31 cube overlay Camera3d AND the portal
    // view-cone capture `Camera2d`s, so `.single()` still resolves the one main
    // game camera (a broad `With<Camera2d>` now matches the captures too).
    camera: Query<
        &Transform,
        (
            With<ambition_platformer_primitives::camera_layers::MainCamera>,
            Without<ForegroundParallax>,
        ),
    >,
    mut layers: Query<(&ForegroundParallax, &mut Transform, &mut Sprite)>,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let visible_size = view_state.visible_view * FOREGROUND_OVERSCAN;
    let camera_xy = camera_transform.translation.truncate();

    for (parallax, mut transform, mut sprite) in &mut layers {
        let camera_delta = camera_xy - parallax.room_center;
        let screen_offset = camera_delta * (1.0 - parallax.factor);
        transform.translation.x = camera_xy.x + screen_offset.x;
        transform.translation.y = camera_xy.y + screen_offset.y;
        transform.translation.z = FOREGROUND_Z;
        sprite.custom_size = Some(visible_size);
    }
}
