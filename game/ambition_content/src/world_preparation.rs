//! Ambition's world, prepared from the asset catalog the host built: its LDtk
//! project, validated against the pack, as the room set a session enters.
//!
//! The provider runs this before it installs anything, so a host that selects
//! Ambition names no part of Ambition's world. The host builds the asset
//! catalog before any plugin (every plugin resolves paths through it), and
//! that is all of the world it builds.

use bevy::prelude::*;

use ambition_platformer2d::ldtk_map as ldtk_world;

/// Programmatic start-room override. Platformer2dSimHarness and other library
/// callers insert this resource before Ambition's provider builds; the
/// provider consumes it (taking precedence over the `--start-room` CLI flag)
/// so callers do not need to manipulate `std::env::args` to pin a starting
/// room.
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
pub struct StartingCharacterOverride(pub ambition_platformer2d_session::initial_body::StartingCharacter);

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

/// The LDtk nouns Ambition's world is authored in: the engine's alone.
pub fn ldtk_vocabulary() -> ldtk_world::LdtkVocabulary {
    ldtk_world::LdtkVocabulary::engine()
}

/// The world did not load or did not validate; the diagnostics are above.
fn preparation_failed() -> ! {
    panic!("Ambition's world failed preparation; see diagnostics above");
}

/// Prepare Ambition's world from the asset catalog the host built, and
/// install it: the prepared world a session enters, the active LDtk project
/// and its hot-reload watcher. Its own start-room and seating requests
/// ([`StartRoomOverride`], [`StartingCharacterOverride`],
/// [`SeatsAMatchInsteadOfAHomeBody`]) are consumed here.
///
/// An App with no asset catalog (a provider declaration test) has no world to
/// load and gets none.
pub(crate) fn prepare_ambition_world(app: &mut App) {
    let Some(catalog) = app
        .world()
        .get_resource::<ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog>()
        .cloned()
    else {
        return;
    };
    let world_manifest = crate::worlds::world_manifest();
    let music_registry = app
        .world()
        .resource::<ambition_platformer2d::audio::spec::MusicRegistry>()
        .clone();
    let character_catalog = app
        .world()
        .resource::<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>()
        .clone();
    #[cfg(feature = "audio")]
    if let Some(path) = catalog
        .path_for(&ambition_platformer2d::asset_manager::platformer_assets::ids::sfx_bank())
    {
        app.insert_resource(ambition_platformer2d::audio::SfxBankAssetPath::new(
            crate::AMBITION_CONTENT_PROVIDER,
            path,
        ));
    }

    let ldtk_project =
        match ldtk_world::LdtkProject::load_default(&catalog, &world_manifest) {
            Ok(project) => project,
            Err(error) => {
                eprintln!("failed to load sandbox LDtk map: {error}");
                preparation_failed();
            }
        };
    // The App's own pack: validation judges the content this App installed.
    let selected_pack = crate::pack::select(app.world_mut());
    let content_report = crate::content_validation::validate_content_graph(
        &selected_pack,
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
        preparation_failed();
    }
    let mut room_set =
        match ldtk_project.to_room_set(&world_manifest, &ldtk_vocabulary()) {
            Ok(room_set) => room_set,
            Err(errors) => {
                eprintln!(
                    "sandbox LDtk world failed validation; fix the configured map before running:"
                );
                for error in &errors {
                    eprintln!("  - {error}");
                }
                preparation_failed();
            }
        };
    // The host states a typed `--start-room` as these two resources, and a
    // programmatic override (Platformer2dSimHarness / library callers) takes
    // precedence over it. If the room does not resolve, the LDtk project's
    // authored start room stays active, unless the request must resolve.
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
    if let Some(start_room) = resource_override {
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
    let ldtk_index = ldtk_world::LdtkRuntimeIndex::from_project(&ldtk_project);
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
    app.insert_resource(crate::provider::AmbitionPreparedWorld {
        room_set,
        ldtk_index,
        starting_character,
        builds_a_home_body,
    });

    // The watcher does not resolve its own path: the catalog and the manifest
    // are here. The feature is the app's `dev_hot_reload`, which it forwards to
    // this crate's.
    let hot_reload = match catalog.hot_reload_local_path(&world_manifest.primary().id) {
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
            catalog.profile().label(),
        )),
    };

    app.insert_resource(ldtk_world::ActiveLdtkProject(ldtk_project))
        // Over the default `DevToolsSimPlugin` initialized.
        .insert_resource(hot_reload);
}
