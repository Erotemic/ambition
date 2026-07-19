//! THE named Ambition game content — everything that names this game's
//! specific world: quests, bosses, items, dialogue, banter, the intro,
//! the enemy roster, music cues, and the cross-content validator.
//!
//! This is the content crate, distinct from the reusable machinery crate
//! `ambition_actors` it depends on. The dependency direction is strict and
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

pub mod audio_registries;
pub mod banter;
pub mod bosses;
/// The character catalog data and curated playable cast, contributed as an
/// immutable provider fragment to the App-local catalog assembly.
pub mod character_catalog;
pub mod content_validation;
pub mod dialogue;
/// The spectator-duel exhibition fight (RoomLoaded consumer + `<<duel>>`).
pub mod duel_arena;
pub mod encounters;
/// The falling-sand prototype room (self-gating content plugin).
#[cfg(feature = "falling_sand")]
pub mod falling_sand;
/// The authored audio registries (music/SFX RON), registered as an App-local
/// provider fragment.
pub mod provider;
// `features` (the feature-ECS actor/boss world) was promoted to
// `ambition_actors::features` (lib root): machinery presentation/dev still read
// its named bits (doc 20 B3/B4), so it stays in the sandbox lib when
// the rest of this content module becomes the `ambition_content`
// crate. Re-exported here so `content::features` paths keep working.
pub use ambition_actors::features;
/// The named hostile-archetype data, contributed as an immutable provider
/// fragment to the App-local roster assembly.
pub mod enemy_roster;
pub mod input_techniques;
pub mod intro;
pub mod items;
#[cfg(feature = "audio")]
pub mod music;
pub mod plugin;
/// Content-owned presentation passes (visible builds; the app adds
/// [`presentation::AmbitionPresentationPlugin`] beside the renderer's plugins).
pub mod presentation;
pub mod projectiles;
pub mod quest;
pub mod quests;
pub mod vanity_card;
/// The LDtk world payload + Ambition's `WorldManifest` (install seam:
/// `ambition_actors::ldtk_world`).
pub mod worlds;

#[cfg(feature = "portal")]
pub mod portal;

pub use plugin::AmbitionContentPlugin;

/// Stable provider identity used by App-local content registries and the shell.
pub const AMBITION_CONTENT_PROVIDER: &str = "ambition";

// Character, hostile-archetype, and boss catalog machinery lives in reusable
// engine crates; this provider contributes only its authored fragments. The
// character entries live in `assets/data/character_catalog.ron`.

/// Facade: the data-manifest *machinery* (spec schema + asset wiring)
/// moved to [`ambition_actors::session::data`]; the authored RON it loads is the
/// content. Inbound `crate::data::…` paths keep working.
pub use ambition_actors::session::data;
