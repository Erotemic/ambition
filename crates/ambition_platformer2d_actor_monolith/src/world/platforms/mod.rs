//! Bevy adapter for world-owned moving-platform state.
//!
//! W3 moved the authored spec, runtime state, and collision-world composition
//! to `ambition_platformer2d_world::platforms`. Gameplay-core keeps only the visual sync
//! adapter because it names Bevy sprite/lifecycle types and the live
//! `MovingPlatformSet` resource.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK};
use ambition_platformer2d_shared_tangle::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::platformer_runtime::lifecycle::RoomVisual;

// ⛔ **the world-owned types are NOT re-exported here, and the absence is the
// point.** This module used to hand out `MovingPlatformSpec`,
// `MovingPlatformState`, `moving_platforms_for_room` and
// `world_with_moving_platforms` under an actor-monolith path, so four consumers
// that only wanted world state — the provider's room lifecycle, the room-
// transition commit, `sim_view`'s facts and the portal host adapter — reached
// through the actor monolith to get it, and read as depending on it. They name
// `ambition_platformer2d_world::platforms` directly now. Import from the owner.
use ambition_platformer2d_world::platforms::MovingPlatformState;

#[derive(Component)]
pub struct MovingPlatformVisual {
    pub index: usize,
}

pub fn spawn_moving_platform(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    index: usize,
    platform: &MovingPlatformState,
) -> Entity {
    commands
        .spawn_session_scoped(
            session_scope,
            (
                Sprite::from_color(
                    Color::srgba(0.35, 0.74, 1.0, 0.92),
                    BVec2::new(platform.size.x, platform.size.y),
                ),
                Transform::from_translation(world_to_bevy(
                    world,
                    platform.pos,
                    WORLD_Z_BLOCK + 4.0,
                )),
                Name::new(format!("Moving platform {index}: {}", platform.name)),
                MovingPlatformVisual { index },
                RoomVisual,
            ),
        )
        .id()
}

pub fn spawn_moving_platforms(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    world: &ae::World,
    platforms: &[MovingPlatformState],
) -> Vec<Entity> {
    platforms
        .iter()
        .enumerate()
        .map(|(index, platform)| {
            spawn_moving_platform(commands, session_scope, world, index, platform)
        })
        .collect()
}

/// Mirror the authoritative [`MovingPlatformSet`] resource onto the platform
/// visuals — a pure read-model sync, with NO reset authority.
///
/// Platform STATE is installed by construction: session setup, room transition
/// (`RoomConstructionPlan`), sandbox reset, LDtk hot-reload, and the N3.2b
/// restore staging each reset the resource and (re)spawn the visuals through
/// the same canonical calls. This system once carried a `Local`-cached
/// room-change reset of its own; that hidden second authority clobbered
/// freshly RESTORED platform state with authored starts on the first tick
/// after a staged cross-room restore (state on a read-model — the same bug
/// class as the moveset dedup accumulator).
pub fn sync_moving_platform(
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    platform_set: Res<ambition_platformer2d_world::collision::MovingPlatformSet>,
    mut query: Query<(&MovingPlatformVisual, &mut Transform, &mut Sprite)>,
) {
    for (visual, mut transform, mut sprite) in &mut query {
        let Some(platform) = platform_set.0.get(visual.index) else {
            continue;
        };
        transform.translation = world_to_bevy(&world.0, platform.pos, WORLD_Z_BLOCK + 4.0);
        sprite.custom_size = Some(BVec2::new(platform.size.x, platform.size.y));
    }
}
