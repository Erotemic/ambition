//! Canonical prepared room construction.
//!
//! Every room lifecycle path prepares the same mutation-free
//! [`RoomConstructionPlan`] before it retires live entities. The plan freezes
//! target identity, authored geometry, resolved placement interpreters,
//! content-staged actor requests, catalogs, moving-platform starts, and the
//! expected authoritative roster. Startup, reset, ordinary transition, LDtk
//! hot reload, and snapshot reconstruction execute this one artifact.
//!
//! The file retains its historical `stage.rs` path, but there is no longer a
//! restore-only staging API. Snapshot reconstruction is a use of canonical construction,
//! not a second construction authority.

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::world::World;
use bevy::prelude::{Commands, Resource};

use super::{RespawnRoomVisualsRequested, RoomSet, RoomSpec};
use crate::features::{self, RoomFeatureConstructionPlan};
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::placements::PlacementLoweringRegistry;
use crate::world::platforms::{self, MovingPlatformState};
use ambition_platformer_primitives::lifecycle::{
    session_world_component, session_world_component_mut, ActiveSessionScope, SessionSpawnScope,
};

/// Stable same-build identity for one prepared construction artifact.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RoomConstructionPlanId(String);

impl RoomConstructionPlanId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why canonical room construction could not be prepared. Every variant is
/// detected before any live-room mutation.
#[derive(Clone, Debug, PartialEq)]
pub enum RoomConstructionError {
    UnknownRoom { room: String },
    MissingService { service: &'static str },
    InvalidFeatures {
        room: String,
        reason: features::RoomFeatureConstructionError,
    },
}

impl std::fmt::Display for RoomConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownRoom { room } => write!(
                f,
                "no room named `{room}` in the prepared RoomSet"
            ),
            Self::MissingService { service } => write!(
                f,
                "room construction needs `{service}`, which this world does not provide"
            ),
            Self::InvalidFeatures { room, reason } => {
                write!(f, "room `{room}` construction is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for RoomConstructionError {}

/// Last successfully scheduled room-construction commit.
///
/// This is developer evidence, not simulation authority: the active RoomSet and
/// spawned ECS entities remain authoritative. It lets diagnostics and tests join
/// a committed room to the exact immutable plan and root roster that produced it.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct LastRoomConstructionCommit {
    pub plan_id: RoomConstructionPlanId,
    pub room_id: String,
    pub authoritative_ids: BTreeSet<String>,
    pub moving_platform_count: usize,
}

/// The one prepared artifact for a room's authoritative simulation contents.
#[derive(Clone)]
pub struct RoomConstructionPlan {
    id: RoomConstructionPlanId,
    target_index: usize,
    features: RoomFeatureConstructionPlan,
    platform_states: Vec<MovingPlatformState>,
    session_scope: SessionSpawnScope,
}

impl std::fmt::Debug for RoomConstructionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomConstructionPlan")
            .field("id", &self.id)
            .field("target_index", &self.target_index)
            .field("room", &self.features.room().id)
            .field(
                "expected_authoritative_ids",
                self.features.expected_authoritative_ids(),
            )
            .field("platform_count", &self.platform_states.len())
            .field("session_scope", &self.session_scope)
            .finish()
    }
}

impl RoomConstructionPlan {
    /// Prepare from the canonical resources in an exclusive `World`.
    pub fn prepare(world: &World, target_room_id: &str) -> Result<Self, RoomConstructionError> {
        let missing = |service| RoomConstructionError::MissingService { service };
        let rooms = session_world_component::<RoomSet>(world).ok_or(missing("session RoomSet"))?;
        let target_index = rooms
            .room_index_by_id(target_room_id)
            .ok_or_else(|| RoomConstructionError::UnknownRoom {
                room: target_room_id.to_string(),
            })?;
        session_world_component::<ambition_engine_core::RoomGeometry>(world)
            .ok_or(missing("session RoomGeometry"))?;
        if world
            .get_resource::<ambition_world::collision::MovingPlatformSet>()
            .is_none()
        {
            return Err(missing("MovingPlatformSet"));
        }
        let session_scope = SessionSpawnScope::for_optional_active_session(
            world.get_resource::<ActiveSessionScope>(),
        )
        .ok_or(missing("ActiveSessionScope"))?;
        let content_staging = world
            .get_resource::<features::RoomContentStagingRegistry>()
            .cloned()
            .unwrap_or_default();
        Self::prepare_from_parts(
            rooms,
            target_index,
            world
                .get_resource::<PlacementLoweringRegistry>()
                .ok_or(missing("PlacementLoweringRegistry"))?,
            &content_staging,
            world
                .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
                .ok_or(missing("CharacterCatalog"))?,
            world
                .get_resource::<features::CharacterRoster>()
                .ok_or(missing("CharacterRoster"))?,
            world
                .get_resource::<crate::boss_encounter::BossCatalog>()
                .ok_or(missing("BossCatalog"))?,
            session_scope,
        )
    }

