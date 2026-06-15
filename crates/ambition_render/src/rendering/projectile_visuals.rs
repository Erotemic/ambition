//! Persistent Bevy sprite entities for in-flight projectiles plus the
//! charge-indicator quad in front of the player. Each projectile owns one linked
//! visual entity that is transformed from the live body and despawned with it.

use ambition_sandbox::engine_core as ae;
use bevy::prelude::*;

/// Marker on the persistent per-projectile sprite entity produced by
/// [`sync_projectile_visuals`].
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

/// Marker on the transient charge-indicator sprite in front of the player.
#[derive(Component)]
pub struct PlayerChargeVisual;

/// Draw + maintain a persistent sprite for each in-flight player projectile +
/// the per-player charge indicator. Runs after `update_projectiles` (which
/// steps / despawns the projectile entities) and on the presentation half only.
///
/// The charge indicator is rebuilt each frame; projectile sprites persist and
/// are transform-updated until their projectile entity despawns.
pub fn sync_projectile_visuals(
    mut commands: Commands,
    world: Res<ambition_sandbox::GameWorld>,
    assets: Option<Res<ambition_sandbox::assets::game_assets::GameAssets>>,
    // Per-player charge UI: iterate every player so each one's charge
    // indicator renders independently. Single-player behavior unchanged.
    player_q: Query<
        (
            &ambition_sandbox::player::BodyKinematics,
            &ambition_sandbox::projectile::PlayerProjectileState,
        ),
        With<ambition_sandbox::player::PlayerEntity>,
    >,
    // Spawn visuals for in-flight projectiles that do not have a link yet.
    new_projectiles: Query<
        (
            Entity,
            &ambition_sandbox::player::BodyKinematics,
            &ambition_sandbox::projectile::ProjectileGameplay,
        ),
        (
            With<ambition_sandbox::projectile::PlayerProjectile>,
            Without<ProjectileVisualLink>,
        ),
    >,
    // Live bodies for the transform refresh of already-linked projectiles.
    bodies: Query<&ambition_sandbox::player::BodyKinematics, With<ambition_sandbox::projectile::PlayerProjectile>>,
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
                .get(ambition_sandbox::assets::game_assets::EntitySprite::ProjectileEnergy)
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
            let base = ambition_sandbox::projectile::ProjectileKind::Fireball.half_extent();
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
                Transform::from_translation(ambition_sandbox::config::world_to_bevy(
                    &world.0,
                    charge_pos,
                    ambition_sandbox::config::WORLD_Z_PLAYER + 1.5,
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
            ambition_sandbox::projectile::ProjectileKind::Fireball => Color::srgba(1.0, 0.74, 0.30, 0.95),
            ambition_sandbox::projectile::ProjectileKind::Hadouken => Color::srgba(0.45, 0.78, 1.0, 0.96),
            // Stronger tint for the Super so the player can see at a
            // glance that they fired the harder gesture.
            ambition_sandbox::projectile::ProjectileKind::HadoukenSuper => Color::srgba(0.30, 0.55, 1.0, 1.0),
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
                Transform::from_translation(ambition_sandbox::config::world_to_bevy(
                    &world.0,
                    kin.pos,
                    ambition_sandbox::config::WORLD_Z_PLAYER + 2.0,
                )),
                PlayerProjectileVisual,
                VisualProjectile(proj_entity),
                Name::new(match game.kind {
                    ambition_sandbox::projectile::ProjectileKind::Fireball => "Player projectile: fireball",
                    ambition_sandbox::projectile::ProjectileKind::Hadouken => "Player projectile: hadouken",
                    ambition_sandbox::projectile::ProjectileKind::HadoukenSuper => {
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
            ambition_sandbox::config::world_to_bevy(&world.0, kin.pos, ambition_sandbox::config::WORLD_Z_PLAYER + 2.0);
        // Track travel direction each frame; fireballs can reverse on bounce.
        sprite.flip_x = kin.vel.x < 0.0;
    }
}
