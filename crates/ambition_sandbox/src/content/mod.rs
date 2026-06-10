//! THE Ambition content module — everything that names this game's
//! specific world: quests, bosses, items, dialogue, banter, the intro,
//! the feature/actor roster, and the cross-content validator.
//!
//! Unified from the former `content/` + `ambition_content/` pair
//! (Stage 20 / A1): one inward-facing content tree with a single
//! dependency direction — content → machinery, never the reverse.
//! Registration flows through one seam, [`AmbitionContentPlugin`].
//! `crate::ambition_content::…` paths still resolve via the alias in
//! `lib.rs`.
//!
//! This module is the seed of the future `ambition_content` crate:
//! when it is promoted, these submodules go there while the reusable
//! machinery (mechanics, runtime, presentation, …) stays behind.

pub mod banter;
pub mod bosses;
pub mod content_validation;
pub mod dialogue;
// `features` (the feature-ECS actor/boss world) was promoted to
// `crate::features` (lib root): machinery presentation/dev still read
// its named bits (doc 20 B3/B4), so it stays in the sandbox lib when
// the rest of this content module becomes the `ambition_content`
// crate. Re-exported here so `content::features` paths keep working.
pub use crate::features;
pub mod intro;
pub mod items;
pub mod plugin;
pub mod quest;
pub mod quests;

#[cfg(feature = "portal")]
pub mod portal;

pub use plugin::AmbitionContentPlugin;

// The character catalog *machinery* (schema, loader, brain resolver,
// validation) moved to `crate::actor::character_catalog`; the authored
// entries live in `assets/data/character_catalog.ron`.

/// Facade: the data-manifest *machinery* (spec schema + asset wiring)
/// moved to [`crate::runtime::data`]; the authored RON it loads is the
/// content. Inbound `crate::content::data::…` paths keep working.
pub use crate::runtime::data;
