//! What stayed behind when the wielded ability kit was carved out (D33,
//! 2026-09-03). The kit itself is [`ambition_abilities`].
//!
//! ⛔⛔ THESE ARE NOT "THE LEFTOVER ABILITIES". They share a directory name with
//! abilities and nothing else:
//!
//! * [`traversal`] — `teleport`, `trapdoor`, `flyline`: authored-world
//!   traversal a move fires, registered by `ambition_platformer2d_runtime`.
//!   Possession is control authority (a seat redirect) and lives in
//!   `crate::control::possession`.
//!
//! The puppy-slug gun stood here too, because it spawned its slug through the
//! kernel's minion spawner. It asks the summon executor now
//! (`ambition_vfx::SummonCalling::Companion`) and lives in
//! `ambition_abilities::thrown`.
//!
//! `docs/planning/engine/actor-monolith-decomposition.md` carries the argument
//! with its numbers, so the group does not get carved by line count.
//!
pub mod traversal;
