#[allow(unused_imports)]
use super::cli::*;
#[allow(unused_imports)]
use super::dev_runtime::*;
#[allow(unused_imports)]
use super::feedback::*;
#[allow(unused_imports)]
use super::hud::*;
#[allow(unused_imports)]
use super::input_systems::*;
#[allow(unused_imports)]
use super::phases::*;
#[allow(unused_imports)]
use super::resources::*;
#[allow(unused_imports)]
use super::schedule::*;
#[allow(unused_imports)]
use super::setup_systems::*;
#[allow(unused_imports)]
use super::sim_systems::*;
#[allow(unused_imports)]
use super::update::*;
#[allow(unused_imports)]
use super::world_flow::*;
#[allow(unused_imports)]
use super::*;

/// Register core simulation plugins, message types, and the gameplay
/// schedule. Headless and visible both call this.
///
/// The body is split into per-set helpers below so each section is short
/// enough to read in one screen and stays under Bevy's 20-system tuple
/// arity limit. New simulation systems should go into the matching
/// `register_*_systems` helper rather than back into this orchestrator.
pub fn add_simulation_plugins(app: &mut App) {
    // AmbitionPhysicsPlugin (Avian2D) is intentionally NOT here. Per
    // ADR 0007 Avian is secondary physics for debris/ragdoll visuals;
    // the player controller is custom via parry2d in ambition_engine.
    // Avian's collider backend needs `SceneSpawner` (from ScenePlugin in
    // DefaultPlugins), which headless doesn't have. Until Avian's debris
    // role is migrated to presentation events end-to-end (or Avian gains
    // a headless-friendly init path), it lives in
    // `add_presentation_plugins`.

    // Declare the canonical simulation-phase ordering. Individual system
    // registrations below only need `.in_set(SandboxSet::X)`; they no longer
    // need to pin a cross-set system via `.after(other_system)`. Intra-set
    // `.chain()` ordering is still expressed per-system.
    configure_sandbox_sets(app);

    install_simulation_messages_and_resources(app);

    register_world_prep_systems(app);
    register_player_input_systems(app);
    register_player_simulation_systems(app);
    register_room_transition_systems(app);
    register_combat_systems(app);
    register_presentation_sync_systems(app);
    register_feature_collection_systems(app);
    register_feature_interaction_systems(app);
    register_ldtk_runtime_spine_systems(app);
    register_encounter_simulation_systems(app);
    register_cutscene_systems(app);
    register_gameplay_effects_systems(app);
    register_progression_chain_systems(app);
    register_progression_populate_systems(app);
    register_feature_view_sync_systems(app);
    register_reset_processing_systems(app);
    register_trace_systems(app);
}

