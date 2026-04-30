//! Bevy visual synchronization for engine state.
//!
//! This module owns the render-only component tags and visual sync systems.
//! Gameplay code should mutate `SandboxRuntime`; this module mirrors that state
//! into Bevy transforms/sprites.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::{world_to_bevy, GRID_STEP, WORLD_Z_BLOCK, WORLD_Z_DUMMY, WORLD_Z_PLAYER};
use crate::dummies::{Dummy, DummyKind};

#[derive(Resource)]
pub struct SceneEntities {
    pub player: Entity,
    pub hud: Entity,
}

#[derive(Component)]
pub struct PlayerVisual;

#[derive(Component)]
pub struct DummyVisual {
    pub index: usize,
}

#[derive(Component)]
pub struct HudText;

pub fn sync_visuals(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut player_query: Query<(&mut Transform, &mut Sprite), (With<PlayerVisual>, Without<DummyVisual>)>,
    mut dummy_query: Query<(&DummyVisual, &mut Transform, &mut Sprite, &mut Visibility), (With<DummyVisual>, Without<PlayerVisual>)>,
) {
    if let Ok((mut transform, mut sprite)) = player_query.get_mut(entities.player) {
        transform.translation = world_to_bevy(&world.0, runtime.player.pos, WORLD_Z_PLAYER);
        sprite.custom_size = Some(BVec2::new(runtime.player.size.x, runtime.player.size.y));
        let alpha = if runtime.flash_timer > 0.0 { 0.72 } else { 1.0 };
        sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut dummy_query {
        let Some(dummy) = runtime.dummies.get(visual.index) else {
            continue;
        };
        transform.translation = world_to_bevy(&world.0, dummy.pos, WORLD_Z_DUMMY);
        sprite.custom_size = Some(BVec2::new(dummy.size.x, dummy.size.y));
        sprite.color = dummy_color(dummy);
        *visibility = if dummy.alive { Visibility::Visible } else { Visibility::Hidden };
    }
}

pub fn spawn_grid(commands: &mut Commands, world: &ae::World) {
    let grid_color = Color::srgba(0.12, 0.15, 0.22, 0.28);
    let mut x = 0.0;
    while x <= world.size.x {
        let center = ae::Vec2::new(x, world.size.y * 0.5);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(1.0, world.size.y)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
        ));
        x += GRID_STEP;
    }
    let mut y = 0.0;
    while y <= world.size.y {
        let center = ae::Vec2::new(world.size.x * 0.5, y);
        commands.spawn((
            Sprite::from_color(grid_color, BVec2::new(world.size.x, 1.0)),
            Transform::from_translation(world_to_bevy(world, center, -20.0)),
        ));
        y += GRID_STEP;
    }
}

pub fn spawn_block(commands: &mut Commands, world: &ae::World, block: &ae::Block) {
    let size = block.aabb.half * 2.0;
    commands.spawn((
        Sprite::from_color(block_color(block.kind), BVec2::new(size.x, size.y)),
        Transform::from_translation(world_to_bevy(world, block.aabb.center, WORLD_Z_BLOCK)),
    ));
}

pub fn block_color(kind: ae::BlockKind) -> Color {
    match kind {
        ae::BlockKind::Solid => Color::srgba(0.25, 0.28, 0.36, 1.0),
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Soft } => Color::srgba(0.32, 0.20, 0.72, 0.88),
        ae::BlockKind::BlinkWall { tier: ae::BlinkWallTier::Hard } => Color::srgba(0.52, 0.14, 0.80, 0.96),
        ae::BlockKind::OneWay => Color::srgba(0.36, 0.43, 0.62, 0.92),
        ae::BlockKind::Hazard => Color::srgba(0.96, 0.18, 0.26, 0.92),
        ae::BlockKind::PogoOrb => Color::srgba(0.30, 0.95, 0.64, 0.95),
        ae::BlockKind::Rebound { .. } => Color::srgba(1.0, 0.60, 0.20, 0.95),
    }
}

pub fn dummy_color(dummy: &Dummy) -> Color {
    if dummy.hit_flash > 0.0 {
        return Color::srgba(1.0, 1.0, 1.0, 1.0);
    }
    match dummy.kind {
        DummyKind::InfiniteSandbag => Color::srgba(0.78, 0.62, 0.42, 1.0),
        DummyKind::FiniteRespawner => Color::srgba(0.86, 0.38, 0.90, 1.0),
    }
}
