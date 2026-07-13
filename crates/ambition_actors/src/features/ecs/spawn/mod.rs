//! ECS-feature spawn facade.
//!
//! Room-level orchestration and public dynamic-mob entry points stay here, while
//! the concrete family-specific spawn helpers live in smaller sibling modules.
//! This keeps the active ECS path readable without changing the entity shapes
//! or scheduling surfaces that callers use.

use crate::boss_encounter::BossCatalog;
use crate::features::CharacterRoster;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer_primitives::lifecycle::SessionSpawnScope;
use bevy::prelude::{Commands, Entity, Query};

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

pub fn spawn_room_feature_entities(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    boss_catalog: &BossCatalog,
    room: &crate::rooms::RoomSpec,
    session_scope: SessionSpawnScope,
) {
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
    registry.register(
        ambition_entity_catalog::placements::PlacementKind::Breakable,
        super::spawn_static::lower_breakable_placement,
    );
    #[cfg(feature = "portal")]
    registry.register(
        ambition_entity_catalog::placements::PlacementKind::Portal,
        super::spawn_static::lower_portal_placement,
    );
    spawn_room_feature_entities_with_registry(
        commands,
        catalog,
        roster,
        boss_catalog,
        room,
        &registry,
        session_scope,
    );
}

/// **Re-run the spawner for ONE authored entity, by its authored id.**
///
/// `docs/planning/engine/netcode.md` N3.1 decision (3): *"restore = despawn-registered +
/// respawn from blobs … room-reset already proves the world can rebuild."* This is the
/// half that proves it. A rollback whose window spans a death has to bring the dead body
/// back, and a blob is not enough: the blob carries what the entity *became*, and only
/// the room carries what it *was*.
///
/// Keyed by the id the snapshot already holds — an authored placement's `SimId` IS its
/// LDtk iid — so nothing new has to be recorded to make a respawn possible.
///
/// Returns `false` for an id this room does not author, which is the honest answer for a
/// dynamically-spawned entity (`SimId::spawned(..)`): it has no authored record, and it
/// needs a spawn recipe of its own or a rollback window that does not span its birth.
pub fn respawn_authored_entity(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    boss_catalog: &BossCatalog,
    room: &crate::rooms::RoomSpec,
    registry: &crate::world::placements::PlacementLoweringRegistry,
    session_scope: SessionSpawnScope,
    authored_id: &str,
) -> bool {
    let paths = room_spec_paths(room);
    let lowering_context = crate::world::placements::ActorPlacementContext::new(catalog, roster);
    if let Some(record) = room
        .placements
        .iter()
        .find(|r| r.id.as_str() == authored_id)
    {
        let mut ctx = crate::world::placements::LoweringCtx {
            commands,
            room_id: &room.id,
            paths: &paths,
            session_scope,
            context: &lowering_context,
        };
        registry.lower(record, &mut ctx);
        return true;
    }
    if let Some(enemy) = room.enemy_spawns.iter().find(|e| e.id == authored_id) {
        super::spawn_actors::spawn_enemy(commands, catalog, roster, session_scope, enemy, &paths);
        return true;
    }
    if let Some(boss) = room.boss_spawns.iter().find(|b| b.id == authored_id) {
        super::spawn_actors::spawn_boss(commands, boss_catalog, session_scope, boss);
        return true;
    }
    false
}

pub fn spawn_room_feature_entities_with_registry(
    commands: &mut Commands,
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    boss_catalog: &BossCatalog,
    room: &crate::rooms::RoomSpec,
    registry: &crate::world::placements::PlacementLoweringRegistry,
    session_scope: SessionSpawnScope,
) {
    let paths = room_spec_paths(room);
    let lowering_context = crate::world::placements::ActorPlacementContext::new(catalog, roster);
    for record in &room.placements {
        let mut ctx = crate::world::placements::LoweringCtx {
            commands,
            room_id: &room.id,
            paths: &paths,
            session_scope,
            context: &lowering_context,
        };
        registry.lower(record, &mut ctx);
    }
    // Fable audit F9.2 arc EXIT: EVERY authored placement family (hazards,
    // interactables, pickups, chests, breakables, portals) now flows through the
    // single `placements` channel above — there is no second typed-Vec spawn
    // path and no dual-emit guard. Hazards with inline motion are lifted to a
    // room-level `KinematicPath` at conversion and resolved by `path_id`.
    for boss in &room.boss_spawns {
        super::spawn_actors::spawn_boss(commands, boss_catalog, session_scope, boss);
    }
    // Pickups now lower through the `placements` channel above (fable audit F9.2).
    for ground_item in &room.ground_items {
        super::spawn_static::spawn_ground_item(commands, session_scope, ground_item);
    }
    #[cfg(feature = "portal")]
    for portal_gun in &room.portal_gun_spawns {
        super::spawn_static::spawn_portal_gun_spawn(commands, session_scope, portal_gun);
    }
    // Static portals now lower through the `placements` channel above (fable
    // audit F9.2) via the cfg(portal) `lower_portal_placement` interpreter.
    for shrine in &room.shrines {
        super::spawn_static::spawn_shrine(commands, session_scope, shrine);
    }
    for gravity_zone in &room.gravity_zones {
        super::spawn_static::spawn_gravity_zone(commands, session_scope, gravity_zone);
    }
    // Chests now lower through the `placements` channel above (fable audit F9.2).
    // Breakables now lower through the `placements` channel above (fable audit F9.2).
    for enemy in &room.enemy_spawns {
        super::spawn_actors::spawn_enemy(commands, catalog, roster, session_scope, enemy, &paths);
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
    catalog: &CharacterCatalog,
    roster: &CharacterRoster,
    session_scope: SessionSpawnScope,
    encounter_id: impl Into<String>,
    id: String,
    brain: ambition_entity_catalog::placements::CharacterBrain,
    pos: ambition_engine_core::Vec2,
    size: ambition_engine_core::Vec2,
) {
    super::spawn_actors::spawn_encounter_mob(
        commands,
        catalog,
        roster,
        session_scope,
        encounter_id,
        id,
        brain,
        pos,
        size,
    );
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
