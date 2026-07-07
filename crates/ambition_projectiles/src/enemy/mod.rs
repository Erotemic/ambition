//! Enemy-fired projectiles (pirate volleys etc).
//!
//! Distinct from the crate's *player* projectile vocabulary (fireball / hadouken).
//! Enemy projectiles:
//!
//! - Are spawned by actor/brain action requests, not by player input.
//! - Damage the *player* on contact (not enemies / breakables).
//! - Use the same [`crate::ProjectileBody`] engine primitive for physics,
//!   collision, and lifetime — only the routing is faction-flipped.
//!
//! Splitting the state keeps the player-vs-enemy faction explicit and
//! avoids a future "is this projectile mine?" flag on each body.
//!
//! The victim-side effect stepper (`apply_projectile_effects`) that damages the
//! player stays in the game's sim heart (`ambition_gameplay_core`); this module
//! owns only the enemy-shot ENTITY marker + spawn state.

pub mod entity;
pub mod state;

pub use entity::EnemyProjectile;
pub use state::{EnemyProjectileSpawn, EnemyProjectileState};