/// Install the simulation-side messages, resources, registered types,
/// Startup / PostStartup systems, and child plugins that the rest of
/// `add_simulation_plugins` depends on.
fn install_simulation_messages_and_resources(app: &mut App) {
    app.add_message::<SfxMessage>()
        .add_message::<VfxMessage>()
        .add_message::<DebrisBurstMessage>()
        .add_message::<PlayerDiedMessage>()
        .add_message::<crate::features::GameplayEffect>()
        .add_message::<crate::features::PlayerDamageEvent>()
        .add_message::<crate::features::DamageEvent>()
        .add_message::<crate::features::PogoBounceEvent>()
        .add_message::<crate::features::ResetRoomFeaturesEvent>()
        .add_message::<crate::features::GameplayBannerRequested>()
        .add_message::<crate::player::PlayerHealRequested>()
        .add_message::<crate::rooms::RoomTransitionRequested>()
        // ADR 0010 — time-control vocabulary. Gameplay code writes
        // ClockScaleRequest instead of mutating SandboxSimState::
        // time_scale directly; apply_clock_scale_requests consults
        // RegimePolicy (default: Solo, grant-all), records the
        // granted target in RequestedClockScale, and
        // smooth_sim_clock_toward_target_system ramps the live
        // time_scale toward the target at feel-tuned rates. See
        // `crate::time_control` for the policy + dispatch and ADR
        // 0010 §Vocabulary for the model.
        .add_message::<crate::time_control::ClockScaleRequest>()
        .insert_resource(crate::time_control::RegimePolicy::default())
        .insert_resource(crate::time_control::RequestedClockScale::default())
        .register_type::<GameMode>()
        // StartupProfiler captures wall-clock at each marked phase so a
        // PostStartup report prints "where did the first frame's
        // worth of init time go" without needing an external profiler
        // attached. See `crate::profiling` for the helper API and
        // `docs/recipes/profiling.md` for Tracy / per-system profiling.
        .insert_resource(crate::profiling::StartupProfiler::default())
        .insert_resource(crate::trace::GameplayTraceBuffer::default())
        .insert_resource(crate::CurrentPlayerAttack::default())
        .insert_resource(crate::dialog::DialogState::default())
        .insert_resource(crate::MovingPlatformSet::default())
        .insert_resource(crate::SandboxSimState::default())
        .insert_resource(crate::SandboxDevState::default())
        .insert_resource(crate::features::GameplayBanner::default())
        .insert_resource(crate::features::FeatureEcsWorldOverlay::default())
        .insert_resource(crate::features::FeatureViewIndex::default())
        .insert_resource(crate::mechanics::MechanicsRegistry::default())
        .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
        .add_plugins(ae::AmbitionStateMachinePlugin)
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("startup_begin"),
                data::load_data_asset_handle,
                crate::profiling::phase_mark("after_load_data_handle"),
                setup_simulation_system,
                crate::profiling::phase_mark("after_setup_simulation"),
            )
                .chain(),
        )
        // Final report. Runs once on the first PostStartup tick. The
        // pre-report mark captures the time between the last Startup
        // mark and PostStartup, so any heavy Startup systems we
        // didn't explicitly mark show up as a delta on the
        // "post_startup_begin" line.
        .add_systems(
            PostStartup,
            (
                crate::profiling::phase_mark("post_startup_begin"),
                crate::profiling::report_startup_phases,
            )
                .chain(),
        )
        .insert_resource(crate::projectile::PlayerProjectileState::default())
        // Enemy projectiles (pirate volleys etc) — separate from
        // player projectiles so faction routing stays explicit.
        .insert_resource(crate::enemy_projectile::EnemyProjectileState::default())
        // Anti-clump attack slot arbitration. Default layout: 3
        // melee ring slots + 3 aerial arc slots around the player.
        .insert_resource(crate::combat_slots::CombatSlotsRes::default())
        // Encounter system. The legacy single-encounter `EncounterState`
        // resource stays for backwards-compat tests; the live
        // multi-encounter store is `EncounterRegistry`, populated
        // from LDtk by `populate_encounter_registry`.
        .insert_resource(crate::encounter::EncounterState::default())
        .insert_resource(crate::encounter::EncounterRegistry::default())
        .insert_resource(crate::encounter::SwitchActivationQueue::default())
        .insert_resource(crate::encounter::EncounterSwitchIndex::default())
        .insert_resource(crate::encounter::EncounterMusicRequest::default())
        .insert_resource(crate::rooms::RoomMusicRequest::default())
        // Sandbox save game (encounter defeat + switch state).
        // Loaded from disk by `load_save_at_startup` in the
        // presentation half so headless / RL drivers don't touch
        // disk; mutated by encounter + switch systems; written by
        // `autosave_sandbox_save` when change-detection fires.
        .insert_resource(crate::save::SandboxSave::default())
        // Quest + cutscene systems. Both are sim-side state machines
        // that read/write the save resource and surface HUD lines via
        // the encounter overlay.
        .insert_resource(crate::quest::QuestRegistry::default())
        .insert_resource(crate::cutscene::default_cutscene_library())
        .insert_resource(crate::cutscene::ActiveCutscene::default())
        .insert_resource(crate::cutscene::CutsceneTriggerQueue::default())
        .insert_resource(crate::cutscene::CutsceneAdvanceRequest::default())
        .insert_resource(crate::cutscene::RoomCutsceneBindings::defaults())
        // Combat-banter registry — story-content lines for the
        // `apply_feature_damage_events` hit handler. Boss barks are
        // installed inline; IntroPlugin adds the intro raiders' lines
        // via a startup system.
        .insert_resource({
            let mut reg = crate::banter::CombatBanterRegistry::default();
            crate::boss_encounter::install_boss_banter(&mut reg);
            reg
        })
        // World-clock dt mirror — `WorldTime::scaled_dt` is the
        // bullet-time-respecting delta for gameplay timers and
        // world-anchored animation timers. `WorldTime::raw_dt`
        // stays the wall-clock dt for UI / debug. Refreshed each
        // frame by `refresh_world_time` registered below; new code
        // should reach for `Res<WorldTime>::scaled_dt` instead of
        // `Res<Time>::delta_secs()` for anything that should slow
        // / freeze when the world slows / freezes.
        .insert_resource(crate::WorldTime::default())
        // Portal registry — per-portal lifecycle state machine
        // (Off / Opening / On / Closing). The portal itself owns
        // traversal readiness; the switch only commands the
        // boot/shutdown sequence. `detect_room_transition_system`
        // blocks the transition unless the named portal's phase is
        // `On`. `tick_portal_phases_system` advances phase from the
        // switch state each frame. Empty by default; IntroPlugin
        // registers the intro_portal_zone → intro_portal_switch
        // hookup.
        .insert_resource(crate::rooms::PortalRegistry::default())
        // Intro story content plugin. Extends CutsceneLibrary +
        // RoomCutsceneBindings (always) and GameAssets.characters.npcs
        // (visible builds only — the sprite installer is a no-op in
        // headless where GameAssets is absent). Keeps story content out
        // of sandbox-owned files in preparation for a future
        // sandbox / game crate split.
        .add_plugins(crate::intro::IntroPlugin)
        .insert_resource(crate::boss_encounter::BossEncounterRegistry::default())
        .insert_resource(crate::map_menu::MapMenuState::default())
        .insert_resource(crate::CameraEaseState::default())
        .insert_resource(crate::CameraEaseTuning::default())
        .insert_resource(crate::rendering::CameraViewState::default())
        .insert_resource(crate::reset::SandboxResetRequested::default());
}

// Core simulation, split into 6 finer-grained sub-sets that are
// chained inside `SandboxSet::CoreSimulation`. See
// `schedule.rs::configure_sandbox_sets` for the sub-set ordering.
// External presentation/audio/HUD systems still pin against
// `SandboxSet::CoreSimulation`; that constraint covers all six
// sub-sets transitively.

/// LDtk hot-reload + feature world overlay + feature ticks. Sets up the
/// collision world that PlayerInput + PlayerSimulation read.
fn register_world_prep_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            ldtk_world::poll_ldtk_file_changes,
            crate::features::rebuild_feature_ecs_world_overlay,
            crate::features::update_ecs_hazards,
            crate::features::update_ecs_actors,
            crate::features::update_ecs_bosses,
        )
            .chain()
            .in_set(SandboxSet::WorldPrep),
    );
}

