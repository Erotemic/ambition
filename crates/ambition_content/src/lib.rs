//! THE named Ambition game content — everything that names this game's
//! specific world: quests, bosses, items, dialogue, banter, the intro,
//! the enemy roster, music cues, and the cross-content validator.
//!
//! This is the content crate, distinct from the reusable machinery crate
//! `ambition_gameplay_core` it depends on. The dependency direction is strict and
//! one-way — content → machinery, never the reverse — so the named cast and
//! data installed here build on top of the generic schemas/pipelines that
//! live machinery-side. Registration flows through one seam,
//! [`AmbitionContentPlugin`].
//!
//! Most top-level modules are thin install plugins ([`plugin`], [`quests`],
//! [`bosses`], [`dialogue`], [`items`]) that seed named rosters into
//! machinery resources, alongside the authored data/content itself
//! ([`quest`], [`enemy_roster`], [`banter`], [`music`], [`intro`]) and the
//! [`content_validation`] cross-reference checker. Several names re-export
//! their machinery half (e.g. [`data`], [`features`]) so historical
//! `crate::…` paths keep resolving.

pub mod banter;
pub mod bosses;
pub mod content_validation;
pub mod dialogue;
// `features` (the feature-ECS actor/boss world) was promoted to
// `ambition_gameplay_core::features` (lib root): machinery presentation/dev still read
// its named bits (doc 20 B3/B4), so it stays in the sandbox lib when
// the rest of this content module becomes the `ambition_content`
// crate. Re-exported here so `content::features` paths keep working.
pub use ambition_gameplay_core::features;
/// The named enemy roster DATA, installed into the machinery lib at
/// content-plugin build time.
pub mod enemy_roster;
pub mod intro;
pub mod items;
#[cfg(feature = "audio")]
pub mod music;
pub mod plugin;
pub mod quest;
pub mod quests;

#[cfg(feature = "portal")]
pub mod portal;

pub use plugin::AmbitionContentPlugin;

// The character catalog *machinery* (schema, loader, brain resolver,
// validation) moved to `ambition_gameplay_core::actor::character_catalog`; the authored
// entries live in `assets/data/character_catalog.ron`.

/// Facade: the data-manifest *machinery* (spec schema + asset wiring)
/// moved to [`ambition_gameplay_core::session::data`]; the authored RON it loads is the
/// content. Inbound `crate::data::…` paths keep working.
pub use ambition_gameplay_core::session::data;
