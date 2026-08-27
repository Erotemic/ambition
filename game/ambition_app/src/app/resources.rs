use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::{IntGridRendering, LdtkSettings, LevelBackground};

use ambition_content::content_validation;
use ambition_platformer2d::actors::session::data;
use ambition_platformer2d::combat::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d::sim as physics;
use ambition_platformer2d::dev_tools::dev_tools::{
    DeveloperTools, EditableAbilitySet, EditableMovementTuning, EditablePlayerStats,
};
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::ldtk_map as ldtk_world;

use super::cli::cli_start_room_arg;

/// Programmatic start-room override. Platformer2dSimHarness and other library
/// callers insert this resource before `init_sandbox_resources` runs;
/// the function consumes it (taking precedence over the
/// `--start-room` CLI flag) so callers do not need to manipulate
/// `std::env::args` to pin a starting room.
#[derive(Resource, Clone, Debug)]
pub struct StartRoomOverride(pub String);

/// Treat an unresolvable [`StartRoomOverride`] as FATAL.
///
/// The ordinary behaviour is a warning and the authored start room, which is
/// right for the game: a stale `--start-room` in someone's shell history should
/// not stop them playing. It is wrong for a VERIFICATION TOOL, where quietly
/// photographing a different room is the worst thing the tool can do — Z1 said
/// so when `capture_scene` was first written (*"it should FAIL LOUDLY on an
/// unknown room or route rather than falling back — a capture that silently
/// photographs somewhere else is how a blind agent reports the wrong thing with
/// confidence"*) and then it shipped without this.
///
/// Opt-in, so nothing else changes behaviour.
#[derive(bevy::prelude::Resource, Debug, Clone, Copy, Default)]
pub struct StartRoomMustResolve;

/// Host composition input selecting the character for the next prepared world.
///
/// This resource is consumed during sandbox preparation and never becomes gameplay authority.
#[derive(Resource, Clone, Debug, Default)]
pub struct StartingCharacterOverride(pub ambition_platformer2d::actors::avatar::StartingCharacter);

/// Host composition input: this composition SEATS A MATCH into Ambition's
/// world, so it must not also lower a home avatar.
///
/// Same shape and same lifetime as [`StartingCharacterOverride`] — consumed
/// during preparation, never gameplay authority — and it answers the other half
/// of the same question: that one picks WHICH character the home body wears,
/// this one says whether there is a home body at all.
///
/// not a convenience. A match's local seat and an exploration home avatar
/// both claim the session's control channel, and preparation refuses that
/// combination by name rather than building two bodies that fight over it. A
/// composition that seats fighters into this world — a rollback match fixture,
/// a CPU-vs-CPU bout rig — has to be able to SAY so before the world is built,
/// because by the time the roster is published the avatar already exists.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SeatsAMatchInsteadOfAHomeBody;

#[cfg(test)]
fn sandbox_init_failed() -> ! {
    panic!("sandbox resource initialization failed; see diagnostics above");
}

#[cfg(not(test))]
fn sandbox_init_failed() -> ! {
    std::process::exit(2);
}