    /// Prepare from already-borrowed services. This is the system-facing seam
    /// used by activation, reset, transition, and hot reload.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_from_parts(
        rooms: &RoomSet,
        target_index: usize,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
        character_roster: &features::CharacterRoster,
        boss_catalog: &crate::boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
    ) -> Result<Self, RoomConstructionError> {
        let spec = rooms
            .rooms
            .get(target_index)
            .cloned()
            .ok_or_else(|| RoomConstructionError::UnknownRoom {
                room: format!("<room-index-{target_index}>"),
            })?;
        Self::prepare_spec(
            target_index,
            spec,
            placement_lowering,
            content_staging,
            character_catalog,
            character_roster,
            boss_catalog,
            session_scope,
        )
    }

    /// Prepare a room whose containing `RoomSet` is itself a candidate artifact,
    /// as in transactional LDtk hot reload.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_spec(
        target_index: usize,
        spec: RoomSpec,
        placement_lowering: &PlacementLoweringRegistry,
        content_staging: &features::RoomContentStagingRegistry,
        character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
        character_roster: &features::CharacterRoster,
        boss_catalog: &crate::boss_encounter::BossCatalog,
        session_scope: SessionSpawnScope,
    ) -> Result<Self, RoomConstructionError> {
        let feature_plan = RoomFeatureConstructionPlan::prepare(
            &spec,
            placement_lowering,
            content_staging,
            character_catalog,
            character_roster,
            boss_catalog,
        )
        .map_err(|reason| RoomConstructionError::InvalidFeatures {
            room: spec.id.clone(),
            reason,
        })?;
        let platform_states = platforms::moving_platforms_for_room(&spec);
        let id = construction_plan_id(&spec, feature_plan.expected_authoritative_ids());
        Ok(Self {
            id,
            target_index,
            features: feature_plan,
            platform_states,
            session_scope,
        })
    }

    pub fn id(&self) -> &RoomConstructionPlanId {
        &self.id
    }

    pub fn target_index(&self) -> usize {
        self.target_index
    }

    pub fn room_id(&self) -> &str {
        &self.features.room().id
    }

    pub fn spec(&self) -> &RoomSpec {
        self.features.room()
    }

    /// Whether a currently installed room definition is byte-for-byte the
    /// authored spec this plan prepared. This rejects a same-id hot reload from
    /// committing a stale in-flight transition.
    pub fn matches_room_spec(&self, candidate: &RoomSpec) -> bool {
        let prepared = serde_json::to_vec(self.spec())
            .expect("prepared RoomSpec must remain serializable");
        let current = serde_json::to_vec(candidate)
            .expect("candidate RoomSpec must remain serializable");
        prepared == current
    }

    pub fn platform_states(&self) -> &[MovingPlatformState] {
        &self.platform_states
    }

    pub fn predicted_authoritative_ids(&self) -> &BTreeSet<String> {
        self.features.expected_authoritative_ids()
    }

    pub fn content_staged_names(&self) -> Vec<String> {
        self.features.content_staged_names()
    }

    pub fn content_staged_requests(&self) -> &[features::SpawnActorRequest] {
        self.features.content_staged_requests()
    }

    /// Rebuild one authored authoritative root through this plan's frozen
    /// interpreter/catalog decisions.
    pub fn respawn_authoritative_entity(
        &self,
        commands: &mut Commands,
        authored_id: &str,
    ) -> bool {
        self.features
            .respawn_authoritative_entity(commands, self.session_scope, authored_id)
    }

    pub fn session_scope(&self) -> SessionSpawnScope {
        self.session_scope
    }

    /// Enqueue the prepared room contents without changing active-room
    /// resources. Session startup uses this after those resources are installed.
    pub fn spawn_contents(&self, commands: &mut Commands) {
        let receipt = features::spawn_room_feature_entities_from_plan(
            commands,
            &self.features,
            self.session_scope,
        );
        debug_assert_eq!(
            receipt.authoritative_ids(),
            self.predicted_authoritative_ids(),
            "room construction execution diverged from its prepared root roster",
        );
        platforms::spawn_moving_platforms(
            commands,
            self.session_scope,
            &self.spec().world,
            &self.platform_states,
        );
        commands.insert_resource(LastRoomConstructionCommit {
            plan_id: self.id.clone(),
            room_id: self.room_id().to_string(),
            authoritative_ids: receipt.authoritative_ids().clone(),
            moving_platform_count: self.platform_states.len(),
        });
    }

    /// Retire the outgoing room's scoped entities. The transiting possessed
    /// body may be carried across the boundary instead of being retired.
    pub fn retire_outgoing<'a>(
        &self,
        commands: &mut Commands,
        outgoing: impl IntoIterator<Item = (Entity, bool)> + 'a,
        carry_body: Option<Entity>,
    ) {
        for (entity, is_physics) in outgoing {
            if carry_body == Some(entity) {
                continue;
            }
            if is_physics {
                physics::retire_physics_entity(commands, entity);
            } else {
                commands.entity(entity).despawn();
            }
        }
    }

    /// Publish target geometry/platform state and enqueue the exact frozen room
    /// contents. Call only after every preflight has succeeded.
    pub fn commit_deferred(
        &self,
        commands: &mut Commands,
        rooms: &mut RoomSet,
        geometry: &mut ambition_engine_core::RoomGeometry,
        moving_platforms: &mut Vec<MovingPlatformState>,
    ) {
        rooms.set_active(self.target_index);
        geometry.0 = self.spec().world.clone();
        *moving_platforms = self.platform_states.clone();
        self.spawn_contents(commands);
    }

    /// Exclusive-world execution for snapshot reconstruction. Preparation is
    /// mutation-free; after it returns, this application has no fallible lookup.
    pub fn apply_to_world(self, world: &mut World) {
        if let Some(mut pending) =
            world.get_resource_mut::<bevy::ecs::message::Messages<features::SpawnActorRequest>>()
        {
            pending.clear();
        }

        let outgoing: Vec<(Entity, bool)> = match world
            .try_query_filtered::<(Entity, Option<&PhysicsRoomEntity>), With<RoomScopedEntity>>()
        {
            Some(mut query) => query
                .iter(world)
                .map(|(entity, physics)| (entity, physics.is_some()))
                .collect(),
            None => Vec::new(),
        };
        {
            let mut commands = world.commands();
            self.retire_outgoing(&mut commands, outgoing, None);
        }
        world.flush();

        if let Some(mut rooms) = session_world_component_mut::<RoomSet>(world) {
            rooms.set_active(self.target_index);
        }
        if let Some(mut geometry) =
            session_world_component_mut::<ambition_engine_core::RoomGeometry>(world)
        {
            geometry.0 = self.spec().world.clone();
        }
        if let Some(mut set) =
            world.get_resource_mut::<ambition_world::collision::MovingPlatformSet>()
        {
            set.0 = self.platform_states.clone();
        }
        {
            let mut commands = world.commands();
            self.spawn_contents(&mut commands);
        }
        world.flush();
        let _ = bevy::ecs::system::RunSystemOnce::run_system_once(
            &mut *world,
            features::apply_spawn_actor_requests,
        );
        world.flush();
        if let Some(mut messages) =
            world.get_resource_mut::<bevy::ecs::message::Messages<RespawnRoomVisualsRequested>>()
        {
            messages.write(RespawnRoomVisualsRequested);
        }
    }
}

