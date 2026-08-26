//! Content-free character identity, control, behavior, and authoring vocabulary.
//!
//! This crate sits above the pure movement kernel and provides actor/control types,
//! character definitions, brains, perception, and preparation. Named world content stays
//! in game content crates. Genre-specific platform-fighter policy currently lives here for
//! dependency reasons but is a carve target; floor-level APIs should remain vocabulary that
//! every platformer composition can reasonably share.

pub mod action_scheme;
pub mod actor;
pub mod binding_namespaces;
pub mod boss_encounter;
pub mod brain;
pub mod control;
pub mod equipment;
pub mod moveset_authoring;
pub mod moveset_prefabs;
pub mod perception;
pub mod prepared;
#[cfg(any(test, feature = "test-support"))]
pub mod prepared_fixtures;
pub mod smash_capture;
pub mod smash_ride;
pub mod smash_fighter;
pub mod smash_repertoire;
mod snapshot_impls;
pub mod technique;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
