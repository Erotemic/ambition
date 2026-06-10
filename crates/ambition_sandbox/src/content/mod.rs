//! Sandbox-specific authored content: data manifests, the cross-content
//! validator, the combat-banter registry, the quest registry, and the
//! sandbox-only gameplay feature systems.
//!
//! Roughly: when the `ambition_game` crate is carved out, the analogues
//! of these modules will go *there* — the rules and types stay reusable
//! via the framework, but the specific entities/dialog/quests do not.

pub mod banter;
pub mod content_validation;
pub mod features;
pub mod quest;

/// Facade: the character catalog *machinery* (schema, loader, brain
/// resolver, validation) moved to [`crate::actor::character_catalog`];
/// the authored entries live in `assets/data/character_catalog.ron`.
pub use crate::actor::character_catalog;

/// Facade: the data-manifest *machinery* (spec schema + asset wiring)
/// moved to [`crate::runtime::data`]; the authored RON it loads is the
/// content. Inbound `crate::content::data::…` paths keep working.
pub use crate::runtime::data;
