//! Per-frame Bevy sprite entities for in-flight projectiles plus the
//! charge-indicator quad in front of the player.

use ambition_engine as ae;
use bevy::prelude::*;

use super::state::PlayerProjectileState;

/// Marker on the per-frame projectile sprite entities produced by
/// [`sync_projectile_visuals`]. Despawned and rebuilt each tick so
/// the entity set always matches `PlayerProjectileState::bodies`.
#[derive(Component)]
pub struct PlayerProjectileVisual;

/// Marker on the charge-indicator sprite that hovers in front of
/// the player while they're holding the fire button. Rebuilt each
/// tick from `PlayerProjectileState::charging` like the projectile
/// sprites, so it disappears the moment the charge resolves.
#[derive(Component)]
pub struct PlayerChargeVisual;

/// Mirror `PlayerProjectileState::bodies` onto Bevy sprite entities so
/// the player can actually see what they fired. Runs after
/// `update_projectiles` (which produces the body Vec) and on the
/// presentation half only — headless drains `state.bodies` without
/// needing visuals.
///
/// Despawn-and-respawn is the simplest match for a small ring of
/// short-lived projectiles (typical in-flight count is 1–3, capped by
/// the spawner's cooldown + resource meter). Anything fancier
/// (per-projectile entity reuse) would need a stable id on
/// `PlayerProjectile`, which today doesn't exist.
pub fn sync_projectile_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    state: Res<PlayerProjectileState>,
    assets: Option<Res<crate::assets::game_assets::GameAssets>>,
    player_body_q: Query<&crate::player::PlayerBody, With<crate::player::PlayerEntity>>,
    existing: Query<Entity, With<PlayerProjectileVisual>>,
    existing_charge: Query<Entity, With<PlayerChargeVisual>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for entity in &existing_charge {
        commands.entity(entity).despawn();
    }
    // Charge indicator: a growing tinted quad in front of the player.
    // Size scales with the live charge tier so the player sees the
    // "winding up" state and can time the release. Only rendered
    // while `state.charging` is `Some` AND the press hasn't yet
    // committed to a Hadouken motion.
    if let Some(hold) = state.charging {
        let tier = state.charge_tuning.tier_for_hold(hold);
        let base = ae::ProjectileKind::Fireball.half_extent();
        let (size_mult, alpha) = match tier {
            0 => (0.7, 0.55),
            1 => (1.1, 0.78),
            _ => (1.5, 0.95),
        };
        let render_size = bevy::math::Vec2::new(base.x * 2.0 * size_mult, base.y * 2.0 * size_mult);
        if let Ok(body) = player_body_q.single() {
        let facing = if body.facing.abs() < f32::EPSILON {
            1.0
        } else {
            body.facing.signum()
        };
        let charge_pos = ae::Vec2::new(
            body.pos.x + facing * (body.size.x * 0.5 + 6.0),
            body.pos.y - body.size.y * 0.20,
        );
        commands.spawn((
            Sprite::from_color(
                Color::srgba(1.0, 0.74, 0.30, alpha),
                bevy::math::Vec2::new(render_size.x, render_size.y),
            ),
            Transform::from_translation(crate::config::world_to_bevy(
                &world.0,
                charge_pos,
                crate::config::WORLD_Z_PLAYER + 1.5,
            )),
            PlayerChargeVisual,
            Name::new("Player projectile charge indicator"),
        ));
        } // if let Ok(body)
    }
    let handle = assets
        .as_deref()
        .and_then(|a| {
            a.entities
                .get(crate::assets::game_assets::EntitySprite::ProjectileEnergy)
        })
        .cloned();
    for projectile in &state.bodies {
        let body = &projectile.body;
        let render_size = bevy::math::Vec2::new(
            (body.half_extent.x * 2.0).max(8.0),
            (body.half_extent.y * 2.0).max(8.0),
        );
        // Hadouken tint (cooler / blue-shifted) vs Fireball (warmer
        // orange). The tint applies whether or not the textured sprite
        // loads; a missing texture falls through to a colored quad.
        let tint = match body.kind {
            ae::ProjectileKind::Fireball => Color::srgba(1.0, 0.74, 0.30, 0.95),
            ae::ProjectileKind::Hadouken => Color::srgba(0.45, 0.78, 1.0, 0.96),
            // Stronger tint for the Super so the player can see at a
            // glance that they fired the harder gesture.
            ae::ProjectileKind::HadoukenSuper => Color::srgba(0.30, 0.55, 1.0, 1.0),
        };
        let mut sprite = match handle.clone() {
            Some(image) => Sprite {
                image,
                color: tint,
                custom_size: Some(render_size),
                ..Default::default()
            },
            None => Sprite::from_color(tint, render_size),
        };
        // Flip the sprite to face travel direction so a leftward
        // fireball doesn't look upside-down.
        sprite.flip_x = body.vel.x < 0.0;
        commands.spawn((
            sprite,
            Transform::from_translation(crate::config::world_to_bevy(
                &world.0,
                body.pos,
                crate::config::WORLD_Z_PLAYER + 2.0,
            )),
            PlayerProjectileVisual,
            Name::new(match body.kind {
                ae::ProjectileKind::Fireball => "Player projectile: fireball",
                ae::ProjectileKind::Hadouken => "Player projectile: hadouken",
                ae::ProjectileKind::HadoukenSuper => "Player projectile: hadouken_super",
            }),
        ));
    }
}
