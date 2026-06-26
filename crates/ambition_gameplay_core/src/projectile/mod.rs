//! Sandbox PLAYER-faction projectile (Fireball / Hadouken) — one of two
//! near-duplicate faction faces of the same idea, the other being
//! `crate::enemy_projectile` (enemy volleys). The reusable projectile PHYSICS
//! (spec / body / per-frame tick / world collision) is SHARED: it lives in
//! `ambition_platformer_primitives::projectile` and is re-exported below, so both
//! factions step through identical motion. This module owns only the
//! player-specific seam: charge/motion-gesture firing, input sampling, and
//! per-player state. Behavior is faction-routed in one unified `step_projectiles`
//! (see [`entity::LiveProjectile`]); the `PlayerProjectile`/`EnemyProjectile`
//! tags only select which renderer draws the shot.
//!
//! Damage routes through `HitEvent` messages — same path as slashes, pogo
//! bounces, and any future damage volume.
//!
//! ## Submodule layout
//!
//! - [`state`] — `PlayerProjectileState` (per-player charge machine + motion
//!   buffer + unlocks) and `ProjectileTraceEvent`.
//! - [`entity`] — the per-projectile ECS components (`LiveProjectile`,
//!   `PlayerProjectile`, `ProjectileOwner`, `ProjectileSeq`, …).
//! - [`systems`] — `step_projectiles` (the unified stepper),
//!   `player_projectile_input`, and the spawn-message consumer.
//! - [`spawn`] — `ProjectileSpawner`: cooldown + resource-meter gating.
//! - [`spawn_message`] — `SpawnProjectile` / `ProjectilePool`: decouples fire
//!   sites from per-pool storage.
//! - [`portal_transit`] — pure portal-aperture transit shared by both factions.
//! - [`diagnostics`] — motion-press logging helper.

mod diagnostics;
mod entity;
mod kind;
mod portal_transit;
mod spawn;
mod spawn_message;
mod state;
mod systems;
mod visual_kind;

#[cfg(test)]
mod tests;

pub use entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeq,
    ProjectileSeqCounter,
};
pub use kind::{FireballChargeTuning, ProjectileKind};
pub use spawn_message::{ProjectilePool, SpawnProjectile};
pub use visual_kind::{
    ProjectileArt, ProjectileArtSource, ProjectileRenderSize, ProjectileRotation,
    ProjectileVisualKind,
};
pub use state::PlayerProjectileState;
pub use systems::{
    apply_player_spawn_projectile_messages, player_projectile_input, step_projectiles,
};
#[allow(unused_imports)]
#[cfg(test)]
mod engine_tests;

// The generic projectile-physics primitive (spec / body / collision) lives in
// `ambition_platformer_primitives::projectile` (Stage 18 T2). Re-export it here so
// `crate::projectile::ProjectileBody` etc. resolve unchanged for every sandbox
// call site, and so `crate::enemy_projectile` consumes the same reusable
// primitive through this facade. The brain-coupled SPAWN (`systems`) stays in
// sandbox as a thin consumer.
pub use ambition_platformer_primitives::projectile::{
    resolve_world_collision, InFlightProjectile, ProjectileBody, ProjectileFaction,
    ProjectileGameplay, ProjectileSolidHit, ProjectileSpec, WorldHitOutcome, WorldHitPolicy,
};

// Sandbox-specific spawn helpers (player input gesture buffer + cooldown meter)
// stay in the sandbox.
// Motion-gesture recognition moved to the `ambition_input` crate (it is pure
// input logic, reusable beyond projectiles). Re-exported so existing
// `crate::projectile::MotionInputBuffer` paths keep resolving.
pub use ambition_input::{MotionDirection, MotionInputBuffer};
pub use portal_transit::try_projectile_portal_transit;
pub use spawn::{ProjectileSpawner, SpawnFailure};
