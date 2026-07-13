use bevy::prelude::*;

#[cfg(feature = "input")]
use leafwing_input_manager::prelude::{ActionState, InputMap};

use ambition::actors::features;
use ambition::actors::ldtk_world;
use ambition::actors::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition::actors::rooms;
use ambition::actors::world::{physics, platforms};
use ambition::dev_tools::dev_tools::{DeveloperTools, EditableMovementTuning};
use ambition::dev_tools::SandboxDevState;
use ambition::engine_core as ae;
use ambition::engine_core::RoomGeometry;
#[cfg(feature = "input")]
use ambition::input::{KeyboardPreset, SandboxAction};
use ambition::render::rendering::{spawn_room_visuals, PlayerVisual, SceneEntities};

/// Presentation-side debug hotkey reader.
///
/// Slice 5 of the events refactor moved this out of the legacy
/// `sandbox_update` orchestrator so the gameplay loop no longer
/// reads `Res<ButtonInput<KeyCode>>`. That lets the player tick run
/// on the headless App-builder track.
///
/// Runs before the player tick so preset/debug-flag mutations land
/// before the gameplay loop reads them this frame.
pub(super) fn handle_debug_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut dev_state: ResMut<SandboxDevState>,
    mut tools: ResMut<DeveloperTools>,
) {
    if keys.just_pressed(KeyCode::F1) {
        dev_state.debug = !dev_state.debug;
    }
    if keys.just_pressed(KeyCode::F2) {
        dev_state.slowmo = !dev_state.slowmo;
    }
    if keys.just_pressed(KeyCode::F3) {
        tools.inspector_visible = !tools.inspector_visible;
    }
    if keys.just_pressed(KeyCode::F4) {
        tools.world_inspector_visible = !tools.world_inspector_visible;
    }
    if keys.just_pressed(KeyCode::F5) {
        tools.overview_camera = !tools.overview_camera;
    }
}

/// When the runtime keyboard preset changes, sync leafwing's `InputMap`
/// on the player entity so the next-frame inputs reflect the new preset.
/// Detected by polling `runtime.preset_index`. Gated behind `input`
/// because it owns leafwing components.
#[cfg(feature = "input")]
pub(super) fn sync_preset_input_map(
    dev_state: Res<SandboxDevState>,
    mut last_preset: Local<Option<usize>>,
    entities: Res<SceneEntities>,
    mut player_input: Query<
        (
            &mut ActionState<SandboxAction>,
            &mut InputMap<SandboxAction>,
        ),
        With<PlayerVisual>,
    >,
) {
    let current = dev_state.preset_index;
    if *last_preset == Some(current) {
        return;
    }
    if let Ok((mut action_state, mut input_map)) = player_input.get_mut(entities.player) {
        *input_map = KeyboardPreset::by_index(dev_state.preset_index).input_map();
        action_state.reset_all();
    }
    *last_preset = Some(current);
}

