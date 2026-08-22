//! Controlled-body projectile integration around the reusable projectile model.
//!
//! The reusable projectile MODEL — shot vocabulary (`ProjectileKind` / the open
//! `ProjectileVisualId` + content-owned visual catalog), the ECS components,
//! `PlayerProjectileState`, the unified `ProjectileSpawnRequest` materialization
//! seam, and pure portal transit — now lives in the
//! [`ambition_projectiles`] crate (E2 carve) and is re-exported below so
//! `crate::projectile::*` paths resolve unchanged for every sandbox consumer.
//!
//! They CONSUME the model crate — the legal sim → model direction.

pub use ambition_projectiles::*;

/// The shot's own combat side. Lives HERE rather than in the model crate because
/// it is built from combat vocabulary (`ActorFaction` / `MatchTeam`) that
/// `ambition_projectiles` is forbidden to link — the boundary that keeps the
/// projectile model content-free is the same boundary that puts this component
/// beside the stepper that reads it.
mod allegiance;
pub use allegiance::{stamp_new_projectile_allegiance, ProjectileAllegiance};

pub mod systems;
pub use systems::{charge_projectile_input, step_projectiles, ProjectileStepSet};

#[cfg(test)]
mod tests;
