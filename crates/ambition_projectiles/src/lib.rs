//! `ambition_projectiles` — the reusable, content-free projectile MODEL.
//!
//! The reusable projectile model for every shot, independent of who fired it.
//! Named shots (Fireball / Hadouken) and content-authored open-visual volleys share
//! one authoritative request/materialization road and one projectile PHYSICS
//! primitive (spec / body / per-frame tick / world collision) that lives
//! in [`ambition_platformer2d_shared_tangle::projectile`] and is re-exported below, so
//! both factions step through identical motion. This crate owns the VOCABULARY
//! and the pieces with no victim/world/brain weave:
//!
//! - [`state`] — [`PlayerProjectileState`] (per-body charge machine + motion
//!   buffer + unlocks) and `ProjectileTraceEvent`.
//! - [`entity`] — the per-projectile ECS components ([`LiveProjectile`],
//!   [`ProjectileOwner`], [`ProjectileSeq`], …).
//! - [`kind`] / [`visual_kind`] — the named shot kinds + their art descriptors.
//! - [`spawn`] — [`ProjectileSpawner`]: cooldown + resource-meter gating.
//! - [`spawn_request`] — [`ProjectileSpawnRequest`]: the single authoritative
//!   projectile materialization request, with explicit first-step timing.
//! - [`materialize`] — the two schedule-bound materializers that preserve the
//!   historical this-tick vs next-tick first-step timing while sharing one
//!   entity-construction implementation.
//! - [`portal_transit`] — pure portal-aperture transit shared by both factions.
//! - [`diagnostics`] — motion-press logging helper.
//!
//! The victim-side hit routing and the player charge/anim INPUT stepper stay in
//! the game's sim heart (`ambition_platformer2d_actor_monolith`) because they are woven with
//! un-carved boss/actor/player-anim/world state; they consume this crate.

pub mod collision_world;
pub mod diagnostics;
pub mod entity;
pub mod kind;
pub mod portal_transit;
mod snapshot_impls;
pub mod spawn;
pub mod spawn_request;
pub mod materialize;
pub mod state;
pub mod visual;

#[cfg(test)]
mod engine_tests;

pub use entity::{LiveProjectile, ProjectileOwner, ProjectileSeq, ProjectileSeqCounter};
pub use kind::{FireballChargeTuning, ProjectileKind};
pub use materialize::{
    materialize_projectiles_for_next_tick, materialize_projectiles_for_this_tick,
};
pub use spawn::{ProjectileSpawner, SpawnFailure};
pub use spawn_request::{
    build_in_flight_projectile, ProjectilePresentation, ProjectileSpawnRequest, ProjectileStart,
};
pub use state::PlayerProjectileState;
pub use visual::{
    ProjectileArt, ProjectileArtSource, ProjectileExpiryBurst, ProjectileRenderSize,
    ProjectileRotation, ProjectileVisualAppExt, ProjectileVisualCatalog, ProjectileVisualId,
};

// The generic projectile-physics primitive (spec / body / collision) lives in
// `ambition_platformer2d_shared_tangle::projectile`. Re-export it at the crate root so
// `ambition_projectiles::ProjectileBody` etc. resolve for every call site, and so
// every producer consumes the same reusable primitive through this facade.
pub use ambition_platformer2d_shared_tangle::projectile::{
    resolve_world_collision, InFlightProjectile, ProjectileBody, ProjectileGameplay,
    ProjectileSolidHit, ProjectileSpec, WorldHitOutcome, WorldHitPolicy,
};
pub use ambition_projectile_spec::ProjectileSpawn;

// Motion-gesture recognition lives in `ambition_input` (pure input logic, reusable
// beyond projectiles). Re-exported so `ambition_projectiles::MotionInputBuffer`
// paths resolve.
pub use ambition_input::{
    MotionDirection, MotionInputBuffer, MotionSample, MotionTechnique, MotionTechniqueAppExt,
    MotionTechniqueCatalog,
};
pub use portal_transit::try_projectile_portal_transit;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
