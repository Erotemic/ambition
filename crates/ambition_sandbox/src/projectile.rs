//! Sandbox player projectile (Fireball / Hadouken).
//!
//! The engine owns the reusable primitives:
//!
//! * `ae::ProjectileSpec` / `ProjectileBody` (data + per-frame tick),
//! * `ae::ProjectileSpawner` (cooldown + resource meter),
//! * `ae::MotionInputBuffer` (quarter / half-circle motion recognition).
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

mod diagnostics;
mod state;
mod systems;
mod visuals;

#[cfg(test)]
mod tests;

pub use state::PlayerProjectileState;
pub use systems::update_projectiles;
pub use visuals::{sync_projectile_visuals, PlayerProjectileVisual};
