//! Persistent Bevy sprite entities for in-flight projectiles plus the
//! charge-indicator quad in front of the player.
//!
//! Phase 3d: the projectile SPRITE now rides the projectile entity. When a
//! projectile entity appears, one visual entity is spawned for it and linked
//! back via [`VisualProjectile`]; each frame the visual's transform (+ flip) is
//! refreshed from the live body; when the projectile entity despawns, its
//! visual is despawned. No more despawn-and-respawn-every-frame ring.

use crate::engine_core as ae;
use bevy::prelude::*;

/// Marker on the persistent per-projectile sprite entity produced by
/// [`sync_projectile_visuals`]. One per in-flight player projectile entity,
/// reused frame to frame (Phase 3d).
#[derive(Component)]
pub struct PlayerProjectileVisual;

/// Back-reference from a projectile visual to its sim projectile entity. Used
/// to refresh the visual's transform each frame and to despawn the visual once
/// the projectile entity is gone.
#[derive(Component, Clone, Copy)]
pub struct VisualProjectile(pub Entity);

/// Forward link from a projectile entity to its spawned visual entity, so the
/// "spawn a visual for projectiles that don't have one yet" pass is idempotent
/// (a projectile is only matched while it lacks this component).
#[derive(Component, Clone, Copy)]
pub struct ProjectileVisualLink(#[allow(dead_code)] pub Entity);

/// Marker on the charge-indicator sprite that hovers in front of
/// the player while they're holding the fire button. Rebuilt each
/// tick from `PlayerProjectileState::charging` like before — it is a
/// transient per-player indicator, not a per-projectile entity.
#[derive(Component)]
pub struct PlayerChargeVisual;

/// Draw + maintain a persistent sprite for each in-flight player projectile +
/// the per-player charge indicator. Runs after `update_projectiles` (which
/// steps / despawns the projectile entities) and on the presentation half only.
///
/// The charge indicator is still rebuilt each frame (it is a transient
/// per-player UI element, not a per-projectile entity). The projectile sprites
/// are persistent (Phase 3d): spawned on first sight, transform-updated each
/// frame, despawned when their projectile entity is gone.
pub fn sync_projectile_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    assets: Option<Res<crate::assets::game_assets::GameAssets>>,
    // Per-player charge UI: iterate every player so each one's charge
    // indicator renders independently. Single-player behavior unchanged.
    player_q: Query<
        (
            &crate::player::BodyKinematics,
            &crate::projectile::PlayerProjectileState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    // In-flight player projectiles are ECS entities (Phase 3c-ii). Projectiles
    // that don't yet have a visual (no `ProjectileVisualLink`) get one spawned.
    new_projectiles: Query<
        (
            Entity,
            &crate::player::BodyKinematics,
            &crate::projectile::ProjectileGameplay,
        ),
        (
            With<crate::projectile::PlayerProjectile>,
            Without<ProjectileVisualLink>,
        ),
    >,
    // Live bodies for the transform refresh of already-linked projectiles.
    bodies: Query<&crate::player::BodyKinematics, With<crate::projectile::PlayerProjectile>>,
    mut visuals: Query<
        (Entity, &VisualProjectile, &mut Transform, &mut Sprite),
        With<PlayerProjectileVisual>,
    >,
    existing_charge: Query<Entity, With<PlayerChargeVisual>>,
) {
    for entity in &existing_charge {
        commands.entity(entity).despawn();
    }
    let handle = assets
        .as_deref()
        .and_then(|a| {
            a.entities
                .get(crate::assets::game_assets::EntitySprite::ProjectileEnergy)
        })
        .cloned();
    for (body, state) in &player_q {
        // Charge indicator: a growing tinted quad in front of the player.
        // Size scales with the live charge tier so the player sees the
        // "winding up" state and can time the release. Only rendered
        // while `state.charging` is `Some` AND the press hasn't yet
        // committed to a Hadouken motion.
        if let Some(hold) = state.charging {
            let tier = state.charge_tuning.tier_for_hold(hold);
            let base = crate::projectile::ProjectileKind::Fireball.half_extent();
            let (size_mult, alpha) = match tier {
                0 => (0.7, 0.55),
                1 => (1.1, 0.78),
                _ => (1.5, 0.95),
            };
            let render_size =
                bevy::math::Vec2::new(base.x * 2.0 * size_mult, base.y * 2.0 * size_mult);
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
        }
    } // end per-player charge loop

    // Spawn one persistent sprite per NEW in-flight player projectile entity.
    for (proj_entity, kin, game) in &new_projectiles {
        let render_size = bevy::math::Vec2::new((kin.size.x).max(8.0), (kin.size.y).max(8.0));
        // Hadouken tint (cooler / blue-shifted) vs Fireball (warmer
        // orange). The tint applies whether or not the textured sprite
        // loads; a missing texture falls through to a colored quad.
        let tint = match game.kind {
            crate::projectile::ProjectileKind::Fireball => Color::srgba(1.0, 0.74, 0.30, 0.95),
            crate::projectile::ProjectileKind::Hadouken => Color::srgba(0.45, 0.78, 1.0, 0.96),
            // Stronger tint for the Super so the player can see at a
            // glance that they fired the harder gesture.
            crate::projectile::ProjectileKind::HadoukenSuper => Color::srgba(0.30, 0.55, 1.0, 1.0),
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
        sprite.flip_x = kin.vel.x < 0.0;
        let visual = commands
            .spawn((
                sprite,
                Transform::from_translation(crate::config::world_to_bevy(
                    &world.0,
                    kin.pos,
                    crate::config::WORLD_Z_PLAYER + 2.0,
                )),
                PlayerProjectileVisual,
                VisualProjectile(proj_entity),
                Name::new(match game.kind {
                    crate::projectile::ProjectileKind::Fireball => "Player projectile: fireball",
                    crate::projectile::ProjectileKind::Hadouken => "Player projectile: hadouken",
                    crate::projectile::ProjectileKind::HadoukenSuper => {
                        "Player projectile: hadouken_super"
                    }
                }),
            ))
            .id();
        commands
            .entity(proj_entity)
            .insert(ProjectileVisualLink(visual));
    }

    // Refresh existing visuals from their live body; despawn orphans whose
    // projectile entity is gone (expired / hit this frame).
    for (visual_entity, link, mut transform, mut sprite) in &mut visuals {
        let Ok(kin) = bodies.get(link.0) else {
            commands.entity(visual_entity).despawn();
            continue;
        };
        transform.translation =
            crate::config::world_to_bevy(&world.0, kin.pos, crate::config::WORLD_Z_PLAYER + 2.0);
        // Track travel direction each frame (a fireball can reverse x on a
        // wall bounce) so the rendered flip matches the old per-frame rebuild.
        sprite.flip_x = kin.vel.x < 0.0;
    }
}
