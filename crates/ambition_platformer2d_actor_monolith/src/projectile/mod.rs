//! Controlled-body projectile integration around the reusable projectile model.
//!
//! The reusable projectile MODEL — shot vocabulary (`ProjectileKind` / the open
//! `ProjectileVisualId` + content-owned visual catalog), the ECS components,
//! `PlayerProjectileState`, the unified `ProjectileSpawnRequest` materialization
//! seam, and pure portal transit — lives in the [`ambition_projectiles`] crate
//! (E2 carve) and is NOT re-exported here: callers name that crate. A glob
//! forward would let this module's own coupling census count the model crate as
//! monolith coupling, which is exactly what it did until 2026-08-26.
//!
//! What is left here CONSUMES the model crate — the legal sim → model direction.

/// The shot's own combat side. Lives HERE rather than in the model crate because
/// it is built from combat vocabulary (`ActorFaction` / `MatchTeam`) that
/// `ambition_projectiles` is forbidden to link — the boundary that keeps the
/// projectile model content-free is the same boundary that puts this component
/// beside the stepper that reads it.
mod allegiance;
pub mod intercept;
pub use allegiance::{stamp_new_projectile_allegiance, ProjectileAllegiance};

pub mod systems;
#[cfg(test)]
mod sentry_bolt_damage_tests;

pub use systems::{charge_projectile_input, step_projectiles, ProjectileStepSet};

#[cfg(test)]
mod tests;
