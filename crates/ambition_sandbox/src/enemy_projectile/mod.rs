//! Enemy-fired projectiles (pirate volleys etc).
//!
//! Distinct from `crate::projectile`, which is the *player* projectile
//! system (fireball / hadouken). Enemy projectiles:
//!
//! - Are spawned by actor/brain action requests, not by player input.
//! - Damage the *player* on contact (not enemies / breakables).
//! - Use the same `crate::projectile::ProjectileBody` engine primitive for physics,
//!   collision, and lifetime — only the routing is faction-flipped.
//!
//! Splitting the state keeps the player-vs-enemy faction explicit and
//! avoids a future "is this projectile mine?" flag on each body.

mod entity;
mod state;
mod systems;
#[cfg(test)]
pub(crate) mod test_support;
mod visuals;

pub use entity::EnemyProjectile;
pub use state::{EnemyProjectileSpawn, EnemyProjectileState};
pub use systems::{apply_enemy_spawn_projectile_messages, update_enemy_projectiles};
pub use visuals::{
    lasersword_projectile_sprite, sync_enemy_projectile_visuals, EnemyProjectileVisual,
    LASERSWORD_SHEET,
};
