use bevy::prelude::*;
use bevy::sprite::SpriteImageMode;

use crate::rooms::{ActiveRoomMetadata, RoomSet};

use super::layers::{parallax_layer_translation, ParallaxLayer};
use super::profiles::{select_parallax_profile, ParallaxLayerProfile, ParallaxProfile};

/// Presentation-only plugin for simple asset-backed parallax backgrounds.
pub struct ParallaxPlugin;

#[derive(Component)]
struct ParallaxBackdropLayer;

#[derive(Resource, Clone, Debug, Default)]
struct ActiveParallaxProfile {
    name: Option<String>,
}

impl Plugin for ParallaxPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveParallaxProfile>()
            .add_systems(PostStartup, spawn_initial_parallax_background)
            .add_systems(
                Update,
                (
                    refresh_parallax_background.after(crate::rooms::sync_active_room_metadata),
                    sync_parallax_layers.after(crate::rendering::camera_follow),
                ),
            );
    }
}

/// Spawn the active room's background stack on startup.
///
/// Profiles are selected from room metadata and ultimately resolve to the
/// generated PNGs under `crates/ambition_sandbox/assets/backgrounds/<profile>/`.
fn spawn_initial_parallax_background(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    room_set: Option<Res<RoomSet>>,
    mut active_profile: ResMut<ActiveParallaxProfile>,
) {
    let profile = room_set
        .as_ref()
        .map(|room_set| select_parallax_profile(room_set.active_metadata()))
        .unwrap_or_else(super::profiles::default_parallax_profile);
    let Some(asset_server) = asset_server else {
        active_profile.name = Some(profile.name.to_string());
        return;
    };
    spawn_profile_stack(&mut commands, &asset_server, profile);
    active_profile.name = Some(profile.name.to_string());
}

/// Swap the parallax profile when the active room metadata points at a
/// different biome/theme profile.
fn refresh_parallax_background(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    active_room: Res<ActiveRoomMetadata>,
    mut active_profile: ResMut<ActiveParallaxProfile>,
    existing_layers: Query<Entity, With<ParallaxBackdropLayer>>,
) {
    let profile = select_parallax_profile(&active_room.0);
    if active_profile.name.as_deref() == Some(profile.name) {
        return;
    }
    let Some(asset_server) = asset_server else {
        return;
    };
    for entity in &existing_layers {
        commands.entity(entity).despawn();
    }
    spawn_profile_stack(&mut commands, &asset_server, profile);
    active_profile.name = Some(profile.name.to_string());
}

fn spawn_profile_stack(
    commands: &mut Commands,
    asset_server: &AssetServer,
    profile: ParallaxProfile,
) {
    for layer in profile.layers {
        spawn_layer(commands, asset_server, layer, profile.name);
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
        ParallaxBackdropLayer,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_is_noop_when_profile_matches() {
        let mut app = App::new();
        app.init_resource::<ActiveParallaxProfile>();
        app.insert_resource(ActiveRoomMetadata(crate::rooms::RoomMetadata {
            biome: Some("hub".into()),
            ..Default::default()
        }));
        app.add_systems(Update, refresh_parallax_background);
        app.world_mut().resource_mut::<ActiveParallaxProfile>().name = Some("hub".into());
        app.update();
        assert_eq!(
            app.world()
                .resource::<ActiveParallaxProfile>()
                .name
                .as_deref(),
            Some("hub")
        );
    }
}
