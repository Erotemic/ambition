//! Ambition game-content boundary.
//!
//! This module groups Ambition-specific glue that maps named game content
//! (input channels, the item roster, …) onto the reusable, content-agnostic
//! mechanics in `crate::portal` and the platformer runtime. Reusable mechanics
//! depend only on messages/components; the Ambition-specific bindings live here.
//!
//! Stage 9 / Task H bootstraps this boundary with the portal adapters; Stage 11
//! / Task J expands it to quests, bosses, worlds, music, and dialogue.

#[cfg(feature = "portal")]
pub mod portal;
