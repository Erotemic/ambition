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
use ambition_render::fx::{ExplosionRequest, FireworksRequest, VfxMessage};
use ambition_sandbox::audio::SfxMessage;
use ambition_sandbox::game_mode::GameMode;
use ambition_sandbox::runtime::data;
use ambition_sandbox::world::physics::DebrisBurstMessage;
use ambition_sandbox::PlayerDiedMessage;

pub struct SandboxSimulationResourcesPlugin;

impl Plugin for SandboxSimulationResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SfxMessage>()
            .add_message::<VfxMessage>()
            .add_message::<ambition_sandbox::projectile::SpawnProjectile>()
            .add_message::<ExplosionRequest>()
            .add_message::<FireworksRequest>()
            .add_message::<DebrisBurstMessage>()
            .add_message::<PlayerDiedMessage>()
            .add_message::<ambition_sandbox::features::SetFlagRequested>()
            .add_message::<ambition_sandbox::features::QuestAdvanceRequested>()
            .add_message::<ambition_sandbox::features::SwitchActivated>()
            .add_message::<ambition_sandbox::features::GameplaySfxRequested>()
            .add_message::<ambition_sandbox::features::HitEvent>()
            .add_message::<ambition_sandbox::features::ActorStimulus>()
            .add_message::<ambition_sandbox::features::ResetRoomFeaturesEvent>()
            .add_message::<ambition_content::bosses::CutRopeRoomReplayRequested>()
            .add_message::<ambition_sandbox::features::GameplayBannerRequested>()
            .add_message::<ambition_sandbox::player::PlayerHealRequested>()
            .add_message::<ambition_sandbox::rooms::RoomTransitionRequested>()
            // ADR 0010 — time-control vocabulary. Gameplay code writes
            // ClockScaleRequest instead of mutating SandboxSimState::
            // time_scale directly; apply_clock_scale_requests consults
            // RegimePolicy (default: Solo, grant-all), records the
            // granted target in RequestedClockScale, and
            // smooth_sim_clock_toward_target_system ramps the live
            // time_scale toward the target at feel-tuned rates. See
            // `ambition_sandbox::time_control` for the policy + dispatch and ADR
            // 0010 §Vocabulary for the model.
            .add_message::<ambition_sandbox::time::time_control::ClockScaleRequest>()
            .insert_resource(ambition_sandbox::time::time_control::RegimePolicy::default())
            .insert_resource(ambition_sandbox::time::time_control::RequestedClockScale::default())
            .insert_resource(ambition_sandbox::time::clock_state::ClockState::default())
            .register_type::<GameMode>()
            // StartupProfiler captures wall-clock at each marked phase so a
            // PostStartup report prints "where did the first frame's
            // worth of init time go" without needing an external profiler
            // attached. See `ambition_sandbox::profiling` for the helper API and
            // `docs/recipes/profiling.md` for Tracy / per-system profiling.
            .insert_resource(ambition_sandbox::dev::profiling::StartupProfiler::default())
            .insert_resource(ambition_sandbox::trace::GameplayTraceBuffer::default())
            .insert_resource(ambition_sandbox::dialog::DialogState::default())
            .insert_resource(ambition_sandbox::MovingPlatformSet::default())
            .insert_resource(ambition_sandbox::SandboxSimState::default())
            .insert_resource(ambition_sandbox::SandboxDevState::default())
            .insert_resource(ambition_sandbox::features::GameplayBanner::default())
            .insert_resource(ambition_content::bosses::CutRopeBossArenaState::default())
            .insert_resource(ambition_content::bosses::CutRopeHeavyObjectCycle::default())
            .insert_resource(ambition_content::bosses::PendingCutRopeRoomReplay::default())
            .insert_resource(ambition_sandbox::features::FeatureEcsWorldOverlay::default())
            .insert_resource(ambition_sandbox::features::FeatureViewIndex::default())
            .add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]))
            // CharacterCatalogPlugin installs the parsed character
            // catalog as a Bevy resource and runs a Startup validator
            // that panics on broken references. See
            // `ambition_sandbox::actor::character_catalog` and ADR 0017
            // (Rust = behavior, RON = content, LDtk = space).
            .add_plugins(ambition_sandbox::character_roster::character_roster_plugin())
            .add_systems(
                Startup,
                (
                    ambition_sandbox::dev::profiling::phase_mark("startup_begin"),
                    data::load_data_asset_handle,
                    ambition_sandbox::dev::profiling::phase_mark("after_load_data_handle"),
                    setup_simulation_system,
                    ambition_sandbox::dev::profiling::phase_mark("after_setup_simulation"),
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
                    ambition_sandbox::dev::profiling::phase_mark("post_startup_begin"),
                    ambition_sandbox::dev::profiling::report_startup_phases,
                )
                    .chain(),
            )
            // Player projectile CONTROLLER state is per-player and lives
            // on each player entity (attached via `PlayerSimulationBundle`).
            // In-flight player projectiles are ECS entities (Phase 3c-ii);
            // their monotonic spawn-id source is this global counter.
            .init_resource::<ambition_sandbox::projectile::ProjectileSeqCounter>()
            // Enemy projectiles (pirate volleys etc) — separate from
            // player projectiles so faction routing stays explicit.
            .insert_resource(ambition_sandbox::enemy_projectile::EnemyProjectileState::default())
            // Anti-clump attack slot arbitration. Default layout: 3
            // melee ring slots + 3 aerial arc slots around the player.
            .insert_resource(ambition_sandbox::combat::slots::CombatSlotsRes::default())
            // Encounter system. The legacy single-encounter `EncounterState`
            // resource stays for backwards-compat tests; the live
            // multi-encounter store is `EncounterRegistry`, populated
            // from LDtk by `populate_encounter_registry`.
            .insert_resource(ambition_sandbox::encounter::EncounterState::default())
            .insert_resource(ambition_sandbox::encounter::EncounterRegistry::default())
            .insert_resource(ambition_sandbox::encounter::SwitchActivationQueue::default())
            .insert_resource(ambition_sandbox::encounter::EncounterSwitchIndex::default())
            .insert_resource(ambition_sandbox::encounter::EncounterMusicRequest::default())
            // Boss music routes through its own resource so the
            // regular encounter tick (which writes `desired_track =
            // None` every frame there's no in-flight encounter)
            // can't clobber the boss's MusicRequested events. The
            // audio backend reads both, boss wins.
            .insert_resource(ambition_sandbox::encounter::BossEncounterMusicRequest::default())
            .insert_resource(ambition_sandbox::rooms::RoomMusicRequest::default())
            // Sandbox save game (encounter defeat + switch state).
            // Loaded from disk by `load_save_at_startup` in the
            // presentation half so headless / RL drivers don't touch
            // disk; mutated by encounter + switch systems; written by
            // `autosave_sandbox_save` when change-detection fires.
            .insert_resource(ambition_sandbox::persistence::save::SandboxSave::default())
            // Quest registry, the named cutscene library + room bindings,
            // and the combat-banter registry are registered by
            // `ambition_content::AmbitionContentPlugin` (Stage 11 /
            // Task J). The runtime cutscene state channels (ActiveCutscene /
            // trigger / advance) move with them since they are part of the
            // cutscene content seam.
            // World-clock dt mirror — `WorldTime::scaled_dt` is the
            // bullet-time-respecting delta for gameplay timers and
            // world-anchored animation timers. `WorldTime::raw_dt`
            // stays the wall-clock dt for UI / debug. Refreshed each
            // frame by `refresh_world_time` registered below; new code
            // should reach for `Res<WorldTime>::scaled_dt` instead of
            // `Res<Time>::delta_secs()` for anything that should slow
            // / freeze when the world slows / freezes.
            .insert_resource(ambition_sandbox::WorldTime::default())
            // Neutral runtime mirror of `WorldTime::sim_dt()` — the
            // platformer-runtime crate's generic systems read scaled dt
            // through this sandbox-free resource. Filled each frame by
            // `mirror_sim_dt_into_runtime` right after `refresh_world_time`.
            .insert_resource(ambition_platformer_primitives::time::SimDt::default())
            // Portal registry — per-portal lifecycle state machine
            // (Off / Opening / On / Closing). The portal itself owns
            // traversal readiness; the switch only commands the
            // boot/shutdown sequence. `detect_room_transition_system`
            // blocks the transition unless the named portal's phase is
            // `On`. `tick_portal_phases_system` advances phase from the
            // switch state each frame. Empty by default; IntroPlugin
            // registers the intro_portal_zone → intro_portal_switch
            // hookup.
            .insert_resource(ambition_sandbox::rooms::GatePortalRegistry::default())
            // The intro/cut-rope story hooks (IntroPlugin) and the boss
            // encounter registry are registered by
            // `ambition_content::AmbitionContentPlugin` (Stage 11 /
            // Task J).
            .insert_resource(ambition_sandbox::menu::map::MapMenuState::default())
            .insert_resource(ambition_sandbox::CameraEaseState::default())
            .insert_resource(ambition_sandbox::CameraEaseTuning::default())
            .insert_resource(ambition_sandbox::time::camera_ease::CameraShakeState::default())
            .insert_resource(ambition_render::rendering::CameraViewState::default())
            .insert_resource(ambition_sandbox::runtime::reset::SandboxResetRequested::default());
    }
}
