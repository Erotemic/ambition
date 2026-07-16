//! Room staging for snapshot restore (netcode.md N3.2b) — the atomic
//! active-room transaction's construction half.
//!
//! A snapshot taken in one room, restored into a world whose active room is
//! another, must first make the SNAPSHOT's room the live one — through the same
//! canonical construction a room transition runs, never a restore-only path.
//! [`RoomStaging`] packages exactly the construction subset of
//! [`load_room_geometry`](super::load_room_geometry): despawn the old room's
//! scoped entities, swap the active `RoomSpec` and its [`RoomGeometry`], rebuild
//! moving platforms, and lower the target room's placements through the
//! App-installed [`PlacementLoweringRegistry`].
//!
//! What the transition does and staging deliberately does NOT: reset the
//! controlled body to a validated arrival, reset clocks/cooldowns, or emit
//! transition feel (SFX, preset flash). Every one of those is restored sim
//! state — the snapshot blobs applied AFTER staging are the authority for the
//! body, the clocks, and everything else registered. Staging builds the stage;
//! the blobs place the actors.
//!
//! ## Transactionality
//!
//! [`RoomStaging::prepare`] is mutation-free: it resolves the target room and
//! clones every service the construction needs, refusing with the world
//! untouched when any is missing. [`RoomStaging::apply`] then cannot refuse —
//! so a caller (snapshot `restore`) can run all of its OTHER preflights between
//! `prepare` and `apply` and still guarantee that a refusal leaves the live
//! room exactly as it was.

use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::world::World;

use super::{RespawnRoomVisualsRequested, RoomSpec};
use crate::features;
use crate::platformer_runtime::lifecycle::RoomScopedEntity;
use crate::world::physics::{self, PhysicsRoomEntity};
use crate::world::placements::PlacementLoweringRegistry;
use crate::world::platforms;
use ambition_platformer_primitives::lifecycle::{
    session_world_component, session_world_component_mut, ActiveSessionScope, SessionSpawnScope,
};

/// Why a room could not be staged. Mutation-free by construction: every
/// variant is detected by [`RoomStaging::prepare`] before the world is touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoomStagingError {
    /// The requested room id names no room in the live session's `RoomSet` —
    /// a snapshot from a different world/content identity.
    UnknownRoom { room: String },
    /// A canonical construction service (the lowering registry, a catalog, the
    /// session scope, the room set itself) is absent — a world that cannot
    /// build rooms cannot have one staged onto it.
    MissingService { service: &'static str },
}

impl std::fmt::Display for RoomStagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoomStagingError::UnknownRoom { room } => write!(
                f,
                "no room named `{room}` in the live session's RoomSet — the snapshot \
                 belongs to a different prepared world"
            ),
            RoomStagingError::MissingService { service } => write!(
                f,
                "room staging needs `{service}`, which this world does not provide"
            ),
        }
    }
}

impl std::error::Error for RoomStagingError {}

/// A prepared room-staging transaction: the target room resolved and every
/// construction service cloned, with the world untouched. See the module doc.
pub struct RoomStaging {
    target_index: usize,
    spec: RoomSpec,
    registry: PlacementLoweringRegistry,
    content_staging: features::RoomContentStagingRegistry,
    character_catalog: ambition_characters::actor::character_catalog::CharacterCatalog,
    character_roster: features::CharacterRoster,
    boss_catalog: crate::boss_encounter::BossCatalog,
    session_scope: SessionSpawnScope,
}

impl RoomStaging {
    /// Resolve the target room and gather the canonical construction services,
    /// mutating nothing. `Err` means the world is exactly as it was.
    pub fn prepare(world: &World, target_room_id: &str) -> Result<Self, RoomStagingError> {
        let missing = |service| RoomStagingError::MissingService { service };
        let rooms =
            session_world_component::<super::RoomSet>(world).ok_or(missing("session RoomSet"))?;
        let target_index = rooms.room_index_by_id(target_room_id).ok_or_else(|| {
            RoomStagingError::UnknownRoom {
                room: target_room_id.to_string(),
            }
        })?;
        let spec = rooms.rooms[target_index].clone();
        // The geometry component must exist for `apply` to swap — checked here so
        // `apply` cannot fail.
        session_world_component::<ambition_engine_core::RoomGeometry>(world)
            .ok_or(missing("session RoomGeometry"))?;
        if world
            .get_resource::<ambition_world::collision::MovingPlatformSet>()
            .is_none()
        {
            return Err(missing("MovingPlatformSet"));
        }
        Ok(Self {
            target_index,
            spec,
            registry: world
                .get_resource::<PlacementLoweringRegistry>()
                .ok_or(missing("PlacementLoweringRegistry"))?
                .clone(),
            // Default when absent: a world with no registered content stagers
            // (a headless fixture) stages rooms with no content-staged
            // occupants, which is exactly what its rooms contain.
            content_staging: world
                .get_resource::<features::RoomContentStagingRegistry>()
                .cloned()
                .unwrap_or_default(),
            character_catalog: world
                .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
                .ok_or(missing("CharacterCatalog"))?
                .clone(),
            character_roster: world
                .get_resource::<features::CharacterRoster>()
                .ok_or(missing("CharacterRoster"))?
                .clone(),
            boss_catalog: world
                .get_resource::<crate::boss_encounter::BossCatalog>()
                .ok_or(missing("BossCatalog"))?
                .clone(),
            session_scope: SessionSpawnScope::for_optional_active_session(
                world.get_resource::<ActiveSessionScope>(),
            )
            .ok_or(missing("ActiveSessionScope"))?,
        })
    }