/// Dev-edit sync + input-driven reset + gameplay timer decay + interact
/// buffer + suspended-time fallback. Each subsequent system depends on
/// the previous one's ControlFrame / component mutation, so they stay
/// chained.
///
/// Ordering subtleties (ADR 0010 §"Suspended time"):
/// * `apply_suspended_time_scale_system` runs FIRST so when gameplay
///   is suspended (pause / dialogue / cutscene / room transition) the
///   sim_clock target and `SandboxSimState::time_scale` are zeroed
///   BEFORE `refresh_world_time` snapshots them. Previously this
///   system ran last in the chain, so `WorldTime::scaled_dt` could
///   be non-zero on the very first suspended frame and presentation
///   systems scaling animations by `time_scale * dt` would tick once
///   after pause landed.
/// * The emit → apply → smooth trio is gated to `gameplay_allowed`
///   so it doesn't immediately re-populate `RequestedClockScale` /
///   `time_scale` back from the zero the suspended fallback just
///   wrote. On the first re-resumed frame they run again and the
///   smoother ramps back up from 0 to 1.0 at the authored rate.
/// * `refresh_world_time` then snapshots whichever path won this
///   frame, so downstream systems always see a coherent `scaled_dt`.
fn register_player_input_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            apply_suspended_time_scale_system.run_if(gameplay_suspended),
            // ADR 0010 — time-control pipeline. Gated to
            // `gameplay_allowed` so suspended frames don't re-emit a
            // default 1.0 request that would compete with the
            // suspended fallback above.
            crate::time_control::emit_player_time_intent_system.run_if(gameplay_allowed),
            crate::time_control::apply_clock_scale_requests.run_if(gameplay_allowed),
            crate::time_control::smooth_sim_clock_toward_target_system.run_if(gameplay_allowed),
            // Unconditional: snapshot whichever path (suspended-zero
            // or gameplay-smoothed) wrote `SandboxSimState::time_scale`
            // this frame into `WorldTime` for downstream readers.
            crate::refresh_world_time,
            sync_live_player_dev_edits_system,
            apply_player_reset_input_system.run_if(gameplay_allowed),
            input_timer_system.run_if(gameplay_allowed),
            interaction_input_system.run_if(gameplay_allowed),
        )
            .chain()
            .in_set(SandboxSet::PlayerInput),
    );
}

/// Main player tick (two-clock control + simulation) plus post-sim
/// damage / safe-respawn.
fn register_player_simulation_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            sandbox_update.run_if(gameplay_allowed),
            apply_player_damage_system.run_if(gameplay_allowed),
        )
            .chain()
            .in_set(SandboxSet::PlayerSimulation),
    );
}

/// Detection emits `RoomTransitionRequested`; apply consumes it and runs
/// `load_room`; the feature-side `reset_ecs_room_features` system tears
/// down per-room ECS state.
fn register_room_transition_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            detect_room_transition_system.run_if(gameplay_allowed),
            apply_room_transition_system,
            crate::features::reset_ecs_room_features,
        )
            .chain()
            .in_set(SandboxSet::RoomTransition),
    );
}

/// Slash/pogo attack lifecycle, projectile tick, and the feature-side
/// damage event apply.
fn register_combat_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            attack_advance_system.run_if(gameplay_allowed),
            crate::projectile::update_projectiles,
            crate::enemy_projectile::update_enemy_projectiles.run_if(gameplay_allowed),
            crate::features::apply_feature_damage_events,
        )
            .chain()
            .in_set(SandboxSet::Combat),
    );
}

/// Player ECS body write-back + presentation timer decays. Runs
/// unconditionally so paused / dialogue modes still wind down flash and
/// landing-pose timers.
fn register_presentation_sync_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::player::write_player_ecs_components,
            cleanup_timers_system,
        )
            .chain()
            .in_set(SandboxSet::PresentationSync),
    );
}

/// Per-frame [`crate::features::FeatureViewIndex`] rebuild.
///
/// Has its own set rather than living at the end of `PresentationSync`
/// because the pickup / chest / switch / encounter-mob / save-driven
/// boss sync mutations land in sets that fire *after* `CoreSimulation`
/// (`FeatureCollection`, `FeatureInteraction`, `EncounterSimulation`,
/// `GameplayEffects`, `Progression`). Rebuilding at the very tail of
/// the sim chain guarantees the cache reflects this frame's full
/// feature state before the presentation half reads it.
fn register_feature_view_sync_systems(app: &mut App) {
    app.add_systems(
        Update,
        crate::features::rebuild_feature_view_index.in_set(SandboxSet::FeatureViewSync),
    );
}

fn register_feature_collection_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::features::collect_ecs_pickups,
            crate::player::apply_player_heal_requests,
        )
            .chain()
            .in_set(SandboxSet::FeatureCollection),
    );
}

fn register_feature_interaction_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::features::interact_ecs_actors_and_switches,
            crate::features::open_ecs_chests,
            crate::features::update_ecs_breakables,
            crate::features::update_ecs_falling_chests,
            crate::features::sync_ecs_switches_from_save,
            crate::encounter::rebuild_encounter_switch_index,
        )
            .chain()
            .in_set(SandboxSet::FeatureInteraction),
    );
}

fn register_ldtk_runtime_spine_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            ldtk_world::sync_plugin_spawned_ambition_entities,
            ldtk_world::rebuild_ldtk_runtime_spine_index,
            ldtk_world::rebuild_ldtk_runtime_solid_index,
            ldtk_world::rebuild_ldtk_runtime_one_way_index,
            ldtk_world::rebuild_ldtk_runtime_damage_index,
            ldtk_world::check_ldtk_runtime_spine_parity,
        )
            .chain()
            .in_set(SandboxSet::LdtkRuntimeSpine),
    );
}

fn register_encounter_simulation_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            platforms::sync_moving_platform,
            crate::encounter::update_encounters_from_world,
            crate::encounter::sync_encounter_controller_states,
            crate::features::apply_gameplay_banner_requests,
            crate::features::tick_gameplay_banner,
        )
            .chain()
            .in_set(SandboxSet::EncounterSimulation),
    );
}

// Progression chain: cutscenes, gameplay-effect routing, boss encounters,
// quest events, and the F3 stats editor sync. Split into several chained
// groups so each tuple stays under Bevy's macro arity limit while
// preserving the old drain-before-progression order.

fn register_cutscene_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::cutscene::auto_trigger_room_cutscenes,
            crate::cutscene::drain_cutscene_triggers,
            crate::cutscene::tick_active_cutscene,
        )
            .chain()
            .in_set(SandboxSet::Cutscene),
    );
}

fn register_gameplay_effects_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::features::apply_flag_effects,
            crate::features::apply_quest_effects,
            crate::features::apply_switch_effects,
            crate::features::apply_boss_damage_effects,
            crate::features::apply_npc_strike_effects,
            crate::features::apply_gameplay_sfx_effects,
        )
            .chain()
            .in_set(SandboxSet::GameplayEffects),
    );
}

