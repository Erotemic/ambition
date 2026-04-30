//! Simple moving-platform test/reference objects.
//!
//! For this iteration the moving platform is deliberately sandbox-side: it is
//! a visible metronome for time-scale tuning, not yet part of the collision
//! simulation. Keeping it here avoids mixing presentation experiments into the
//! pure movement engine until we decide how moving solids should carry/push the
//! player.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::{world_to_bevy, WORLD_Z_BLOCK};

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
    /// away from the immediate dummy/combat lane.
    pub fn time_reference(_world: &ae::World) -> Self {
        Self {
            pos: ae::Vec2::new(470.0, 560.0),
            size: ae::Vec2::new(155.0, 18.0),
            min_x: 350.0,
            max_x: 690.0,
            speed: 130.0,
            dir: 1.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.pos.x += self.speed * self.dir * dt;
        if self.pos.x > self.max_x {
            self.pos.x = self.max_x;
            self.dir = -1.0;
        } else if self.pos.x < self.min_x {
            self.pos.x = self.min_x;
            self.dir = 1.0;
        }
    }
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
            MovingPlatformVisual,
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
