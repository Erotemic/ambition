//! Per-frame discovery system that spawns Bevy `FeatureVisual` entities for
//! dynamically introduced features (encounter mobs, staged duel actors,
//! post-boss NPCs, and reward chests). Static LDtk-derived features are
//! handled by [`super::world::spawn_room_visuals`] at room load.
//!
//! Pure consumer of the sim-built
//! [`ambition_sim_view::DynamicFeatureViews`] rows (E4 slice
//! 9): the sim resolves identity / geometry / placeholder-sprite facts; this
//! pass only spawns the missing visuals.

use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use super::primitives::{feature_color, feature_z, FeatureVisual, RoomVisual};
use ambition_engine_core::config::world_to_bevy;
use ambition_sim_view::DynamicFeatureViews;
use ambition_sprite_sheet::game_assets::{entity_sprite_or_color, GameAssets};

/// Spawn `FeatureVisual` entities for dynamically introduced ECS features
/// that don't have one yet. Static LDtk-derived features get their visuals
/// from `spawn_room_visuals` at room load; the sim's `DynamicFeatureViews`
/// carries everything introduced after that point.
///
/// `sync_visuals` reads the matching `FeatureView` and
/// `upgrade_actor_sprites` swaps in the character spritesheet on the
/// same frame; chests pick up their sprite via `state_aware_entity_sprite`.
pub fn spawn_dynamic_feature_visuals(
    mut commands: Commands,
    world: Res<ambition_engine_core::RoomGeometry>,
    assets: Option<Res<GameAssets>>,
    existing: Query<&FeatureVisual>,
    dynamic: Res<DynamicFeatureViews>,
) {
    let known: std::collections::HashSet<&str> = existing.iter().map(|v| v.id.as_str()).collect();
    let assets_ref = assets.as_deref();
    for fact in &dynamic.0 {
        if known.contains(fact.id.as_str()) {
            continue;
        }
        let render = BVec2::new(fact.size.x, fact.size.y);
        let fallback = feature_color(fact.visual_kind, fact.fighting, false);
        let sprite = match assets_ref {
            Some(a) => entity_sprite_or_color(a, fact.sprite_key, render, fallback),
            None => Sprite::from_color(fallback, render),
        };
        commands.spawn((
            sprite,
            Transform::from_translation(world_to_bevy(
                &world.0,
                fact.pos,
                feature_z(fact.visual_kind),
            )),
            Name::new(format!("{}: {}", fact.family, fact.label)),
            FeatureVisual {
                id: fact.id.clone(),
            },
            RoomVisual,
        ));
    }
}
