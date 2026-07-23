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
        // Authored-id uniqueness across every family, checked in the RAW authored
        // namespace while the outgoing room is still whole. This stays separate
        // from the plan-derived roster built below: several families (placements,
        // bosses, non-giant enemies) are not plan rows yet, so their ids are only
        // knowable from the `RoomSpec` here.
        let authored_ids = room
            .placements
            .iter()
            .map(|placement| placement.id.0.clone())
            .chain(room.enemy_spawns.iter().map(|enemy| enemy.id.clone()))
            .chain(room.boss_spawns.iter().map(|boss| boss.id.clone()))
            .chain(room.ground_items.iter().map(|item| item.id.clone()))
            .chain(content_requests.iter().map(|request| request.id.clone()));
        let mut seen_authored_ids = BTreeSet::new();
        for id in authored_ids {
            if !seen_authored_ids.insert(id.clone()) {
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
                roster,
            ));
        }
        // Phase 4a/4b: EVERY authored enemy and boss is a plan row — ordinary
        // enemies as `AuthoredEnemy`, `"giant"`-class hosts as host + two hand
        // rows joined by limb relations, bosses as `AuthoredBoss`. The family
        // loops that used to build these in `spawn` are deleted.
        requests.extend(crate::construction::authored_actor_requests(
            room, roster, &paths,
        ));
        // Authored mount links are planned `ambition.mount` relations between
        // those rows; a link naming nobody fails HERE instead of being retried
        // forever by the deleted frame-later resolver.
        crate::construction::attach_authored_mount_links(room, &mut requests)
            .map_err(RoomFeatureConstructionError::ActorConstruction)?;
        // Actor-domain relation semantics, checked while the outgoing room is
        // still whole: cardinality (one host per limb, one rider per mount),
        // family legality, and pilot/mount class compatibility. The generic
        // planner below enforces the structural rules; these are the ones only
        // this domain can state.
        crate::construction::preflight_actor_relations(&requests, roster, boss_catalog)
            .map_err(RoomFeatureConstructionError::ActorConstruction)?;
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

        // The authoritative roster the room PREDICTS, derived from the completed
        // plan rather than re-enumerated by hand: `planned_ids()` covers every
        // migrated family INCLUDING giant hands (a `SimId::spawned` row absent
        // from the authored id list above), all in the one `SimId` namespace. The
        // families that are still separate spawn loops are unioned in explicitly
        // by [`non_plan_authoritative_ids`] and are the Phase-4 migration surface.
        let mut expected_authoritative_ids: BTreeSet<String> = construction_plan
            .planned_ids()
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        expected_authoritative_ids.extend(non_plan_authoritative_ids(room, roster));

        let placement_context =
            crate::world::placements::ActorPlacementContext::new(catalog, roster);
        Ok(Self {
            room: room.clone(),
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
            return self.respawn_authoritative_sim_id(commands, session_scope, &planned_id);
        }
        if self.placements.lower_one(
            commands,
            session_scope,
            &self.construction_services.context,
            authored_id,
        ) {
            return true;
        }
        // Phase 4a/4b: no enemy or boss fallback — both families are plan
        // rows, so the planned branch above already covers every authored id
        // they own. The one remaining family-specific path is placements.
        false
    }

    /// Rebuild one PLANNED authoritative root by its stable identity — the form
    /// that can name a derived row.
    ///
    /// [`Self::respawn_authoritative_entity`] converts its authored id through
    /// `SimId::placement`, which can never spell a `SimId::spawned` identity —
    /// so a giant's HAND was planned, closable, and yet unreachable through the
    /// production API. Dynamic and derived authoritative roots need
    /// reconstruction exactly as much as authored ones; this is their entry
    /// point, and the authored-id form is now a convenience wrapper over the
    /// same closure commit.
    ///
    /// Rebuilds the RELATION CLOSURE, not the bare row. A row at either end of a
    /// planned relation cannot be rebuilt alone — rebuilding one end strands the
    /// other on a dead `Entity` handle
    /// (`ConstructionError::RelationCutBySubset`). A giant host and its two
    /// hands are exactly such a cluster: asking for ANY one of the three — host,
    /// left hand, right hand — rebuilds all three. For an unrelated row the
    /// closure is just itself, so this is a plain single-row commit.
    pub fn respawn_authoritative_sim_id(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
        sim_id: &ambition_platformer_primitives::sim_id::SimId,
    ) -> bool {
        if self.construction.get(sim_id).is_none() {
            return false;
        }
        let closure = self
            .construction
            .relation_closure(&std::collections::BTreeSet::from([sim_id.clone()]));
        let mut ctx = ambition_platformer_primitives::construction::ConstructionExecCtx {
            commands,
            scope: self.construction.scope(),
            session: session_scope,
            services: &self.construction_services,
        };
        match self.construction.commit_subset(&closure, &mut ctx) {
            Ok(_) => true,
            Err(error) => {
                bevy::log::error!(
                    target: "ambition::construction",
                    "`{sim_id}` is planned but its reconstruction closure could not be rebuilt: \
                     {error}"
                );
                false
            }
        }
    }

    /// Apply the exact feature decisions captured by [`Self::prepare`].
    ///
    /// **This does not publish the room, and does not verify it.** It used to do
    /// both, bracketing its own work with a baseline capture and a
    /// verify-and-publish. That boundary was in the wrong place: this function's
    /// CALLER queues moving platforms and the last-commit receipt after it
    /// returns, and command queues apply in insertion order, so `RoomLoaded` was
    /// written before the room was finished being built. A feature plan is one
    /// participant in a room transaction, not the transaction, so it cannot know
    /// when the room is complete. The bracket lives with the outer artifact that
    /// does — see [`crate::world::rooms::transaction`].
    pub fn spawn(
        &self,
        commands: &mut Commands,
        session_scope: SessionSpawnScope,
    ) -> RoomFeatureConstructionReceipt {
        self.placements
            .lower_all(commands, session_scope, &self.construction_services.context);
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
        // Phase 4a/4b: enemies and bosses are plan rows, committed below with
        // their relations. No family loop remains for either.
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

        // The COMMITTED roster: what the plan families actually built
        // (`committed_ids()`, giant hands included) unioned with the same
        // not-yet-migrated families the prediction counted. Sharing
        // `non_plan_authoritative_ids` with `prepare` means the outer
        // predicted-vs-committed cross-check (`stage::spawn_contents`) reduces to
        // "did every plan row commit", the one comparison that can differ.
        let mut authoritative_ids: BTreeSet<String> = construction
            .committed_ids()
            .iter()
            .map(std::string::ToString::to_string)
            .collect();
        authoritative_ids.extend(non_plan_authoritative_ids(
            &self.room,
            &self.construction_services.context.roster,
        ));

        RoomFeatureConstructionReceipt {
            authoritative_ids,
            construction,
        }
    }
}

/// The authoritative identities of the families that are NOT construction plan
/// rows yet — placements, bosses, and non-giant enemies — in the `SimId` string
/// namespace the plan uses, so a unified roster does not drift on spelling.
///
/// This is the Phase-4 migration surface. Every family here is still built by its
/// own spawn loop rather than as a plan row, and its body receives its `SimId`
/// AFTER the boundary verifier runs (`ensure_sim_id`), so these identities are
/// invisible to `AuthoritativeScope::gather` at verification time — the honest
/// "incomplete visibility for legacy families" the campaign doc records. As each
/// family becomes a plan row it leaves this function and appears in
/// `planned_ids()` on its own. Giant hosts and mount-link participants (rider
/// bosses included) are already plan rows, so the enumerations skip them to
/// avoid a second spelling of the same identity.
fn non_plan_authoritative_ids(
    room: &crate::rooms::RoomSpec,
    _roster: &CharacterRoster,
) -> BTreeSet<String> {
    use ambition_platformer_primitives::sim_id::SimId;
    // Phase 4a/4b removed enemies and bosses from this enumeration; authored
    // placements are the one authoritative family still outside the planner.
    room.placements
        .iter()
        .map(|placement| SimId::placement(&placement.id.0).to_string())
        .collect()
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
