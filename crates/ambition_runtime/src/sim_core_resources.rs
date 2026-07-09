//! The engine-generic simulation messages + resource defaults (E5 step 6).
//!
//! Moved from the app's `SandboxSimulationResourcesPlugin` so a demo app gets
//! a bootable sim from the engine group alone (THE DEMO GATE). Everything here
//! is `init_resource`/default semantics — a host overrides any of these by
//! `insert_resource` BEFORE adding the group (init never clobbers), which is
//! the documented host-override convention (`SandboxSetsPlugin` docs).
//!
//! What the engine group deliberately does NOT provide (the game/fixture
//! must): the INSTALLED WORLD state (`RoomSet`, `RoomGeometry`,
//! `ActiveRoomMetadata` — which world is loaded is the game's choice), the
//! content catalogs/registries (character catalog, music/sfx registries,
//! item roster), and the app startup chain (`setup_simulation_system`).
//!
//! Ownership notes (anti-god rule 5): several defaults here belong to
//! domains whose plugins haven't been carved yet — the dev-tools editables
//! (E1d), `MapMenuState` (E1e), `DialogState` (E1c). They are initialized
//! here so the group is self-sufficient TODAY and re-home with their carves.

use bevy::prelude::*;

use ambition_actors::session::data;
use ambition_actors::ActorDiedMessage;
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::{ExplosionRequest, FireworksRequest, VfxMessage};

/// Registers the engine sim messages and resource defaults (module docs).
/// Part of [`crate::PlatformerEnginePlugins`], right after the sets plugin.
pub struct SimCoreResourcesPlugin;

