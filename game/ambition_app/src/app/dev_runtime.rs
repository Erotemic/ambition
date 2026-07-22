use bevy::prelude::*;

#[cfg(feature = "input")]
use leafwing_input_manager::prelude::{ActionState, InputMap};

use ambition::actors::ldtk_world;
use ambition::actors::platformer_runtime::lifecycle::RoomScopedEntity;
use ambition::actors::rooms;
use ambition::actors::world::physics;
use ambition::dev_tools::dev_tools::DeveloperTools;
use ambition::dev_tools::SandboxDevState;
use ambition::engine_core as ae;
use ambition::engine_core::RoomGeometry;
#[cfg(feature = "input")]
use ambition::input::{KeyboardPreset, SandboxAction};
use ambition::platformer::developer_hotkeys::DeveloperAction;
use ambition::render::rendering::spawn_room_visuals;

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
    mut actions: MessageReader<DeveloperAction>,
    mut dev_state: ResMut<SandboxDevState>,
    mut tools: ResMut<DeveloperTools>,
) {
    for action in actions.read() {
        match action {
            DeveloperAction::ToggleDebugOverlay => dev_state.debug = !dev_state.debug,
            DeveloperAction::ToggleSlowMotion => dev_state.slowmo = !dev_state.slowmo,
            DeveloperAction::ToggleInspector => {
                tools.inspector_visible = !tools.inspector_visible;
            }
            DeveloperAction::ToggleWorldInspector => {
                tools.world_inspector_visible = !tools.world_inspector_visible;
            }
            DeveloperAction::ToggleOverviewCamera => {
                tools.overview_camera = !tools.overview_camera;
            }
            _ => {}
        }
    }
}

/// When the persisted keyboard preset changes (the settings menu writes
/// `UserSettings.controls.keyboard_preset_index` — the ONE preset authority),
/// sync leafwing's `InputMap` on the persistent input participant so the
/// next-frame inputs reflect the new preset. Gated behind `input` because it
/// owns leafwing components.
#[cfg(feature = "input")]
pub(super) fn sync_preset_input_map(
    settings: Res<ambition::persistence::settings::UserSettings>,
    mut last_preset: Local<Option<usize>>,
    mut player_input: Query<
        (
            &mut ActionState<SandboxAction>,
            &mut InputMap<SandboxAction>,
        ),
        With<ambition::input::InputParticipant>,
    >,
) {
    let current = settings.controls.keyboard_preset_index;
    if *last_preset == Some(current) {
        return;
    }
    if let Ok((mut action_state, mut input_map)) = player_input.single_mut() {
        *input_map = KeyboardPreset::by_index(current).input_map();
        action_state.reset_all();
    }
    *last_preset = Some(current);
}

fn local_ggrs_restart_policy(
    ownership: Option<ambition::runtime::rollback::RollbackSessionOwnership>,
) -> Result<Option<ambition::runtime::rollback::SyncTestSettings>, &'static str> {
    match ownership {
        Some(ambition::runtime::rollback::RollbackSessionOwnership::External) => Err(
            "LDtk hot reload cannot replace an external/P2P GGRS session; peers need a coordinated content barrier",
        ),
        Some(ambition::runtime::rollback::RollbackSessionOwnership::LocalSyncTest(settings)) => {
            Ok(Some(ambition::runtime::rollback::SyncTestSettings {
                check_distance: 0,
                max_prediction_window: settings.max_prediction_window,
            }))
        }
        None => Ok(None),
    }
}

