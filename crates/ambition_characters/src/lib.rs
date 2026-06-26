//! The actor BEHAVIOR + identity layer — the "minds and cast" of the
//! workspace.
//!
//! Sits one level above [`ambition_engine_core`] (the pure movement/physics
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
//! `ambition_content`; `ambition_gameplay_core` re-exports these modules at the
//! historical `crate::actor` / `crate::brain` paths.

pub mod actor;
pub mod boss_encounter;
pub mod brain;
pub mod perception;
