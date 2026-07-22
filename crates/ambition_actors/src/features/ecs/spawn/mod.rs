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
pub use content_staging::{
    RoomContentStagingError, RoomContentStagingRegistrationError, RoomContentStagingRegistry,
};

pub(crate) use super::spawn_actors::{spawn_runtime_minion, spawn_runtime_minion_into};

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
    DuplicateAuthoritativeId {
        room: String,
        id: String,
    },
    /// The planned families (authored ground items, provider-staged actors)
    /// could not be resolved into a valid construction plan.
    Construction(ambition_platformer_primitives::construction::ConstructionError),
    /// A planned family's parameters could not be resolved from content — an
    /// authored ground item naming a held item no registry provides.
    ActorConstruction(crate::construction::ActorConstructionError),
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
            Self::Construction(error) => error.fmt(f),
            Self::ActorConstruction(error) => error.fmt(f),
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
    content_requests: Vec<super::spawn_actors::SpawnActorRequest>,
    /// The three planned origin families (Phase 3): authored ground items and
    /// provider-staged actors, planned here; summoned minions plan the same way
    /// at the moment they are summoned. Everything else in this room is still
    /// constructed by the family-specific loops in [`Self::spawn`], which
    /// Phase 4 migrates.
    construction: crate::construction::ActorConstructionPlan,
    /// The frozen catalogs this plan reads — character catalog, hostile roster,
    /// boss profiles. THE copy: the recipes read it through
    /// `ConstructionExecCtx`, and the family-specific loops in [`Self::spawn`]
    /// read it directly, so a cached plan holds one of each rather than a pair.
    construction_services: crate::construction::ActorConstructionServices,
    expected_authoritative_ids: BTreeSet<String>,
}

/// What construction planning needs beyond the room's authored content: the
/// recipe table, and the content generation the plan is being prepared against.
#[derive(Clone, Copy)]
pub struct ActorConstructionContext<'a> {
    pub recipes: &'a crate::construction::ActorConstructionRegistry,
    /// Which generation of prepared content this room plan is bound to. A room
    /// is always content-derived, so this is always
    /// [`ContentBinding::Content`] — the enum exists because the planner also
    /// serves runtime-dynamic construction, which is not.
    pub binding: ambition_platformer_primitives::construction::ContentBinding,
}

impl<'a> ActorConstructionContext<'a> {
    pub fn new(
        recipes: &'a crate::construction::ActorConstructionRegistry,
        content_epoch: ambition_engine_core::ContentEpoch,
    ) -> Self {
        Self {
            recipes,
            binding: ambition_platformer_primitives::construction::ContentBinding::Content(
                content_epoch,
            ),
        }
    }
}

/// Inspectable receipt for the authoritative roots scheduled by one feature plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomFeatureConstructionReceipt {
    authoritative_ids: BTreeSet<String>,
    construction: ambition_platformer_primitives::construction::ConstructionReceipt,
}

impl RoomFeatureConstructionReceipt {
    pub fn authoritative_ids(&self) -> &BTreeSet<String> {
        &self.authoritative_ids
    }

    /// What the Phase-3 planned families actually committed, keyed by identity.
    /// Compared against the plan's roster to prove plan-to-world parity.
    pub fn construction(
        &self,
    ) -> &ambition_platformer_primitives::construction::ConstructionReceipt {
        &self.construction
    }
}

/// A room plan's `Debug` leads with the construction plan's canonical dump —
/// the roster it would commit — because that is what is worth reading when a
/// room appears in a failure message.
impl std::fmt::Debug for RoomFeatureConstructionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomFeatureConstructionPlan")
            .field("room", &self.room.id)
            .field(
                "expected_authoritative_ids",
                &self.expected_authoritative_ids,
            )
            .field("construction", &self.construction)
            .finish()
    }
}

