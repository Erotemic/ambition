//! `ambition_projectiles` — the reusable, content-free projectile MODEL.
//!
//! One of two faction faces of a single idea — the PLAYER shot (Fireball /
//! Hadouken) and the ENEMY volley ([`enemy`]) — sharing one reusable projectile
//! PHYSICS primitive (spec / body / per-frame tick / world collision) that lives
//! in [`ambition_platformer_primitives::projectile`] and is re-exported below, so
//! both factions step through identical motion. This crate owns the VOCABULARY
//! and the pieces with no victim/world/brain weave:
//!
//! - [`state`] — [`PlayerProjectileState`] (per-body charge machine + motion
//!   buffer + unlocks) and `ProjectileTraceEvent`.
//! - [`entity`] — the per-projectile ECS components ([`LiveProjectile`],
//!   [`PlayerProjectile`], [`ProjectileOwner`], [`ProjectileSeq`], …).
//! - [`kind`] / [`visual_kind`] — the named shot kinds + their art descriptors.
//! - [`spawn`] — [`ProjectileSpawner`]: cooldown + resource-meter gating.
//! - [`spawn_message`] — [`SpawnProjectile`] / [`ProjectilePool`]: decouples fire
//!   sites from per-pool storage.
//! - [`spawn_systems`] — [`apply_player_spawn_projectile_messages`], the
//!   player-pool spawn consumer (pure: reads the pool message, spawns entities).
//! - [`enemy::apply_enemy_projectile_effect_requests`] — the enemy-pool
//!   `Effect::Projectiles` spawn consumer (pure: reads effect vocabulary,
//!   spawns projectile entities).
//! - [`portal_transit`] — pure portal-aperture transit shared by both factions.
//! - [`diagnostics`] — motion-press logging helper.
//!
//! The victim-side hit routing and the player charge/anim INPUT stepper stay in
//! the game's sim heart (`ambition_actors`) because they are woven with
//! un-carved boss/actor/player-anim/world state; they consume this crate.

pub mod diagnostics;
pub mod enemy;
pub mod entity;
pub mod kind;
pub mod portal_transit;
pub mod spawn;
pub mod spawn_message;
pub mod spawn_systems;
pub mod state;
pub mod visual_kind;

pub use entity::{
    LiveProjectile, PlayerProjectile, ProjectileOwner, ProjectileOwnerId, ProjectileSeq,
    ProjectileSeqCounter,
};
pub use kind::{FireballChargeTuning, ProjectileKind};
pub use spawn::{ProjectileSpawner, SpawnFailure};
pub use spawn_message::{ProjectilePool, SpawnProjectile};
pub use spawn_systems::apply_player_spawn_projectile_messages;
pub use state::PlayerProjectileState;
pub use visual_kind::{
    ProjectileArt, ProjectileArtSource, ProjectileRenderSize, ProjectileRotation,
    ProjectileVisualKind,
};

// The generic projectile-physics primitive (spec / body / collision) lives in
// `ambition_platformer_primitives::projectile`. Re-export it at the crate root so
// `ambition_projectiles::ProjectileBody` etc. resolve for every call site, and so
// the enemy faction consumes the same reusable primitive through this facade.
pub use ambition_platformer_primitives::projectile::{
    resolve_world_collision, InFlightProjectile, ProjectileBody, ProjectileGameplay,
    ProjectileSolidHit, ProjectileSpec, WorldHitOutcome, WorldHitPolicy,
};

// Motion-gesture recognition lives in `ambition_input` (pure input logic, reusable
// beyond projectiles). Re-exported so `ambition_projectiles::MotionInputBuffer`
// paths resolve.
pub use ambition_input::{MotionDirection, MotionInputBuffer};
pub use portal_transit::try_projectile_portal_transit;
