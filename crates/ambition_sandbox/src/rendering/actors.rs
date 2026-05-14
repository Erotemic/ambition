//! Per-frame Bevy systems that mirror engine actor state into Bevy
//! sprites + animations. Covers the player, enemies, and bosses
//! along with the upgrade-to-spritesheet pass that converts the
//! initial colored rectangles into authored character sprites once
//! the asset is loaded.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{
    feature_color, feature_z, switch_on_color, FeatureVisual, PlayerSpriteBaseline, PlayerVisual,
    SceneEntities,
};
use crate::boss_sprites::{self, BossAnimState, BossAnimator};
use crate::character_sprites::{build_character_sprite, feet_anchor_for, CharacterAnimator};
use crate::config::{world_to_bevy, WORLD_Z_PLAYER};
use crate::features::{
    BreakableFeature, ChestFeature, Collected, FeatureAabb, FeatureId, FeatureVisualKind, Opened,
    PickupFeature,
};
use crate::game_assets::{self, EntitySprite, GameAssets};

pub fn sync_visuals(
    world: Res<crate::GameWorld>,
    runtime: Res<crate::SandboxRuntime>,
    entities: Res<SceneEntities>,
    assets: Option<Res<GameAssets>>,
    mut player_query: Query<
        (&mut Transform, &mut Sprite, Option<&PlayerSpriteBaseline>),
        With<PlayerVisual>,
    >,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
    ecs_pickups: Query<(&FeatureId, &FeatureAabb, Option<&Collected>), With<PickupFeature>>,
    ecs_chests: Query<(&FeatureId, &FeatureAabb, Option<&Opened>), With<ChestFeature>>,
    ecs_breakables: Query<(&FeatureId, &FeatureAabb, &BreakableFeature)>,
    ecs_chest_states: Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakable_states: Query<(&FeatureId, &BreakableFeature)>,
) {
    if let Ok((mut transform, mut sprite, baseline)) = player_query.get_mut(entities.player) {
        transform.translation = world_to_bevy(&world.0, runtime.player.pos, WORLD_Z_PLAYER);
        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Colored-rectangle fallback only — stretch to the collision-box
            // size and tint by flash. Textured sprites (atlas OR plain image)
            // keep their authored size and are tinted in the animation system.
            sprite.custom_size = Some(BVec2::new(runtime.player.size.x, runtime.player.size.y));
            let alpha = if runtime.flash_timer > 0.0 { 0.72 } else { 1.0 };
            sprite.color = Color::srgba(0.80, 0.95, 1.0, alpha);
        } else if let Some(baseline) = baseline {
            // HACK(crouch-sprite-row): when the player crouches (or
            // morphs / crawls / slides), the engine shrinks the AABB
            // and slides `pos.y` down to keep feet planted. The
            // textured sprite was sized for the standing pose, so
            // without compensation it floats below the floor by half
            // the height delta. Re-scale the sprite's vertical extent
            // by the same ratio the collision shrunk; the normalized
            // sprite anchor preserves foot alignment automatically.
            // Phase 1 also lets the development menu swap standing body
            // profiles live. Scale the placeholder art against the recorded
            // startup collision so body-profile experiments remain visual.
            // Replace with authored body-profile rows once the generator emits
            // them — see PlayerSpriteBaseline doc.
            let base_y = runtime.player.base_size.y.max(1.0);
            let stance_ratio_y = (runtime.player.size.y / base_y).clamp(0.1, 1.0);
            let scale_x = runtime.player.base_size.x / baseline.standing_collision.x.max(1.0);
            let scale_y = runtime.player.base_size.y / baseline.standing_collision.y.max(1.0);
            sprite.custom_size = Some(BVec2::new(
                baseline.standing_render.x * scale_x,
                baseline.standing_render.y * scale_y * stance_ratio_y,
            ));
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = runtime
            .features
            .view(&visual.id)
            .or_else(|| crate::features::ecs_feature_view(&visual.id, &ecs_pickups, &ecs_chests, &ecs_breakables))
        else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = world_to_bevy(&world.0, view.pos, feature_z(view.kind));

        // State-aware sprite swap for breakables and chests. Pickups are
        // chosen at spawn time and never change kind. Enemies are animated
        // through the character spritesheet path.
        if let Some(assets) = assets.as_deref() {
            if let Some(target_key) =
                state_aware_entity_sprite(
                    &visual.id,
                    view.kind,
                    &runtime.features,
                    &ecs_chest_states,
                    &ecs_breakable_states,
                )
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
    runtime_features: &crate::features::FeatureRuntime,
    ecs_chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Breakable => runtime_features
            .breakable_state(id)
            .or_else(|| crate::features::ecs_breakable_state(id, ecs_breakables))
            .map(game_assets::breakable_state_sprite),
        FeatureVisualKind::Chest => runtime_features
            .chest_opened(id)
            .or_else(|| crate::features::ecs_chest_opened(id, ecs_chests))
            .map(game_assets::chest_state_sprite),
        _ => None,
    }
}

/// Marker recording which `FeatureVisualKind` the current sprite +
/// `CharacterAnimator` were bound for. The upgrade systems read this
/// to detect mid-life kind changes — e.g. when a peaceful NPC turns
/// hostile and `apply_save` migrates the runtime entry from `npcs`
/// to `enemies`. Without this marker, the existing
/// `Without<CharacterAnimator>` filter hid the entity from the enemy
/// upgrade pass and the kernel guide stayed visually a kernel guide
/// after the third strike.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundFeatureKind(pub FeatureVisualKind);

/// Bind enemy/sandbag visuals to the appropriate character sheet
/// once the asset is available — and re-bind when an existing visual
/// changes kind (e.g. NPC → Enemy on hostility flip).
pub fn upgrade_enemy_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    runtime: Res<crate::SandboxRuntime>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = runtime.features.view(&visual.id) else {
            continue;
        };
        if !matches!(
            view.kind,
            FeatureVisualKind::Enemy | FeatureVisualKind::Sandbag
        ) {
            continue;
        }
        // Already bound to the correct kind — nothing to do this frame.
        if matches!(bound, Some(BoundFeatureKind(k)) if *k == view.kind) {
            continue;
        }
        // Sprite-override path: an enemy that was spawned by migrating
        // a hostile NPC carries the original LDtk display name so the
        // renderer can keep that NPC's sheet (with its authored slash
        // / hit rows). Only the Kernel Guide migration leaves the
        // override blank, so kernel→goblin keeps its dedicated visual
        // gag while every other faction NPC stays themselves when
        // hostile.
        let character_asset = match runtime.features.enemy_sprite_override(&visual.id) {
            Some(name) => assets
                .characters
                .npc_asset_for_name(name)
                .or_else(|| assets.characters.enemy_asset(view.kind)),
            None => assets.characters.enemy_asset(view.kind),
        };
        let Some(character_asset) = character_asset else {
            continue;
        };
        // Android loads assets out of the APK asynchronously, and missing or
        // platform-rejected images still have a Handle. Do not replace the
        // colored fallback with an atlas sprite until the texture is actually
        // present in Assets<Image>; otherwise a failed or delayed load renders
        // the NPC/enemy invisible.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(character_asset.spec, collision),
            CharacterAnimator::new(character_asset.spec),
            BoundFeatureKind(view.kind),
        ));
    }
}

