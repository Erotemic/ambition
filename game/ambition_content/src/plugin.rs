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
        // Contribute Ambition's hostile-archetype fragment to this App before
        // any spawn system runs. Registration is deterministic, transactional,
        // and independent of other Apps in the process.
        super::enemy_roster::register(app);

        // Register every named projectile look (player kit, apple rain,
        // lasersword, glider) into the reusable, empty-by-default projectile
        // visual catalog. Sim-scoped (not presentation-only) so the lasersword's
        // detonation FX resolves in headless builds too; the reusable projectile
        // crate names none of these looks.
        super::projectiles::register(app);

        // Register the player kit's named motion techniques (qcf / qcf_grace /
        // hcf) into the reusable, empty-by-default motion-technique catalog. The
        // reusable input crate names no gesture; the fire system asks by id.
        super::input_techniques::register(app);

        // Contribute authored music/SFX to this Bevy App. Re-registering the
        // identical provider fragment is idempotent, so hosts may compose this
        // plugin without coordinating a process-global install order.
        super::audio_registries::register(app);

        // Contribute the provider's character fragment to the App-local
        // assembly. Runtime simulation, presentation, dialogue, and authored
        // attack geometry all read the assembled resource explicitly.
        super::character_catalog::register(app);

        // Publish this provider's world manifest (which .ldtk files exist +
        // the entry room) as an App-local resource, so in-schedule readers —
        // the tile-render spine's handle load, the hot-reload transaction,
        // per-session visual spine spawn — take it as a `Res`. Pre-App and
        // plugin-build readers get the same value as a `&WorldManifest`
        // argument from whoever prepares them. No process global: a second
        // provider in this process publishes its own into its own App.
        app.insert_resource(super::worlds::world_manifest());

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
        // it. A content-less provider simply contributes no adaptive catalog;
        // the director resolves through the selected provider's App-local
        // `AdaptiveMusicCatalogRegistry`. The catalog is CONTENT — it left the
        // machinery lib's audio plugin here (the B1 seam). The App-local
        // registry holds complete provider catalogs; the active audio context
        // selects one provider's definitions at runtime.
        #[cfg(feature = "audio")]
        {
            let cue_catalog = crate::music::ambition_music_cue_catalog();
            use ambition_audio::music::AdaptiveMusicCatalogAppExt;
            app.register_adaptive_music_catalog(crate::AMBITION_CONTENT_PROVIDER, cue_catalog);
        }

        // Install authored encounter wave timelines (goblin mob-lab, …) into the
        // machinery lib's wave book before the encounter loader runs — the engine
        // hard-codes no encounter's waves.
        ambition_encounter::install_encounter_waves(
            ron::from_str(include_str!(
                "../assets/data/encounters/goblin_encounter.ron"
            ))
            .expect("goblin_encounter.ron should parse as an encounter wave book"),
        );

        // The spectator duel is the arena room's registered content staging:
        // part of room construction (every path — activation, transition,
        // reset, restore staging — rebuilds it), not a RoomLoaded consumer.
        app.init_resource::<ambition_actors::features::RoomContentStagingRegistry>();
        super::duel_arena::register_duel_content_staging(
            &mut app
                .world_mut()
                .resource_mut::<ambition_actors::features::RoomContentStagingRegistry>(),
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
        app.add_plugins(super::encounters::AmbitionEncounterContentPlugin);
        app.add_plugins(super::dialogue::AmbitionDialogueContentPlugin);

        // Installs intro cutscenes, room bindings, dialogue, and visible-build NPC
        // sprite rows while keeping story content out of sandbox-owned files.
        app.add_plugins(crate::intro::IntroPlugin);

        // Ambition-specific portal adapters; the reusable portal core is installed
        // separately in `add_simulation_plugins`.
        #[cfg(feature = "portal")]
        app.add_plugins(super::portal::AmbitionPortalAdaptersPlugin);

        // The falling-sand room's SIM half (deterministic sand grid + settled
        // ledger). Headless-safe — the module is ungated — but registered
        // under the feature so bundle semantics match the presentation half.
        #[cfg(feature = "falling_sand")]
        app.add_plugins(crate::falling_sand_sim::FallingSandSimPlugin);
    }
}
