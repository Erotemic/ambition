//! Lifecycle vocabulary for entities spawned by reusable platformer systems.
//!
//! The public API is the helper verb (`spawn_room_scoped`, `spawn_run_scoped`,
//! `spawn_mode_scoped`, `spawn_persistent`) rather than the marker component
//! convention. Marker components remain public because existing cleanup queries
//! and tests need to name them, but new spawn sites should prefer
//! [`SpawnScopedExt`].

mod cleanup;
mod markers;
mod session;
mod spawn_ext;

pub use cleanup::despawn_scoped_entity;
pub use markers::{
    FeatureSimEntity, LoadingZoneVisual, ModeScopedEntity, PersistentEntity, PlayerVisual,
    RoomScopedEntity, RoomVisual, RunScopedEntity, SceneEntities,
};
pub use session::{
    despawn_retired_session_entities, simulation_authorized, ActiveSessionScope, SessionCommands,
    insert_session_world_component, session_world_component, session_world_component_mut,
    session_world_entity, session_world_exists,
    SessionGatedSimulation, SessionRoot, SessionScopeId, SessionScopePlugin,
    SessionScopeRetired, SessionScopeSet, SessionScopedEntity, SessionSpawnScope,
    SessionWorldMut, SessionWorldRef, SpawnSessionScopedExt,
};
pub use spawn_ext::SpawnScopedExt;