fn register_progression_chain_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::boss_encounter::update_boss_encounters,
            crate::features::sync_ecs_actors_with_save,
            crate::features::sync_ecs_bosses_with_save,
            crate::quest::push_room_entered_quest_events,
            crate::quest::apply_quest_advance_events,
            crate::quest::grant_quest_completion_rewards,
            crate::body_mode::update_body_mode,
            crate::rooms::sync_active_room_metadata,
            crate::rooms::sync_room_music_request,
            // Portal lifecycle: advance every registered portal's
            // phase from its switch state + per-phase timers.
            // Pure state update; the visibility + ring-spin
            // systems below consume the phase. Lives in the
            // Progression set so the portal state is current
            // before `detect_room_transition_system` runs (which
            // is in CoreSimulation, ordered after Progression).
            crate::rooms::tick_portal_phases_system,
            crate::map_menu::track_room_visits,
            crate::map_menu::sync_map_from_save,
            dev_tools::sync_player_stats_with_inspector,
        )
            .chain()
            .in_set(SandboxSet::Progression),
    );
}

/// Populate the encounter / quest / boss registries from the LDtk
/// project + save. These run on Update (not Startup) with their
/// existing `specs_loaded` / `initialized` short-circuits so:
///   1. The first Update tick populates them (Startup is done by
///      the time any Update fires, so SandboxLdtkProject + save
///      are ready).
///   2. The "reset sandbox" flow (`process_sandbox_reset_request`)
///      can flip those flags back to false and the next tick
///      repopulates from the freshly-cleared save — without us
///      having to inline the populate logic in two places.
/// The cost when already loaded is one ResMut acquisition + one
/// bool check per registry per frame: negligible.
fn register_progression_populate_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::quest::populate_quest_registry,
            crate::boss_encounter::populate_boss_encounter_registry,
            crate::encounter::populate_encounter_registry,
        )
            .in_set(SandboxSet::Progression),
    );
}

/// Sandbox reset processor: consumes pending reset requests (set by the
/// pause-menu "Reset Sandbox" item or any other caller). In set
/// ResetProcessing (configured after CoreSimulation) so it can't race
/// with in-flight gameplay mutations, and before the populate systems on
/// the next frame so they see the cleared registries when re-running.
fn register_reset_processing_systems(app: &mut App) {
    app.add_systems(
        Update,
        crate::reset::process_sandbox_reset_request.in_set(SandboxSet::ResetProcessing),
    );
}

/// Trace recorder lives at the simulation seam: `record_frame_system`
/// captures one frame per Update tick after CoreSimulation has resolved
/// player state; `flush_pending_dump` writes any pending dump to disk on
/// the same tick. Both run on the simulation half so headless and
/// visible builds share trace output.
fn register_trace_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            crate::trace::record_frame_system,
            crate::trace::flush_pending_dump.after(crate::trace::record_frame_system),
        )
            .in_set(SandboxSet::Trace),
    );
}

/// Register Bevy's `LdtkPlugin` plus the supporting Ambition glue
/// (entity registrations, asset collection, LdtkWorldBundle spawn,
/// level-set sync, asset handle preload). Visible binary only —
/// `LdtkPlugin` panics in headless because its tile pipeline expects a
/// `RenderApp` sub-app, and `asset_server.load::<LdtkProject>` requires
/// the LDtk asset type to be registered.
///
/// Once the LDtk runtime-spine roadmap finishes promoting LDtk entity
/// categories to direct Ambition components (see
/// `project_ldtk_roadmap` memory), this dependency goes away and
/// headless can spawn the same entity set without bevy_ecs_ldtk's
/// rendering machinery.
pub fn add_ldtk_runtime_plugin(app: &mut App) {
    // `SandboxAssetCollection` includes a typed LDtk handle, so the LDtk
    // asset type and loader must be initialized before bevy_asset_loader
    // allocates collection handles. Keep this before `init_collection`.
    app.add_plugins(LdtkPlugin)
        .init_collection::<loading::SandboxAssetCollection>()
        .add_plugins(ldtk_world::AmbitionLdtkRegistrationPlugin)
        .add_systems(
            Startup,
            (
                ldtk_world::load_ldtk_asset_handle,
                spawn_ldtk_world_root.after(setup_simulation_system),
            ),
        )
        .add_systems(
            Update,
            (
                ldtk_world::sync_ldtk_level_set,
                // ADR 0015 §Coordinate-frame reconciliation — keep the
                // LdtkWorldBundle's root transform aligned with the
                // current active area's centered frame. Runs every
                // frame; cheap and idempotent.
                ldtk_world::sync_ldtk_world_transform,
            ),
        );
}

/// Spawn the `LdtkWorldBundle` entity. Runs in `add_ldtk_runtime_plugin`
/// (visible binary only) after `setup_simulation_system` so the
/// `LdtkRuntimeIndex` resource is available.
pub(super) fn spawn_ldtk_world_root(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ldtk_index: Res<ldtk_world::LdtkRuntimeIndex>,
    room_set: Res<rooms::RoomSet>,
    ldtk_asset: Option<Res<ldtk_world::SandboxLdtkAsset>>,
    intro_asset: Option<Res<ldtk_world::IntroLdtkAsset>>,
    sandbox_asset_collection: Option<Res<loading::SandboxAssetCollection>>,
) {
    let ldtk_handle = ldtk_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .or_else(|| {
            sandbox_asset_collection
                .as_ref()
                .map(|collection| collection.ldtk_project.clone())
        })
        .unwrap_or_else(|| asset_server.load(ldtk_world::sandbox_ldtk_asset_path()));
    let initial_level_set = ldtk_index.level_set_for(&room_set.active_spec().id);
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: ldtk_handle.into(),
            level_set: initial_level_set.clone(),
            // AMBITION_REVIEW(spatial): migrate each registered marker from
            // adapter-driven semantics to direct Ambition components.
            ..default()
        },
        ldtk_world::SandboxLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root (sandbox.ldtk)"),
    ));
    // Secondary intro LDtk bundle. bevy_ecs_ldtk's asset loader is
    // per-file; Ambition's merged JSON loader doesn't propagate into
    // the Bevy asset system. Each .ldtk file therefore needs its own
    // bundle to get its painted tile layers rendered. The shared
    // sync system writes the same LevelSet to both bundles; only the
    // bundle whose loaded asset contains the active level iids spawns
    // any levels.
    let intro_handle = intro_asset
        .as_ref()
        .map(|asset| asset.0.clone())
        .unwrap_or_else(|| asset_server.load("ambition/worlds/intro.ldtk"));
    commands.spawn((
        bevy_ecs_ldtk::prelude::LdtkWorldBundle {
            ldtk_handle: intro_handle.into(),
            level_set: initial_level_set,
            ..default()
        },
        ldtk_world::IntroLdtkWorldRoot,
        Name::new("LDtk Runtime Spine Root (intro.ldtk)"),
    ));
}

