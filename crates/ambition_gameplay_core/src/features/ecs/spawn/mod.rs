//! ECS-feature spawn facade.
//!
//! Room-level orchestration and public dynamic-mob entry points stay here, while
//! the concrete family-specific spawn helpers live in smaller sibling modules.
//! This keeps the active ECS path readable without changing the entity shapes
//! or scheduling surfaces that callers use.

use crate::features::util::room_spec_paths;
use bevy::prelude::{Commands, Entity, Query};

pub(crate) use super::spawn_actors::spawn_runtime_minion;

/// Spawn ECS-native feature entities for every authored static
/// feature in a room. One loop per family.
pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let paths = room_spec_paths(room);
    for hazard in &room.hazards {
        super::spawn_static::spawn_hazard(commands, hazard, &paths);
    }
    for boss in &room.boss_spawns {
        super::spawn_actors::spawn_boss(commands, boss);
    }
    for pickup in &room.pickups {
        super::spawn_static::spawn_pickup(commands, pickup);
    }
    for ground_item in &room.ground_items {
        super::spawn_static::spawn_ground_item(commands, ground_item);
    }
    #[cfg(feature = "portal")]
    for portal_gun in &room.portal_gun_spawns {
        super::spawn_static::spawn_portal_gun_spawn(commands, portal_gun);
    }
    #[cfg(feature = "portal")]
    for portal in &room.portals {
        super::spawn_static::spawn_portal(commands, portal);
    }
    for shrine in &room.shrines {
        super::spawn_static::spawn_shrine(commands, shrine);
    }
    for gravity_zone in &room.gravity_zones {
        super::spawn_static::spawn_gravity_zone(commands, gravity_zone);
    }
    for chest in &room.chests {
        super::spawn_static::spawn_chest(commands, chest);
    }
    for breakable in &room.breakables {
        super::spawn_static::spawn_breakable(commands, breakable);
    }
    for enemy in &room.enemy_spawns {
        super::spawn_actors::spawn_enemy(commands, enemy, &paths);
    }
    for interactable in &room.interactables {
        super::spawn_actors::spawn_interactable(commands, interactable, &paths);
    }
    // DebugLabel and DestinationLabel are presentation-only and don't
    // spawn ECS feature entities today. The presentation layer reads
    // them off `RoomSpec` directly.

    // Room-scoped faction targeting: reset to the combat baseline every room load
    // so one room's relations overrides never linger into the next. The spectator
    // duel arena needs NO relations mutation — its two fighters are plain `Npc`s
    // whose mutual grudge (cross-wired below) drives the fight — but other rooms may
    // still augment relations, so the per-load reset stays.
    commands.insert_resource(crate::features::FactionRelations::default());

    // The spectator duel arena auto-spawns its two fighters (already fighting the
    // instant the player walks in, no trigger). They feud with EACH OTHER via a
    // mutual grudge, never the observing player: once a fighter's grudge foe dies it
    // goes target-less and stands down like any NPC. The player can still be caught
    // by a stray (physical damage, different faction) or PROVOKE a stood-down fighter
    // by striking it past the retaliation threshold.
    if let Some(requests) = crate::features::stage_room_duel(room) {
        let mut staged = Vec::new();
        for req in requests {
            if let crate::features::SpawnActorKind::Enemy { brain } = &req.kind {
                let authored = crate::rooms::Authored::new(
                    req.id.clone(),
                    req.name.clone(),
                    ambition_engine_core::Aabb::new(req.pos, req.half_size),
                    brain.clone(),
                );
                // Mark the staged fighter so the renderer's runtime-visual
                // discovery gives it a sprite — it isn't in the authored
                // `spec.enemy_spawns` the static render pass iterates.
                if let Some(entity) = super::spawn_actors::spawn_enemy_with_faction(
                    commands,
                    &authored,
                    &[],
                    req.faction,
                ) {
                    commands
                        .entity(entity)
                        .insert(crate::features::RuntimeStagedActor);
                    staged.push((req.id.clone(), entity, req.grudge_against.clone()));
                }
            }
        }
        // Cross-wire the mutual grudge now that both fighters exist.
        super::spawn_actors::wire_staged_grudges(commands, &staged);
    }
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: ambition_characters::actor::EnemyBrain,
    pos: ambition_engine_core::Vec2,
    size: ambition_engine_core::Vec2,
) {
    super::spawn_actors::spawn_encounter_mob(commands, encounter_id, id, brain, pos, size);
}

/// Despawn all ECS mobs owned by an encounter attempt.
pub fn despawn_encounter_mobs(
    commands: &mut Commands,
    mobs: &Query<(
        Entity,
        &super::EncounterMob,
        &super::FeatureId,
        &super::BodyCombat,
    )>,
    encounter_id: &str,
) {
    super::spawn_actors::despawn_encounter_mobs(commands, mobs, encounter_id);
}

#[cfg(test)]
mod tests;
