use bevy::prelude::*;
use bevy::sprite::SpriteImageMode;

use super::layers::{parallax_layer_translation, ParallaxLayer};
use super::profiles::{default_parallax_profile, ParallaxLayerProfile};

/// Presentation-only plugin for simple asset-backed parallax backgrounds.
pub struct ParallaxPlugin;

impl Plugin for ParallaxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_default_parallax_background)
            .add_systems(
                Update,
                sync_parallax_layers.after(crate::rendering::camera_follow),
            );
    }
}

/// Spawn the current default background stack.
///
/// The generated PNGs live in `assets/backgrounds/default`. Hand-painted art can
/// replace those files without touching Rust code.
pub fn spawn_default_parallax_background(mut commands: Commands, asset_server: Res<AssetServer>) {
    let profile = default_parallax_profile();
    for layer in profile.layers {
        spawn_layer(&mut commands, &asset_server, layer, profile.name);
    }
}

fn spawn_layer(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layer: &ParallaxLayerProfile,
    profile_name: &str,
) {
    let handle: Handle<Image> = asset_server.load(layer.asset_path);
    let mut sprite = Sprite::from_image(handle);
    sprite.custom_size = Some(layer.size);
    if layer.tile_x || layer.tile_y {
        sprite.image_mode = SpriteImageMode::Tiled {
            tile_x: layer.tile_x,
            tile_y: layer.tile_y,
            stretch_value: 1.0,
        };
    }

    let component = ParallaxLayer::new(layer.factor, layer.offset, layer.z);
    commands.spawn((
        sprite,
        Transform::from_translation(parallax_layer_translation(Vec3::ZERO, component)),
        component,
        Name::new(format!("Parallax {profile_name}: {}", layer.name)),
    ));
}

/// Keep parallax layers aligned to the active camera while preserving their
/// cheaper-than-world parallax factor.
pub fn sync_parallax_layers(
    cameras: Query<&Transform, (With<Camera>, Without<ParallaxLayer>)>,
    mut layers: Query<(&ParallaxLayer, &mut Transform), Without<Camera>>,
) {
    let Some(camera_transform) = cameras.iter().next() else {
        return;
    };
    let camera = camera_transform.translation;
    for (layer, mut transform) in &mut layers {
        transform.translation = parallax_layer_translation(camera, *layer);
    }
}