/// Replace the static `EntitySprite::NpcTerminal` placeholder with a
/// faction-specific spritesheet once the asset is loaded. Today the
/// dispatch is keyed off the NPC's authored name (see
/// `CharacterSpriteAssets::npc_asset_for_name`); when LDtk grows a
/// `category` field on `NpcSpawn`, switch this to lookup-by-category
/// so the dispatch survives display-name edits.
///
/// NPCs without a registered sprite (the common case for the existing
/// hub guides etc.) keep the default terminal placeholder — symmetric
/// with `enemy_asset` returning `None` for non-enemy kinds.
pub fn upgrade_npc_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    runtime: Res<crate::SandboxRuntime>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = runtime.features.view(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Npc) {
            continue;
        }
        if matches!(bound, Some(BoundFeatureKind(k)) if *k == view.kind) {
            continue;
        }
        let Some(name) = runtime.features.npc_name(&visual.id) else {
            continue;
        };
        let Some(character_asset) = assets.characters.npc_asset_for_name(name) else {
            continue;
        };
        // Keep the visible terminal/rectangle fallback until the PNG has
        // actually loaded. This is especially important on Android, where the
        // asset exists inside the APK but individual textures can still fail
        // or arrive later.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        let sprite = build_character_sprite(character_asset, collision);
        commands.entity(entity).insert((
            sprite,
            feet_anchor_for(character_asset.spec, collision),
            CharacterAnimator::new(character_asset.spec),
            BoundFeatureKind(view.kind),
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

/// Drive enemy AND NPC sprite animation, atlas index, and facing flip.
///
/// Enemies and NPCs both render through `CharacterAnimator`; their
/// per-frame state is owned by separate runtime lists, but a feature
/// id only ever appears in one of them at a time. We try the enemy
/// lookup first (most entities in the room) and fall through to the
/// NPC lookup, so a stationary General sheet ticks its 8 idle frames
/// once the animator is attached.
///
/// One system instead of two avoids the borrow conflict on the
/// shared `(&mut Sprite, &mut CharacterAnimator)` query.
pub fn animate_characters(
    time: Res<Time>,
    runtime: Res<crate::SandboxRuntime>,
    mut query: Query<(&FeatureVisual, &mut Sprite, &mut CharacterAnimator), Without<PlayerVisual>>,
) {
    let dt = time.delta_secs();
    for (visual, mut sprite, mut animator) in &mut query {
        let (anim, facing, hit_flash, attacking) =
            if let Some(state) = runtime.features.enemy_anim_state(&visual.id) {
                (
                    crate::character_sprites::pick_enemy_anim(state),
                    state.facing,
                    state.hit_flash,
                    state.attack_active || state.attack_windup,
                )
            } else if let Some(state) = runtime.features.npc_anim_state(&visual.id) {
                (
                    crate::character_sprites::pick_npc_anim(state),
                    state.facing,
                    state.hit_flash,
                    false,
                )
            } else {
                continue;
            };
        animator.request(anim);
        let index = animator.tick(dt);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = index;
        }
        sprite.flip_x = facing < 0.0;
        sprite.color = if hit_flash {
            Color::srgba(1.0, 0.55, 0.55, 1.0)
        } else if attacking {
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
    images: Res<Assets<Image>>,
    runtime: Res<crate::SandboxRuntime>,
    new_bosses: Query<
        (Entity, &FeatureVisual),
        (Without<CharacterAnimator>, Without<BossAnimator>),
    >,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual) in &new_bosses {
        let Some(view) = runtime.features.view(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Boss) {
            continue;
        }
        // Pick the per-boss sheet by authored name. The mockingbird
        // ships its own 6-row sheet (hover / thrust / bite / slash /
        // hit / death) installed by the standalone python generator;
        // other bosses fall back to the gradient-sentinel sheet that
        // ships with the main `ambition_sprite2d_renderer` package.
        // If neither is available we skip — the colored rectangle
        // fallback in `sync_visuals` continues to render.
        let boss_name = runtime.features.boss_name(&visual.id).unwrap_or("");
        let boss_asset = if boss_name.eq_ignore_ascii_case("mockingbird") {
            assets.mockingbird.as_ref().or(assets.boss.as_ref())
        } else {
            assets.boss.as_ref()
        };
        let Some(boss_asset) = boss_asset else {
            continue;
        };
        if images.get(&boss_asset.texture).is_none() {
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
