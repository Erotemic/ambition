//! Per-frame Bevy systems that mirror engine actor state into Bevy
//! sprites + animations. Covers the player, enemies, and bosses
//! along with the upgrade-to-spritesheet pass that converts the
//! initial colored rectangles into authored character sprites once
//! the asset is loaded.

#![allow(unused_imports)]
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{
    feature_color, feature_z, switch_on_color, FeatureVisual, PlayerSpriteBaseline, PlayerVisual,
    PropVisual, SceneEntities,
};
use ambition_gameplay_core::assets::game_assets::{self, EntitySprite, GameAssets};
use ambition_gameplay_core::boss_encounter::sprites::{self, BossAnimState, BossAnimator};
use ambition_gameplay_core::character_sprites::{
    build_character_sprite, build_character_sprite_with_render_size, feet_anchor_for,
    feet_anchor_for_render_size, CharacterAnimator,
};
use ambition_gameplay_core::combat::BoundFeatureKind;
use ambition_gameplay_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_gameplay_core::features::{
    ActorRenderSize, BossClusterRef, BreakableFeature, ChestFeature, FeatureId, FeatureViewIndex,
    FeatureVisualKind, Opened,
};

mod animation;
mod boss;
mod overlays;

pub use animation::*;
pub use boss::*;
pub use overlays::*;

