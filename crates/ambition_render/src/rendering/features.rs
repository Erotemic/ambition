//! Per-frame discovery system that spawns Bevy `FeatureVisual` entities for
//! dynamically introduced features (encounter mobs, reward chests, and any
//! remaining legacy runtime additions). Static LDtk-derived features are handled
//! by [`super::world::spawn_room_visuals`] at room load.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{feature_color, feature_z, FeatureVisual, RoomVisual};
use ambition_engine_core::config::world_to_bevy;
use ambition_gameplay_core::assets::game_assets::{self, entity_sprite_or_color, GameAssets};
use ambition_gameplay_core::features::{
    ActorDisposition, BossRewardChest, CenteredAabb, ChestFeature, EncounterMob,
    EncounterRewardChest, FeatureId, FeatureName, FeatureVisualKind,
};

/// Spawn `FeatureVisual` entities for dynamically introduced ECS features
/// that don't have one yet. Static LDtk-derived features get their visuals
/// from `spawn_room_visuals` at room load; encounter mobs and reward chests
/// are spawned after that point and need a per-frame discovery pass.
///
/// `sync_visuals` reads the matching `FeatureView` and
/// `upgrade_actor_sprites` swaps in the character spritesheet on the
/// same frame; chests pick up their sprite via `state_aware_entity_sprite`.
pub fn spawn_dynamic_feature_visuals(
    mut commands: Commands,
    world: Res<ambition_engine_core::RoomGeometry>,
    assets: Option<Res<GameAssets>>,
    existing: Query<&FeatureVisual>,
    ecs_mobs: Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ActorDisposition,
            Option<&ambition_gameplay_core::features::ActorConfig>,
        ),
        With<EncounterMob>,
    >,
    post_boss_npcs: Query<
        (
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            &ActorDisposition,
            Option<&ambition_gameplay_core::features::ActorConfig>,
            Option<&ambition_gameplay_core::features::ActorInteraction>,
        ),
        With<ambition_gameplay_core::features::PostBossNpc>,
    >,
    ecs_reward_chests: Query<
        (&FeatureId, &CenteredAabb, &ChestFeature),
        Or<(With<EncounterRewardChest>, With<BossRewardChest>)>,
    >,
    // Hostile actors staged imperatively at room load OUTSIDE the authored
    // `spec.enemy_spawns` (the spectator-duel fighters). They aren't in the static
    // render pass and aren't encounter mobs, so without this they render
    // invisibly. `upgrade_actor_sprites` swaps in the real character sheet next.
    staged_actors: Query<
        (
            &FeatureId,
            &CenteredAabb,
            &ActorDisposition,
            Option<&ambition_gameplay_core::features::ActorConfig>,
        ),
        With<ambition_gameplay_core::features::RuntimeStagedActor>,
    >,
) {
    let known: std::collections::HashSet<&str> = existing.iter().map(|v| v.id.as_str()).collect();
    let assets_ref = assets.as_deref();
    for (id, aabb, disposition, config) in &ecs_mobs {
        if known.contains(id.as_str()) {
            continue;
        }
        // Encounter mobs are hostile by construction; skip any peaceful one.
        let (false, Some(config)) = (disposition.is_peaceful(), config) else {
            continue;
        };
        // ONE actor kind; hostile-by-construction ⇒ the fighting placeholder tint
        // (the sandbag depiction is resolved by the sprite-upgrade fallback, not
        // a render kind).
        let kind = FeatureVisualKind::Actor;
        let fighting = true;
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        let entity_key = game_assets::entity_sprite_for_enemy(&config.brain);
        let sprite = match assets_ref {
            Some(a) => {
                entity_sprite_or_color(a, entity_key, render, feature_color(kind, fighting, false))
            }
            None => Sprite::from_color(feature_color(kind, fighting, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(&world.0, aabb.center, feature_z(kind))),
            Name::new(format!("Encounter mob: {}", config.name)),
            FeatureVisual {
                id: id.as_str().to_string(),
            },
            RoomVisual,
        ));
    }
    for (id, aabb, disposition, config) in &staged_actors {
        if known.contains(id.as_str()) {
            continue;
        }
        // Staged duel fighters are hostile by construction; skip a peaceful one.
        let (false, Some(config)) = (disposition.is_peaceful(), config) else {
            continue;
        };
        // ONE actor kind; hostile-by-construction ⇒ the fighting placeholder tint
        // (the sandbag depiction is resolved by the sprite-upgrade fallback, not
        // a render kind).
        let kind = FeatureVisualKind::Actor;
        let fighting = true;
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        let entity_key = game_assets::entity_sprite_for_enemy(&config.brain);
        let sprite = match assets_ref {
            Some(a) => {
                entity_sprite_or_color(a, entity_key, render, feature_color(kind, fighting, false))
            }
            None => Sprite::from_color(feature_color(kind, fighting, false), render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(&world.0, aabb.center, feature_z(kind))),
            Name::new(format!("Staged actor: {}", config.name)),
            FeatureVisual {
                id: id.as_str().to_string(),
            },
            RoomVisual,
        ));
    }
    for (id, name, aabb, disposition, config, interaction) in &post_boss_npcs {
        if known.contains(id.as_str()) {
            continue;
        }
        let kind = FeatureVisualKind::Actor;
        // A provoked post-boss NPC reads as fighting (warm placeholder tint); an
        // at-rest one stays peaceful/cool.
        let fighting = !disposition.is_peaceful();
        let render = BVec2::new(aabb.size().x, aabb.size().y);
        // A peaceful post-boss NPC resolves its sprite from the dialogue
        // interactable; a hostile one (provoked) from its archetype brain.
        let entity_key = if disposition.is_peaceful() {
            match interaction {
                Some(i) => game_assets::entity_sprite_for_interactable(&i.interactable),
                None => continue,
            }
        } else {
            match config {
                Some(c) => game_assets::entity_sprite_for_enemy(&c.brain),
                None => continue,
            }
        };
        let sprite = match assets_ref {
            Some(a) => {
                entity_sprite_or_color(a, entity_key, render, feature_color(kind, fighting, false))
            }
            None => Sprite::from_color(feature_color(kind, fighting, false), render),
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
                feature_color(FeatureVisualKind::Chest, false, false),
            ),
            None => Sprite::from_color(
                feature_color(FeatureVisualKind::Chest, false, false),
                render,
            ),
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
