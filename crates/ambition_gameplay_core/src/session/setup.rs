//! Sim/presentation split for the sandbox's startup setup.
//!
//! Slice 4 of ADR 0012's events refactor: an earlier monolithic setup
//! system mixed simulation-only world construction
//! (`LdtkWorldBundle`, the player entity's gameplay components) with
//! presentation-only spawns (Camera2d, sprites, HUD text, and generated
//! audio library setup). This module factors the sim half into
//! [`simulation_world`] so the headless binary can build the world without
//! presentation, while the visible-app setup keeps that seam clean.
//!
//! [`simulation_world`] takes `&mut Commands` plus borrowed resource handles
//! ([`SimulationSetup`]) so it can be invoked from any Bevy startup system
//! that has gathered the right parameters. It is not a Bevy system itself;
//! the `ambition_app` crate's startup setup (`app/setup_systems.rs`) does the
//! param wiring and pairs it with the presentation-side spawns.

use ambition_engine_core as ae;
use bevy::prelude::*;

use crate::assets::loading::SandboxAssetCollection;
use crate::dev::dev_tools::{EditableAbilitySet, EditableMovementTuning};
use crate::ldtk_world::LdtkRuntimeIndex;
use crate::platformer_runtime::lifecycle::{PlayerVisual, SceneEntities};
use crate::rooms::RoomSet;
use crate::session::data::SandboxDataAsset;
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_engine_core::RoomGeometry;

/// Borrowed inputs for `simulation_world`.
///
/// Grouped as a struct because Bevy's max-system-param budget is tight and
/// keeping these as positional args would push the calling startup system
/// past 16 params again. The struct also documents what the simulation
/// half of setup actually needs.
pub struct SimulationSetup<'a> {
    pub world: &'a RoomGeometry,
    pub room_set: &'a RoomSet,
    pub ldtk_index: &'a LdtkRuntimeIndex,
    pub editable_abilities: &'a EditableAbilitySet,
    pub editable_tuning: &'a EditableMovementTuning,
    /// Which catalog character the local player spawns as. `is_default()` (the
    /// `player` protagonist) takes the untouched `from_scratch` path.
    pub starting_character: &'a crate::player::StartingCharacter,
    pub sandbox_data_asset: Option<&'a SandboxDataAsset>,
    pub sandbox_asset_collection: Option<&'a SandboxAssetCollection>,
    pub asset_server: &'a AssetServer,
}

/// Spawn simulation-only entities and resources.
///
/// Returns the player entity so `presentation_world` (or any future RL
/// adapter) can attach presentation components without re-querying.
///
/// This includes:
/// * pre-fetching sandbox/LDtk asset handles to keep the asset server alive
/// * logging room layout warnings
/// * spawning the `LdtkWorldBundle` so `bevy_ecs_ldtk` can own LDtk entity
///   lifecycle and the runtime-spine systems have something to query
/// * spawning the player entity with gameplay-essential ECS components
///   (`PlayerSimulationBundle` for sim clusters plus `Transform`,
///   `PlayerVisual`, etc.).
///   Leafwing's `ActionState` and `InputMap` get attached by the
///   presentation-side `attach_player_input_components` startup system;
///   sim-only builds stay leafwing-free per the ADR 0012 input seam.
/// * inserting a `SceneEntities` resource with `hud: Entity::PLACEHOLDER`
///   that `presentation_world` overwrites once the HUD entity exists
pub fn simulation_world(commands: &mut Commands, params: SimulationSetup<'_>) -> Entity {
    let SimulationSetup {
        world,
        room_set,
        ldtk_index,
        editable_abilities,
        editable_tuning,
        starting_character,
        sandbox_data_asset,
        sandbox_asset_collection,
        asset_server,
    } = params;

    if let Some(handle) = sandbox_data_asset {
        let _asset_handle_for_async_reload = handle.0.clone();
    }
    if let Some(collection) = sandbox_asset_collection {
        let _loaded_sandbox_data_handle = collection.sandbox_data.clone();
        let _loaded_ldtk_project_handle = collection.ldtk_project.clone();
    }
    for warning in room_set.layout_warnings() {
        bevy::log::debug!(target: "ambition::room_layout", "{warning}");
    }
    // The LdtkWorldBundle spawn lives in the Ldtk-runtime startup system
    // (`crate::schedule::add_ldtk_runtime_plugin`) because asset_server.load on a
    // typed `LdtkProject` handle requires `LdtkPlugin` to be registered.
    // Headless builds skip LdtkPlugin (its tile pipeline needs RenderApp),
    // so this function must not assume the LDtk asset type is available.
    // Suppress the unused-binding warnings until follow-up patches retire
    // the `ldtk_index` / `asset_server` params or move them.
    let _ = asset_server;
    let _ = ldtk_index;

    crate::features::spawn_room_feature_entities(commands, room_set.active_spec());

    let mut initial_scratch =
        crate::player::primary_player_scratch(world.0.spawn, editable_abilities.as_engine());
    ae::refresh_movement_resources_clusters(
        &initial_scratch.abilities,
        &mut initial_scratch.dash,
        &mut initial_scratch.jump,
        editable_tuning.as_engine(),
    );

    // The player is a control box that WEARS a character. The protagonist takes
    // the untouched canonical path; any other selected character overlays its
    // moveset + name onto the same box (its sprite is bound presentation-side).
    let player_health = ambition_characters::actor::Health::new(20);
    let player_bundle = if starting_character.is_default() {
        crate::player::PlayerSimulationBundle::from_scratch(initial_scratch, player_health)
    } else {
        crate::player::PlayerSimulationBundle::from_scratch_as_character(
            initial_scratch,
            player_health,
            &starting_character.character_id,
        )
    };
    let player = commands
        .spawn((
            Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
            PlayerVisual,
            player_bundle,
        ))
        .id();

    // Movement identity travels WITH the worn character (Q16 / S2): a character
    // authoring surface-momentum params (Sanic) makes the home box ride chains;
    // any other character removes the model so the box stays axis-swept. The
    // default `player` row authors no momentum, so this is a no-op for the
    // protagonist.
    crate::player::apply_worn_motion_model(commands, player, &starting_character.character_id);

    // HUD entity is presentation-side; placeholder until presentation_world
    // overwrites this resource.
    commands.insert_resource(SceneEntities {
        player,
        hud: Entity::PLACEHOLDER,
        quest_panel: Entity::PLACEHOLDER,
    });

    player
}