pub fn sync_visuals(
    world: Res<ambition_gameplay_core::RoomGeometry>,
    entities: Res<SceneEntities>,
    assets: Option<Res<GameAssets>>,
    feature_views: Res<FeatureViewIndex>,
    mut player_query: Query<
        (
            &mut Transform,
            &mut Sprite,
            Option<&PlayerSpriteBaseline>,
            &ambition_gameplay_core::actor::BodyKinematics,
            &ambition_gameplay_core::actor::BodyBaseSize,
            &ambition_gameplay_core::actor::BodyCombat,
            Option<&ambition_gameplay_core::platformer_runtime::orientation::ActorRoll>,
        ),
        With<PlayerVisual>,
    >,
    mut feature_query: Query<
        (&FeatureVisual, &mut Transform, &mut Sprite, &mut Visibility),
        Without<PlayerVisual>,
    >,
    ecs_chest_states: Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakable_states: Query<(&FeatureId, &BreakableFeature)>,
) {
    if let Ok((mut transform, mut sprite, baseline, body, base_size, player_combat, roll)) =
        player_query.get_mut(entities.player)
    {
        transform.translation = world_to_bevy(&world.0, body.pos, WORLD_Z_PLAYER);
        // Aerial roll (portal somersault / future gravity-room orientation).
        transform.rotation = Quat::from_rotation_z(roll.map_or(0.0, |r| r.angle));
        if sprite.texture_atlas.is_none() && sprite.image == Handle::default() {
            // Colored-rectangle fallback only — stretch to the collision-box
            // size and tint by flash. Textured sprites (atlas OR plain image)
            // keep their authored size and are tinted in the animation system.
            sprite.custom_size = Some(BVec2::new(body.size.x, body.size.y));
            let alpha = if player_combat.hit_flash > 0.0 {
                0.72
            } else {
                1.0
            };
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
            let base_y = base_size.base_size.y.max(1.0);
            let stance_ratio_y = (body.size.y / base_y).clamp(0.1, 1.0);
            let scale_x = base_size.base_size.x / baseline.standing_collision.x.max(1.0);
            let scale_y = base_size.base_size.y / baseline.standing_collision.y.max(1.0);
            sprite.custom_size = Some(BVec2::new(
                baseline.standing_render.x * scale_x,
                baseline.standing_render.y * scale_y * stance_ratio_y,
            ));
        }
    }

    for (visual, mut transform, mut sprite, mut visibility) in &mut feature_query {
        let Some(view) = feature_views.get(&visual.id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation = world_to_bevy(&world.0, view.pos, feature_z(view.kind));
        // Surface-walking enemies (PuppySlug) rotate the sprite so
        // its authored "up" axis aligns with the surface normal —
        // the slug crawls along walls / ceilings with its body
        // visibly clinging to them. All other actors stay axis-
        // aligned (rotation_rad = 0).
        transform.rotation = Quat::from_rotation_z(view.rotation_rad);

        // State-aware sprite swap for breakables and chests. Pickups are
        // chosen at spawn time and never change kind. Enemies are animated
        // through the character spritesheet path.
        if let Some(assets) = assets.as_deref() {
            if let Some(target_key) = state_aware_entity_sprite(
                &visual.id,
                view.kind,
                view.switch_on,
                &ecs_chest_states,
                &ecs_breakable_states,
            ) {
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
    switch_on: bool,
    ecs_chests: &Query<(&FeatureId, Option<&Opened>), With<ChestFeature>>,
    ecs_breakables: &Query<(&FeatureId, &BreakableFeature)>,
) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Breakable => {
            ambition_gameplay_core::features::ecs_breakable_state(id, ecs_breakables)
                .map(game_assets::breakable_state_sprite)
        }
        FeatureVisualKind::Chest => {
            ambition_gameplay_core::features::ecs_chest_opened(id, ecs_chests)
                .map(game_assets::chest_state_sprite)
        }
        // Switch shows its on/off button sprite (armed = on, disabled = off)
        // instead of a flat colored block (#57).
        FeatureVisualKind::Switch => Some(if switch_on {
            EntitySprite::SwitchArmed
        } else {
            EntitySprite::SwitchDisabled
        }),
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

/// Bind enemy/sandbag visuals to the appropriate character sheet
/// once the asset is available — and re-bind when an existing visual
/// changes kind (e.g. NPC → Enemy on hostility flip).
pub fn upgrade_enemy_sprites(
    mut commands: Commands,
    assets: Option<Res<GameAssets>>,
    images: Res<Assets<Image>>,
    feature_views: Res<FeatureViewIndex>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
    ecs_actors: Query<ambition_gameplay_core::features::ActorSpriteData>,
    // Shared sprite-metadata render size — present on an enemy that was a
    // body-metrics NPC before it turned hostile, so its sprite keeps the
    // authored size instead of re-applying `collision_scale` to the body box.
    render_sizes: Query<(&FeatureId, &ActorRenderSize)>,
    // Names we've already warned about resolving no sprite, so the warning fires
    // once per offending name instead of every frame the actor is unbound.
    mut warned_sprite_names: Local<std::collections::HashSet<String>>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(
            view.kind,
            FeatureVisualKind::Enemy | FeatureVisualKind::TrainingDummy
        ) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        // Already bound to the correct kind and collision footprint — nothing
        // to do this frame. The collision-size check is still useful for rare
        // intentional runtime size changes, but shark riders should normally
        // keep the same visual/collision scale across mount and dismount.
        if bound.is_some_and(|b| b.matches(view.kind, view.size)) {
            continue;
        }
        // Sprite-override path: an enemy that was spawned by migrating
        // a hostile NPC carries the original LDtk display name so the
        // renderer can keep that NPC's sheet (with its authored slash
        // / hit rows). Only the Kernel Guide migration leaves the
        // override blank, so kernel→goblin keeps its dedicated visual
        // gag while every other faction NPC stays themselves when
        // hostile.
        //
        // Fallback for direct EnemySpawn entities (no NPC migration
        // history): try the enemy's display name against the same
        // NPC sprite registry. Intro raiders resolve to their
        // placeholder sheet this way without authors having to
        // duplicate the registry entry on an
        // enemy-side table.
        let override_name =
            ambition_gameplay_core::features::ecs_enemy_sprite_override(&visual.id, &ecs_actors);
        let enemy_name = ambition_gameplay_core::features::ecs_enemy_name(&visual.id, &ecs_actors);
        // Resolve a *named* sprite first (override label, then the enemy's own
        // name), then fall back to the generic kind sheet.
        let named = override_name
            .as_deref()
            .and_then(|n| assets.characters.npc_asset_for_name(n))
            .or_else(|| {
                enemy_name
                    .as_deref()
                    .and_then(|n| assets.characters.npc_asset_for_name(n))
            });
        let character_asset = match named {
            Some(asset) => Some(asset),
            None => {
                // Falling back to the generic kind sheet is intended for nameless /
                // truly-generic enemies, but a *named* actor that lands here almost
                // always means its `display_name` doesn't match the character
                // catalog — a content/code bug (e.g. a decorated variant like
                // "Puppy Slug (ally)" instead of the catalog "Puppy Slug"), which
                // used to render the goblin default silently. Surface it once per
                // name (a warning, not a panic — a genuinely missing/late asset
                // file is handled gracefully by the `images.get(..).is_none()`
                // guard below, so the game still runs).
                if let Some(missed) = override_name.as_deref().or(enemy_name.as_deref()) {
                    if warned_sprite_names.insert(missed.to_string()) {
                        bevy::log::warn!(
                            target: "ambition::sprites",
                            "actor '{missed}' resolved no registered sprite — using the {:?} \
                             default sheet. If it should have its own sprite, its display_name \
                             doesn't match the character catalog (likely a typo / decorated name).",
                            view.kind,
                        );
                    }
                }
                assets.characters.enemy_asset(view.kind)
            }
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
        // Honor a shared sprite-metadata render size (e.g. a hostile-flipped
        // body-metrics NPC): render at the stored quad, NOT collision*scale,
        // so the sprite doesn't balloon once collision already equals the body.
        let render_size =
            ambition_gameplay_core::features::ecs_actor_render_size(&visual.id, &render_sizes)
                .map(|r| BVec2::new(r.x, r.y));
        let (sprite, anchor) = match render_size {
            Some(render_size) => (
                build_character_sprite_with_render_size(character_asset, render_size),
                feet_anchor_for_render_size(&character_asset.spec, collision, render_size),
            ),
            None => (
                build_character_sprite(character_asset, collision),
                feet_anchor_for(&character_asset.spec, collision),
            ),
        };
        // The feet anchor plants the sprite's authored feet (`feet_anchor_y` from
        // sprite metadata) on the gravity-side edge of the collision box. It is a
        // 1-D anchor that rotates WITH the sprite, so for a surface-walker clung to
        // a wall it correctly plants the contact edge once the collision box itself
        // is oriented (see `update_enemy_actors`). No per-family special-casing.
        // Cache the base render size + anchor so a trimmed (alpha-packed) sheet
        // can recompute per-frame size/anchor; untrimmed sheets ignore it.
        let basis_size = sprite.custom_size.unwrap_or(BVec2::new(1.0, 1.0));
        let basis_anchor = anchor.0;
        commands.entity(entity).insert((
            sprite,
            anchor,
            CharacterAnimator::new(character_asset).with_render_basis(basis_size, basis_anchor),
            BoundFeatureKind::new(view.kind, collision),
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
    feature_views: Res<FeatureViewIndex>,
    features: Query<(Entity, &FeatureVisual, Option<&BoundFeatureKind>)>,
    ecs_actors: Query<ambition_gameplay_core::features::ActorSpriteData>,
    render_sizes: Query<(&FeatureId, &ActorRenderSize)>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, visual, bound) in &features {
        let Some(view) = feature_views.get(&visual.id) else {
            continue;
        };
        if !matches!(view.kind, FeatureVisualKind::Npc) {
            continue;
        }
        let collision = BVec2::new(view.size.x, view.size.y);
        if bound.is_some_and(|b| b.matches(view.kind, view.size)) {
            continue;
        }
        let Some(name) = ambition_gameplay_core::features::ecs_npc_name(&visual.id, &ecs_actors)
        else {
            continue;
        };
        let Some(character_asset) = assets.characters.npc_asset_for_name(&name) else {
            continue;
        };
        // Keep the visible terminal/rectangle fallback until the PNG has
        // actually loaded. This is especially important on Android, where the
        // asset exists inside the APK but individual textures can still fail
        // or arrive later.
        if images.get(&character_asset.texture).is_none() {
            continue;
        }
        // When the NPC's collision was derived from published sprite
        // `body_metrics`, `collision` IS the visible body — so the sprite must
        // render at the stored quad size, not `collision * collision_scale`
        // (which would double-scale). NPCs without body metrics fall through to
        // the legacy collision-driven render.
        let render_size =
            ambition_gameplay_core::features::ecs_actor_render_size(&visual.id, &render_sizes)
                .map(|r| BVec2::new(r.x, r.y));
        let (sprite, anchor) = match render_size {
            Some(render_size) => (
                build_character_sprite_with_render_size(character_asset, render_size),
                feet_anchor_for_render_size(&character_asset.spec, collision, render_size),
            ),
            None => (
                build_character_sprite(character_asset, collision),
                feet_anchor_for(&character_asset.spec, collision),
            ),
        };
        // Cache the base render size + anchor so a trimmed (alpha-packed) sheet
        // can recompute per-frame size/anchor; untrimmed sheets ignore it.
        let basis_size = sprite.custom_size.unwrap_or(BVec2::new(1.0, 1.0));
        let basis_anchor = anchor.0;
        commands.entity(entity).insert((
            sprite,
            anchor,
            CharacterAnimator::new(character_asset).with_render_basis(basis_size, basis_anchor),
            BoundFeatureKind::new(view.kind, collision),
        ));
    }
}
