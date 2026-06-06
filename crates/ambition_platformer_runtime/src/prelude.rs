//! Convenience imports for reusable platformer-runtime call sites.
//!
//! This prelude is intentionally content-free: it only re-exports the lifecycle
//! and schedule vocabulary owned by this crate. Sandbox-specific seams (for
//! example the solid-world raycast helper) are re-exported alongside this
//! prelude by the consuming crate's own `platformer_runtime::prelude` facade.

pub use crate::lifecycle::{
    despawn_scoped_entity, PersistentEntity, RoomScopedEntity, RunScopedEntity, SpawnScopedExt,
};
pub use crate::schedule::PlatformerRuntimeSet;
