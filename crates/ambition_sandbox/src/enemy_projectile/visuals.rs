//! Per-frame sprite rebuild for enemy projectiles. Mirrors the
//! player-projectile visuals system but with a hostile red/orange tint
//! so the player can tell incoming volleys from their own fireballs at
//! a glance.

use bevy::prelude::*;

use super::state::EnemyProjectileState;

#[derive(Component)]
pub struct EnemyProjectileVisual;

pub fn sync_enemy_projectile_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    state: Res<EnemyProjectileState>,
    existing: Query<Entity, With<EnemyProjectileVisual>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for projectile in &state.bodies {
        let body = &projectile.body;
        let render_size = bevy::math::Vec2::new(
            (body.half_extent.x * 2.0).max(8.0),
            (body.half_extent.y * 2.0).max(8.0),
        );
        // Hostile orange-red: readable against the sky-blue background
        // of the pirate arena and visually distinct from the warm
        // yellow of player fireballs.
        let tint = Color::srgba(1.0, 0.45, 0.18, 0.95);
        let mut sprite = Sprite::from_color(tint, render_size);
        sprite.flip_x = body.vel.x < 0.0;
        commands.spawn((
            sprite,
            Transform::from_translation(crate::config::world_to_bevy(
                &world.0,
                body.pos,
                crate::config::WORLD_Z_PLAYER + 1.8,
            )),
            EnemyProjectileVisual,
            Name::new("Enemy projectile"),
        ));
    }
}
