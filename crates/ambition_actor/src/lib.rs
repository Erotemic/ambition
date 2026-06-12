//! Unified actor system.
//!
//! This crate owns the content-free vocabulary that makes an entity controllable:
//!
//! - [`actor`] — control frames, AI state components, pose/faction vocabulary, and
//!   character-catalog data.
//! - [`boss_encounter`] — boss phase progression and spec schema.
//! - [`brain`] — the universal brain/action-set dispatch for players, NPCs,
//!   enemies, and bosses.
//!
//! Named world content stays in `ambition_content`; `ambition_sandbox` re-exports
//! these modules at the historical `crate::actor` / `crate::brain` paths.

pub mod actor;
pub mod boss_encounter;
pub mod brain;