/// Register presentation-side plugins (input, dialogue, inspector, audio
/// and VFX subscribers, HUD, debug overlays). Visible binary only.
pub fn add_presentation_plugins(app: &mut App) {
    install_presentation_resources_and_subplugins(app);
    install_settings_and_save_systems(app);
    install_menu_setup_and_hotkeys(app);
    install_visual_animation_systems(app);
    install_camera_and_debug_overlay_systems(app);
    install_fx_and_hud_systems(app);
    install_misc_visual_sync_systems(app);
    install_player_visual_systems(app);
    install_projectile_and_vfx_systems(app);
}

/// Visible-side resources, registered types, and presentation child
/// plugins (input, audio, dev_tools, physics_debris, ui, mobile touch,
/// FPS overlay, font loader).
fn install_presentation_resources_and_subplugins(app: &mut App) {
    app.insert_resource(ClearColor(Color::srgb(0.020, 0.024, 0.035)))
        .insert_resource(windowing::DisplayModeState::default())
        .register_type::<DeveloperTools>()
        .register_type::<PlayerBodyProfile>()
        .register_type::<MovementProfile>()
        .register_type::<EditableAbilitySet>()
        .register_type::<EditableMovementTuning>()
        .register_type::<EditablePlayerStats>()
        .register_type::<SandboxFeelTuning>();

    app.add_plugins(crate::platform::PlatformPlugin);
    app.add_plugins(crate::screen_effects::ScreenEffectsPlugin);
    add_dev_tools_plugins(app);
    add_physics_debris_plugins(app);
    add_ui_plugins(app);
    add_input_plugins(app);
    add_audio_plugins(app);
    add_mobile_touch_plugin(app);
    // Lightweight FPS / frame-time overlay. ON by default on wasm,
    // OFF on desktop; F3 toggles. See `crate::fps_overlay`.
    app.add_plugins(crate::fps_overlay::FpsOverlayPlugin);

    app.add_systems(Startup, ui_fonts::load_ui_fonts);
}

/// Settings + sandbox-save persistence. Both load on startup and
/// autosave when the relevant resource changes (`Res::is_changed`
/// throttle). Headless drivers do not register these systems, so
/// a `cargo run --bin headless` never reads or writes user files.
fn install_settings_and_save_systems(app: &mut App) {
    app.add_systems(
        Startup,
        (
            crate::settings::persistence::load_settings_at_startup,
            crate::save::load_save_at_startup,
        ),
    )
    .add_systems(
        Update,
        (
            crate::settings::persistence::save_settings_on_change,
            crate::save::autosave_sandbox_save,
        ),
    );
}

/// Pause menu, inventory, map menu, presentation startup, dev/dialog
/// hotkeys.
fn install_menu_setup_and_hotkeys(app: &mut App) {
    app.insert_resource(pause_menu::PauseMenuState::default())
        .insert_resource(inventory::InventoryUiState::default())
        .insert_resource(inventory::PlayerInventory::starter())
        .add_systems(
            Startup,
            (
                pause_menu::spawn_pause_menu,
                inventory::spawn_inventory_panel,
            )
                .after(setup_simulation_system),
        )
        .add_systems(
            Update,
            (
                pause_menu::sync_pause_menu,
                pause_menu::sync_settings_panel_rows,
                inventory::sync_inventory_panel,
                crate::map_menu::sync_map_menu,
            )
                .after(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("before_setup_presentation"),
                setup_presentation_system,
                crate::profiling::phase_mark("after_setup_presentation"),
                crate::map_menu::populate_map_rooms,
                crate::map_menu::spawn_map_menu,
                crate::profiling::phase_mark("after_map_menu_spawn"),
            )
                .chain()
                .after(setup_simulation_system)
                .after(ui_fonts::load_ui_fonts),
        )
        .add_systems(
            Update,
            (
                dialog::dialog_input,
                handle_ldtk_hot_reload,
                handle_debug_hotkeys,
                dev_tools::sync_developer_body_profile,
                crate::trace::handle_trace_hotkey,
                crate::map_menu::handle_map_menu_hotkeys,
            )
                .chain()
                .after(SandboxSet::CoreSimulation),
        );
}

/// Visual entity spawn for dynamic features + sprite/animation pipelines.
/// Chained after `handle_map_menu_hotkeys` (the last input system in the
/// presentation half) and placed in [`SandboxSet::PresentationVisualSync`].
///
/// `sync_visuals` reads `FeatureViewIndex` and `upgrade_enemy_sprites`
/// + `upgrade_npc_sprites` (chained later via `.after(sync_visuals)`)
/// read it too. The `.after(SandboxSet::FeatureViewSync)` constraint
/// lives on the set itself in `configure_sandbox_sets`, NOT on this
/// `.chain()` — that way the ordering contract is testable via a
/// probe in the same set rather than re-typed on every call site.
fn install_visual_animation_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            // Spawn visual entities for encounter-spawned enemies
            // BEFORE sync_visuals reads positions for them.
            crate::rendering::spawn_dynamic_feature_visuals,
            sync_visuals,
            upgrade_enemy_sprites,
            upgrade_boss_sprites,
            animate_player,
            animate_characters,
            crate::rendering::animate_props,
            animate_bosses,
            // Pirate rider composite — reads ECS actor state and
            // spawns/despawns presentation entities each frame, so
            // it belongs in `PresentationVisualSync` (after
            // `FeatureViewSync`) alongside `sync_visuals` rather
            // than the projectile/VFX batch. Placing it here means
            // a room reset's actor despawn is observed the same
            // frame the rider visual disappears — no stale
            // rider-on-no-shark across resets/transitions.
            crate::rendering::sync_pirate_rider_visuals,
        )
            .chain()
            .in_set(SandboxSet::PresentationVisualSync)
            .after(crate::map_menu::handle_map_menu_hotkeys),
    );
}

