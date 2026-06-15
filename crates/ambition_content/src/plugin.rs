//! [`AmbitionContentPlugin`] — named Ambition game-content registration.
//!
//! The app installs this composer once during simulation setup so visible and
//! headless builds register the same named content through one seam.
//!
//! Today it composes the quest roster, boss roster, dialogue/cutscene/banter
//! content, intro/cut-rope story hooks, and — behind the `portal` feature —
//! Ambition-specific portal input/inventory adapters.
//!
//! Mechanic runtime state, world/asset catalog bootstrapping, and the starter item
//! roster keep their existing app-assembly insertion points because those depend on
//! runtime ordering outside named-content registration.

use bevy::prelude::*;

/// Installs all named Ambition game-content registration.
///
/// Mounted by `app::plugins::add_simulation_plugins` (after the runtime /
/// mechanic plugins) so both visible and headless builds register the same
/// named content in the same place.
pub struct AmbitionContentPlugin;

impl Plugin for AmbitionContentPlugin {
    fn build(&self, app: &mut App) {
        // Install the named enemy roster into the machinery lib BEFORE any
        // spawn system runs (plugin build precedes all systems), so every
        // enemy spawn resolves against this authored data, not the lib's
        // standalone fallback.
        super::enemy_roster::install();

        // Insert Ambition's authored music-cue catalog (the goblin adaptive tune
        // + its encounter binding) so the reusable ambition_audio director plays
        // it. The director takes Option<Res<MusicCueCatalog>>, so a content-less
        // build just has no adaptive music. The catalog is CONTENT — it left the
        // machinery lib's audio plugin here (the B1 seam).
        #[cfg(feature = "audio")]
        app.insert_resource(crate::music::ambition_music_cue_catalog());

        // Install authored encounter wave timelines (goblin mob-lab, …) into the
        // machinery lib's wave book before the encounter loader runs — the engine
        // hard-codes no encounter's waves.
        ambition_sandbox::encounter::install_encounter_waves(
            ron::from_str(include_str!(
                "../assets/data/encounters/goblin_encounter.ron"
            ))
            .expect("goblin_encounter.ron should parse as an encounter wave book"),
        );

        app.add_plugins(super::quests::AmbitionQuestContentPlugin);
        app.add_plugins(super::bosses::AmbitionBossContentPlugin);
        app.add_plugins(super::dialogue::AmbitionDialogueContentPlugin);

        // Installs intro cutscenes, room bindings, dialogue, and visible-build NPC
        // sprite rows while keeping story content out of sandbox-owned files.
        app.add_plugins(crate::intro::IntroPlugin);

        // Ambition-specific portal adapters; the reusable portal core is installed
        // separately in `add_simulation_plugins`.
        #[cfg(feature = "portal")]
        app.add_plugins(super::portal::AmbitionPortalAdaptersPlugin);
    }
}
