//! The actor BEHAVIOR + identity layer — the "minds and cast" of the
//! workspace.
//!
//! Sits one level above [`ambition_platformer2d_core`] (the pure movement/physics
//! model): this crate owns the content-free vocabulary that makes an entity
//! controllable and gives it a behavior and an identity. The same brain +
//! control-frame contract drives players, NPCs, enemies, and bosses.
//!
//! - [`actor`] — the `ActorControl`/`ActorControlFrame` contract that
//!   simulation code consumes uniformly, plus AI intent
//!   (`CharacterAiIntent`), pose/faction vocabulary, and the
//!   character-catalog/roster data.
//! - [`brain`] — the universal brain/action-set dispatch (`StateMachine`,
//!   `BossPattern`, player, and Smash-style brains) that reads a snapshot
//!   and writes intent into an `ActorControlFrame`.
//! - [`boss_encounter`] — boss phase progression and spec schema (the
//!   phase logic; per-phase attack data lives in [`brain`]).
//!
//! Named world content (the actual cast of bosses/enemies) stays in
//! `ambition_content`; `ambition_platformer2d_actor_monolith` re-exports these modules at the
//! historical `crate::actor` / `crate::brain` paths.
//!
//! # "CONTENT-FREE" IS THE WRONG AXIS FOR THIS CRATE, AND THAT IS MEASURED
//!
//! The rule above is stated, and it could not have refused the thing that
//! actually accumulated here. **15,928 lines of platform-fighter policy**
//! (`brain/fighter`, `brain/smash`) sit in this crate, and **zero of them are
//! named world content** — nothing there mentions a `CharacterId` or the
//! character catalog. Every line passed the stated boundary while being exactly
//! what a floor crate must not hold.
//!
//! **this crate is a FLOOR — every composition links it**, including a
//! movement-only game with no fighters in it. So the axis that matters is not
//! content-vs-vocabulary but:
//!
//! **DOES EVERY GAME BUILT ON THIS ENGINE NEED THIS WORD?** A brain contract,
//! a control frame, a faction, a pose: yes. A platform fighter's option
//! generator, its ledge-recovery search, its capture repertoire: **no** —
//! content-free and genre-specific are not the same thing, and only one of them
//! is a reason to live in the floor.

pub mod action_scheme;
pub mod actor;
pub mod binding_namespaces;
pub mod boss_encounter;
pub mod brain;
pub mod equipment;
pub mod moveset_authoring;
pub mod moveset_prefabs;
pub mod perception;
pub mod prepared;
#[cfg(any(test, feature = "test-support"))]
pub mod prepared_fixtures;
pub mod smash_capture;
pub mod smash_fighter;
pub mod smash_repertoire;
mod snapshot_impls;
pub mod technique;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
