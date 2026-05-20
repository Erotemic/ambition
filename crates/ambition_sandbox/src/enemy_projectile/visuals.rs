//! Per-frame sprite rebuild for enemy projectiles. Mirrors the
//! player-projectile visuals system but with a hostile red/orange tint
//! so the player can tell incoming volleys from their own fireballs at
//! a glance.

use bevy::prelude::*;

use super::state::EnemyProjectileState;

#[derive(Component)]
pub struct EnemyProjectileVisual;

/// `owner_id` prefix stamp used by GNU-ton's apple-rain attack so the
/// visuals layer can swap the default red rectangle for the apple
/// shape (red body + green leaf + brown stem). Lives here so the
/// shared sandbox crate can match the prefix without depending on
/// the features module.
const APPLE_OWNER_PREFIX: &str = "gnu_ton_apple";

fn is_apple_owner(owner_id: &str) -> bool {
    owner_id.starts_with(APPLE_OWNER_PREFIX)
}

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
        let translation =
            crate::config::world_to_bevy(&world.0, body.pos, crate::config::WORLD_Z_PLAYER + 1.8);
        if is_apple_owner(&projectile.owner_id) {
            spawn_apple_visual(&mut commands, translation, render_size);
            continue;
        }
        // Hostile orange-red: readable against the sky-blue background
        // of the pirate arena and visually distinct from the warm
        // yellow of player fireballs.
        let tint = Color::srgba(1.0, 0.45, 0.18, 0.95);
        let mut sprite = Sprite::from_color(tint, render_size);
        sprite.flip_x = body.vel.x < 0.0;
        commands.spawn((
            sprite,
            Transform::from_translation(translation),
            EnemyProjectileVisual,
            Name::new("Enemy projectile"),
        ));
    }
}

/// Three-sprite apple: red body, green leaf, brown stem. The stack
/// is intentionally simple — we don't have an apple PNG yet and the
/// flat-color triplet still reads at a glance against the GNU-ton
/// arena's warm background. All three sprites carry
/// `EnemyProjectileVisual` so the per-frame cleanup loop sweeps
/// them together with the rest of the projectile visuals.
fn spawn_apple_visual(
    commands: &mut Commands,
    translation: bevy::math::Vec3,
    render_size: bevy::math::Vec2,
) {
    // Bump the apparent radius a touch above the collision box so the
    // shape reads as round even though the body is a flat square.
    let body_size = bevy::math::Vec2::new(render_size.x * 1.05, render_size.y * 1.05);
    let body_color = Color::srgba(0.82, 0.16, 0.18, 1.0);
    commands.spawn((
        Sprite::from_color(body_color, body_size),
        Transform::from_translation(translation),
        EnemyProjectileVisual,
        Name::new("Apple"),
    ));
    let stem_color = Color::srgba(0.30, 0.18, 0.08, 1.0);
    let stem_size = bevy::math::Vec2::new(2.0_f32.max(render_size.x * 0.12), render_size.y * 0.30);
    let stem_translation = translation + bevy::math::Vec3::new(0.0, body_size.y * 0.55, 0.04);
    commands.spawn((
        Sprite::from_color(stem_color, stem_size),
        Transform::from_translation(stem_translation),
        EnemyProjectileVisual,
        Name::new("Apple stem"),
    ));
    let leaf_color = Color::srgba(0.20, 0.62, 0.22, 1.0);
    let leaf_size = bevy::math::Vec2::new(body_size.x * 0.55, body_size.y * 0.30);
    let leaf_translation =
        translation + bevy::math::Vec3::new(body_size.x * 0.30, body_size.y * 0.60, 0.05);
    commands.spawn((
        Sprite::from_color(leaf_color, leaf_size),
        Transform::from_translation(leaf_translation),
        EnemyProjectileVisual,
        Name::new("Apple leaf"),
    ));
}
