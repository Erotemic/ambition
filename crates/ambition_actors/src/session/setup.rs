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
use crate::ldtk_world::LdtkRuntimeIndex;
use crate::platformer_runtime::lifecycle::PlayerVisual;
use crate::rooms::RoomSet;
use crate::session::data::SandboxDataAsset;
use ambition_dev_tools::dev_tools::{EditableAbilitySet, EditableMovementTuning};
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_PLAYER};
use ambition_engine_core::RoomGeometry;
use ambition_platformer_primitives::lifecycle::{SessionSpawnScope, SpawnSessionScopedExt};

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
    pub starting_character: &'a crate::avatar::StartingCharacter,
    /// App-local assembled character definitions used by spawn and re-wear.
    pub character_catalog: &'a ambition_characters::actor::character_catalog::CharacterCatalog,
    /// App-local hostile archetype definitions used by authored room lowering.
    pub character_roster: &'a crate::features::CharacterRoster,
    /// The installed App-local placement-lowering authority. Setup lowers the
    /// start room's authored placements through THIS registry — the same one
    /// room transition and snapshot restore consume — so there is no
    /// setup-only reconstruction of the six built-in interpreters.
    pub placement_lowering: &'a crate::world::placements::PlacementLoweringRegistry,
    /// The App-installed room-content staging seam. Setup drains the start
    /// room's registered content stagers exactly as transition, reset,
    /// hot-reload, and restore staging do — one construction authority.
    pub content_staging: &'a crate::features::RoomContentStagingRegistry,
    /// App-local boss profiles, encounter specs, sheets, and special rows.
    pub boss_catalog: &'a crate::boss_encounter::BossCatalog,
    /// Provider-selected default used only when `StartingCharacter` is empty.
    pub default_character_id: &'a str,
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
pub fn simulation_world(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    params: SimulationSetup<'_>,
) -> Entity {
    let SimulationSetup {
        world,
        room_set,
        ldtk_index,
        editable_abilities,
        editable_tuning,
        starting_character,
        character_catalog,
        character_roster,
        placement_lowering,
        content_staging,
        boss_catalog,
        default_character_id,
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

    let room_plan = crate::rooms::RoomConstructionPlan::prepare_from_parts(
        room_set,
        room_set.active,
        placement_lowering,
        content_staging,
        character_catalog,
        character_roster,
        boss_catalog,
        session_scope,
    )
    .unwrap_or_else(|error| panic!("initial room construction failed: {error}"));
    room_plan.spawn_contents(commands);
    commands.insert_resource(ambition_world::collision::MovingPlatformSet(
        room_plan.platform_states().to_vec(),
    ));

    // Capability set travels WITH the worn character when the row authors one
    // (the per-character analogue of the motion model below): a restricted-kit
    // demo character — classic run + jump — declares it in the catalog instead of
    // forcing the whole multi-game host onto the shared `EditableAbilitySet`. A
    // row without an authored set keeps that shared sandbox set, so Ambition's own
    // protagonist is untouched.
    let base_abilities = character_catalog
        .ability_set(starting_character.effective_id(default_character_id))
        .unwrap_or_else(|| editable_abilities.as_engine());
    let mut initial_scratch = crate::avatar::primary_player_scratch(world.0.spawn, base_abilities);
    ae::refresh_movement_resources_clusters(
        &initial_scratch.abilities,
        &mut initial_scratch.dash,
        &mut initial_scratch.jump,
        editable_tuning.as_engine().air_jumps,
    );

    // The player is a control box that WEARS a character. The protagonist takes
    // the untouched canonical path; any other selected character overlays its
    // moveset + name onto the same box (its sprite is bound presentation-side).
    let player_health = ambition_characters::actor::Health::new(20);
    let player_bundle = if starting_character.is_default() {
        crate::avatar::PlayerSimulationBundle::from_scratch(initial_scratch, player_health)
    } else {
        crate::avatar::PlayerSimulationBundle::from_scratch_as_character(
            character_catalog,
            initial_scratch,
            player_health,
            &starting_character.character_id,
        )
    };
    // Session ownership is captured by the caller when world construction
    // is requested. Deferred command application cannot reassign this body to a
    // later activation. Historical startup/RL callers pass `UNSCOPED`.
    let player = commands
        .spawn_session_scoped(
            session_scope,
            (
                Transform::from_translation(world_to_bevy(&world.0, world.0.spawn, WORLD_Z_PLAYER)),
                PlayerVisual,
                // The canonical playable-persona identity: WHICH catalog character
                // this control box wears. Simulation-owned, so gameplay config AND
                // presentation both derive from this ONE relationship instead of
                // rediscovering the selection from separate authorities. Resolved to
                // a concrete id (the content default when unset) so the identity is
                // never empty on the entity.
                ambition_characters::actor::WornCharacter::new(
                    starting_character.effective_id(default_character_id),
                ),
                player_bundle,
            ),
        )
        .id();

    // Movement identity travels WITH the worn character. Every body already
    // carries one explicit policy; the App-local catalog selects or refreshes
    // that policy without using component absence as an axis-swept sentinel.
    crate::avatar::apply_worn_motion_model(
        character_catalog,
        commands,
        player,
        starting_character.effective_id(default_character_id),
    );

    // The player entity is returned to the caller (the provider session builder
    // or the direct-entry startup system). Presentation discovers this home
    // avatar by its `PrimaryPlayer` marker — no process-global handle bag records
    // it — and spawns the HUD/quest text as session-scoped, marker-tagged
    // entities during its own setup.
    player
}
