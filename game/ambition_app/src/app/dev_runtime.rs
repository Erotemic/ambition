use bevy::prelude::*;

use ambition_platformer2d::actors::rooms;
use ambition_platformer2d::platformer::lifecycle::RoomResident;
use ambition_platformer2d::world::rooms as world_rooms;

use ambition_platformer2d::dev_tools::dev_tools::DeveloperTools;
use ambition_platformer2d::dev_tools::DeveloperRuntimeState;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::RoomGeometry;
use ambition_platformer2d::ldtk_map as ldtk_world;
use ambition_platformer2d::platformer::developer_hotkeys::DeveloperAction;
use ambition_platformer2d::render::rendering::spawn_room_visuals;
use ambition_platformer2d::sim as physics;
use ambition_platformer2d::world::world_manifest;

/// Presentation-side debug hotkey reader.
///
/// Runs before the player tick so preset/debug-flag mutations land
/// before the gameplay loop reads them this frame.
pub(super) fn handle_debug_hotkeys(
    mut actions: MessageReader<DeveloperAction>,
    mut dev_state: ResMut<DeveloperRuntimeState>,
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

fn local_ggrs_restart_policy(
    ownership: Option<ambition_platformer2d::rollback::RollbackSessionOwnership>,
) -> Result<Option<ambition_platformer2d::rollback::SyncTestSettings>, &'static str> {
    match ownership {
        Some(ambition_platformer2d::rollback::RollbackSessionOwnership::External) => Err(
            "LDtk hot reload cannot replace an external/P2P GGRS session; peers need a coordinated content barrier",
        ),
        Some(ambition_platformer2d::rollback::RollbackSessionOwnership::LocalSyncTest { settings, .. }) => {
            // THE SAME SESSION, RESTARTED — so it inherits from the session it
            // replaces, and only the deliberate override is spelled out.
            //
            // `..settings` inverts it: preservation is the default and dropping
            // something is the thing you have to type. `check_distance: 0` is
            // that thing — a rebase is not a proof pulse.
            Ok(Some(ambition_platformer2d::rollback::SyncTestSettings {
                check_distance: 0,
                ..settings
            }))
        }
        None => Ok(None),
    }
}

