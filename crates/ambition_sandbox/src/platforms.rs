//! Simple moving-platform test/reference objects.
//!
//! The moving platform remains sandbox-side as a design experiment, but it now
//! contributes a temporary solid block to the engine collision world each frame.
//! That gives us rideable/collidable behavior without committing moving-solid
//! semantics to `ambition_engine` before we have tests for carrying, crushing,
//! and one-way platform interactions.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::{world_to_bevy, WORLD_Z_BLOCK};
use crate::rendering::RoomVisual;

/// A deterministic horizontal platform used as a visible game-time reference.
#[derive(Clone, Copy, Debug)]
pub struct MovingPlatformState {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    min_x: f32,
    max_x: f32,
    speed: f32,
    dir: f32,
}

impl MovingPlatformState {
    /// Place the reference platform high enough to be visible from spawn, but
    /// away from the immediate combat-practice lane.
    pub fn time_reference(world: &ae::World) -> Self {
        let min_x = (world.size.x * 0.28).max(100.0);
        let max_x = (world.size.x * 0.48).max(min_x + 180.0);
        let y = (world.size.y * 0.60).min(world.size.y - 210.0).max(170.0);
        Self {
            pos: ae::Vec2::new(min_x, y),
            size: ae::Vec2::new(155.0, 18.0),
            min_x,
            max_x,
            speed: 130.0,
            dir: 1.0,
        }
    }

    /// Advance the platform and return its displacement this frame.
    pub fn update(&mut self, dt: f32) -> ae::Vec2 {
        let old = self.pos;
        self.pos.x += self.speed * self.dir * dt;
        if self.pos.x > self.max_x {
            self.pos.x = self.max_x;
            self.dir = -1.0;
        } else if self.pos.x < self.min_x {
            self.pos.x = self.min_x;
            self.dir = 1.0;
        }
        self.pos - old
    }

    pub fn aabb(&self) -> ae::Aabb {
        ae::Aabb::new(self.pos, self.size * 0.5)
    }

    pub fn as_collision_block(&self) -> ae::Block {
        ae::Block {
            name: "moving time-reference platform".to_string(),
            aabb: self.aabb(),
            // Moving platforms are ordinary solids for walking/riding because
            // `BlockKind::BlinkWall` still resolves as solid collision on both
            // axes. They are deliberately *not* hard blink blockers: if the
            // player has the soft blink-through upgrade, blink pathing may pass
            // through the moving platform just like a soft blink membrane.
            kind: ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Soft },
        }
    }

    /// Detect whether the player was riding this platform at the start of a
    /// frame. We carry the player by the platform delta before collision
    /// resolution so standing on it feels stable.
    pub fn is_riding(&self, player: &ae::Player) -> bool {
        if !player.on_ground {
            return false;
        }
        let player_box = player.aabb();
        let platform_box = self.aabb();
        let horizontally_overlapping = player_box.right() > platform_box.left() + 3.0
            && player_box.left() < platform_box.right() - 3.0;
        let feet_near_top = (player_box.bottom() - platform_box.top()).abs() <= 6.0;
        horizontally_overlapping && feet_near_top
    }
}

/// Return a temporary collision world with the current moving platform inserted.
///
/// The inserted block is solid for normal collision, but blink-passable for
/// upgraded blink pathing. This keeps the debug preview, blink destination
/// resolution, and actual movement collision in agreement.
pub fn world_with_moving_platform(world: &ae::World, platform: &MovingPlatformState) -> ae::World {
    let mut collision_world = world.clone();
    collision_world.blocks.push(platform.as_collision_block());
    collision_world
}

#[derive(Component)]
pub struct MovingPlatformVisual;

pub fn spawn_moving_platform(commands: &mut Commands, world: &ae::World, platform: MovingPlatformState) -> Entity {
    commands
        .spawn((
            Sprite::from_color(
                Color::srgba(0.35, 0.74, 1.0, 0.92),
                BVec2::new(platform.size.x, platform.size.y),
            ),
            Transform::from_translation(world_to_bevy(world, platform.pos, WORLD_Z_BLOCK + 4.0)),
            Name::new("Moving time-reference platform"),
            MovingPlatformVisual,
            RoomVisual,
        ))
        .id()
}

pub fn sync_moving_platform(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    mut query: Query<(&mut Transform, &mut Sprite), With<MovingPlatformVisual>>,
) {
    for (mut transform, mut sprite) in &mut query {
        transform.translation = world_to_bevy(&world.0, runtime.moving_platform.pos, WORLD_Z_BLOCK + 4.0);
        sprite.custom_size = Some(BVec2::new(runtime.moving_platform.size.x, runtime.moving_platform.size.y));
    }
}
