//! THE named Ambition game content — everything that names this game's
//! specific world: quests, bosses, items, dialogue, banter, the intro,
//! the enemy roster, music cues, and the cross-content validator.
//!
//! This is the content crate, distinct from the reusable machinery crate
//! `ambition_platformer2d_actor_monolith` it depends on. The dependency direction is strict and
//! one-way — content → machinery, never the reverse — so the named cast and
//! data installed here build on top of the generic schemas/pipelines that
//! live machinery-side. Registration flows through one seam,
//! [`AmbitionContentPlugin`].
//!
//! Most top-level modules are thin install plugins ([`plugin`], [`quests`],
//! [`bosses`], [`dialogue`], [`items`]) that seed named rosters into
//! machinery resources, alongside the authored data/content itself
//! ([`quest`], [`banter`], [`music`], [`intro`]) and the
//! [`content_validation`] cross-reference checker. Several names re-export
//! their machinery half (e.g. [`data`], [`features`]) so historical
//! `crate::…` paths keep resolving.

pub mod audio_registries;
pub(crate) mod authored;
pub mod banter;
pub mod bosses;
/// The character catalog data and curated playable cast, contributed as an
/// immutable provider fragment to the App-local catalog assembly.
pub mod character_catalog;
/// The authored encounter wave timelines, embedded once and read through the
/// PACK — not `ron::from_str(include_str!(..))` at plugin-build time.
///
/// that call site was two readers of one file: the pack could validate bytes
/// the runtime never consulted, and the runtime could `expect`-panic at startup
/// on a serde message the pack never saw.
pub const ENCOUNTER_WAVES_RON: &str =
    include_str!("../assets/data/encounters/goblin_encounter.ron");

pub mod content_validation;
pub mod dialogue;
/// Which of Ambition's named cast may stop thinking — the per-actor dormancy
/// stances for the bosses, the arena mobs, the duel, and the placed cast.
pub mod dormancy;
/// The spectator-duel exhibition fight (RoomLoaded consumer + `<<duel>>`).
pub mod duel_arena;
pub mod encounters;
/// The falling-sand room's `bevy_falling_sand` bridge (water/oil) +
/// presentation (self-gating content plugin, visible binary only).
#[cfg(feature = "falling_sand")]
pub mod falling_sand;
/// The falling-sand room's SIMULATION: the deterministic sand grid, the FS3
/// settled-sand ledger, and the room/switch/spout state. Ungated so its
/// conservation/settling proofs run in every `cargo test -p ambition_content`
/// and the headless harness can drive the room (the F13 lesson: a
/// feature-gated test silently stops running).
pub mod falling_sand_sim;
pub mod pack;
/// The authored audio registries (music/SFX RON), registered as an App-local
/// provider fragment.
pub mod provider;
// `features` (the feature-ECS actor/boss world) was promoted to
// `ambition_platformer2d_actor_monolith::features` (lib root): machinery presentation/dev still read
// its named bits (doc 20 B3/B4), so it stays in the sandbox lib when
// the rest of this content module becomes the `ambition_content`
// crate. Re-exported here so `content::features` paths keep working.
pub use ambition_platformer2d_actor_monolith::features;
pub mod alice_moveset;
/// How a fighter borrows an archetype's timings under its own name.
pub mod archetype_moveset;
pub mod author_moveset;
pub mod bob_moveset;
pub mod carl_stargan_moveset;
pub mod cellular_automaton_moveset;
pub mod emmy_noether_moveset;
/// The named hostile-archetype data, contributed as an immutable provider
/// fragment to the App-local roster assembly.
pub mod goblin_moveset;
pub mod input_techniques;
pub mod intro;
pub mod items;
/// Test-only: it owns the cross-table invariant no single fighter's module can state — that an
/// authored burst is heard exactly once.
pub mod authored_movesets;
#[cfg(test)]
mod moveset_sound;
#[cfg(feature = "audio")]
pub mod music;
pub mod ninja_shadow_oni_leader_moveset;
pub mod medic_moveset;
pub mod officer_moveset;
pub mod special_slots;
pub mod oiler_moveset;
pub mod patent_clerk_moveset;
pub mod performer_moveset;
pub mod pirate_admiral_moveset;
pub mod player_robot_lineage;
pub mod player_robot_moveset;
pub mod plugin;
pub mod pointed_polygon_moveset;
/// Content-owned presentation passes (visible builds; the app adds
/// [`presentation::AmbitionPresentationPlugin`] beside the renderer's plugins).
pub mod presentation;
pub mod projectile_polygon_moveset;
pub mod projectiles;
pub mod pugnacious_polygon_moveset;
pub mod quest;
pub mod quests;
/// This game's Yarn vocabulary — `<<give_item>>`, `<<buy_item>>`,
/// `<<challenge>>` and the save-state mirror its `<<if>>` functions read.
///
/// it lived in the ENGINE crate until. `ambition_dialog` exposes
/// `YarnContentBindings` so a host pushes its own commands in from outside, and
/// this crate already pushed two installers through it; this is the third and
/// largest.
#[cfg(feature = "ui")]
pub mod yarn_vocabulary;
// no `vanity_card` module. The rendered frames, their manifest and `tools/vanity_card_prep` stay on
// disk as REFERENCE art — nothing in the game reads them.
/// The LDtk world payload + Ambition's `WorldManifest` (install seam:
/// `ambition_platformer2d_ldtk`).
pub mod worlds;

#[cfg(feature = "portal")]
pub mod portal;

pub use plugin::AmbitionContentPlugin;

/// Stable provider identity used by App-local content registries and the shell.
pub const AMBITION_CONTENT_PROVIDER: &str = "ambition";

// Character, hostile-archetype, and boss catalog machinery lives in reusable
// engine crates; this provider contributes only its authored fragments. The
// character entries live in `assets/data/character_catalog.ron`.

/// Inbound `crate::data::…` paths keep working.
pub use ambition_platformer2d_actor_monolith::session::data;

/// Declare every rollback row owned by Ambition-specific content.
///
/// Content plugins record the same declarations into host-independent schema
/// metadata. The application composition calls this with its concrete rollback
/// registrar after selecting a backend, keeping content independent of GGRS.
pub fn register_rollback_state(
    registrar: &mut impl ambition_platformer2d_core::snapshot::RollbackRegistrar,
) {
    bosses::register_rollback_state(registrar);
    #[cfg(feature = "falling_sand")]
    falling_sand::register_rollback_state(registrar);
    #[cfg(feature = "portal")]
    portal::register_rollback_state(registrar);
}
