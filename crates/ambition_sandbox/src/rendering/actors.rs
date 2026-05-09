//! Per-frame Bevy systems that mirror engine actor state into Bevy
//! sprites + animations. Covers the player, enemies, and bosses
//! along with the upgrade-to-spritesheet pass that converts the
//! initial colored rectangles into authored character sprites once
//! the asset is loaded.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{
    feature_color, feature_z, switch_on_color, FeatureVisual, PlayerVisual, SceneEntities,
};
use crate::boss_sprites::{self, BossAnimState, BossAnimator};
use crate::character_sprites::{build_character_sprite, feet_anchor_for, CharacterAnimator};
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::features::FeatureVisualKind;
use crate::game_assets::{self, EntitySprite, GameAssets};

pub fn sync_visuals(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<SceneEntities>,
    assets: Option<Res<GameAssets>>,
    mut player_query: Query<(&mut Transform, &mut Sprite), With<PlayerVisual>>,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
) {
    if let Ok((mut transform, mut sprite)) = player_query.get_mut(entities.player) {
        transform.translation = world_to_bevy(&world.0, runtime.player.pos, WORLD_Z_PLAYER);
        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Colored-rectangle fallback only — stretch to the collision-box
            // size and tint by flash. Textured sprites (atlas OR plain image)
            // keep their authored size and are tinted in the animation system.
            sprite.custom_size = Some(BVec2::new(runtime.player.size.x, runtime.player.size.y));
            let alpha = if runtime.flash_timer > 0.0 { 0.72 } else { 1.0 };
            sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = runtime.features.view(&visual.id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = world_to_bevy(&world.0, view.pos, feature_z(view.kind));

        // State-aware sprite swap for breakables and chests. Pickups are
        // chosen at spawn time and never change kind. Enemies are animated
        // through the character spritesheet path.
        if let Some(assets) = assets.as_deref() {
            if let Some(target_key) =
                state_aware_entity_sprite(&visual.id, view.kind, &runtime.features)
            {
                if let Some(handle) = assets.entities.get(target_key) {
                    if sprite.image != *handle {
                        sprite.image = handle.clone();
                    }
                }
            }
        }

        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Bare colored rectangle (no entity sprite available, no atlas).
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = if matches!(view.kind, FeatureVisualKind::Switch) && view.switch_on {
                switch_on_color()
            } else {
                feature_color(view.kind, view.flash)
            };
        } else if sprite.texture_atlas.is_none() {
            // Textured single-image entity sprite. Keep author size; tint
            // for hit-flash, otherwise white.
            sprite.custom_size = Some(BVec2::new(view.size.x, view.size.y));
            sprite.color = if view.flash {
                Color::srgba(1.0, 0.55, 0.55, 1.0)
            } else {
                Color::WHITE
            };
        }
        *visibility = if view.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn state_aware_entity_sprite(
    id: &str,
    kind: FeatureVisualKind,
    features: &crate::features::FeatureRuntime,
) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Breakable => features
            .breakable_state(id)
            .map(game_assets::breakable_state_sprite),
        FeatureVisualKind::Chest => features
            .chest_opened(id)
            .map(game_assets::chest_state_sprite),
        _ => None,
    }
}

/// Replace the colored-rectangle sprite on enemy/sandbag entities with the
/// appropriate character sprite-sheet sprite once the asset is available. Newly-spawned
/// feature visuals (initial setup or room transitions) are picked up here.
pub fn upgrade_enemy_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    runtime: Res<crate::SandboxRuntime>,
    new_features: Query<(Entity, &FeatureVisual), Without<CharacterAnimator>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual) in &new_features {
        let Some(view) = runtime.features.view(&visual.id) else {
            continue;
        };
        if !matches!(
            view.kind,
            FeatureVisualKind::Enemy | FeatureVisualKind::Sandbag
        ) {
            continue;
        }
        let Some(character_asset) = assets.characters.enemy_asset(view.kind) else {
            continue;
        };
        let collision = BVec2::new(view.size.x, view.size.y);
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(character_asset.spec, collision),
            CharacterAnimator::new(character_asset.spec),
        ));
    }
}

