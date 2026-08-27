//! Ambition game-content registration.
//!
//! Visible and headless builds register the same named quests, bosses,
//! dialogue, characters, authored catalogs, and game-specific adapters here.
//! Runtime mechanisms and asset/world bootstrapping remain in app assembly.

use bevy::prelude::*;

/// Install Ambition's named content into the simulation app.
pub struct AmbitionContentPlugin;

impl Plugin for AmbitionContentPlugin {
    fn build(&self, app: &mut App) {
        // Game-authored dormancy policy per actor.
        super::dormancy::register(app);

        // Named projectile presentation used by Ambition content.
        super::projectiles::register(app);

        // Named motion techniques used by the player kit.
        super::input_techniques::register(app);

        // App-local authored audio registries.
        super::audio_registries::register(app);

        // App-local character catalog fragment.
        super::character_catalog::register(app);

        // Register each robot incarnation as a complete character.
        super::player_robot_lineage::register(app);
        // Ensure every character offered by this provider is constructible.
        super::player_robot_lineage::register_declared_cast(app);

        // App-local world manifest shared by runtime and presentation readers.
        app.insert_resource(super::worlds::world_manifest());

        // Install the validated item catalog lowered from the prepared pack.
        ambition_items::install_item_catalog(
            ambition_items::content_schema::lowered_item_catalog(crate::pack::prepared())
                .expect("the items schema lowers its catalog for every pack that compiles")
                .clone(),
        );

        // Register Ambition's adaptive music catalog under its content provider.
        #[cfg(feature = "audio")]
        {
            let cue_catalog = crate::music::ambition_music_cue_catalog();
            use ambition_audio::music::AdaptiveMusicCatalogAppExt;
            app.register_adaptive_music_catalog(crate::AMBITION_CONTENT_PROVIDER, cue_catalog);
        }

        // Install validated encounter waves lowered from the prepared pack.
        ambition_encounter::install_encounter_waves(
            app,
            ambition_encounter::content_schema::lowered_encounter_waves(crate::pack::prepared())
                .cloned()
                .expect("the encounter schema lowers its book for every pack that compiles"),
        );

        // Game-authored CPU difficulty ladder consumed during fighter construction.
        app.insert_resource(ambition_characters::brain::fighter::AuthoredFighterLadder(
            ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(
                crate::pack::prepared(),
            )
            .cloned()
            .expect("the fighter-ladder schema lowers its rungs for every pack that compiles"),
        ));

        // Spectator duel staging is room-construction content.
        app.init_resource::<ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>();
        super::duel_arena::register_duel_content_staging(
            &mut app
                .world_mut()
                .resource_mut::<ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>(),
        );
        #[cfg(feature = "ui")]
        {
            app.init_resource::<ambition_dialog::YarnContentBindings>();
            app.world_mut()
                .resource_mut::<ambition_dialog::YarnContentBindings>()
                .installers
                .push(super::duel_arena::install_duel_yarn_binding);
            // the game's OWN vocabulary, pushed through the same seam as the two content
            // installers beside it.
            app.world_mut()
                .resource_mut::<ambition_dialog::YarnContentBindings>()
                .installers
                .push(super::yarn_vocabulary::install_game_bindings);
            // The per-frame mirror the Yarn `<<if>>` functions read. It joins the
            // CONSUMER side of `YarnStateMirrorRefreshed`, which is where a
            // game's own mirror belongs — the engine's set has one member and it
            // is not this.
            app.add_systems(
                bevy::prelude::Update,
                super::yarn_vocabulary::refresh_yarn_state_mirror
                    .in_set(ambition_dialog::YarnStateMirrorRefreshed)
                    .after(ambition_dialog::YarnPresentationCueCleared),
            );
        }
        // `<<duel>>` stages two fighters, which is a simulation act reached from
        // the non-rewound Yarn runner — so the request is stamped against the
        // conversation and released on the tick it belongs to. Registered
        // unconditionally: the ledger and its release are `ui`-free, and only
        // the command that fills it is behind the runner.
        app.add_plugins(ambition_conversation::NarrativeInputPlugin::<
            ambition_platformer2d_actor_monolith::features::SpawnActorRequest,
        >::default());

        app.add_plugins(super::quests::AmbitionQuestContentPlugin);
        app.add_plugins(super::bosses::AmbitionBossContentPlugin);
        app.add_plugins(super::encounters::AmbitionEncounterContentPlugin);
        app.add_plugins(super::dialogue::AmbitionDialogueContentPlugin);

        // Intro story content and visible-build presentation rows.
        app.add_plugins(crate::intro::IntroPlugin);

        // Ambition-specific portal adapters; the reusable portal core is installed
        // separately in `add_simulation_plugins`.
        #[cfg(feature = "portal")]
        app.add_plugins(super::portal::AmbitionPortalAdaptersPlugin);

        // Deterministic falling-sand simulation, gated with its feature bundle.
        #[cfg(feature = "falling_sand")]
        app.add_plugins(crate::falling_sand_sim::FallingSandSimPlugin);
    }
}
