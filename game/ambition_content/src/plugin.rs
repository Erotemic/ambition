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

        // Install the authored music/SFX registries into the engine's
        // audio-data seam (R3.2 — the engine ships no tracks/cues). The app
        // startup choke point also installs; first install wins.
        super::audio_registries::install();

        // Install the character catalog into the engine's roster seam before
        // any lookup (LDtk NpcSpawn conversion, spawn paths, sprite joins).
        super::character_catalog::install();

        // Install the world manifest (which .ldtk files exist + the entry
        // room) before any catalog build or world load reads it.
        super::worlds::install();

        // Install the authored item catalog (C1 — content out of core) into the
        // machinery lib before any item flavor/wiring is read. Byte-identical to
        // the engine's built-in 24-item default table (pinned by
        // `items_ron_matches_builtin_defaults`), so shipped items read unchanged;
        // a content game re-authors an item by editing its row in `items.ron`.
        // Additive: absent rows / a content-less build fall back to the built-in
        // default, so this never gates a spawn on the install.
        ambition_items::install_item_catalog(ambition_items::ItemCatalog::from_ron(include_str!(
            "../assets/data/items.ron"
        )));

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
        ambition_encounter::install_encounter_waves(
            ron::from_str(include_str!(
                "../assets/data/encounters/goblin_encounter.ron"
            ))
            .expect("goblin_encounter.ron should parse as an encounter wave book"),
        );

        // The spectator duel stages itself when its room finishes loading
        // (the engine's RoomLoaded fact — JD4 room-mechanics-by-kind seam).
        app.add_systems(
            bevy::prelude::Update,
            super::duel_arena::stage_duel_on_room_loaded,
        );
        #[cfg(feature = "ui")]
        {
            app.init_resource::<ambition_dialog::YarnContentBindings>();
            app.world_mut()
                .resource_mut::<ambition_dialog::YarnContentBindings>()
                .installers
                .push(super::duel_arena::install_duel_yarn_binding);
        }

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
