//! Content-free character identity, control, behavior, and authoring vocabulary.
//!
//! This crate sits above the pure movement kernel and provides actor/control types,
//! character definitions, brains, perception, and preparation. Named world content stays
//! in game content crates. Genre-specific platform-fighter policy currently lives here for
//! dependency reasons but is a carve target; floor-level APIs should remain vocabulary that
//! every platformer composition can reasonably share.
//!
//! ⛔⛔ WHAT THIS CRATE REFUSES, because a destination that says nothing accepts
//! everything.
//!
//! - **The actor INTEGRATION layer.** A character is what a body IS and what
//!   decides for it; wiring bodies into a running simulation is the layer above.
//!   Enforced rather than promised —
//!   `characters-do-not-depend-on-the-actor-integration-layer` in
//!   `scripts/check_absence_contracts.py` is that sentence as a check.
//! - **Anything the SHEET derives.** Frame rects, measured body extents and
//!   render sizes belong to `ambition_sprite_sheet`, which depends on this crate;
//!   hosting them here inverts that edge. `ActorSpriteMetrics` was refused on
//!   exactly this ground before it found its owner.

pub mod action_scheme;
pub mod actor;
pub mod binding_namespaces;
pub mod boss_encounter;
pub mod brain;
pub mod control;
pub mod equipment;
pub mod load_demand;
pub mod moveset_authoring;
pub mod moveset_prefabs;
pub mod perception;
pub mod prepared;
#[cfg(any(test, feature = "test-support"))]
pub mod prepared_fixtures;
pub mod smash_bomb;
pub mod smash_capture;
pub mod smash_counter;
pub mod smash_fighter;
pub mod smash_flyline;
pub mod smash_portal;
pub mod smash_repertoire;
pub mod smash_ride;
pub mod smash_teleport;
pub mod smash_trapdoor;
pub mod smash_vitality;
mod snapshot_impls;
pub mod technique;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
