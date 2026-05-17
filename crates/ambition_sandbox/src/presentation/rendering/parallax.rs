//! Room-scoped generated background/parallax spawning and camera-relative motion.
//!
//! Crate choice note: this stays local instead of pulling in a parallax plugin.
//! The sandbox already owns camera follow / room transitions, and generated
//! background assets are optional. A few small components keep the current
//! fallback-friendly loading behavior without forcing the room renderer through
//! an external API.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::RoomVisual;
use crate::config::{WINDOW_H, WINDOW_W};
use crate::game_assets::{GameAssets, ParallaxLayerAsset, ParallaxTheme};
use crate::rooms::RoomMetadata;

#[derive(Component, Clone, Copy, Debug)]
pub struct ParallaxLayerVisual {
    /// 0.0 is screen locked; 1.0 tracks gameplay/world motion.
    pub factor: Vec2,
    pub z: f32,
    /// Screen-space room-relative travel budget. We avoid tile repetition by
    /// keeping each layer as a single large panel and shifting it within this
    /// budget based on camera position inside the room.
    pub travel: Vec2,
    pub world_size: Vec2,
}

#[derive(Clone, Copy)]
struct RuntimeParallaxLayerSpec {
    asset: ParallaxLayerAsset,
    factor: f32,
    z: f32,
    panel_scale: f32,
}

const RUNTIME_PARALLAX_LAYERS: &[RuntimeParallaxLayerSpec] = &[
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::Sky,
        factor: 0.10,
        z: -18.0,
        panel_scale: 1.20,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::FarBackplate,
        factor: 0.20,
        z: -17.0,
        panel_scale: 1.34,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::NearBackground,
        factor: 0.42,
        z: -16.0,
        panel_scale: 1.52,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::ForegroundAtmosphere,
        factor: 0.60,
        z: -15.0,
        panel_scale: 1.72,
    },
];

pub fn spawn_parallax_layers(
    commands: &mut Commands,
    world: &ae::World,
    metadata: &RoomMetadata,
    assets: Option<&GameAssets>,
) {
    let Some(assets) = assets else {
        return;
    };
    if assets.parallax_layers.is_empty() {
        return;
    }
    let theme = ParallaxTheme::from_room_metadata(metadata);
    let viewport = BVec2::new(WINDOW_W as f32, WINDOW_H as f32);
    let panel_base = viewport.x.max(viewport.y);
    for spec in RUNTIME_PARALLAX_LAYERS {
        let Some(image) = assets.parallax_layers.get(theme, spec.asset) else {
            continue;
        };
        let panel_extent = panel_base * spec.panel_scale;
        let panel_size = BVec2::splat(panel_extent);
        let travel = ((panel_size - viewport) * 0.5).max(BVec2::ZERO);
        let mut sprite = Sprite::from_image(image.clone());
        sprite.custom_size = Some(panel_size);
        commands.spawn((
            sprite,
            Transform::from_translation(Vec3::new(0.0, 0.0, spec.z)),
            ParallaxLayerVisual {
                factor: Vec2::splat(spec.factor),
                z: spec.z,
                travel: Vec2::new(travel.x, travel.y),
                world_size: Vec2::new(world.size.x.max(1.0), world.size.y.max(1.0)),
            },
            RoomVisual,
            Name::new(format!(
                "Background parallax layer: {} {}",
                theme.key(),
                spec.asset.key()
            )),
        ));
    }
}

pub fn sync_parallax_layers(
    camera: Query<&Transform, (With<Camera>, Without<ParallaxLayerVisual>)>,
    mut layers: Query<(&mut Transform, &ParallaxLayerVisual), Without<Camera>>,
) {
    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let camera_xy = camera_transform.translation.truncate();
    for (mut transform, layer) in &mut layers {
        let tx = if layer.world_size.x > 1.0 {
            (camera_xy.x / layer.world_size.x).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let ty = if layer.world_size.y > 1.0 {
            (camera_xy.y / layer.world_size.y).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let centered = Vec2::new(tx * 2.0 - 1.0, ty * 2.0 - 1.0);
        let offset = Vec2::new(
            -centered.x * layer.travel.x * layer.factor.x,
            -centered.y * layer.travel.y * layer.factor.y,
        );
        transform.translation.x = camera_xy.x + offset.x;
        transform.translation.y = camera_xy.y + offset.y;
        transform.translation.z = layer.z;
    }
}