fn construction_plan_id(
    spec: &RoomSpec,
    expected_ids: &BTreeSet<String>,
) -> RoomConstructionPlanId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    spec.id.hash(&mut hasher);
    // RoomSpec is the canonical authored room artifact; JSON avoids depending
    // on map insertion order because its fields are vectors/ordered values.
    serde_json::to_vec(spec)
        .expect("RoomSpec serialization must succeed for construction identity")
        .hash(&mut hasher);
    expected_ids.hash(&mut hasher);
    RoomConstructionPlanId(format!("room-plan:{:016x}", hasher.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_engine_core as ae;

    fn empty_spec(id: &str) -> RoomSpec {
        RoomSpec::new(
            id,
            ae::World::new(
                id,
                ae::Vec2::new(640.0, 480.0),
                ae::Vec2::new(96.0, 96.0),
                Vec::new(),
            ),
        )
    }

    fn prepare(spec: RoomSpec) -> Result<RoomConstructionPlan, RoomConstructionError> {
        RoomConstructionPlan::prepare_spec(
            0,
            spec,
            &PlacementLoweringRegistry::default(),
            &features::RoomContentStagingRegistry::default(),
            &ambition_characters::actor::character_catalog::CharacterCatalog::empty(),
            &features::CharacterRoster::default(),
            &crate::boss_encounter::BossCatalog::default(),
            SessionSpawnScope::UNSCOPED,
        )
    }

    #[test]
    fn equivalent_room_construction_has_stable_identity() {
        let a = prepare(empty_spec("same")).expect("first plan");
        let b = prepare(empty_spec("same")).expect("second plan");
        assert_eq!(a.id(), b.id());
        assert_eq!(a.predicted_authoritative_ids(), b.predicted_authoritative_ids());
    }

    #[test]
    fn duplicate_authoritative_roots_fail_before_commit() {
        let mut spec = empty_spec("duplicate");
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0));
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "same-id",
            "first",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Combatant,
        ));
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "same-id",
            "second",
            aabb,
            ambition_entity_catalog::placements::CharacterBrain::Combatant,
        ));
        let error = prepare(spec).expect_err("duplicate roots must fail preparation");
        assert!(matches!(
            error,
            RoomConstructionError::InvalidFeatures {
                reason: features::RoomFeatureConstructionError::DuplicateAuthoritativeId { .. },
                ..
            }
        ));
    }

    #[test]
    fn commit_receipt_matches_the_prepared_root_roster() {
        let mut spec = empty_spec("receipt");
        spec.enemy_spawns.push(crate::rooms::Authored::new(
            "enemy-1",
            "enemy",
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::splat(16.0)),
            ambition_entity_catalog::placements::CharacterBrain::Combatant,
        ));
        let plan = prepare(spec).expect("plan");
        let expected = plan.predicted_authoritative_ids().clone();
        let expected_id = plan.id().clone();

        let mut app = bevy::prelude::App::new();
        app.add_message::<crate::rooms::RoomLoaded>();
        app.add_message::<features::SpawnActorRequest>();
        {
            let mut commands = app.world_mut().commands();
            plan.spawn_contents(&mut commands);
        }
        app.world_mut().flush();

        let receipt = app.world().resource::<LastRoomConstructionCommit>();
        assert_eq!(receipt.plan_id, expected_id);
        assert_eq!(receipt.room_id, "receipt");
        assert_eq!(receipt.authoritative_ids, expected);

        let actual = {
            let mut query = app
                .world_mut()
                .query::<&ambition_combat::components::FeatureId>();
            query
                .iter(app.world())
                .map(|feature| feature.0.clone())
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            actual, expected,
            "the committed authoritative roots must match the prepared roster",
        );
    }

    #[test]
    fn plan_rejects_a_same_id_room_spec_changed_after_preparation() {
        let plan = prepare(empty_spec("mutable")).expect("plan");
        let mut changed = empty_spec("mutable");
        changed.world.spawn.x += 1.0;
        assert!(plan.matches_room_spec(plan.spec()));
        assert!(!plan.matches_room_spec(&changed));
    }
}
