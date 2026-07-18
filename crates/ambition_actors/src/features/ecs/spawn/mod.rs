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
use bevy::prelude::Commands;
use std::collections::BTreeSet;

mod content_staging;
pub use content_staging::{RoomContentStagingError, RoomContentStagingRegistry};

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

/// A mutation-free room feature construction failure.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomFeatureConstructionError {
    Placement(crate::world::placements::PlacementLoweringError),
    ContentStaging(RoomContentStagingError),
    DuplicateAuthoritativeId { room: String, id: String },
}

impl std::fmt::Display for RoomFeatureConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Placement(error) => error.fmt(f),
            Self::ContentStaging(error) => error.fmt(f),
            Self::DuplicateAuthoritativeId { room, id } => write!(
                f,
                "room `{room}` constructs authoritative id `{id}` more than once",
            ),
        }
    }
}

impl std::error::Error for RoomFeatureConstructionError {}

/// The complete feature-side artifact prepared before a room mutation begins.
///
/// Interpreter lookup, path flattening, content-stager execution, roster
/// validation, and catalog selection all happen here. Execution only applies
/// these frozen decisions, so startup, reset, transition, hot reload, and
/// restore cannot drift into different room-construction behavior.
#[derive(Clone)]
pub struct RoomFeatureConstructionPlan {
    room: crate::rooms::RoomSpec,
    paths: Vec<(String, ambition_engine_core::KinematicPath)>,
    placements: crate::world::placements::PlacementLoweringPlan<
        crate::world::placements::ActorPlacementContext,
    >,
    placement_context: crate::world::placements::ActorPlacementContext,
    boss_catalog: BossCatalog,
    content_requests: Vec<super::spawn_actors::SpawnActorRequest>,
    expected_authoritative_ids: BTreeSet<String>,
}

/// Inspectable receipt for the authoritative roots scheduled by one feature plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomFeatureConstructionReceipt {
    authoritative_ids: BTreeSet<String>,
}

impl RoomFeatureConstructionReceipt {
    pub fn authoritative_ids(&self) -> &BTreeSet<String> {
        &self.authoritative_ids
    }
}

impl RoomFeatureConstructionPlan {
    pub fn prepare(
        room: &crate::rooms::RoomSpec,
        registry: &crate::world::placements::PlacementLoweringRegistry,
        content_staging: &RoomContentStagingRegistry,
        catalog: &CharacterCatalog,
        roster: &CharacterRoster,
        boss_catalog: &BossCatalog,
    ) -> Result<Self, RoomFeatureConstructionError> {
        let paths = room_spec_paths(room);
        let placements = registry
            .plan_room(&room.id, &paths, &room.placements)
            .map_err(RoomFeatureConstructionError::Placement)?;
        let content_requests = content_staging
            .try_requests_for(room)
            .map_err(RoomFeatureConstructionError::ContentStaging)?;
        let authoritative_ids = room
            .placements
            .iter()
            .map(|placement| placement.id.0.clone())
            .chain(room.enemy_spawns.iter().map(|enemy| enemy.id.clone()))
            .chain(room.boss_spawns.iter().map(|boss| boss.id.clone()))
            .chain(content_requests.iter().map(|request| request.id.clone()));
        let mut expected_authoritative_ids = BTreeSet::new();
        for id in authoritative_ids {
            if !expected_authoritative_ids.insert(id.clone()) {
                return Err(RoomFeatureConstructionError::DuplicateAuthoritativeId {
                    room: room.id.clone(),
                    id,
                });
            }
        }
        Ok(Self {
            room: room.clone(),
            paths,
            placements,
            placement_context: crate::world::placements::ActorPlacementContext::new(
                catalog, roster,
            ),
            boss_catalog: boss_catalog.clone(),
            content_requests,
            expected_authoritative_ids,
        })
    }

    pub fn room(&self) -> &crate::rooms::RoomSpec {
        &self.room
    }

    pub fn expected_authoritative_ids(&self) -> &BTreeSet<String> {
        &self.expected_authoritative_ids
    }

    pub fn content_staged_names(&self) -> Vec<String> {
        self.content_requests
            .iter()
            .map(|request| request.name.clone())
            .collect()
    }

    pub fn content_staged_requests(&self) -> &[super::spawn_actors::SpawnActorRequest] {
        &self.content_requests
    }

    /// Rebuild one authored authoritative root through the exact interpreter
    /// and catalogs frozen by this plan.
    pub fn respawn_authoritative_entity(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        authored_id: &str,
    ) -> bool {
        if self.placements.lower_one(
            commands,
            session_scope,
            &self.placement_context,
            authored_id,
        ) {
            return true;
        }
        if let Some(enemy) = self
            .room
            .enemy_spawns
            .iter()
            .find(|enemy| enemy.id == authored_id)
        {
            super::spawn_actors::spawn_enemy(
                commands,
                &self.placement_context.characters,
                &self.placement_context.roster,
                session_scope,
                enemy,
                &self.paths,
            );
            return true;
        }
        if let Some(boss) = self
            .room
            .boss_spawns
            .iter()
            .find(|boss| boss.id == authored_id)
        {
            super::spawn_actors::spawn_boss(commands, &self.boss_catalog, session_scope, boss);
            return true;
        }
        false
    }

    /// Apply the exact feature decisions captured by [`Self::prepare`].
    pub fn spawn(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
    ) -> RoomFeatureConstructionReceipt {
        self.placements
            .lower_all(commands, session_scope, &self.placement_context);
        for boss in &self.room.boss_spawns {
            super::spawn_actors::spawn_boss(commands, &self.boss_catalog, session_scope, boss);
        }
        for ground_item in &self.room.ground_items {
            super::spawn_static::spawn_ground_item(commands, session_scope, ground_item);
        }
        #[cfg(feature = "portal")]
        for portal_gun in &self.room.portal_gun_spawns {
            super::spawn_static::spawn_portal_gun_spawn(commands, session_scope, portal_gun);
        }
        for shrine in &self.room.shrines {
            super::spawn_static::spawn_shrine(commands, session_scope, shrine);
        }
        for gravity_zone in &self.room.gravity_zones {
            super::spawn_static::spawn_gravity_zone(commands, session_scope, gravity_zone);
        }
        for enemy in &self.room.enemy_spawns {
            super::spawn_actors::spawn_enemy(
                commands,
                &self.placement_context.characters,
                &self.placement_context.roster,
                session_scope,
                enemy,
                &self.paths,
            );
        }
        commands.insert_resource(crate::features::PendingMountLinks(
            self.room.mount_links.clone(),
        ));
        commands.insert_resource(crate::features::FactionRelations::default());
        for request in &self.content_requests {
            commands.write_message(request.clone());
        }
        commands.write_message(crate::rooms::RoomLoaded {
            room_id: self.room.id.clone(),
        });
        RoomFeatureConstructionReceipt {
            authoritative_ids: self.expected_authoritative_ids.clone(),
        }
    }
}

/// Execute a previously prepared feature plan.
pub fn spawn_room_feature_entities_from_plan(
    commands: &mut Commands,
    plan: &RoomFeatureConstructionPlan,
    session_scope: SessionSpawnScope,
) -> RoomFeatureConstructionReceipt {
    plan.spawn(commands, session_scope)
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

#[cfg(test)]
mod tests;
