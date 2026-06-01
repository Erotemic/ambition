//! Simulation-side resource and message installation, factored out of
//! `app/plugins.rs` as a Bevy [`Plugin`] so the registration code can
//! be tested in isolation and the orchestrator file stays focused on
//! schedule wiring.
//!
//! Owns the registrations every sandbox build (visible and headless)
//! needs:
//!
//! - the simulation messages (SFX/VFX/damage/heal/room-transition/
//!   gameplay-effect/clock-scale buffers)
//! - the long list of `insert_resource(default())` for sim state
//!   (`SandboxSimState`, encounter / boss / quest / cutscene /
//!   trace / banner / portal registries, projectile state, etc.)
//! - the LDtk data-asset Startup chain and the startup-profiler
//!   PostStartup report
//! - the `IntroPlugin` install (story content)
//!
//! [`SandboxSimulationResourcesPlugin`] is mounted by
//! [`super::add_simulation_plugins`] before the per-set
//! `register_*_systems` helpers so the resources / messages those
//! systems depend on already exist when they register.

use bevy::prelude::*;
use bevy_common_assets::ron::RonAssetPlugin;

use super::setup_systems::setup_simulation_system;
use crate::audio::SfxMessage;
use crate::content::data;
use crate::game_mode::GameMode;
use crate::presentation::fx::{ExplosionRequest, FireworksRequest, VfxMessage};
use crate::world::physics::DebrisBurstMessage;
use crate::PlayerDiedMessage;

pub struct SandboxSimulationResourcesPlugin;

impl Plugin for SandboxSimulationResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SfxMessage>()
            .add_message::<VfxMessage>()
            .add_message::<ExplosionRequest>()
            .add_message::<FireworksRequest>()
            .add_message::<DebrisBurstMessage>()
            .add_message::<PlayerDiedMessage>()
            .add_message::<crate::features::GameplayEffect>()
            .add_message::<crate::features::HitEvent>()
            .add_message::<crate::features::ResetRoomFeaturesEvent>()
            .add_message::<crate::boss_encounter::CutRopeRoomReplayRequested>()
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
            .add_message::<crate::time::time_control::ClockScaleRequest>()
            .insert_resource(crate::time::time_control::RegimePolicy::default())
            .insert_resource(crate::time::time_control::RequestedClockScale::default())
            .register_type::<GameMode>()
            // StartupProfiler captures wall-clock at each marked phase so a
            // PostStartup report prints "where did the first frame's
            // worth of init time go" without needing an external profiler
            // attached. See `crate::profiling` for the helper API and
            // `docs/recipes/profiling.md` for Tracy / per-system profiling.
            .insert_resource(crate::dev::profiling::StartupProfiler::default())
            .insert_resource(crate::trace::GameplayTraceBuffer::default())
            .insert_resource(crate::dialog::DialogState::default())
            .insert_resource(crate::MovingPlatformSet::default())
            .insert_resource(crate::SandboxSimState::default())
            .insert_resource(crate::SandboxDevState::default())
            .insert_resource(crate::features::GameplayBanner::default())
            .insert_resource(crate::boss_encounter::CutRopeBossArenaState::default())
            .insert_resource(crate::boss_encounter::CutRopeHeavyObjectCycle::default())
            .insert_resource(crate::boss_encounter::PendingCutRopeRoomReplay::default())
            .insert_resource(crate::features::FeatureEcsWorldOverlay::default())
            .insert_resource(crate::features::FeatureViewIndex::default())
            .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
            // CharacterCatalogPlugin installs the parsed character
            // catalog as a Bevy resource and runs a Startup validator
            // that panics on broken references. See
            // `crate::content::character_catalog` and ADR 0017
            // (Rust = behavior, RON = content, LDtk = space).
            .add_plugins(crate::content::character_catalog::CharacterCatalogPlugin)
            .add_systems(
                Startup,
                (
                    crate::dev::profiling::phase_mark("startup_begin"),
                    data::load_data_asset_handle,
                    crate::dev::profiling::phase_mark("after_load_data_handle"),
                    setup_simulation_system,
                    crate::dev::profiling::phase_mark("after_setup_simulation"),
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
                    crate::dev::profiling::phase_mark("post_startup_begin"),
                    crate::dev::profiling::report_startup_phases,
                )
                    .chain(),
            )
            // Player projectile state is per-player and lives on each
            // player entity (attached via `PlayerSimulationBundle`).
            // No global resource registration needed.
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
            // Boss music routes through its own resource so the
            // regular encounter tick (which writes `desired_track =
            // None` every frame there's no in-flight encounter)
            // can't clobber the boss's MusicRequested events. The
            // audio backend reads both, boss wins.
            .insert_resource(crate::encounter::BossEncounterMusicRequest::default())
            .insert_resource(crate::rooms::RoomMusicRequest::default())
            // Sandbox save game (encounter defeat + switch state).
            // Loaded from disk by `load_save_at_startup` in the
            // presentation half so headless / RL drivers don't touch
            // disk; mutated by encounter + switch systems; written by
            // `autosave_sandbox_save` when change-detection fires.
            .insert_resource(crate::persistence::save::SandboxSave::default())
            // Quest + cutscene systems. Both are sim-side state machines
            // that read/write the save resource and surface HUD lines via
            // the encounter overlay.
            .insert_resource(crate::content::quest::QuestRegistry::default())
            .insert_resource(crate::presentation::cutscene::default_cutscene_library())
            .insert_resource(crate::presentation::cutscene::ActiveCutscene::default())
            .insert_resource(crate::presentation::cutscene::CutsceneTriggerQueue::default())
            .insert_resource(crate::presentation::cutscene::CutsceneAdvanceRequest::default())
            .insert_resource(crate::presentation::cutscene::RoomCutsceneBindings::defaults())
            // Combat-banter registry — story-content lines for the
            // `apply_feature_hit_events` hit handler. Boss barks are
            // installed inline; IntroPlugin adds the intro raiders' lines
            // via a startup system.
            .insert_resource({
                let mut reg = crate::content::banter::CombatBanterRegistry::default();
                crate::boss_encounter::install_boss_banter(&mut reg);
                crate::content::banter::install_pirate_banter(&mut reg);
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
            .insert_resource(crate::time::camera_ease::CameraShakeState::default())
            .insert_resource(crate::presentation::rendering::CameraViewState::default())
            .insert_resource(crate::runtime::reset::SandboxResetRequested::default());
    }
}
