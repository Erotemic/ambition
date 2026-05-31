//! Per-frame discovery system that spawns Bevy `FeatureVisual` entities for
//! dynamically introduced features (encounter mobs, reward chests, and any
//! remaining legacy runtime additions). Static LDtk-derived features are handled
//! by [`super::world::spawn_room_visuals`] at room load.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{feature_color, feature_z, FeatureVisual, RoomVisual};
use crate::assets::game_assets::{self, entity_sprite_or_color, GameAssets};
use crate::config::world_to_bevy;
use crate::features::{
    ActorRuntime, BossRewardChest, ChestFeature, EncounterMob, EncounterRewardChest, FeatureAabb,
    FeatureId, FeatureName, FeatureVisualKind,
};

/// Spawn `FeatureVisual` entities for dynamically introduced ECS features
/// that don't have one yet. Static LDtk-derived features get their visuals
/// from `spawn_room_visuals` at room load; encounter mobs and reward chests
/// are spawned after that point and need a per-frame discovery pass.
///
/// `sync_visuals` reads the matching `FeatureView` and
/// `upgrade_enemy_sprites` swaps in the character spritesheet on the
/// same frame; chests pick up their sprite via `state_aware_entity_sprite`.
pub fn spawn_dynamic_feature_visuals(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    assets: Option<Res<GameAssets>>,
    existing: Query<&FeatureVisual>,
    ecs_mobs: Query<(&FeatureId, &FeatureAabb, &ActorRuntime), With<EncounterMob>>,
    post_boss_npcs: Query<
        (&FeatureId, &FeatureName, &FeatureAabb, &ActorRuntime),
        With<crate::boss_encounter::SmirkingBehemothVictoryNpc>,
    >,
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
        let entity_key = match actor {
            ActorRuntime::Hostile(enemy) => game_assets::entity_sprite_for_enemy(&enemy.brain),
            ActorRuntime::Peaceful(_) => continue,
        };
        let sprite = match assets_ref {
            Some(a) => entity_sprite_or_color(a, entity_key, render, feature_color(kind, false)),
            None => Sprite::from_color(feature_color(kind, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(&world.0, aabb.center, feature_z(kind))),
            Name::new(format!("Encounter mob: {}", actor.name())),
            FeatureVisual {
                id: id.as_str().to_string(),
            },
            RoomVisual,
        ));
    }
    for (id, name, aabb, actor) in &post_boss_npcs {
        if known.contains(id.as_str()) {
            continue;
        }
        let kind = FeatureVisualKind::Npc;
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        let entity_key = match actor {
            ActorRuntime::Peaceful(npc) => {
                game_assets::entity_sprite_for_interactable(&npc.interactable)
            }
            ActorRuntime::Hostile(enemy) => game_assets::entity_sprite_for_enemy(&enemy.brain),
        };
        let sprite = match assets_ref {
            Some(a) => entity_sprite_or_color(a, entity_key, render, feature_color(kind, false)),
            None => Sprite::from_color(feature_color(kind, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(&world.0, aabb.center, feature_z(kind))),
            Name::new(format!("Post-boss NPC: {}", name.0.as_str())),
            FeatureVisual {
                id: id.as_str().to_string(),
            },
            RoomVisual,
        ));
    }
    for (id, aabb, chest) in &ecs_reward_chests {
        if known.contains(id.as_str()) {
            continue;
        }
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        let entity_key = game_assets::entity_sprite_for_chest(&chest.chest);
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
            FeatureVisual {
                id: id.as_str().to_string(),
            },
            RoomVisual,
        ));
    }
}
