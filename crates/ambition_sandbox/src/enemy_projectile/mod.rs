//! Enemy-fired projectiles (pirate volleys etc).
//!
//! Distinct from `crate::projectile`, which is the *player* projectile
//! system (fireball / hadouken). Enemy projectiles:
//!
//! - Are spawned by `EnemyRuntime` choreography requests, not by
//!   input.
//! - Damage the *player* on contact (not enemies / breakables).
//! - Use the same `ae::ProjectileBody` engine primitive for physics,
//!   collision, and lifetime — only the routing is faction-flipped.
//!
//! Splitting the state keeps the player-vs-enemy faction explicit and
//! avoids a future "is this projectile mine?" flag on each body.

mod state;
mod systems;
mod visuals;

pub use state::{EnemyProjectileSpawn, EnemyProjectileState};
pub use systems::update_enemy_projectiles;
pub use visuals::{sync_enemy_projectile_visuals, EnemyProjectileVisual};
