//! Sandbox-specific authored content: data manifests, the cross-content
//! validator, the combat-banter registry, the quest registry, and the
//! sandbox-only gameplay feature systems.
//!
//! Roughly: when the `ambition_game` crate is carved out, the analogues
//! of these modules will go *there* — the rules and types stay reusable
//! via the framework, but the specific entities/dialog/quests do not.

pub mod banter;
pub mod character_catalog;
pub mod content_validation;
pub mod data;
pub mod features;
pub mod quest;