/// Drive the player sprite's animation state, atlas index, and facing flip.
/// Runs every frame; no-op on color-rectangle fallbacks (no `CharacterAnimator`).
pub fn animate_player(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<SceneEntities>,
    mut query: Query<(&mut Sprite, &mut CharacterAnimator), With<PlayerVisual>>,
) {
    let Ok((mut sprite, mut animator)) = query.get_mut(entities.player) else {
        return;
    };
    let anim = crate::character_sprites::pick_player_anim(&runtime);
    animator.request(anim);
    let index = animator.tick(time.delta_secs());
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = index;
    }
    sprite.flip_x = runtime.player.facing < 0.0;
    // Keep the textured sprite at full opacity by default, with a subtle
    // red tint when invulnerable / hit so the existing flash signal still
    // reads. Tints multiply the texture color, so values below 1.0 darken
    // the channel.
    sprite.color = if runtime.flash_timer > 0.0 {
        Color::srgba(1.0, 0.55, 0.55, 1.0)
    } else {
        Color::WHITE
    };
}

/// Drive enemy sprite animation, atlas index, and facing flip.
pub fn animate_enemies(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    mut query: Query<(&FeatureVisual, &mut Sprite, &mut CharacterAnimator), Without<PlayerVisual>>,
) {
    let dt = time.delta_secs();
    for (visual, mut sprite, mut animator) in &mut query {
        let Some(state) = runtime.features.enemy_anim_state(&visual.id) else {
            continue;
        };
        let anim = crate::character_sprites::pick_enemy_anim(state);
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.flip_x = state.facing < 0.0;
        sprite.color = if state.hit_flash {
            Color::srgba(1.0, 0.55, 0.55, 1.0)
        } else if state.attack_active || state.attack_windup {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}

/// Replace the static `boss_core.png` look on boss feature entities with
/// the animated boss spritesheet once the asset is available. Symmetric
/// with `upgrade_enemy_sprites` but uses `BossAnimator` instead of
/// `CharacterAnimator` because the boss generator emits its own row set.
pub fn upgrade_boss_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    runtime: Res<crate::SandboxRuntime>,
    new_bosses: Query<
        (Entity, &FeatureVisual),
        (Without<CharacterAnimator>, Without<BossAnimator>),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    let Some(boss_asset) = &assets.boss else {
        return;
    };
    for (entity, visual) in &new_bosses {
        let Some(view) = runtime.features.view(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Boss) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let mut sprite = Sprite::from_atlas_image(
            boss_asset.texture.clone(),
            bevy::image::TextureAtlas {
                layout: boss_asset.layout.clone(),
                index: boss_asset.spec.flat_index(boss_sprites::BossAnim::Rest, 0),
            },
        );
        sprite.custom_size = Some(boss_asset.spec.render_size(collision));
        commands.entity(entity).insert((
            sprite,
            boss_asset.spec.collision_anchor(collision),
            BossAnimator::new(boss_asset.spec),
        ));
    }
}

/// Per-frame state-driven animation for boss entities.
pub fn animate_bosses(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    mut query: Query<(&FeatureVisual, &mut Sprite, &mut BossAnimator), Without<PlayerVisual>>,
) {
    let dt = time.delta_secs();
    for (visual, mut sprite, mut animator) in &mut query {
        let Some(state): Option<BossAnimState> = runtime.features.boss_anim_state(&visual.id)
        else {
            continue;
        };
        let anim = boss_sprites::pick_boss_anim(state);
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.color = if state.hit_flash {
            Color::srgba(1.0, 0.55, 0.55, 1.0)
        } else if state.attack_active || state.attack_windup {
            Color::srgba(1.0, 0.85, 0.55, 1.0)
        } else {
            Color::WHITE
        };
    }
}
