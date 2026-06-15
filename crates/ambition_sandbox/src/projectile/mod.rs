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
//! Damage is routed through `HitEvent` messages — the same path
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
mod entity;
mod portal_transit;
mod spawn;
mod spawn_message;
mod state;
mod systems;

#[cfg(test)]
mod tests;

pub use entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeq,
    ProjectileSeqCounter,
};
pub use spawn_message::{ProjectilePool, SpawnProjectile};
pub use state::PlayerProjectileState;
pub use systems::{
    apply_player_spawn_projectile_messages, player_projectile_input, step_projectiles,
};
#[allow(unused_imports)]

#[cfg(test)]
mod engine_tests;

// The generic projectile-physics primitive (spec / body / collision) lives in
// `ambition_platformer_runtime::projectile` (Stage 18 T2). Re-export it here so
// `crate::projectile::ProjectileBody` etc. resolve unchanged for every sandbox
// call site, and so `crate::enemy_projectile` consumes the same reusable
// primitive through this facade. The brain-coupled SPAWN (`systems`) stays in
// sandbox as a thin consumer.
pub use ambition_platformer_runtime::projectile::{
    resolve_world_collision, FireballChargeTuning, InFlightProjectile, ProjectileBody,
    ProjectileFaction, ProjectileGameplay, ProjectileKind, ProjectileSolidHit, ProjectileSpec,
    WorldHitOutcome, WorldHitPolicy,
};

// Sandbox-specific spawn helpers (player input gesture buffer + cooldown meter)
// stay in the sandbox.
// Motion-gesture recognition moved to the `ambition_input` crate (it is pure
// input logic, reusable beyond projectiles). Re-exported so existing
// `crate::projectile::MotionInputBuffer` paths keep resolving.
pub use crate::input::{MotionDirection, MotionInputBuffer};
pub use portal_transit::try_projectile_portal_transit;
pub use spawn::{ProjectileSpawner, SpawnFailure};