pub(super) fn handle_ldtk_hot_reload(
    mut commands: ambition::platformer::lifecycle::SessionCommands<'_, '_>,
    mut hotkey_actions: MessageReader<DeveloperAction>,
    mut world: ambition::platformer::lifecycle::SessionWorldMut<RoomGeometry>,
    mut room_set: ambition::platformer::lifecycle::SessionWorldMut<rooms::RoomSet>,
    mut dev_state: ResMut<SandboxDevState>,
    mut sim_state: ResMut<ambition::actors::SandboxSimState>,
    mut dialogue: ResMut<ambition::dialog::DialogState>,
    mut ldtk_index: ambition::platformer::lifecycle::SessionWorldMut<ldtk_world::LdtkRuntimeIndex>,
    mut ldtk_reload: ResMut<ldtk_world::LdtkHotReloadState>,
    // Bundled to keep this system within Bevy's 16 top-level SystemParam limit.
    tuning: (
        Res<ambition::engine_core::ActiveMovementTuning>,
        Res<physics::PhysicsSandboxSettings>,
    ),
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
    catalogs: (
        Res<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>,
        Res<ambition::characters::actor::character_catalog::CharacterCatalog>,
        Res<ambition::actors::features::CharacterRoster>,
        Res<ambition::actors::boss_encounter::BossCatalog>,
        Res<ambition::actors::world::placements::PlacementLoweringRegistry>,
        Res<ambition::actors::features::RoomContentStagingRegistry>,
        Res<ambition::actors::construction::ActorConstructionRegistry>,
        Res<ldtk_world::WorldManifest>,
    ),
    mut content_identity: (
        ambition::platformer::lifecycle::SessionWorldMut<ambition::runtime::PreparedContent>,
        ambition::platformer::lifecycle::SessionWorldMut<
            ambition::runtime::PreparedContentIdentity,
        >,
        ResMut<ambition::runtime::ContentEpochSequence>,
        Option<Res<ambition::runtime::rollback::RollbackRegistry>>,
        Option<Res<ambition::runtime::rollback::RollbackSessionOwnership>>,
    ),
) {
    let mut requested = false;
    for action in hotkey_actions.read() {
        match action {
            DeveloperAction::ToggleLdtkAutoApply => {
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
            DeveloperAction::ApplyLdtkReload => requested = true,
            _ => {}
        }
    }

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

    let restart_local_ggrs = match local_ggrs_restart_policy(content_identity.4.as_deref().copied())
    {
        Ok(restart) => restart,
        Err(error) => {
            eprintln!("LDtk hot reload rejected: {error}");
            ldtk_reload.mark_failed(vec![error.to_owned()]);
            return;
        }
    };

    if let Some(settings) = restart_local_ggrs {
        ambition::runtime::rollback::stop_session_deferred(&mut commands);
        commands.insert_resource(RestartLocalGgrsAfterLdtkReload { settings });
    }
    if let Ok((mut cluster_item, mut motion_model, mut combat, mut safety)) = player_q.single_mut()
    {
        let Some(session_scope) = commands.spawn_scope() else {
            return;
        };
        let mut clusters = cluster_item.as_clusters_mut();
        let snapshot_schema = content_identity
            .3
            .as_deref()
            .cloned()
            .unwrap_or_default()
            .schema_fingerprint();
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
            tuning.0 .0,
            *tuning.1,
            &mut platform_set.0,
            &room_visuals,
            visual_assets.0.as_deref(),
            visual_assets.1.as_deref(),
            &watch_path,
            &catalogs.0,
            &catalogs.1,
            &catalogs.2,
            &catalogs.3,
            &catalogs.4,
            &catalogs.5,
            &catalogs.6,
            &catalogs.7,
            &mut content_identity.0,
            &mut content_identity.1,
            &mut content_identity.2,
            snapshot_schema,
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

#[derive(Resource, Clone, Copy, Debug)]
struct RestartLocalGgrsAfterLdtkReload {
    settings: ambition::runtime::rollback::SyncTestSettings,
}

/// Rebind the cheap local baseline after the Update-stage content transaction
/// and its deferred session removal have both committed.
pub(super) fn restart_local_ggrs_after_hot_reload(world: &mut World) {
    let Some(restart) = world.remove_resource::<RestartLocalGgrsAfterLdtkReload>() else {
        return;
    };

    #[cfg(feature = "dev_tools")]
    crate::dev::rollback_observatory::reset_for_content_reload(world);
    if ambition::runtime::rollback::session_is_active(world) {
        ambition::runtime::rollback::stop_session(world);
    }
    match ambition::runtime::rollback::start_sync_test_session(world, restart.settings) {
        Ok(()) => {
            #[cfg(feature = "dev_tools")]
            crate::dev::rollback_observatory::mark_baseline_restarted(world);
            info!("LDtk hot reload rebased the local GGRS baseline");
        }
        Err(error) => {
            #[cfg(feature = "dev_tools")]
            crate::dev::rollback_observatory::mark_baseline_restart_failed(
                world,
                &error.to_string(),
            );
            error!("failed to restart local GGRS after LDtk hot reload: {error}");
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
    watch_path: &std::path::Path,
    catalog: &ambition::asset_manager::sandbox_assets::SandboxAssetCatalog,
    manifest: &ldtk_world::WorldManifest,
    current_room_id: &str,
    preserved_pos: ae::Vec2,
    player_size: ae::Vec2,
) -> Result<LdtkReloadTransaction, Vec<String>> {
    let project = ldtk_world::LdtkProject::load_from_disk_at(watch_path, catalog, manifest)
        .map_err(|error| vec![error])?;
    let report = project.validate();
    report.print_to_stderr();
    if !report.is_ok() {
        return Err(report.errors);
    }

    let mut next_room_set = project.to_room_set(manifest)?;
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
    character_catalog: &ambition::characters::actor::character_catalog::CharacterCatalog,
    character_roster: &ambition::actors::features::CharacterRoster,
    boss_catalog: &ambition::actors::boss_encounter::BossCatalog,
    placement_lowering: &ambition::actors::world::placements::PlacementLoweringRegistry,
    content_staging: &ambition::actors::features::RoomContentStagingRegistry,
    construction_recipes: &ambition::actors::construction::ActorConstructionRegistry,
    world_manifest: &ldtk_world::WorldManifest,
    prepared_content: &mut ambition::runtime::PreparedContent,
    prepared_identity: &mut ambition::runtime::PreparedContentIdentity,
    epochs: &mut ambition::runtime::ContentEpochSequence,
    snapshot_schema: ambition::runtime::SnapshotSchemaFingerprint,
    session_scope: ambition::platformer::lifecycle::SessionSpawnScope,
) -> Result<String, Vec<String>> {
    let current_room_id = room_set.active_spec().id.clone();
    let preserved_pos = clusters.kinematics.pos;
    let transaction = prepare_ldtk_reload_transaction(
        watch_path,
        catalog,
        world_manifest,
        &current_room_id,
        preserved_pos,
        clusters.kinematics.size,
    )?;

    let mut candidate_index = ldtk_index.clone();
    candidate_index.replace_from_project(&transaction.project, transaction.next_spec.id.clone());
    let candidate_source = prepared_content.source().with_world(
        transaction.next_room_set.clone(),
        RoomGeometry(transaction.next_spec.world.clone()),
        rooms::ActiveRoomMetadata(transaction.next_spec.metadata.clone()),
        candidate_index.clone(),
    );
    let candidate_content = ambition::provider::prepare_world_replacement_candidate(
        prepared_content,
        candidate_source,
        snapshot_schema,
    )
    .map_err(|error| vec![error.to_string()])?;

    let construction_plan = rooms::RoomConstructionPlan::prepare_spec(
        transaction.next_room_set.active,
        transaction.next_spec.clone(),
        placement_lowering,
        content_staging,
        character_catalog,
        character_roster,
        boss_catalog,
        session_scope,
        ambition::actors::features::ActorConstructionContext::new(
            construction_recipes,
            // The generation currently live. A materially changed definition
            // allocates a new one below, AFTER every preflight has succeeded —
            // so a plan prepared here always states the epoch it was validated
            // against, never one that does not exist yet.
            prepared_content.epoch(),
        ),
    )
    .map_err(|error| vec![error.to_string()])?;

    // Everything above this line is non-mutating, including preparation of the
    // exact candidate content identity. Equivalent reloads preserve both the
    // fingerprint and epoch; materially changed definitions allocate a new
    // epoch only now, when every preflight has succeeded.
    let committed_content = if candidate_content.fingerprint() == prepared_content.fingerprint()
        && candidate_content.snapshot_schema() == prepared_content.snapshot_schema()
    {
        prepared_content.clone()
    } else {
        candidate_content.with_epoch(epochs.allocate())
    };

    // Commit exactly the prepared construction artifact rather than
    // rediscovering spawn decisions here.
    let outgoing = room_visuals
        .iter()
        .map(|(entity, physics_entity)| (entity, physics_entity.is_some()));
    construction_plan.retire_outgoing(commands, outgoing, None);

    let active_room = construction_plan.room_id().to_string();
    *room_set = transaction.next_room_set;
    construction_plan.commit_deferred(commands, room_set, world, moving_platforms);

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
    dialogue.close();
    combat.hitstop_timer = 0.0;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    sim_state.room_transition_cooldown = 0.10;
    dev_state.preset_flash = 1.0;

    *ldtk_index = candidate_index;
    *prepared_identity = committed_content.identity();
    *prepared_content = committed_content;

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
    Ok(active_room)
}

#[cfg(test)]
mod hot_reload_session_tests {
    use super::*;
    use ambition::runtime::rollback::{RollbackSessionOwnership, SyncTestSettings};

    #[test]
    fn f1_action_toggles_the_app_debug_overlay_both_directions() {
        let bindings = ambition::platformer::developer_hotkeys::DeveloperHotkeyBindings::default();
        assert_eq!(
            bindings.chord_for(DeveloperAction::ToggleDebugOverlay),
            Some(ambition::platformer::developer_hotkeys::DeveloperKeyChord::key(KeyCode::F1,))
        );

        let mut app = App::new();
        app.add_message::<DeveloperAction>();
        app.init_resource::<SandboxDevState>();
        app.init_resource::<DeveloperTools>();
        app.add_systems(Update, handle_debug_hotkeys);

        assert!(!app.world().resource::<SandboxDevState>().debug);
        app.world_mut()
            .write_message(DeveloperAction::ToggleDebugOverlay);
        app.update();
        assert!(app.world().resource::<SandboxDevState>().debug);

        app.world_mut()
            .write_message(DeveloperAction::ToggleDebugOverlay);
        app.update();
        assert!(!app.world().resource::<SandboxDevState>().debug);
    }

    #[test]
    fn local_sync_test_reload_returns_to_a_zero_distance_baseline() {
        let restart = local_ggrs_restart_policy(Some(RollbackSessionOwnership::LocalSyncTest(
            SyncTestSettings {
                check_distance: 6,
                max_prediction_window: 8,
            },
        )))
        .expect("local developer sessions may be rebased")
        .expect("an active local session needs a replacement");

        assert_eq!(restart.check_distance, 0);
        assert_eq!(restart.max_prediction_window, 8);
    }

    #[test]
    fn external_ggrs_reload_requires_a_coordinated_barrier() {
        let error = local_ggrs_restart_policy(Some(RollbackSessionOwnership::External))
            .expect_err("one peer must not replace an external session");
        assert!(error.contains("coordinated content barrier"));
    }

    #[test]
    fn non_ggrs_reload_needs_no_session_restart() {
        assert_eq!(
            local_ggrs_restart_policy(None).expect("no session is a direct reload"),
            None
        );
    }
}