fn install_camera_and_debug_overlay_systems(app: &mut App) {
    app.add_systems(
        Update,
        (camera_follow, debug_overlay::draw_debug_overlay)
            .chain()
            .after(animate_bosses),
    );
}

fn install_fx_and_hud_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            fx::update_particles,
            fx::update_impacts,
            fx::update_slash_previews,
            fx::update_speech_bubbles,
            windowing::window_mode_hotkeys,
        )
            .chain()
            .after(debug_overlay::draw_debug_overlay),
    )
    .add_systems(
        Update,
        (
            update_hud,
            dialog::sync_dialog_ui,
            crate::cutscene::sync_cutscene_ui,
        )
            .chain()
            .after(windowing::window_mode_hotkeys),
    );
}

/// Health overlays, portal sprite sync, parallax, dialog redirect,
/// lock-wall visuals, NPC sprite upgrade, map-menu pointer dismiss,
/// quest panel. Each system is its own `add_systems` call because the
/// big presentation tuple is already at Bevy's 20-system arity ceiling.
fn install_misc_visual_sync_systems(app: &mut App) {
    app.add_systems(
        Update,
        crate::rendering::sync_health_overlays.after(sync_visuals),
    )
    // Portal presentation: read PortalRegistry.phase + apply
    // visibility / animation row / ring-spin to the matching
    // FeatureName-tagged sprites + hide the redundant debug
    // door-zone visual for portal-mode LoadingZones. Visible-
    // only; headless has no FeatureName ↔ Bevy-entity binding
    // anyway. Runs after sync_visuals so the sprite entities
    // exist this frame.
    .add_systems(
        Update,
        (
            crate::rooms::sync_portal_sprite_visibility,
            crate::rooms::sync_portal_sprite_animation,
            crate::rooms::sync_portal_ring_rotation_system,
            crate::rooms::hide_portal_loading_zone_visuals,
        )
            .after(sync_visuals),
    )
    .add_systems(
        Update,
        crate::rendering::sync_parallax_layers.after(camera_follow),
    )
    // Quest-state-driven dialog redirect: flips the live dialog
    // branch the moment the underlying world state advances past
    // the conversation's prompt (e.g. mockingbird is now dead).
    // Must run after CoreSimulation (which is where dialog start
    // happens) and before `sync_dialog_ui` (which renders the
    // chosen branch) so the redirected mode is the one drawn.
    .add_systems(
        Update,
        dialog::redirect_post_quest_dialog
            .after(SandboxSet::CoreSimulation)
            .before(dialog::sync_dialog_ui),
    )
    // Encounter-driven LockWall visuals. Reconciles `LockWallVisual`
    // Bevy entities against `world.blocks` so the wall is visible
    // for the player when an encounter slams it shut. Must run
    // after `update_encounters_from_world` (which inserts /
    // removes the backing `lockwall:*` blocks) so we observe the
    // current frame's world state, not last frame's.
    .add_systems(
        Update,
        crate::rendering::sync_lock_wall_visuals
            .after(crate::encounter::update_encounters_from_world),
    )
    // NPC spritesheet upgrade. `.after(sync_visuals)` preserves the
    // ordering guarantee the chain otherwise provided (FeatureVisuals
    // must exist before we look them up).
    .add_systems(
        Update,
        crate::rendering::upgrade_npc_sprites.after(sync_visuals),
    )
    // Mouse / touch dismissal for the map menu.
    .add_systems(Update, crate::map_menu::map_menu_pointer_dismiss)
    // Quest panel runs alongside the verbose HUD.
    .add_systems(Update, update_quest_panel.after(dialog::sync_dialog_ui));
}

/// Player-bound visuals: morph-ball sprite + bubble-shield sprite.
/// Both follow the same pattern — build the texture once at startup,
/// spawn lazily once the player entity exists, sync visibility / tint
/// every frame after `sync_visuals` has mirrored the player transform.
fn install_player_visual_systems(app: &mut App) {
    app.add_systems(Startup, crate::body_mode::build_morph_ball_sprite)
        .add_systems(
            Update,
            (
                crate::body_mode::spawn_morph_ball_visual,
                crate::body_mode::sync_morph_ball_visual,
            )
                .chain()
                .after(sync_visuals),
        )
        // Bubble shield visual: similar pattern — toggle / tint every
        // frame from `PlayerBody.shielding` and `PlayerBody.parrying`.
        // Must run after `write_player_ecs_components` so `PlayerBody`
        // is current.
        .add_systems(Startup, crate::bubble_shield::build_bubble_shield_sprite)
        .add_systems(
            Update,
            (
                crate::bubble_shield::spawn_bubble_shield_visual,
                crate::bubble_shield::sync_bubble_shield_visual,
            )
                .chain()
                .after(sync_visuals),
        );
}

/// Projectile sprite ring + VFX/debris message subscribers + (input-
/// feature-gated) blink preview ring.
fn install_projectile_and_vfx_systems(app: &mut App) {
    // Player projectile visuals: rebuild the sprite ring each tick
    // from `PlayerProjectileState::bodies`. Must run after
    // `update_projectiles` so the body list reflects this frame's
    // spawn / tick / collision before the visuals are rebuilt —
    // otherwise newly-fired projectiles would only become visible
    // one frame late.
    app.add_systems(
        Update,
        (
            crate::projectile::sync_projectile_visuals.after(crate::projectile::update_projectiles),
            crate::enemy_projectile::sync_enemy_projectile_visuals
                .after(crate::enemy_projectile::update_enemy_projectiles),
        ),
    )
    // VFX + debris subscribe on the visible binary only. Audio's
    // subscriber lives in `add_audio_plugins` so the entire kira
    // chain stays behind the `audio` feature. Headless builds omit
    // these so the message queues drain without entity spawns or
    // audio playback.
    .add_systems(Update, vfx_spawn_messages.after(SandboxSet::CoreSimulation));
    // Live blink-destination preview ring. Reads leafwing action state to
    // know when the blink button is held, so it lives behind the `input`
    // feature alongside the other gameplay-input-driven presentation.
    #[cfg(feature = "input")]
    app.add_systems(
        Update,
        fx::update_blink_preview.after(SandboxSet::CoreSimulation),
    );
}

