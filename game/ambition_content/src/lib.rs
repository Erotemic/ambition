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
//! ([`quest`], [`music`], [`intro`]) and the
//! [`content_validation`] cross-reference checker. Several names re-export
//! their machinery half (e.g. [`data`], [`features`]) so historical
//! `crate::…` paths keep resolving.

pub mod audio_registries;
pub mod bosses;
// The native wielded abilities, now modules, and the parity suite that holds
// each module to them.
#[cfg(test)]
mod wielded_ability_parity_tests;
#[cfg(test)]
mod wielded_ability_reference_tests;
// Two entries on one held item pay for one action at a time.
#[cfg(test)]
mod wielded_cost_admission_tests;
#[cfg(test)]
mod sentry_parity_tests;
#[cfg(test)]
mod vortex_parity_tests;
/// The character catalog data and curated playable cast, contributed as an
/// immutable provider fragment to the App-local catalog assembly.
pub mod character_catalog;
/// The authored encounter wave timelines, embedded once and read through the
/// PACK — not `ron::from_str(include_str!(..))` at plugin-build time.
///
/// that call site was two readers of one file: the pack could validate bytes
/// the runtime never consulted, and the runtime could `expect`-panic at startup
/// on a serde message the pack never saw.
///
/// A pack source, read off disk in desktop development and embedded under
/// `static_content` (see [`pack::pack_source_text`]).

pub mod content_validation;
pub mod dialogue;
/// Which of Ambition's named cast may stop thinking — the per-actor dormancy
/// stances for the bosses, the arena mobs, the duel, and the placed cast.
/// The spectator-duel exhibition fight (RoomLoaded consumer + `<<duel>>`).
pub mod duel_arena;
pub mod encounters;
pub mod pack;
/// The running game plays a content edit: the declared sources are watched,
/// and a change asks for a reload.
#[cfg(not(feature = "static_content"))]
pub mod content_watch;
/// The running host's move-table reload — the revision road's first customer.
pub mod reload;
/// The candidate a reload is about.
///
/// ⛔ RE-EXPORTED BECAUSE [`reload::request_reload`] TAKES IT AND NOTHING ELSE
/// HERE HANDS IT OVER. A public function whose parameter type a caller cannot
/// name from the same crate is a public function nobody outside the workspace's
/// own dependency graph can call — measured by trying to write the app-level
/// edit→play witness, which had to reach past `ambition_content` for a type
/// `ambition_content`'s own signature demands.
pub use ambition_content_pack::CandidateGeneration;
/// The authored audio registries (music/SFX RON), registered as an App-local
/// provider fragment.
pub mod provider;
// `features` (the feature-ECS actor/boss world) was promoted to
// `ambition_platformer2d_actor_monolith::features` (lib root): machinery presentation/dev still read
// its named bits (doc 20 B3/B4), so it stays in the sandbox lib when
// the rest of this content module becomes the `ambition_content`
// crate. Re-exported here so `content::features` paths keep working.
pub use ambition_platformer2d_actor_monolith::features;
#[cfg(test)]
mod alice_moveset;
/// How a fighter borrows an archetype's timings under its own name.
#[cfg(test)]
mod director_moveset;
#[cfg(test)]
mod bob_moveset;
#[cfg(test)]
mod carl_stargan_moveset;
#[cfg(test)]
mod cellular_automaton_moveset;
#[cfg(test)]
mod emmy_noether_moveset;
/// The named hostile-archetype data, contributed as an immutable provider
/// fragment to the App-local roster assembly.
#[cfg(test)]
mod goblin_moveset;
pub mod input_techniques;
/// The prop sheets its rooms name: the pack's `data/prop_sheets.ron`.
pub(crate) mod prop_sheets;
pub mod intro;
/// Ambition's day: its length, its phases, its light.
pub mod day_cycle;
pub mod items;
/// The town of the Math dimension (`math.ldtk`) and the gate pair to it.
pub mod math_town;
pub mod primeval;
/// Test-only: it owns the cross-table invariant no single fighter's module can state — that an
/// authored burst is heard exactly once.
pub mod authored_movesets;
mod moves_are_content;
#[cfg(test)]
mod moveset_sound;
/// Test-only: can a loadable artifact carry the SHIPPED roster's move tables
/// exactly? I2's precondition, asked of the whole set rather than one fighter.
#[cfg(test)]
mod moveset_artifact;
#[cfg(feature = "audio")]
pub mod music;
#[cfg(test)]
mod ninja_shadow_oni_leader_moveset;
#[cfg(test)]
mod medic_moveset;
#[cfg(test)]
mod officer_moveset;
#[cfg(test)]
mod oiler_moveset;
#[cfg(test)]
mod gnu_ton_moveset;
#[cfg(test)]
mod patent_clerk_moveset;
#[cfg(test)]
mod performer_moveset;
#[cfg(test)]
mod pirate_admiral_moveset;
#[cfg(test)]
mod ramen_nujan_moveset;
pub mod player_robot_lineage;
pub mod player_robot_moveset;
pub mod plugin;
#[cfg(test)]
mod pointed_polygon_moveset;
/// Content-owned presentation passes (visible builds; the app adds
/// [`presentation::AmbitionPresentationPlugin`] beside the renderer's plugins).
pub mod presentation;
#[cfg(test)]
mod projectile_polygon_moveset;
pub mod projectiles;
pub mod tuning;
#[cfg(test)]
mod pugnacious_polygon_moveset;
pub mod quest;
pub mod room_look_state;
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

/// The sandbox save file is Ambition's: the shell gives the live save to the
/// experience with this id, so the two ids must be one.
#[cfg(test)]
#[test]
fn ambition_owns_the_sandbox_save_file() {
    assert_eq!(AMBITION_CONTENT_PROVIDER, ambition_persistence::save::SANDBOX_SAVE_OWNER);
}

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
}
