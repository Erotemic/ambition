//! Lifecycle vocabulary for entities spawned by reusable platformer systems.
//!
//! The public API is the helper verb (`spawn_room_scoped`,
//! `spawn_run_scoped`, `spawn_persistent`) rather than the marker component
//! convention. Marker components remain public because existing cleanup queries
//! and tests need to name them, but new spawn sites should prefer
//! [`SpawnScopedExt`].

mod cleanup;
mod markers;
mod spawn_ext;

pub use cleanup::despawn_scoped_entity;
pub use markers::{
    LoadingZoneVisual, PersistentEntity, PlayerVisual, RoomScopedEntity, RoomVisual,
    RunScopedEntity, SceneEntities,
};
pub use spawn_ext::SpawnScopedExt;
