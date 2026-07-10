//! Projectile schedule seams owned by the runtime composition tier.
//!
//! The projectile MODEL lives in `ambition_projectiles`. The victim-routing and
//! charge-input steppers still live in the actor sim heart because they touch
//! un-carved actor/player/boss/world state. Enemy/boss projectile effect-request
//! spawning is now substrate-owned by `ambition_projectiles`. Callers outside
//! `ambition_runtime` should schedule against these runtime names rather than
//! reaching through `ambition_actors::{projectile, enemy_projectile}`; this keeps
//! the residual glue enumerable while the remaining actor-side projectile
//! steppers are split.

pub use ambition_actors::projectile::{charge_projectile_input, step_projectiles};
pub use ambition_projectiles::apply_player_spawn_projectile_messages;
pub use ambition_projectiles::collision_world::ProjectileCollisionWorld;
pub use ambition_projectiles::enemy::apply_enemy_projectile_effect_requests as apply_enemy_projectile_effects;