    /// The staged room's id.
    pub fn room_id(&self) -> &str {
        &self.spec.id
    }

    /// Every identity `apply` will construct, predicted without mutating
    /// anything: the authored placement/enemy/boss ids (the same three lists
    /// `respawn_authored_entity` reconstructs from) plus the content-staged
    /// occupants (the registered stagers are pure functions of the spec).
    ///
    /// This is the roster half of a restore's preflight: a snapshot identity
    /// that neither survives the sweep nor appears here cannot come back
    /// complete, and the restore refuses BEFORE the world is touched.
    pub fn predicted_authored_ids(&self) -> std::collections::BTreeSet<String> {
        self.spec
            .placements
            .iter()
            .map(|p| p.id.0.clone())
            .chain(self.spec.enemy_spawns.iter().map(|e| e.id.clone()))
            .chain(self.spec.boss_spawns.iter().map(|b| b.id.clone()))
            .chain(self.content_staging.staged_ids_for(&self.spec))
            .collect()
    }

    /// Make the prepared room the live one, through the canonical construction:
    /// the same scoped-entity sweep, active-spec/geometry swap, moving-platform
    /// rebuild, and placement lowering a room transition runs. Infallible —
    /// every refusal already happened in [`prepare`](Self::prepare).
    pub fn apply(self, world: &mut World) {
        // Requests queued by the future we are abandoning must not materialize
        // in the room we are staging: drop any pending spawn requests BEFORE
        // the fresh construction writes its own into the same channel.
        if let Some(mut pending) =
            world.get_resource_mut::<bevy::ecs::message::Messages<features::SpawnActorRequest>>()
        {
            pending.clear();
        }

        // Despawn the outgoing room's scoped entities — the transition's sweep,
        // with no carry-body exemption: a restore's survivors are exactly the
        // non-room-scoped entities, and the body's state comes back from blobs.
        let scoped: Vec<(Entity, bool)> = match world
            .try_query_filtered::<(Entity, Option<&PhysicsRoomEntity>), With<RoomScopedEntity>>()
        {
            Some(mut q) => q.iter(world).map(|(e, p)| (e, p.is_some())).collect(),
            None => Vec::new(),
        };
        {
            let mut commands = world.commands();
            for (entity, is_physics) in scoped {
                if is_physics {
                    physics::retire_physics_entity(&mut commands, entity);
                } else {
                    commands.entity(entity).despawn();
                }
            }
        }
        world.flush();

        // Swap the active spec and its geometry.
        if let Some(mut rooms) = session_world_component_mut::<super::RoomSet>(world) {
            rooms.set_active(self.target_index);
        }
        if let Some(mut geometry) =
            session_world_component_mut::<ambition_engine_core::RoomGeometry>(world)
        {
            geometry.0 = self.spec.world.clone();
        }

        // Rebuild moving platforms from the authored spec (the registered
        // `moving_platform_set` blob patches the live kinematics afterwards).
        let platform_states = platforms::moving_platforms_for_room(&self.spec);
        if let Some(mut set) =
            world.get_resource_mut::<ambition_world::collision::MovingPlatformSet>()
        {
            set.0 = platform_states.clone();
        }

        // Lower the room's placements through the installed registry — the one
        // authority room activation, transition, reset, and restore all share.
        {
            let mut commands = world.commands();
            features::spawn_room_feature_entities_with_registry(
                &mut commands,
                &self.character_catalog,
                &self.character_roster,
                &self.boss_catalog,
                &self.spec,
                &self.registry,
                &self.content_staging,
                self.session_scope,
            );
            platforms::spawn_moving_platforms(
                &mut commands,
                self.session_scope,
                &self.spec.world,
                &platform_states,
            );
        }
        world.flush();

        // Materialize the content-staged occupants NOW, through the same
        // canonical applier the schedule runs — a restore reconciles against
        // this roster synchronously and cannot wait a frame for the scheduled
        // drain. (The channel held only this staging's requests: the
        // abandoned future's were cleared above.)
        let _ = bevy::ecs::system::RunSystemOnce::run_system_once(
            &mut *world,
            features::apply_spawn_actor_requests,
        );
        world.flush();

        // Presentation rebuilds from the request — the same message the sandbox
        // reset emits. A headless world has no consumer and correctly skips.
        if let Some(mut messages) =
            world.get_resource_mut::<bevy::ecs::message::Messages<RespawnRoomVisualsRequested>>()
        {
            messages.write(RespawnRoomVisualsRequested);
        }
    }
}
