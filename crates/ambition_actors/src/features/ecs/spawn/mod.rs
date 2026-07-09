//! ECS-feature spawn facade.
//!
//! Room-level orchestration and public dynamic-mob entry points stay here, while
//! the concrete family-specific spawn helpers live in smaller sibling modules.
//! This keeps the active ECS path readable without changing the entity shapes
//! or scheduling surfaces that callers use.

use bevy::prelude::{Commands, Entity, Query};
use std::collections::HashSet;

pub(crate) use super::spawn_actors::spawn_runtime_minion;

/// Spawn ECS-native feature entities for every authored static
/// feature in a room. One loop per family.

/// Flatten a room's authored `KinematicPathSpec`s into `(lookup key, path)`
/// pairs (id first, name alias second). Lives spawn-side: `RoomSpec` is
/// world-IR vocabulary the combat kit must not name (E2).
pub(crate) fn room_spec_paths(
    room: &crate::rooms::RoomSpec,
) -> Vec<(String, ambition_engine_core::KinematicPath)> {
    let mut paths: Vec<(String, ambition_engine_core::KinematicPath)> = Vec::new();
    for spec in &room.kinematic_paths {
        paths.push((spec.id.clone(), spec.path.clone()));
        if spec.name != spec.id {
            paths.push((spec.name.clone(), spec.path.clone()));
        }
    }
    paths
}

pub fn spawn_room_feature_entities(commands: &mut Commands, room: &crate::rooms::RoomSpec) {
    let mut registry = crate::world::placements::PlacementLoweringRegistry::default();
    registry.register(
        ambition_entity_catalog::placements::PlacementKind::Hazard,
        super::spawn_static::lower_hazard_placement,
    );
    registry.register(
        ambition_entity_catalog::placements::PlacementKind::Interactable,
        super::spawn_static::lower_interactable_placement,
    );
    registry.register(
        ambition_entity_catalog::placements::PlacementKind::Pickup,
        super::spawn_static::lower_pickup_placement,
    );
    registry.register(
        ambition_entity_catalog::placements::PlacementKind::Chest,
        super::spawn_static::lower_chest_placement,
    );
    spawn_room_feature_entities_with_registry(commands, room, &registry);
}

pub fn spawn_room_feature_entities_with_registry(
    commands: &mut Commands,
    room: &crate::rooms::RoomSpec,
    registry: &crate::world::placements::PlacementLoweringRegistry,
) {
    let paths = room_spec_paths(room);
    for record in &room.placements {
        let mut ctx = crate::world::placements::LoweringCtx {
            commands,
            room_id: &room.id,
            paths: &paths,
        };
        registry.lower(record, &mut ctx);
    }
    let lowered_hazard_ids: HashSet<&str> = room
        .placements
        .iter()
        .filter(|record| {
            matches!(
                record.schema,
                ambition_entity_catalog::placements::PlacementSchema::Hazard(_)
            )
        })
        .map(|record| record.id.as_str())
        .collect();
    for hazard in &room.hazards {
        if lowered_hazard_ids.contains(hazard.id.as_str()) {
            continue;
        }
        super::spawn_static::spawn_hazard(commands, hazard, &paths);
    }
    for boss in &room.boss_spawns {
        super::spawn_actors::spawn_boss(commands, boss);
    }
    // Pickups now lower through the `placements` channel above (fable audit F9.2).
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
    // Chests now lower through the `placements` channel above (fable audit F9.2).
    for breakable in &room.breakables {
        super::spawn_static::spawn_breakable(commands, breakable);
    }
    for enemy in &room.enemy_spawns {
        super::spawn_actors::spawn_enemy(commands, enemy, &paths);
    }
    // ADR 0020: hand the room's authored `(rider, mount)` links to the
    // resolver resource. It links them by `FeatureId` once the actors above
    // have spawned (deferred commands flush first). A fresh room overwrites
    // any prior room's pending links.
    commands.insert_resource(crate::features::PendingMountLinks(room.mount_links.clone()));
    // Interactables now lower through the `placements` channel above
    // (fable audit F9.2), so there is no typed `room.interactables` loop.
    // DebugLabel and DestinationLabel are presentation-only and don't
    // spawn ECS feature entities today. The presentation layer reads
    // them off `RoomSpec` directly.

    // Room-scoped faction targeting: reset to the combat baseline every room load
    // so one room's relations overrides never linger into the next. (Per-room
    // content staging — e.g. the spectator duel — happens in CONTENT systems
    // consuming the `RoomLoaded` fact below, not here.)
    commands.insert_resource(crate::features::FactionRelations::default());

    // The staging fact (JD4): every path that stages a room's contents flows
    // through this function, so this is the ONE emission site for
    // `RoomLoaded`. Content systems consume it for imperative per-room
    // staging. Requires `Messages<RoomLoaded>` to be registered (the sim
    // resources plugin does; minimal test worlds add_message it).
    commands.write_message(crate::rooms::RoomLoaded {
        room_id: room.id.clone(),
    });
}

/// Spawn one hostile actor for an encounter wave.
///
/// The encounter system still owns wave timing, but the mob itself is a normal
/// feature entity queried by actor, projectile, rendering, and health systems.
pub fn spawn_encounter_mob(
    commands: &mut Commands,
    encounter_id: impl Into<String>,
    id: String,
    brain: ambition_entity_catalog::placements::CharacterBrain,
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