impl Plugin for SimCoreResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SfxMessage>()
            .add_message::<VfxMessage>()
            .add_message::<ambition_projectiles::SpawnProjectile>()
            .add_message::<ExplosionRequest>()
            .add_message::<FireworksRequest>()
            .add_message::<DebrisBurstMessage>()
            .add_message::<ActorDiedMessage>()
            .add_message::<ambition_combat::SetFlagRequested>()
            .add_message::<ambition_actors::features::QuestAdvanceRequested>()
            .add_message::<ambition_actors::features::SwitchActivated>()
            .add_message::<ambition_combat::GameplaySfxRequested>()
            .add_message::<ambition_combat::HitEvent>()
            .add_message::<ambition_actors::features::ActorStimulus>()
            .add_message::<ambition_combat::ResetRoomFeaturesEvent>()
            .add_message::<ambition_combat::GameplayBannerRequested>()
            .add_message::<ambition_actors::player::PlayerHealRequested>()
            .add_message::<ambition_actors::rooms::RoomTransitionRequested>()
            // Staging fact: a room's contents finished spawning (JD4).
            .add_message::<ambition_actors::rooms::RoomLoaded>()
            // ADR 0010 — time-control vocabulary. Gameplay code writes
            // ClockScaleRequest instead of mutating time_scale directly.
            .add_message::<ambition_actors::time::time_control::ClockScaleRequest>()
            .init_resource::<ambition_actors::time::time_control::RegimePolicy>()
            .init_resource::<ambition_actors::time::time_control::RequestedClockScale>()
            .init_resource::<ambition_time::ClockState>()
            .register_type::<ambition_platformer_primitives::schedule::GameMode>()
            // Startup wall-clock profiler (the PostStartup report is the
            // app's; the resource is engine so phase_mark works anywhere).
            .init_resource::<ambition_dev_tools::profiling::StartupProfiler>()
            .init_resource::<ambition_actors::trace::GameplayTraceBuffer>()
            .init_resource::<ambition_dialog::DialogState>()
            .init_resource::<ambition_actors::MovingPlatformSet>()
            .init_resource::<ambition_actors::SandboxSimState>()
            .init_resource::<ambition_dev_tools::SandboxDevState>()
            .init_resource::<ambition_combat::GameplayBanner>()
            .init_resource::<ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay>()
            .init_resource::<ambition_sim_view::FeatureViewIndex>()
            .init_resource::<ambition_sim_view::ActorRenderIndex>()
            .init_resource::<ambition_sim_view::BossRenderIndex>()
            // Session data-spec RON loader (the engine's own asset format).
            .add_plugins(bevy_common_assets::ron::RonAssetPlugin::<
                data::SandboxDataSpec,
            >::new(&["ron"]))
            // In-flight player projectiles are ECS entities; their monotonic
            // spawn-id source is this global counter.
            .init_resource::<ambition_projectiles::ProjectileSeqCounter>()
            // Enemy projectiles (pirate volleys etc) — separate from player
            // projectiles so faction routing stays explicit.
            .init_resource::<ambition_projectiles::enemy::EnemyProjectileState>()
            // Anti-clump attack slot arbitration.
            .init_resource::<ambition_actors::combat::slots::CombatSlotsRes>()
            // Encounter system: the live multi-encounter store is
            // `EncounterRegistry`, populated from the installed world.
            .init_resource::<ambition_encounter::EncounterState>()
            .init_resource::<ambition_encounter::EncounterRegistry>()
            .init_resource::<ambition_actors::encounter::SwitchActivationQueue>()
            .init_resource::<ambition_actors::encounter::EncounterSwitchIndex>()
            .init_resource::<ambition_encounter::EncounterMusicRequest>()
            // Boss music routes through its own resource so the regular
            // encounter tick can't clobber the boss's MusicRequested events.
            .init_resource::<ambition_encounter::BossEncounterMusicRequest>()
            .init_resource::<ambition_actors::rooms::RoomMusicRequest>()
            // Sandbox save (encounter defeat + switch state). Loaded from
            // disk by the presentation half only — headless/RL never touch
            // disk; mutated by encounter/switch systems.
            .init_resource::<ambition_persistence::save::SandboxSave>()
            // World-clock dt mirror — `WorldTime::scaled_dt` is the
            // bullet-time-respecting delta for gameplay timers.
            .init_resource::<ambition_time::WorldTime>()
            // Neutral runtime mirror of `WorldTime::sim_dt()`.
            .init_resource::<ambition_platformer_primitives::time::SimDt>()
            // Portal registry — per-portal lifecycle state machine.
            .init_resource::<ambition_actors::rooms::GatePortalRegistry>()
            .init_resource::<ambition_actors::menu::map::MapMenuState>()
            .init_resource::<ambition_platformer_primitives::camera_ease::CameraEaseState>()
            .init_resource::<ambition_platformer_primitives::camera_ease::CameraEaseTuning>()
            .init_resource::<ambition_platformer_primitives::camera_ease::CameraShakeState>()
            .init_resource::<ambition_actors::session::reset::SandboxResetRequested>()
            // The always-available input seam (RL/headless writes it
            // programmatically; the windowed host's bridge fills it from
            // devices).
            .init_resource::<ambition_input::ControlFrame>()
            // Feel + physics tuning defaults; the game's assembly pre-inserts
            // its authored values (init never clobbers).
            .init_resource::<ambition_actors::time::feel::SandboxFeelTuning>()
            .init_resource::<ambition_actors::world::physics::PhysicsSandboxSettings>()
            // Dev-editable tuning mirrors (read by the dev-edit sync in the
            // player frame). Ownership moves to the dev-tools carve (E1d).
            .init_resource::<ambition_dev_tools::dev_tools::DeveloperTools>()
            .init_resource::<ambition_dev_tools::dev_tools::EditablePlayerStats>()
            .init_resource::<ambition_dev_tools::dev_tools::EditableMovementTuning>()
            .init_resource::<ambition_dev_tools::dev_tools::EditableAbilitySet>()
            // Engine-typed settings/inventory defaults; games pre-insert
            // their authored starters.
            .init_resource::<ambition_persistence::settings::UserSettings>()
            .init_resource::<ambition_actors::items::OwnedItems>()
            // The quest + boss-encounter registries are ENGINE vocabulary
            // read by the encounter/progression chains; content POPULATES
            // them (never owns the init).
            .init_resource::<ambition_persistence::quest::QuestRegistry>()
            .init_resource::<ambition_actors::boss_encounter::BossEncounterRegistry>();
    }
}