pub fn init_sandbox_resources(app: &mut App) {
    // Register this provider's authored audio and character fragments into the
    // App-local registries. Boss content is contributed by
    // `AmbitionBossContentPlugin` through the same App-local composition model.
    ambition_content::audio_registries::register(app);
    ambition_content::character_catalog::register(app);
    ambition_content::bosses::register(app);
    // This inserted the same value the content plugin does — idempotent, since
    // both call `ambition_content::worlds::world_manifest()` — but two writers
    // of one global is the shape that has cost this repo a roster, a rebuild, a
    // retirement and a countdown. The provider that OWNS the worlds publishes
    // the declaration; the host reads it.
    //
    // the local value stays: it is threaded BY REFERENCE into every
    // preparation-time reader below (catalog rows, the LDtk load, the room-set
    // conversion, the hot-reload watcher), which run before any schedule and so
    // cannot take a `Res`. That is the K2a shape — no process global — and it is
    // unaffected by who inserts the resource.
    let world_manifest = ambition_content::worlds::world_manifest();

    let sandbox_data = data::Platformer2dGameplayDefaults::load_embedded();
    // Audio lives in its own registries, separate from sandbox tuning and
    // from each other (SFX synthesis vs. generated music pointers).
    let (music_registry, sfx_registry) = {
        let catalogs = app
            .world()
            .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
        (
            catalogs
                .music_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .expect("Ambition music fragment registered")
                .clone(),
            catalogs
                .sfx_for(ambition_content::AMBITION_CONTENT_PROVIDER)
                .expect("Ambition SFX fragment registered")
                .clone(),
        )
    };
    // K2b edit 4: the direct-entry AUDIO branch is gone with its host.
    //
    // It selected the active audio authority statically at composition, on the
    // argument that a direct-entry process runs exactly one provider. There is
    // no direct-entry process any more: `select_shell_audio_context`
    // (`game_shell/src/session.rs`) owns selection AND `SfxEmissionContext` on
    // activation, and it did even while this branch existed — the branch simply
    // ran first in a composition the shell never touched.
    //
    // the registries above are still read here, and still needed: they are
    // what the provider's audio fragment is published FROM. Only the static
    // selection went.
    let character_catalog = app
        .world()
        .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
        .clone();
    let boss_catalog = app
        .world()
        .resource::<ambition_platformer2d::boss_encounter::BossCatalog>()
        .clone();
    // Provider-authored sheets (U1). Cloned like the catalogs above; empty is
    // the ordinary state for an app whose providers author none, and the intro
    // sprite rows resolve exactly as before when it is.
    let authored_sheets = app
        .world()
        .get_resource::<ambition_platformer2d::character::AuthoredSheets>()
        .cloned()
        .unwrap_or_default();

    // Build the singleton Platformer2dAssetCatalog before anything else asks
    // it for a path. Every asset path/source policy in the visible
    // sandbox flows through this — LDtk, SFX bank, fonts, sprites,
    // music. Consumes the music registry so music-track ids land in the
    // catalog.
    let asset_config = app
        .world()
        .get_resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssetConfig>()
        .cloned()
        .unwrap_or_default();
    let sandbox_catalog =
        ambition_platformer2d::actors::assets::platformer_assets::build_sandbox_catalog_with(
            &asset_config,
            &character_catalog,
            &boss_catalog,
            &music_registry,
            &world_manifest,
            |manifest| {
                ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                    manifest,
                    &asset_config.sprite_folder,
                    &authored_sheets,
                    &character_catalog,
                );
            },
        );
    #[cfg(feature = "audio")]
    let sfx_bank_asset_path = sandbox_catalog
        .path_for(&ambition_platformer2d::asset_manager::platformer_assets::ids::sfx_bank())
        .map(|path| {
            ambition_platformer2d::audio::SfxBankAssetPath::new(
                ambition_content::AMBITION_CONTENT_PROVIDER,
                path,
            )
        });

    let ldtk_project =
        match ldtk_world::LdtkProject::load_default(&sandbox_catalog, &world_manifest) {
            Ok(project) => project,
            Err(error) => {
                eprintln!("failed to load sandbox LDtk map: {error}");
                sandbox_init_failed();
            }
        };
    let content_report = content_validation::validate_content_graph(
        &music_registry,
        &ldtk_project,
        &character_catalog,
    );
    for warning in &content_report.warnings {
        eprintln!("content validation warning: {warning}");
    }
    if !content_report.is_ok() {
        eprintln!("sandbox content graph failed validation; fix authored content before running:");
        for error in &content_report.errors {
            eprintln!("  - {error}");
        }
        sandbox_init_failed();
    }
    let editable_abilities = EditableAbilitySet::from(sandbox_data.abilities);
    let editable_tuning = EditableMovementTuning::from(sandbox_data.tuning);
    // The simulation's authority, seeded from the same authored value.
    let active_tuning =
        ambition_platformer2d::engine_core::ActiveMovementTuning(sandbox_data.tuning);
    let mut room_set =
        match ldtk_project.to_room_set(&world_manifest, &crate::composed_ldtk_vocabulary()) {
            Ok(room_set) => room_set,
            Err(errors) => {
                eprintln!(
                    "sandbox LDtk world failed validation; fix the configured map before running:"
                );
                for error in &errors {
                    eprintln!("  - {error}");
                }
                sandbox_init_failed();
            }
        };
    // Programmatic override (Platformer2dSimHarness / library callers) takes
    // precedence over the CLI flag. Either one resolving by id wins;
    // the other is silently ignored. If neither matches, the LDtk
    // project's authored start room stays active.
    let strict_start_room = app
        .world_mut()
        .remove_resource::<StartRoomMustResolve>()
        .is_some();
    let resource_override = app
        .world_mut()
        .remove_resource::<StartRoomOverride>()
        .map(|r| r.0);
    // A FLAG A HUMAN TYPED IS A REQUEST, NOT A PREFERENCE.
    //
    // * a PROGRAMMATIC override comes from a library caller (`Platformer2dSimHarness`, the RL
    //   harness) that may legitimately name a room outside this composition;
    //   falling back is the tolerant, correct answer.
    // * the CLI FLAG was typed, just now, by somebody who wanted that room. The
    //   likeliest mistake is using an LDtk LEVEL name (`central_hub_main`) for a
    //   runtime room id (`central_hub_complex`), which is exactly the error a
    //   silent fallback hides — and the same one that cost the room sweep two
    //   captures before `StartRoomMustResolve` existed.
    //
    // the tolerant case the marker's doc argues for — *"a stale `--start-room`
    // in someone's shell history should not stop them playing"* — is the one this
    // does NOT change its mind about lightly. But a stale flag in history is still
    // a flag the user is passing, and booting a different room without saying so
    // is not playing: it is playing something else. The error lists every valid
    // id, so recovering is reading one line rather than hunting a file.
    let cli_start_room = cli_start_room_arg();
    let strict_start_room = strict_start_room || cli_start_room.is_some();
    if let Some(start_room) = resource_override.or(cli_start_room) {
        if room_set.set_start_by_id(&start_room) {
            eprintln!("[ambition] start room: {start_room}");
        } else if strict_start_room {
            // NAME WHAT IS AVAILABLE. The ids this resolves against are runtime
            // room ids, which are NOT the LDtk level identifiers — reading the
            // `.ldtk` file and using the level names it lists is how the sweep
            // that found this asked for two rooms that do not exist. An error
            // that only says "no" sends the reader back to the wrong file.
            let mut available: Vec<&str> = room_set.rooms.iter().map(|r| r.id.as_str()).collect();
            available.sort_unstable();
            panic!(
                "start-room '{start_room}' did not match any room id/name. Something asked \
                 for a specific room and the boot would otherwise have silently used the \
                 authored start room instead — either `--start-room` on the command line or \
                 `StartRoomMustResolve`.\n\
                 ⚠ these are RUNTIME ROOM IDS, not LDtk level identifiers: `central_hub_main` \
                 is a level, the room is `central_hub_complex`.\n\
                 Available room ids ({}): {}",
                available.len(),
                available.join(", ")
            );
        } else {
            eprintln!(
                "[ambition] warning: start-room '{start_room}' did not match any room id/name"
            );
        }
    }
    let ldtk_index = ldtk_world::LdtkRuntimeIndex::from_project(
        &ldtk_project,
        room_set.active_spec().id.clone(),
    );
    // `StartingCharacterOverride` is composition input, not live gameplay
    // authority. Consume it before publishing the session-root
    // `StartingCharacter` component so the title route cannot observe a
    // process-resident character selection.
    let starting_character = app
        .world_mut()
        .remove_resource::<StartingCharacterOverride>()
        .map(|selection| selection.0)
        .unwrap_or_default();
    // Consumed the same way and for the same reason: a composition decision,
    // made before the world exists, that must not survive as process state.
    let builds_a_home_body = app
        .world_mut()
        .remove_resource::<SeatsAMatchInsteadOfAHomeBody>()
        .is_none();

    // Immutable boot preparation. Every shell activation clones this value and
    // inserts the resulting bundle on its exact session root.
    app.insert_resource(ambition_content::provider::AmbitionPreparedWorld {
        room_set: room_set.clone(),
        ldtk_index: ldtk_index.clone(),
        starting_character: starting_character.clone(),
        builds_a_home_body,
    });

    // the watcher does not resolve its own path any more. The catalog, the manifest and this
    // binary's `dev_hot_reload` feature all live HERE, so the resolution does too.
    //
    // and the feature check is only truthful here.
    let hot_reload = match sandbox_catalog.hot_reload_local_path(&world_manifest.primary().id) {
        Some(path) => {
            let mut state = ambition_platformer2d::dev_tools::WorldSourceHotReload::watching(path);
            if state.last_modified.is_some() {
                state.last_status = if cfg!(feature = "dev_hot_reload") {
                    "LDtk hot reload watching; use Apply Reload or toggle Auto-Apply from the developer controls"
                        .to_string()
                } else {
                    "LDtk hot reload polling; run with --features dev_hot_reload for Bevy file watching too"
                        .to_string()
                };
            }
            state
        }
        None => ambition_platformer2d::dev_tools::WorldSourceHotReload::unavailable(format!(
            "LDtk hot reload inactive: profile {} does not support filesystem watching",
            sandbox_catalog.profile().label(),
        )),
    };

    app.insert_resource(ldtk_world::ActiveLdtkProject(ldtk_project.clone()))
        .insert_resource(hot_reload)
        .insert_resource(ldtk_world::LdtkRuntimeSpineStats::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSolidIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeOneWayIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeDamageIndex::default())
        .insert_resource(ldtk_world::LdtkRuntimeSpineParity::default())
        // PhysicsSandboxSettings is read by setup_simulation_system; on the
        // visible binary AmbitionPhysicsPlugin re-inserts the default value
        // (harmless — same default), but headless does not load that plugin
        // (it depends on ScenePlugin), so the resource must be available
        // up front.
        .insert_resource(physics::PhysicsSandboxSettings::default())
        .insert_resource(LdtkSettings {
            // Ambition still renders runtime rooms for now; let bevy_ecs_ldtk
            // own level/entity lifecycle without also drawing LDtk background
            // rectangles behind every level.
            level_background: LevelBackground::Nonexistent,
            // bevy_ecs_ldtk's default `IntGridRendering::Colorful` spawns a
            // colored tile sprite per non-zero IntGrid cell when no tileset
            // is configured (1004 sprites for central_hub_main alone). Those
            // tiles render in raw LDtk world-pixel coordinates from
            // `LdtkWorldBundle`'s default transform, while our compose path
            // (`int_grid_value_to_block` → `spawn_block`) renders in
            // Ambition's centered Bevy frame via `world_to_bevy`. The two
            // frames disagree by ~half-room-width on x, so the plugin's
            // IntGrid output appeared as a duplicated, horizontally-shifted
            // copy of our render. Force the plugin to emit no visual at all
            // for IntGrid cells; the runtime-spine `LdtkSolid` component
            // (our typed authority) is unaffected by this setting.
            int_grid_rendering: IntGridRendering::Invisible,
            ..default()
        })
        .insert_resource(sandbox_data)
        .insert_resource(music_registry)
        .insert_resource(sfx_registry)
        .insert_resource(sandbox_catalog)
        .insert_resource(DeveloperTools::default())
        .insert_resource(EditablePlayerStats::default())
        .insert_resource(Platformer2dFeelTuningMonolith::default())
        // The OwnedItems catalog is simulation state, not only presentation UI
        // state. Headless Platformer2dSimHarness runs quest reward systems (which grant into
        // OwnedItems) without loading `add_presentation_plugins`, so the resource
        // must exist before the first Update tick.
        .insert_resource(ambition_platformer2d::items::OwnedItems::starter())
        .insert_resource(editable_abilities)
        // The neutral authority the SIMULATION reads, seeded from authored
        // content. `editable_tuning` beside it is the inspector's reflected
        // mirror; in a developer build `apply_editable_movement_tuning` pushes
        // its edits into this one. Nothing in the sim reads the mirror.
        .insert_resource(active_tuning)
        .insert_resource(editable_tuning)
        // Sim/presentation seam for input (ADR 0012): the sim reads
        // `Res<ControlFrame>`. Visible builds populate it from leafwing in
        // `populate_control_frame_from_actions`; headless tests can write
        // directly. Default = no buttons pressed = idle player.
        .init_resource::<ControlFrame>()
        // Aggregate user settings (video/audio/controls/gameplay).
        // Mutated by the pause menu; read by audio/video/gameplay
        // systems and the input deadzone/hysteresis filter.
        .insert_resource(ambition_platformer2d::persistence::settings::UserSettings::default());
    #[cfg(feature = "audio")]
    if let Some(path) = sfx_bank_asset_path {
        app.insert_resource(path);
    }
}

// what replaced it is `shell_host::compose_ambition_gameplay_host`: the shell host booted straight
// to the gameplay route, which is what direct entry was always supposed to be.
//
// `SessionScopeId(0)` went with it. It was a placeholder minted because a
// build-time root has no activation to mint one from, and `session_world_entity`
// panicked with "more than one canonical SessionRoot exists" whenever a
// composition managed to have both.