pub(super) fn handle_ldtk_hot_reload(
    mut commands: ambition::platformer::lifecycle::SessionCommands<'_, '_>,
    keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<RoomGeometry>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut dev_state: ResMut<SandboxDevState>,
    mut sim_state: ResMut<ambition::actors::SandboxSimState>,
    mut dialogue: ResMut<ambition::dialog::DialogState>,
    mut ldtk_index: ResMut<ldtk_world::LdtkRuntimeIndex>,
    mut ldtk_reload: ResMut<ldtk_world::LdtkHotReloadState>,
    editable_tuning: Res<EditableMovementTuning>,
    physics_settings: Res<physics::PhysicsSandboxSettings>,
    mut platform_set: ResMut<ambition::world::collision::MovingPlatformSet>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    visual_assets: (
        Option<Res<ambition::sprite_sheet::game_assets::GameAssets>>,
        Option<Res<ambition::render::quality::ResolvedVisualQuality>>,
    ),
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition::actors::features::MotionModel,
            &mut ambition::characters::actor::BodyCombat,
            &mut ambition::actors::avatar::PlayerSafetyState,
        ),
        // PRIMARY-only: LDtk hot-reload repositions the camera body to the
        // validated spawn — a single-player dev flow.
        ambition::actors::actor::PrimaryPlayerOnly,
    >,
    catalog: Res<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>,
) {
    if keys.just_pressed(KeyCode::F12) {
        ldtk_reload.auto_apply = !ldtk_reload.auto_apply;
        ldtk_reload.last_status = format!(
            "LDtk auto-apply {}",
            if ldtk_reload.auto_apply {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    let requested = keys.just_pressed(KeyCode::F11);
    let should_apply = requested || (ldtk_reload.pending && ldtk_reload.auto_apply);
    if !should_apply {
        return;
    }

    // Hot reload reads the same `watch_path` the file-change poller
    // armed at startup (per the catalog's
    // `SandboxAssetCatalog::hot_reload_local_path`). If the active
    // asset profile doesn't support filesystem watching the
    // `watch_path` is `None` and the reload is silently skipped.
    let Some(watch_path) = ldtk_reload.watch_path.clone() else {
        eprintln!(
            "LDtk hot reload pressed but watch_path is unset; the active asset profile \
             does not support filesystem watching"
        );
        ldtk_reload.pending = false;
        return;
    };
    if let Ok((mut cluster_item, mut motion_model, mut combat, mut safety)) = player_q.single_mut()
    {
        let Some(session_scope) = commands.spawn_scope() else {
            return;
        };
        let mut clusters = cluster_item.as_clusters_mut();
        let result = reload_ldtk_world_from_disk(
            &mut commands,
            &mut world,
            &mut room_set,
            &mut motion_model,
            &mut clusters,
            &mut dev_state,
            &mut sim_state,
            &mut safety,
            &mut dialogue,
            &mut combat,
            &mut ldtk_index,
            editable_tuning.as_engine(),
            *physics_settings,
            &mut platform_set.0,
            &room_visuals,
            visual_assets.0.as_deref(),
            visual_assets.1.as_deref(),
            &watch_path,
            &catalog,
            session_scope,
        );
        match result {
            Ok(active_room) => {
                ldtk_reload.mark_applied(&active_room);
                eprintln!("LDtk hot reload applied to active room '{active_room}'");
            }
            Err(errors) => {
                for error in &errors {
                    eprintln!("LDtk hot reload rejected: {error}");
                }
                ldtk_reload.mark_failed(errors);
            }
        }
    }
    // When no player entity exists, hot-reload is silently skipped.
    // The game always has a player entity during normal play; this
    // branch only fires in unusual teardown states.
}

pub(super) struct LdtkReloadTransaction {
    project: ldtk_world::LdtkProject,
    next_room_set: rooms::RoomSet,
    next_spec: rooms::RoomSpec,
    safe_player_pos: ae::Vec2,
}

pub(super) fn prepare_ldtk_reload_transaction(
    watch_path: &std::path::Path,
    catalog: &ambition::asset_manager::sandbox_assets::SandboxAssetCatalog,
    current_room_id: &str,
    preserved_pos: ae::Vec2,
    player_size: ae::Vec2,
) -> Result<LdtkReloadTransaction, Vec<String>> {
    let project = ldtk_world::LdtkProject::load_from_disk_at(watch_path, catalog)
        .map_err(|error| vec![error])?;
    let report = project.validate();
    report.print_to_stderr();
    if !report.is_ok() {
        return Err(report.errors);
    }

    let mut next_room_set = project.to_room_set()?;
    let Some(next_active) = next_room_set
        .rooms
        .iter()
        .position(|room| room.id == current_room_id)
    else {
        return Err(vec![format!(
            "LDtk reload would delete current active area '{current_room_id}'. Move the player elsewhere or restore that activeArea before applying."
        )]);
    };
    next_room_set.active = next_active;
    let next_spec = next_room_set.active_spec().clone();

    let mut hard_errors = Vec::new();
    for warning in next_room_set.layout_warnings() {
        if warning.contains("references missing") {
            hard_errors.push(format!("LDtk reload graph error: {warning}"));
        } else {
            bevy::log::debug!(target: "ambition::room_layout", "LDtk reload: {warning}");
        }
    }
    if !hard_errors.is_empty() {
        return Err(hard_errors);
    }

    let safe_player_pos = rooms::validated_spawn(&next_spec.world, preserved_pos, player_size);
    Ok(LdtkReloadTransaction {
        project,
        next_room_set,
        next_spec,
        safe_player_pos,
    })
}

pub(super) fn reload_ldtk_world_from_disk(
    commands: &mut Commands,
    world: &mut RoomGeometry,
    room_set: &mut rooms::RoomSet,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut SandboxDevState,
    sim_state: &mut ambition::actors::SandboxSimState,
    safety: &mut ambition::actors::avatar::PlayerSafetyState,
    dialogue: &mut ambition::dialog::DialogState,
    combat: &mut ambition::characters::actor::BodyCombat,
    ldtk_index: &mut ldtk_world::LdtkRuntimeIndex,
    tuning: ae::MovementTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    moving_platforms: &mut Vec<ambition::actors::world::platforms::MovingPlatformState>,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomScopedEntity>>,
    assets: Option<&ambition::sprite_sheet::game_assets::GameAssets>,
    quality: Option<&ambition::render::quality::ResolvedVisualQuality>,
    watch_path: &std::path::Path,
    catalog: &ambition::asset_manager::sandbox_assets::SandboxAssetCatalog,
    session_scope: ambition::platformer::lifecycle::SessionSpawnScope,
) -> Result<String, Vec<String>> {
    let current_room_id = room_set.active_spec().id.clone();
    let preserved_pos = clusters.kinematics.pos;
    let transaction = prepare_ldtk_reload_transaction(
        watch_path,
        catalog,
        &current_room_id,
        preserved_pos,
        clusters.kinematics.size,
    )?;

    // Everything above this line is non-mutating: invalid edits, deleted active
    // areas, bad graph links, and unsafe player positions are rejected before
    // touching the live world. Only commit after the complete replacement room
    // graph and repaired player position have been built.
    for (entity, physics_entity) in room_visuals.iter() {
        if physics_entity.is_some() {
            physics::retire_physics_entity(commands, entity);
        } else {
            commands.entity(entity).despawn();
        }
    }

    let active_room = transaction.next_spec.id.clone();
    *room_set = transaction.next_room_set;
    world.0 = transaction.next_spec.world.clone();

    // The repaired placement is a discrete TRANSIT (ADR 0024 authority):
    // momentum kept for a same-spot reload, contacts/attachment reconciled
    // against the replaced geometry.
    ae::movement::transit_body(
        motion_model,
        clusters,
        transaction.safe_player_pos,
        ae::movement::TransitVelocity::Keep,
    );
    ae::refresh_movement_resources_clusters(
        clusters.abilities,
        &mut *clusters.dash,
        &mut *clusters.jump,
        tuning.air_jumps,
    );
    safety.last_safe_pos = transaction.safe_player_pos;
    *moving_platforms = platforms::moving_platforms_for_room(&transaction.next_spec);
    features::spawn_room_feature_entities(commands, &transaction.next_spec, session_scope);
    dialogue.close();
    combat.hitstop_timer = 0.0;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    sim_state.room_transition_cooldown = 0.10;
    dev_state.preset_flash = 1.0;

    ldtk_index.replace_from_project(&transaction.project, active_room.clone());

    ambition::render::rendering::spawn_parallax_layers(
        commands,
        session_scope,
        &world.0,
        &room_set.active_spec().metadata,
        assets,
        quality.map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        commands,
        session_scope,
        room_set.active_spec(),
        physics_settings,
        assets,
    );
    platforms::spawn_moving_platforms(commands, session_scope, &world.0, moving_platforms);

    Ok(active_room)
}
