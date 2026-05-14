//! Per-frame discovery system that spawns Bevy `FeatureVisual` entities for
//! dynamically introduced features (encounter mobs, reward chests, and any
//! remaining legacy runtime additions). Static LDtk-derived features are handled
//! by [`super::world::spawn_room_visuals`] at room load.

use ambition_engine as ae;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{feature_color, feature_z, FeatureVisual, RoomVisual};
use crate::config::world_to_bevy;
use crate::features::{
    ActorRuntime, BossRewardChest, ChestFeature, EncounterMob, EncounterRewardChest, FeatureAabb,
    FeatureId, FeatureVisualKind,
};
use crate::game_assets::{self, entity_sprite_or_color, GameAssets};

/// Spawn `FeatureVisual` entities for `FeatureRuntime` features that
/// were appended at runtime and don't have one yet. Static LDtk-
/// derived features get their visuals from `spawn_room_visuals` at
/// room load; runtime additions (`FeatureRuntime::spawn_enemy`,
/// `spawn_chest`) appear after that point and need a per-frame
/// discovery pass to attach their sprite.
///
/// Bevy automatically picks up the new sprites from then on:
/// `sync_visuals` reads the matching `FeatureView` and
/// `upgrade_enemy_sprites` swaps in the character spritesheet on the
/// same frame; chests pick up their sprite via the
/// `state_aware_entity_sprite` path in `sync_visuals`.
pub fn spawn_dynamic_feature_visuals(
    mut commands: Commands,
    runtime: Res<crate::SandboxRuntime>,
    world: Res<crate::GameWorld>,
    assets: Option<Res<GameAssets>>,
    existing: Query<&FeatureVisual>,
    ecs_mobs: Query<(&FeatureId, &FeatureAabb, &ActorRuntime), With<EncounterMob>>,
    ecs_reward_chests: Query<
        (&FeatureId, &FeatureAabb, &ChestFeature),
        Or<(With<EncounterRewardChest>, With<BossRewardChest>)>,
    >,
) {
    let known: std::collections::HashSet<&str> = existing.iter().map(|v| v.id.as_str()).collect();
    let assets_ref = assets.as_deref();
    for (id, aabb, actor) in &ecs_mobs {
        if known.contains(id.as_str()) {
            continue;
        }
        let kind = actor.visual_kind();
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        let entity_kind = match actor {
            ActorRuntime::Hostile(enemy) => ae::RoomObjectKind::EnemySpawn(enemy.brain.clone()),
            ActorRuntime::Peaceful(_) => continue,
        };
        let entity_key = game_assets::entity_sprite_for_room_object(&entity_kind);
        let sprite = match assets_ref {
            Some(a) => entity_sprite_or_color(a, entity_key, render, feature_color(kind, false)),
            None => Sprite::from_color(feature_color(kind, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(&world.0, aabb.center, feature_z(kind))),
            Name::new(format!("Encounter mob: {}", actor.name())),
            FeatureVisual { id: id.as_str().to_string() },
            RoomVisual,
        ));
    }
    for (id, aabb, chest) in &ecs_reward_chests {
        if known.contains(id.as_str()) {
            continue;
        }
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        let entity_kind = ae::RoomObjectKind::Chest(chest.chest.clone());
        let entity_key = game_assets::entity_sprite_for_room_object(&entity_kind);
        let sprite = match assets_ref {
            Some(a) => entity_sprite_or_color(
                a,
                entity_key,
                render,
                feature_color(FeatureVisualKind::Chest, false),
            ),
            None => Sprite::from_color(feature_color(FeatureVisualKind::Chest, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(
                &world.0,
                aabb.center,
                feature_z(FeatureVisualKind::Chest),
            )),
            Name::new(format!("Reward chest: {}", id.as_str())),
            FeatureVisual { id: id.as_str().to_string() },
            RoomVisual,
        ));
    }
    for enemy in &runtime.features.enemies {
        if known.contains(enemy.id.as_str()) {
            continue;
        }
        let archetype_kind = if matches!(enemy.brain, ae::EnemyBrain::Custom(ref n) if n.starts_with("sandbag_"))
        {
            FeatureVisualKind::Sandbag
        } else {
            FeatureVisualKind::Enemy
        };
        let render = BVec2::new(enemy.size.x, enemy.size.y);
        let entity_kind = ae::RoomObjectKind::EnemySpawn(enemy.brain.clone());
        let entity_key = game_assets::entity_sprite_for_room_object(&entity_kind);
        let sprite = match assets_ref {
            Some(a) => {
                entity_sprite_or_color(a, entity_key, render, feature_color(archetype_kind, false))
            }
            None => Sprite::from_color(feature_color(archetype_kind, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(
                &world.0,
                enemy.pos,
                feature_z(archetype_kind),
            )),
            Name::new(format!("Encounter mob: {}", enemy.name)),
            FeatureVisual {
                id: enemy.id.clone(),
            },
            RoomVisual,
        ));
    }
    for chest in &runtime.features.chests {
        if known.contains(chest.id.as_str()) {
            continue;
        }
        let render = BVec2::new(chest.size.x, chest.size.y);
        let entity_kind = ae::RoomObjectKind::Chest(chest.chest.clone());
        let entity_key = game_assets::entity_sprite_for_room_object(&entity_kind);
        let sprite = match assets_ref {
            Some(a) => entity_sprite_or_color(
                a,
                entity_key,
                render,
                feature_color(FeatureVisualKind::Chest, false),
            ),
            None => Sprite::from_color(feature_color(FeatureVisualKind::Chest, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(
                &world.0,
                chest.pos,
                feature_z(FeatureVisualKind::Chest),
            )),
            Name::new(format!("Reward chest: {}", chest.name)),
            FeatureVisual {
                id: chest.id.clone(),
            },
            RoomVisual,
        ));
    }
}
