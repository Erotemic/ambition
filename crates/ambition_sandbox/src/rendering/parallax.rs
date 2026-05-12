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
}

#[derive(Clone, Copy)]
struct RuntimeParallaxLayerSpec {
    asset: ParallaxLayerAsset,
    factor: f32,
    z: f32,
    /// Repeat the transparent atmosphere plates so their edge-biased motifs
    /// frame the current camera view. Keep the sky untiled so suns/moons/stars
    /// stay singular.
    tiled: bool,
}

const RUNTIME_PARALLAX_LAYERS: &[RuntimeParallaxLayerSpec] = &[
    // Keep all generated layers in front of the debug grid so an opaque sky
    // actually replaces it visually, while still staying far behind gameplay.
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::Sky,
        factor: 0.08,
        z: -18.0,
        tiled: false,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::FarBackplate,
        factor: 0.18,
        z: -17.0,
        tiled: true,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::NearBackground,
        factor: 0.55,
        z: -16.0,
        tiled: true,
    },
    RuntimeParallaxLayerSpec {
        asset: ParallaxLayerAsset::ForegroundAtmosphere,
        factor: 0.82,
        z: -15.0,
        tiled: true,
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
    let render_size = parallax_render_size(world);
    for spec in RUNTIME_PARALLAX_LAYERS {
        let Some(image) = assets.parallax_layers.get(theme, spec.asset) else {
            continue;
        };
        let mut sprite = Sprite::from_image(image.clone());
        sprite.custom_size = Some(render_size);
        if spec.tiled {
            // Transparent plates contain edge-framing silhouettes / haze. Tiling
            // keeps them visible around the current camera view instead of
            // stretching those motifs out to the far edges of a large room.
            sprite.image_mode = bevy::sprite::SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.0,
            };
        }
        commands.spawn((
            sprite,
            Transform::from_translation(Vec3::new(0.0, 0.0, spec.z)),
            ParallaxLayerVisual {
                factor: Vec2::splat(spec.factor),
                z: spec.z,
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
    for (mut transform, layer) in &mut layers {
        transform.translation.x = camera_transform.translation.x * (1.0 - layer.factor.x);
        transform.translation.y = camera_transform.translation.y * (1.0 - layer.factor.y);
        transform.translation.z = layer.z;
    }
}

fn parallax_render_size(world: &ae::World) -> BVec2 {
    // Big enough to cover camera-relative offset at the edges of large rooms;
    // generated layers are fixed-size tiles, so this remains cheap.
    let margin_x = WINDOW_W as f32 * 2.0;
    let margin_y = WINDOW_H as f32 * 2.0;
    BVec2::new(
        (world.size.x + margin_x).max(WINDOW_W as f32 * 2.5),
        (world.size.y + margin_y).max(WINDOW_H as f32 * 2.5),
    )
}
