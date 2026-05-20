//! Generic projectile primitives.
//!
//! `ProjectileSpec` defines a single projectile fired by the player
//! (or in principle an enemy). `ProjectileKind` distinguishes the
//! variants — today the sandbox uses `Fireball` (cheap, weaker) and
//! `Hadouken` (strong, costs more resource, stronger arc).
//!
//! `ProjectileSpawner` is a tiny stateless helper that converts a
//! "user pressed Projectile + facing right" intent into a
//! `ProjectileSpec` honoring a resource meter and a per-projectile
//! cooldown timer. Sandbox owns the per-frame physics tick because
//! the engine doesn't yet have a generic kinematic-body type.
//!
//! The motion-input recognizer (`MotionInputBuffer`) lives here too so
//! both keyboard and gamepad consumers can detect quarter-circle /
//! half-circle gestures before deciding which `ProjectileKind` to
//! fire.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`spec`] — `ProjectileKind`, `ProjectileSpec`, `FireballChargeTuning`.
//! - [`body`] — `ProjectileBody` (per-frame state + tick / hit
//!   resolution) and `ProjectileSolidHit`.
//! - [`motion_input`] — `MotionDirection`, `MotionSample`,
//!   `MotionInputBuffer` for QCF / half-circle gestures.
//! - [`spawn`] — `ProjectileSpawner`, `SpawnFailure`.

mod body;
mod motion_input;
mod spawn;
mod spec;

#[cfg(test)]
mod tests;

pub use body::{ProjectileBody, ProjectileFaction, ProjectileSolidHit};
pub use motion_input::{MotionDirection, MotionInputBuffer, MotionSample};
pub use spawn::{ProjectileSpawner, SpawnFailure};
pub use spec::{FireballChargeTuning, ProjectileKind, ProjectileSpec};
