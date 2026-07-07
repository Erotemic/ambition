//! Bevy adapter for world-owned moving-platform state.
//!
//! W3 moved the authored spec, runtime state, and collision-world composition
//! to `ambition_world::platforms`. Gameplay-core keeps only the visual sync
//! adapter because it names Bevy sprite/lifecycle types and the live
//! `MovingPlatformSet` resource.

use ambition_engine_core as ae;
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::platformer_runtime::lifecycle::RoomVisual;
use crate::rooms::RoomSet;

pub use ambition_world::platforms::{
    moving_platforms_for_room, world_with_moving_platforms, MovingPlatformSpec, MovingPlatformState,
};

#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
}

pub fn spawn_moving_platform(
    commands: &mut Commands,
    world: &ae::World,
    index: usize,
    platform: &MovingPlatformState,
) -> Entity {
    commands
        .spawn((
            Sprite::from_color(
                Color::srgba(0.35, 0.74, 1.0, 0.92),
                BVec2::new(platform.size.x, platform.size.y),
            ),
            Transform::from_translation(world_to_bevy(world, platform.pos, WORLD_Z_BLOCK + 4.0)),
            Name::new(format!("Moving platform {index}: {}", platform.name)),
            MovingPlatformVisual { index },
            RoomVisual,
        ))
        .id()
}

pub fn spawn_moving_platforms(
    commands: &mut Commands,
    world: &ae::World,
    platforms: &[MovingPlatformState],
) -> Vec<Entity> {
    platforms
        .iter()
        .enumerate()
        .map(|(index, platform)| spawn_moving_platform(commands, world, index, platform))
        .collect()
}

pub fn sync_moving_platform(
    mut commands: Commands,
    world: Res<ambition_engine_core::RoomGeometry>,
    room_set: Res<RoomSet>,
    mut platform_set: ResMut<crate::MovingPlatformSet>,
    mut active_platform_room: Local<Option<String>>,
    mut active_platform_source: Local<Option<Vec<MovingPlatformState>>>,
    mut query: Query<(Entity, &MovingPlatformVisual, &mut Transform, &mut Sprite)>,
) {
    let active_spec = room_set.active_spec();
    let desired_start = moving_platforms_for_room(active_spec);

    let source_changed = active_platform_room.as_deref() != Some(active_spec.id.as_str())
        || active_platform_source
            .as_ref()
            .map(|source| source != &desired_start)
            .unwrap_or(true);
    if source_changed {
        platform_set.0 = desired_start.clone();
        *active_platform_room = Some(active_spec.id.clone());
        *active_platform_source = Some(desired_start.clone());

        let visual_count = query.iter().count();
        if visual_count != desired_start.len() {
            for (entity, _, _, _) in &mut query {
                commands.entity(entity).despawn();
            }
            spawn_moving_platforms(&mut commands, &world.0, &platform_set.0);
            return;
        }
    }

    for (_, visual, mut transform, mut sprite) in &mut query {
        let Some(platform) = platform_set.0.get(visual.index) else {
            continue;
        };
        transform.translation = world_to_bevy(&world.0, platform.pos, WORLD_Z_BLOCK + 4.0);
        sprite.custom_size = Some(BVec2::new(platform.size.x, platform.size.y));
    }
}
