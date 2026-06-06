//! Unified body kinematics for every controllable body in the platformer.
//!
//! [`BodyKinematics`] is the single position / velocity / AABB-size / facing
//! component shared by the player, enemies/NPCs, and bosses. It replaces the
//! three historical parallel types (`PlayerKinematics`, `ActorKinematics`,
//! `BossKinematics`) so any code that operates on "a body" (orientation,
//! transit, vortex, brain effects, …) holds ONE query instead of branching
//! across three.
//!
//! ## Query-conflict discipline
//!
//! Because player, enemy, and boss entities now all carry `BodyKinematics`, any
//! single system that holds more than one `&mut BodyKinematics` query (or a
//! `&mut` query alongside another that can alias the same entity) must make the
//! queries provably disjoint with marker filters
//! (`With<PlayerEntity>` / `With<EnemyConfig>` / `With<BossConfig>`, plus
//! `Without<…>` guards where needed). Player / enemy / boss are mutually
//! exclusive archetypes, so those filters are sound. This is the same failure
//! mode that originally forced the boss onto its own type — handle it with
//! filters, never by re-splitting the component.

// [`BodyKinematics`] is the foundational body state engine_core's movement
// operates on, so its definition lives in the foundation crate
// (`ambition_engine_core`, below the runtime — see ADR 0019). Re-export it here
// so the runtime's `body::BodyKinematics` path resolves and the sandbox facade
// can forward it.
pub use ambition_engine_core::BodyKinematics;
