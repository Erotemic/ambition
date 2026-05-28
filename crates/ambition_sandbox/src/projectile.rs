//! Sandbox player projectile (Fireball / Hadouken).
//!
//! The engine owns the reusable primitives:
//!
//! * `crate::projectile::ProjectileSpec` / `ProjectileBody` (data + per-frame tick),
//! * `crate::projectile::ProjectileSpawner` (cooldown + resource meter),
//! * `crate::projectile::MotionInputBuffer` (quarter / half-circle motion recognition).
//!
//! This module wires those primitives into the Bevy sandbox: input
//! sampling, collision against the active world, and trace events.
//! Damage is routed through `DamageEvent` messages — the same path
//! slashes, pogo-bounces, and any future tool / hazard / spell that
//! produces a damage volume go through.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`state`] — `PlayerProjectileState`, `PlayerProjectile`,
//!   `ProjectileUnlocks`, `ProjectileTraceEvent`.
//! - [`systems`] — the `update_projectiles` Bevy system + private
//!   `try_fire_projectile` helper.
//! - [`visuals`] — `sync_projectile_visuals` system + visual marker
//!   components.
//! - [`diagnostics`] — internal motion-press logging helper.

mod body;
mod motion_input;
mod spawn;
mod spec;
mod collision;
mod diagnostics;
mod state;
mod systems;
mod visuals;

#[cfg(test)]
mod tests;

pub use collision::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
pub use state::PlayerProjectileState;
pub use systems::update_projectiles;
pub use visuals::{sync_projectile_visuals, PlayerProjectileVisual};

#[cfg(test)]
mod engine_tests;

// Re-export the engine-side projectile primitives (moved from crate::engine_core 2026-05-28).
pub use body::{ProjectileBody, ProjectileFaction, ProjectileSolidHit};
pub use motion_input::{MotionDirection, MotionInputBuffer, MotionSample};
pub use spawn::{ProjectileSpawner, SpawnFailure};
pub use spec::{FireballChargeTuning, ProjectileKind, ProjectileSpec};