pub(super) fn handle_ldtk_hot_reload(
    mut commands: ambition_platformer2d::platformer::lifecycle::SessionCommands<'_, '_>,
    mut hotkey_actions: MessageReader<DeveloperAction>,
    mut world: ambition_platformer2d::platformer::lifecycle::SessionWorldMut<RoomGeometry>,
    mut room_set: ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
        world_rooms::RoomSet,
    >,
    mut dev_state: ResMut<DeveloperRuntimeState>,
    mut sim_state: ResMut<ambition_platformer2d::platformer::safe_position::RoomTransitionCooldown>,
    mut dialogue: ResMut<ambition_platformer2d::dialog::DialogState>,
    mut ldtk_index: ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
        ldtk_world::LdtkRuntimeIndex,
    >,
    mut ldtk_reload: ResMut<ambition_platformer2d::dev_tools::WorldSourceHotReload>,
    // Bundled to keep this system within Bevy's 16 top-level SystemParam limit.
    tuning: (
        Res<ambition_platformer2d::engine_core::ActiveMovementTuning>,
        Res<physics::PhysicsSandboxSettings>,
    ),
    mut platform_set: ResMut<ambition_platformer2d::world::collision::MovingPlatformSet>,
    // RESIDENTS of the room being replaced — an object in a body's custody rides
    // the reload with its holder, exactly as it rides a room transition. See
    // `RoomResident`.
    room_visuals: Query<
        (
            Entity,
            Option<&ambition_platformer2d::actors::world::physics::PhysicsRoomEntity>,
        ),
        RoomResident,
    >,
    // Bundled into one tuple param to stay within Bevy's 16-param system limit.
    visual_assets: (
        Option<Res<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>>,
        Option<Res<ambition_platformer2d::render::quality::ResolvedVisualQuality>>,
    ),
    mut player_q: Query<
        (
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d::actor::MotionModel,
            &mut ambition_platformer2d::characters::actor::BodyCombat,
            &mut ambition_platformer2d::platformer::safe_position::PlayerSafetyState,
        ),
        // PRIMARY-only: LDtk hot-reload repositions the camera body to the
        // validated spawn — a single-player dev flow.
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    catalogs: (
        Res<ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog>,
        Res<ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>,
        Res<ambition_platformer2d::character::AuthoredSheets>,
        Res<ambition_platformer2d::boss_encounter::BossCatalog>,
        Res<ambition_platformer2d::actors::world::placements::PlacementLoweringRegistry>,
        Res<ambition_platformer2d::actors::features::RoomContentStagingRegistry>,
        Res<ambition_platformer2d::actors::construction::ActorConstructionRegistry>,
        Res<world_manifest::WorldManifest>,
        Option<Res<ambition_platformer2d::characters::prepared::PreparedCharacterRegistry>>,
        Option<
            Res<ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry>,
        >,
    ),
    mut content_identity: (
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::runtime::PreparedContent,
        >,
        ambition_platformer2d::platformer::lifecycle::SessionWorldMut<
            ambition_platformer2d::runtime::PreparedContentIdentity,
        >,
        ResMut<ambition_platformer2d::runtime::ContentEpochSequence>,
        Option<Res<ambition_platformer2d::rollback::RollbackRegistry>>,
        Option<Res<ambition_platformer2d::rollback::RollbackSessionOwnership>>,
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
    // `Platformer2dAssetCatalog::hot_reload_local_path`). If the active
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

    // the SETTINGS are no longer carried across the reload: the session owner holds the policy,
    // and a content reload does not change it.
    if restart_local_ggrs.is_some() {
        ambition_platformer2d::rollback::stop_session_deferred(&mut commands);
        commands.insert_resource(RestartLocalGgrsAfterLdtkReload);
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
            catalogs.8.as_deref(),
            catalogs.9.as_deref(),
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
struct RestartLocalGgrsAfterLdtkReload;

/// Rebind the cheap local baseline after the Update-stage content transaction
/// and its deferred session removal have both committed.
pub(super) fn restart_local_ggrs_after_hot_reload(world: &mut World) {
    let Some(_restart) = world.remove_resource::<RestartLocalGgrsAfterLdtkReload>() else {
        return;
    };

    #[cfg(feature = "dev_tools")]
    crate::dev::rollback_observatory::reset_for_content_reload(world);
    if ambition_platformer2d::rollback::session_is_active(world) {
        ambition_platformer2d::rollback::stop_session(world);
    }
    // Releasing ownership is the whole of what this path owes: `maintain_local_session` sees no
    // session on the next frame and starts one, with the SAME policy and the SAME frozen seating,
    // because neither of those is what a content reload changed.
    world
        .resource_mut::<ambition_platformer2d::rollback::local_session::LocalSessionOwnership>()
        .release();
    info!("LDtk hot reload released the local GGRS baseline; the session owner will rebase it");
}

pub(super) struct LdtkReloadTransaction {
    project: ldtk_world::LdtkProject,
    next_room_set: world_rooms::RoomSet,
    next_spec: world_rooms::RoomSpec,
    safe_player_pos: ae::Vec2,
}

pub(super) fn prepare_ldtk_reload_transaction(
    watch_path: &std::path::Path,
    catalog: &ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog,
    manifest: &world_manifest::WorldManifest,
    current_room_id: &str,
    preserved_pos: ae::Vec2,
    player_size: ae::Vec2,
) -> Result<LdtkReloadTransaction, Vec<String>> {
    let project = ldtk_world::LdtkProject::load_from_disk_at(watch_path, catalog, manifest)
        .map_err(|error| vec![error])?;
    let report = project.validate(&crate::composed_ldtk_vocabulary());
    report.print_to_stderr();
    if !report.is_ok() {
        return Err(report.errors);
    }

    let mut next_room_set = project.to_room_set(manifest, &crate::composed_ldtk_vocabulary())?;
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
            bevy::log::debug!(target: "ambition_platformer2d::room_layout", "LDtk reload: {warning}");
        }
    }
    if !hard_errors.is_empty() {
        return Err(hard_errors);
    }

    let safe_player_pos =
        world_rooms::validated_spawn(&next_spec.world, preserved_pos, player_size);
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
    room_set: &mut world_rooms::RoomSet,
    motion_model: &mut ae::MotionModel,
    clusters: &mut ae::BodyClustersMut<'_>,
    dev_state: &mut DeveloperRuntimeState,
    sim_state: &mut ambition_platformer2d::platformer::safe_position::RoomTransitionCooldown,
    safety: &mut ambition_platformer2d::platformer::safe_position::PlayerSafetyState,
    dialogue: &mut ambition_platformer2d::dialog::DialogState,
    combat: &mut ambition_platformer2d::characters::actor::BodyCombat,
    ldtk_index: &mut ldtk_world::LdtkRuntimeIndex,
    tuning: ae::MovementTuning,
    physics_settings: physics::PhysicsSandboxSettings,
    moving_platforms: &mut Vec<ambition_platformer2d::world::platforms::MovingPlatformState>,
    room_visuals: &Query<
        (
            Entity,
            Option<&ambition_platformer2d::actors::world::physics::PhysicsRoomEntity>,
        ),
        RoomResident,
    >,
    assets: Option<&ambition_platformer2d::sprite_sheet::game_assets::GameAssets>,
    quality: Option<&ambition_platformer2d::render::quality::ResolvedVisualQuality>,
    watch_path: &std::path::Path,
    catalog: &ambition_platformer2d::asset_manager::platformer_assets::Platformer2dAssetCatalog,
    character_catalog: &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_platformer2d::character::AuthoredSheets,
    boss_catalog: &ambition_platformer2d::boss_encounter::BossCatalog,
    placement_lowering: &ambition_platformer2d::actors::world::placements::PlacementLoweringRegistry,
    content_staging: &ambition_platformer2d::actors::features::RoomContentStagingRegistry,
    construction_recipes: &ambition_platformer2d::actors::construction::ActorConstructionRegistry,
    world_manifest: &world_manifest::WorldManifest,
    prepared_characters: Option<
        &ambition_platformer2d::characters::prepared::PreparedCharacterRegistry,
    >,
    brain_profiles: Option<
        &ambition_platformer2d::characters::actor::character_catalog::BrainProfileRegistry,
    >,
    prepared_content: &mut ambition_platformer2d::runtime::PreparedContent,
    prepared_identity: &mut ambition_platformer2d::runtime::PreparedContentIdentity,
    epochs: &mut ambition_platformer2d::runtime::ContentEpochSequence,
    snapshot_schema: ambition_platformer2d::runtime::SnapshotSchemaFingerprint,
    session_scope: ambition_platformer2d::platformer::lifecycle::SessionSpawnScope,
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
    let candidate_source = prepared_content
        .source()
        .with_world(
            transaction.next_room_set.clone(),
            RoomGeometry(transaction.next_spec.world.clone()),
            world_rooms::ActiveRoomMetadata(transaction.next_spec.metadata.clone()),
        )
        // `with_world` carries the OLD index forward on purpose, so the road
        // that reloaded an LDtk project has to state its replacement. This is
        // that road, and the reload is exactly what changed the index.
        .with_installed_ldtk_index(candidate_index.clone());
    let candidate_content = ambition_platformer2d::provider::prepare_world_replacement_candidate(
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
        authored_sheets,
        boss_catalog,
        session_scope,
        ambition_platformer2d::actors::features::ActorConstructionContext::for_room_construction(
            construction_recipes,
            // The generation currently live. A materially changed definition
            // allocates a new one below, AFTER every preflight has succeeded —
            // so a plan prepared here always states the epoch it was validated
            // against, never one that does not exist yet.
            prepared_content.epoch(),
            None,
            prepared_characters,
            brain_profiles,
            // A hot reload replaces the authored content wholesale, so the
            // dispositions of occurrences minted from the OLD definitions say
            // nothing about the new ones. Rebuilt from the records alone.
            None,
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
    // The session's live content binding follows the COMMITTED content. Queued
    // after `commit_deferred`, so this transaction still verifies against the
    // binding it was prepared under (the epoch that existed at preflight);
    // every LATER transaction must state the new one or be refused as stale.
    commands.insert_resource(
        ambition_platformer2d::actors::rooms::ActiveContentBinding::content(
            committed_content.epoch(),
        ),
    );

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
        &mut *clusters.dodge,
        tuning.air_jumps,
        // A dev transit re-seats the body somewhere safe; that answers for
        // anything it had committed.
        ae::RecoveryRefresh::Answered,
    );
    safety.last_safe_pos = transaction.safe_player_pos;
    dialogue.close();
    combat.hitstop_timer = 0.0;
    combat.hitstun_timer = 0.0;
    combat.recoil_lock_timer = 0.0;
    sim_state.remaining = 0.10;
    dev_state.preset_flash = 1.0;

    *ldtk_index = candidate_index;
    *prepared_identity = committed_content.identity();
    *prepared_content = committed_content;

    ambition_platformer2d::render::rendering::spawn_parallax_layers(
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
    use ambition_platformer2d::rollback::{RollbackSessionOwnership, SyncTestSettings};

    #[test]
    fn f1_action_toggles_the_app_debug_overlay_both_directions() {
        let bindings =
            ambition_platformer2d::platformer::developer_hotkeys::DeveloperHotkeyBindings::default(
            );
        assert_eq!(
            bindings.chord_for(DeveloperAction::ToggleDebugOverlay),
            Some(
                ambition_platformer2d::platformer::developer_hotkeys::DeveloperKeyChord::key(
                    KeyCode::F1,
                )
            )
        );

        let mut app = App::new();
        app.add_message::<DeveloperAction>();
        app.init_resource::<DeveloperRuntimeState>();
        app.init_resource::<DeveloperTools>();
        app.add_systems(Update, handle_debug_hotkeys);

        assert!(!app.world().resource::<DeveloperRuntimeState>().debug);
        app.world_mut()
            .write_message(DeveloperAction::ToggleDebugOverlay);
        app.update();
        assert!(app.world().resource::<DeveloperRuntimeState>().debug);

        app.world_mut()
            .write_message(DeveloperAction::ToggleDebugOverlay);
        app.update();
        assert!(!app.world().resource::<DeveloperRuntimeState>().debug);
    }

    #[test]
    fn local_sync_test_reload_returns_to_a_zero_distance_baseline() {
        let restart = local_ggrs_restart_policy(Some(RollbackSessionOwnership::LocalSyncTest {
            owner: ambition_platformer2d::rollback::SyncTestOwner::LocalMaintainer,
            settings: SyncTestSettings {
                check_distance: 6,
                max_prediction_window: 8,
                ..SyncTestSettings::for_players(1)
            },
        }))
        .expect("local developer sessions may be rebased")
        .expect("an active local session needs a replacement");

        assert_eq!(restart.check_distance, 0);
        assert_eq!(restart.max_prediction_window, 8);
    }

    /// A reload must not evict player two.
    ///
    /// The rebase preserved `max_prediction_window` and rebuilt everything else
    /// from `..Default::default()`, whose player count is ONE — so a couch
    /// session of two to four silently became single-player after any LDtk hot
    /// reload, with seats 1..3 keeping their bodies and leaving the rollback
    /// session. `check_distance` going to zero is deliberate (a fresh baseline);
    /// WHO IS PLAYING is not a baseline, it is topology.
    #[test]
    fn a_reload_keeps_every_local_player_in_the_session() {
        let restart = local_ggrs_restart_policy(Some(RollbackSessionOwnership::LocalSyncTest {
            owner: ambition_platformer2d::rollback::SyncTestOwner::LocalMaintainer,
            settings: SyncTestSettings {
                check_distance: 6,
                max_prediction_window: 8,
                players: 4,
            },
        }))
        .expect("local developer sessions may be rebased")
        .expect("an active local session needs a replacement");

        assert_eq!(
            restart.players,
            4,
            "the reload dropped {} of the 4 local players; a couch session must \
             not lose seats to a content reload",
            4 - restart.players
        );
        // And the deliberate reset is still deliberate.
        assert_eq!(restart.check_distance, 0);
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
