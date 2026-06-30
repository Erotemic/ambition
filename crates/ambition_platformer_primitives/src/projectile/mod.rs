//! Reusable, game-agnostic projectile physics primitive.
//!
//! This module is the brain-free physics core for projectiles in a 2D
//! platformer: authored-intent specs, a per-frame kinematic body, and a
//! world-vs-body collision resolver. It depends only on
//! `ambition_engine_core` (geometry + world) — no spawn logic, no damage
//! routing, no actor roster, and no Ambition-specific content. Any
//! platformer (or an agent building one) can drop it in and feed it a
//! [`World`](ambition_engine_core::World).
//!
//! The game-specific *spawners* (player-fired fireball driven off input,
//! enemy-fired volleys driven off AI) stay in the consuming crate as thin
//! consumers: they build a [`ProjectileBody`] from a [`ProjectileSpec`],
//! tick it each frame, and route the [`WorldHitOutcome`] /
//! [`HitEvent`]-style damage themselves.
//!
//! ## Submodules
//! - [`spec`] — the generic, content-free [`ProjectileSpec`] (authored intent).
//!   Named projectile kinds + their stat tables are a *game's* concern (for
//!   Ambition: `ambition_gameplay_core::projectile::kind`); the engine never
//!   names them.
//! - [`body`] — the kinematic/gameplay split: [`ProjectileGameplay`]
//!   (per-frame motion + solid/one-way resolution over a
//!   [`ambition_engine_core::BodyKinematics`]), the [`ProjectileBody`]
//!   composite, [`InFlightProjectile`],
//!   [`ProjectileSolidHit`].
//! - [`collision`] — [`resolve_world_collision`] (body-vs-world scan over
//!   the split kinematic + gameplay halves, dispatched on the spec's
//!   [`WorldHitPolicy`]).

pub mod body;
pub mod collision;
pub mod spec;

pub use body::{
    InFlightProjectile, ProjectileBody, ProjectileGameplay, ProjectileSolidHit,
};
pub use collision::{resolve_world_collision, WorldHitOutcome, WorldHitPolicy};
pub use spec::{EnemyProjectileSpawn, ProjectileSpec};
