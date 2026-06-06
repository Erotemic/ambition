//! Convenience imports for proto-runtime call sites.

pub use super::collision::raycast_solids;
pub use super::lifecycle::{
    despawn_scoped_entity, PersistentEntity, RoomScopedEntity, RunScopedEntity, SpawnScopedExt,
};
pub use super::schedule::PlatformerRuntimeSet;