impl RoomFeatureConstructionPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        room: &crate::rooms::RoomSpec,
        registry: &crate::world::placements::PlacementLoweringRegistry,
        content_staging: &RoomContentStagingRegistry,
        catalog: &CharacterCatalog,
        roster: &CharacterRoster,
        boss_catalog: &BossCatalog,
        construction: ActorConstructionContext<'_>,
    ) -> Result<Self, RoomFeatureConstructionError> {
        let paths = room_spec_paths(room);
        let placements = registry
            .plan_room(&room.id, &paths, &room.placements)
            .map_err(RoomFeatureConstructionError::Placement)?;
        let owned_content_requests = content_staging
            .try_owned_requests_for(room)
            .map_err(RoomFeatureConstructionError::ContentStaging)?;
        let content_requests: Vec<_> = owned_content_requests
            .iter()
            .map(|(_, request)| request.clone())
            .collect();
        let authoritative_ids = room
            .placements
            .iter()
            .map(|placement| placement.id.0.clone())
            .chain(room.enemy_spawns.iter().map(|enemy| enemy.id.clone()))
            .chain(room.boss_spawns.iter().map(|boss| boss.id.clone()))
            .chain(room.ground_items.iter().map(|item| item.id.clone()))
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

        // The planned families. Resolution failures (an authored ground item
        // naming a held item nothing provides) and identity/relation failures
        // surface HERE, while the outgoing room is still whole.
        let mut requests = crate::construction::authored_ground_item_requests(room)
            .map_err(RoomFeatureConstructionError::ActorConstruction)?;
        for (provider, request) in &owned_content_requests {
            requests.extend(crate::construction::staged_actor_requests(
                &room.id,
                provider,
                std::slice::from_ref(request),
            ));
        }
        let construction_plan = crate::construction::ActorConstructionPlan::prepare(
            ambition_platformer_primitives::construction::ConstructionScope {
                binding: construction.binding,
                room: Some(room.id.clone()),
            },
            requests,
            // A room plan is prepared against the room it replaces, so nothing
            // it constructs is live yet by definition.
            &Default::default(),
            construction.recipes,
        )
        .map_err(RoomFeatureConstructionError::Construction)?;

        let placement_context =
            crate::world::placements::ActorPlacementContext::new(catalog, roster);
        Ok(Self {
            room: room.clone(),
            paths,
            placements,
            construction_services: crate::construction::ActorConstructionServices {
                context: placement_context,
                boss_catalog: boss_catalog.clone(),
            },
            content_requests,
            construction: construction_plan,
            expected_authoritative_ids,
        })
    }

    /// The Phase-3 construction plan for this room's planned families.
    pub fn construction(&self) -> &crate::construction::ActorConstructionPlan {
        &self.construction
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

    /// Rebuild one authored authoritative root through the exact interpreter
    /// and catalogs frozen by this plan.
    ///
    /// For the planned families this is [`ConstructionPlan::construct_one`] —
    /// the SAME recipe ordinary construction runs, which is the property Phase 3
    /// exists to buy. The remaining families still take the family-specific
    /// branches below; Phase 4 migrates them.
    pub fn respawn_authoritative_entity(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        authored_id: &str,
    ) -> bool {
        let planned_id = ambition_platformer_primitives::sim_id::SimId::placement(authored_id);
        if self.construction.get(&planned_id).is_some() {
            let mut ctx = ambition_platformer_primitives::construction::ConstructionExecCtx {
                commands,
                scope: self.construction.scope(),
                session: session_scope,
                services: &self.construction_services,
            };
            return match self.construction.construct_one(&planned_id, &mut ctx) {
                Ok(_) => true,
                Err(error) => {
                    // This row IS planned, so falling through to the other
                    // families would be wrong — and returning a bare `false`
                    // would report "no such entity" for what is really a
                    // refusal. A relation-bearing row cannot be rebuilt alone
                    // (see `ConstructionError::RelationOutsideSubset`); saying
                    // so is the whole value of the refusal.
                    bevy::log::error!(
                        target: "ambition::construction",
                        "`{authored_id}` is planned but could not be rebuilt on its own: {error}"
                    );
                    false
                }
            };
        }
        if self.placements.lower_one(
            commands,
            session_scope,
            &self.construction_services.context,
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
                &self.construction_services.context.characters,
                &self.construction_services.context.roster,
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
            super::spawn_actors::spawn_boss(
                commands,
                &self.construction_services.boss_catalog,
                session_scope,
                boss,
            );
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
            .lower_all(commands, session_scope, &self.construction_services.context);
        for boss in &self.room.boss_spawns {
            super::spawn_actors::spawn_boss(
                commands,
                &self.construction_services.boss_catalog,
                session_scope,
                boss,
            );
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
                &self.construction_services.context.characters,
                &self.construction_services.context.roster,
                session_scope,
                enemy,
                &self.paths,
            );
        }
        commands.insert_resource(crate::features::PendingMountLinks(
            self.room.mount_links.clone(),
        ));
        commands.insert_resource(crate::features::FactionRelations::default());

        // The planned families commit through the one planner. Provider-staged
        // actors used to be written as `SpawnActorRequest` MESSAGES and applied
        // a system later; they are constructed here instead, so a room's
        // occupants all exist at the same instant and a staged actor is a plan
        // row rather than a deferred side effect.
        let construction = {
            let mut ctx = ambition_platformer_primitives::construction::ConstructionExecCtx {
                commands,
                scope: self.construction.scope(),
                session: session_scope,
                services: &self.construction_services,
            };
            self.construction.commit(&mut ctx)
        };
        debug_assert_eq!(
            construction.committed_ids(),
            self.construction.planned_ids(),
            "construction execution diverged from its prepared roster",
        );

        commands.write_message(crate::rooms::RoomLoaded {
            room_id: self.room.id.clone(),
        });
        RoomFeatureConstructionReceipt {
            authoritative_ids: self.expected_authoritative_ids.clone(),
            construction,
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