/// Install the egui inspector plugins. Gated by the `dev_tools` feature so
/// shipping/headless builds don't pay for `bevy-inspector-egui` /
/// `bevy_egui` in the dep graph. The inspector quick plugins require
/// EguiPlugin first; that's why both live behind the same gate.
#[cfg(feature = "dev_tools")]
pub(super) fn add_dev_tools_plugins(app: &mut App) {
    app.add_plugins(EguiPlugin::default())
        .add_plugins(
            ResourceInspectorPlugin::<DeveloperTools>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableAbilitySet>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditableMovementTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<EditablePlayerStats>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(
            ResourceInspectorPlugin::<SandboxFeelTuning>::default()
                .run_if(dev_tools::inspector_visible),
        )
        .add_plugins(WorldInspectorPlugin::new().run_if(dev_tools::world_inspector_visible));
}

#[cfg(not(feature = "dev_tools"))]
pub(super) fn add_dev_tools_plugins(_app: &mut App) {}

/// Install the Avian2D secondary-physics plugin and its presentation-side
/// debris subscriber. Gated by `physics_debris` so headless / minimal
/// builds drop `avian2d` from the dep graph entirely. Per ADR 0007, this
/// is secondary physics for debris/ragdoll visuals only — the player
/// controller stays kinematic.
#[cfg(feature = "physics_debris")]
pub(super) fn add_physics_debris_plugins(app: &mut App) {
    app.add_plugins(physics::AmbitionPhysicsPlugin).add_systems(
        Update,
        physics_spawn_debris_messages.after(SandboxSet::CoreSimulation),
    );
}

#[cfg(not(feature = "physics_debris"))]
pub(super) fn add_physics_debris_plugins(_app: &mut App) {}

/// Install UI-shell plugins: Yarn Spinner runtime and bevy_material_ui's
/// styling layer. The dialogue overlay (`dialog::sync_dialog_ui`) draws
/// with Bevy's core UI primitives and stays installed unconditionally;
/// only the optional plugins live behind `ui`.
#[cfg(feature = "ui")]
pub(super) fn add_ui_plugins(app: &mut App) {
    app.add_plugins(dialog::yarn_spinner_plugin())
        .add_plugins(MaterialUiPlugin);
}

#[cfg(not(feature = "ui"))]
pub(super) fn add_ui_plugins(_app: &mut App) {}

/// Install the leafwing-input-manager plugin, the player-input attach
/// startup system, and the bridge that keeps `Res<ControlFrame>` in sync
/// with leafwing's `ActionState`. Gated behind `input` so headless /
/// minimal builds can drop `leafwing-input-manager` from the dep graph;
/// the sim itself reads `Res<ControlFrame>` (always-available) and is
/// agnostic to where the frame came from.
#[cfg(feature = "input")]
pub(super) fn add_input_plugins(app: &mut App) {
    app.init_resource::<MenuInputState>()
        .init_resource::<MenuControlFrame>()
        .init_resource::<PlayerDashTriggerState>()
        .add_plugins(InputManagerPlugin::<SandboxAction>::default())
        .add_systems(
            Startup,
            attach_player_input_components.after(setup_simulation_system),
        )
        // Collect semantic menu intent before gameplay input is suppressed.
        // `populate_control_frame_from_actions` may zero the sim-side
        // `ControlFrame` in UI modes, but it must not mutate leafwing's
        // `ActionState`; held keyboard/menu buttons should not become
        // `just_pressed` again on every dialog frame.
        //
        // Therefore the order is:
        // 1. read keyboard/gamepad menu actions into `MenuControlFrame`,
        // 2. read/suppress gameplay into `ControlFrame`,
        // 3. let touch folds merge into both seams before the consumers below.
        //
        // Touch fold (mobile_input plugin) runs
        // `.after(populate_control_frame_from_actions)` for gameplay and
        // `.after(populate_menu_control_frame_from_actions)` for menus, then
        // `.before(pause_menu_toggle)`, so pause / inventory / navigation see
        // keyboard, gamepad, and touch contributions in one frame.
        .add_systems(
            Update,
            (
                populate_menu_control_frame_from_actions,
                populate_control_frame_from_actions,
                apply_menu_frame_to_cutscene_request,
                dialog::dialog_pointer_input,
                pause_menu::pause_menu_toggle,
                inventory::inventory_input,
                pause_menu::pause_menu_pointer_input,
                pause_menu::settings_slider_drag_input,
                pause_menu::settings_scrollbar_drag_input,
                inventory::inventory_pointer_input,
                pause_menu::pause_menu_navigate,
            )
                .chain()
                .before(SandboxSet::CoreSimulation),
        )
        .add_systems(
            Update,
            sync_preset_input_map.before(SandboxSet::CoreSimulation),
        );
}

#[cfg(not(feature = "input"))]
pub(super) fn add_input_plugins(_app: &mut App) {}

/// Register the mobile-touch input plugin (`virtual_joystick` sticks
/// + on-screen action buttons that fold into ControlFrame). Gated
/// behind the `mobile_touch` feature; on desktop builds without the
/// feature this is a no-op.
///
/// The mobile plugin runs ALONGSIDE the desktop input pipeline --
/// both write into the same `ControlFrame` resource, with the
/// mobile-side write happening after the desktop one in this
/// session's chain. On a phone, the desktop pipeline produces
/// neutral output (no keyboard / gamepad); on desktop, the mobile
/// stick UI is invisible without touch input, so neither path
/// stomps the other in practice. A future polish pass can detect
/// the active input source (touch vs keyboard) and skip the
/// inactive folder.
#[cfg(feature = "mobile_touch")]
pub(super) fn add_mobile_touch_plugin(app: &mut App) {
    app.add_plugins(crate::mobile_input::bevy_plugin::MobileTouchPlugin);
}

#[cfg(not(feature = "mobile_touch"))]
pub(super) fn add_mobile_touch_plugin(_app: &mut App) {}

/// Install the kira audio backend, channel resources, default music
/// startup, and the SFX subscriber. Gated by `audio` so headless / RL
/// builds drop `bevy_kira_audio` from the dep graph entirely. The sim
/// still emits `SfxMessage`s; without this plugin the message queue
/// just drains harmlessly per the ADR 0012 seam.
#[cfg(feature = "audio")]
pub(super) fn add_audio_plugins(app: &mut App) {
    app.add_plugins(KiraAudioPlugin)
        // Async SFX-bank loader for profiles whose bank is not picked
        // up by the sync fast path in `setup::try_load_sfx_bank_via_catalog`
        // (web HTTP fetch, plain desktop loose FS without env override).
        // Idempotent against the sync path; both insert the same
        // `SfxBankResource` and the second writer no-ops.
        .add_plugins(crate::audio::SfxBankAssetPlugin)
        // Browser AudioContext unlock telemetry. No-op on desktop
        // except the one-shot "audio unlocked" log; on wasm it also
        // emits the startup "audio locked until first gesture" line
        // so anyone watching devtools knows why audio is silent
        // before they click.
        .add_plugins(crate::audio::WebAudioUnlockPlugin)
        .init_resource::<crate::audio::RadioStationState>()
        .init_resource::<crate::audio::SfxBankHandleCache>()
        .init_resource::<AudioEnvironment>()
        .init_resource::<DefaultMusicStarted>()
        .add_audio_channel::<MusicChannel>()
        .add_audio_channel::<SfxChannel>()
        .add_audio_channel::<crate::music::MusicLayer0AChannel>()
        .add_audio_channel::<crate::music::MusicLayer1AChannel>()
        .add_audio_channel::<crate::music::MusicLayer2AChannel>()
        .add_audio_channel::<crate::music::MusicLayer3AChannel>()
        .add_audio_channel::<crate::music::MusicLayer4AChannel>()
        .add_audio_channel::<crate::music::MusicLayer5AChannel>()
        .add_audio_channel::<crate::music::MusicLayer0BChannel>()
        .add_audio_channel::<crate::music::MusicLayer1BChannel>()
        .add_audio_channel::<crate::music::MusicLayer2BChannel>()
        .add_audio_channel::<crate::music::MusicLayer3BChannel>()
        .add_audio_channel::<crate::music::MusicLayer4BChannel>()
        .add_audio_channel::<crate::music::MusicLayer5BChannel>()
        .add_systems(
            Startup,
            (
                crate::profiling::phase_mark("before_audio_init"),
                crate::music::load_music_cues,
                crate::profiling::phase_mark("after_audio_init"),
            )
                .chain()
                .after(setup_presentation_system),
        )
        // Deferred music start: polls each Update for (a) user
        // gesture observed (AudioUnlockState) and (b) the default
        // music track's asset handle finished loading. On wasm the
        // gesture gate is what prevents `play()` from no-op'ing
        // against a suspended AudioContext; on desktop the gate
        // flips during Startup so behavior matches the old direct-
        // startup system.
        .add_systems(Update, start_default_music_when_ready)
        .add_systems(
            Update,
            audio_play_sfx_messages.after(SandboxSet::CoreSimulation),
        )
        // Observe the player's WaterContact and request the matching
        // audio environment; the smoother ramps `wetness`, then
        // `apply_audio_environment` writes the combined user-mixer
        // × environment volume to Kira. Order: detect → smooth →
        // apply so a single frame fully propagates a state change.
        .add_systems(
            Update,
            (
                detect_audio_environment,
                smooth_audio_environment,
                apply_audio_environment,
            )
                .chain()
                .after(SandboxSet::CoreSimulation),
        )
        // Unified director: resolves room/encounter simple tracks and
        // adaptive cue states behind one music intent layer.
        .add_systems(
            Update,
            crate::music::drive_music_director.after(SandboxSet::CoreSimulation),
        );
}

#[cfg(not(feature = "audio"))]
pub(super) fn add_audio_plugins(_app: &mut App) {}

// ── Domain plugin structs ──────────────────────────────────────────────────
//
// These are the public Bevy `Plugin` API for callers that just want to
// `app.add_plugins(…)` without knowing about the internal helper functions.
// The helper functions (`init_sandbox_resources`, `add_simulation_plugins`,
// etc.) stay public so callers that need to inject resources between steps
// (e.g. inserting `StartRoomOverride` before resources are consumed) can
// still call them in sequence.

/// Installs all sandbox simulation resources and systems — the subset
/// that is safe for both visible and headless builds. Calls
/// `init_sandbox_resources` then `add_simulation_plugins`.
pub struct SandboxSimulationPlugin;

impl Plugin for SandboxSimulationPlugin {
    fn build(&self, app: &mut App) {
        init_sandbox_resources(app);
        add_simulation_plugins(app);
    }
}

/// Installs LDtk runtime spine registrations and `LdtkPlugin`. Visible
/// binary only — `LdtkPlugin` panics in headless (no `RenderApp`).
pub struct SandboxLdtkPlugin;

impl Plugin for SandboxLdtkPlugin {
    fn build(&self, app: &mut App) {
        add_ldtk_runtime_plugin(app);
    }
}

/// Installs all presentation-side plugins: input, audio, VFX, HUD, debug
/// overlays, and platform plugins. Visible binary only.
pub struct SandboxPresentationPlugin;

impl Plugin for SandboxPresentationPlugin {
    fn build(&self, app: &mut App) {
        add_presentation_plugins(app);
    }
}
