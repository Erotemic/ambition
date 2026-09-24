//! What stayed behind when the wielded ability kit was carved out (D33,
//! 2026-09-03). The kit itself is [`ambition_abilities`].
//!
//! ⛔⛔ THESE ARE NOT "THE LEFTOVER ABILITIES". They are two groups that share a
//! directory name with abilities and nothing else:
//!
//! * [`traversal`] — `teleport`, `trapdoor`, `flyline`: authored-world
//!   traversal a move fires, registered by `ambition_platformer2d_runtime`.
//!   Possession is control authority (a seat redirect) and lives in
//!   `crate::control::possession`.
//! * [`thrown`] — the puppy-slug gun, which SPAWNS A BODY through the
//!   crate-private `features::spawn_runtime_minion`. The cross-crate seam for
//!   that already exists (`ambition_vfx::Effect::Summon` → `SummonSpec` →
//!   `ActorConstructionParams::SummonedMinion`); the gun is the one caller
//!   bypassing it, so moving it is a behaviour change and not a file move.
//!
//! `docs/planning/engine/actor-monolith-decomposition.md` carries both
//! arguments with their numbers, so neither group gets carved by line count.
//!
pub mod thrown;
pub mod traversal;
