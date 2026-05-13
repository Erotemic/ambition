#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::plugins::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Presentation-side debug hotkey reader.
///
/// Slice 5 of the events refactor moved this out of `sandbox_update` so the
/// gameplay loop no longer reads `Res<ButtonInput<KeyCode>>`. That lets
/// `sandbox_update` run on the headless App-builder track.
///
/// Runs before `sandbox_update` so preset/debug-flag mutations land before
/// the gameplay loop reads them this frame.
pub(super) fn handle_debug_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut runtime: ResMut<SandboxRuntime>,
    mut tools: ResMut<DeveloperTools>,
) {
    if keys.just_pressed(KeyCode::F1) {
        runtime.debug = !runtime.debug;
    }
    if keys.just_pressed(KeyCode::F9) {
        runtime.preset_index =
            (runtime.preset_index + runtime.presets.len() - 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
    }
    if keys.just_pressed(KeyCode::F10) {
        runtime.preset_index = (runtime.preset_index + 1) % runtime.presets.len();
        runtime.preset_flash = 1.2;
    }
    if keys.just_pressed(KeyCode::F2) {
        runtime.slowmo = !runtime.slowmo;
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

/// When the player cycles input presets via F9/F10, sync leafwing's
/// `InputMap` on the player entity so the next-frame inputs reflect the
/// new preset. Detected by polling `runtime.preset_index`. Gated behind
/// `input` because it owns leafwing components.
#[cfg(feature = "input")]
pub(super) fn sync_preset_input_map(
    runtime: Res<SandboxRuntime>,
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
    let current = runtime.preset_index;
    if *last_preset == Some(current) {
        return;
    }
    if let Ok((mut action_state, mut input_map)) = player_input.get_mut(entities.player) {
        *input_map = runtime.preset().input_map();
        action_state.reset_all();
    }
    *last_preset = Some(current);
}

pub(super) fn handle_ldtk_hot_reload(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<GameWorld>,
    mut room_set: ResMut<rooms::RoomSet>,
    mut runtime: ResMut<SandboxRuntime>,
    mut ldtk_index: ResMut<ldtk_world::LdtkRuntimeIndex>,
    mut ldtk_reload: ResMut<ldtk_world::LdtkHotReloadState>,
    editable_tuning: Res<EditableMovementTuning>,
    room_visuals: Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    game_assets: Option<Res<crate::game_assets::GameAssets>>,
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

    match reload_ldtk_world_from_disk(
        &mut commands,
        &mut world,
        &mut room_set,
        &mut runtime,
        &mut ldtk_index,
        editable_tuning.as_engine(),
        &room_visuals,
        game_assets.as_deref(),
    ) {
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

pub(super) struct LdtkReloadTransaction {
    project: ldtk_world::LdtkProject,
    next_room_set: rooms::RoomSet,
    next_spec: rooms::RoomSpec,
    safe_player_pos: ae::Vec2,
}

pub(super) fn prepare_ldtk_reload_transaction(
    current_room_id: &str,
    preserved_pos: ae::Vec2,
    player_size: ae::Vec2,
) -> Result<LdtkReloadTransaction, Vec<String>> {
    let project = ldtk_world::LdtkProject::load_from_disk().map_err(|error| vec![error])?;
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
    world: &mut GameWorld,
    room_set: &mut rooms::RoomSet,
    runtime: &mut SandboxRuntime,
    ldtk_index: &mut ldtk_world::LdtkRuntimeIndex,
    tuning: ae::MovementTuning,
    room_visuals: &Query<(Entity, Option<&physics::PhysicsRoomEntity>), With<RoomVisual>>,
    assets: Option<&crate::game_assets::GameAssets>,
) -> Result<String, Vec<String>> {
    let current_room_id = room_set.active_spec().id.clone();
    let preserved_pos = runtime.player.pos;
    let transaction =
        prepare_ldtk_reload_transaction(&current_room_id, preserved_pos, runtime.player.size)?;

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

    runtime.player.pos = transaction.safe_player_pos;
    runtime.player.refresh_movement_resources(tuning);
    runtime.last_safe_player_pos = transaction.safe_player_pos;
    runtime.moving_platforms = platforms::moving_platforms_for_room(&transaction.next_spec);
    runtime.features = features::FeatureRuntime::from_world(&world.0);
    runtime.dialogue.close();
    runtime.hitstop_timer = 0.0;
    runtime.hitstun_timer = 0.0;
    runtime.room_transition_cooldown = 0.10;
    runtime.preset_flash = 1.0;

    ldtk_index.replace_from_project(&transaction.project, active_room.clone());

    crate::rendering::spawn_parallax_layers(
        commands,
        &world.0,
        &room_set.active_spec().metadata,
        assets,
    );
    spawn_room_visuals(
        commands,
        &world.0,
        &room_set.active_spec().loading_zones,
        runtime.physics_settings,
        assets,
    );
    platforms::spawn_moving_platforms(commands, &world.0, &runtime.moving_platforms);

    Ok(active_room)
}
