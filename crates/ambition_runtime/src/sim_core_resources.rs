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
//! authored content catalogs/registries (character catalog, music/sfx
//! registries, item roster), and the app startup chain
//! (`setup_simulation_system`). The content-free [`CharacterRoster`] default
//! below is only an explicit authority resource for Apps with no hostile
//! provider; provider registration replaces it transactionally.
//!
//! Ownership notes (anti-god rule 5): the dev-tools editables, `DialogState`/
//! `DialogueNodeIndex`, the encounter registries, and `MapMenuState` re-homed
//! to their domain plugins (`DevToolsSimPlugin`, `DialogSimStatePlugin`,
//! `EncounterRegistryPlugin`, `MapStatePlugin` — track 6, decision #9); this
//! bundle keeps only engine-owned sim vocabulary.

use bevy::prelude::*;

use ambition_actors::session::data;
use ambition_actors::ActorDiedMessage;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::{ExplosionRequest, FireworksRequest, VfxMessage};

/// Registers the engine sim messages and resource defaults (module docs).
/// Part of [`crate::PlatformerEnginePlugins`], right after the sets plugin.
pub struct SimCoreResourcesPlugin;

impl Plugin for SimCoreResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ambition_sfx::OwnedSfxMessage>()
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
            .add_message::<ambition_actors::avatar::PlayerHealRequested>()
            .add_message::<ambition_actors::rooms::RoomTransitionRequested>()
            // Staging fact: a room's contents finished spawning (JD4).
            .add_message::<ambition_actors::rooms::RoomLoaded>()
            // ADR 0010 — time-control vocabulary. Gameplay code writes
            // time-control messages instead of mutating time_scale directly.
            .add_message::<ambition_actors::time::time_control::ClockScaleRequest>()
            .add_message::<ambition_actors::time::time_control::ClockResetRequest>()
            .init_resource::<ambition_actors::time::time_control::RegimePolicy>()
            .init_resource::<ambition_actors::time::time_control::RequestedClockScale>()
            .init_resource::<ambition_time::ClockState>()
            .register_type::<ambition_platformer_primitives::schedule::GameMode>()
            .init_resource::<ambition_actors::trace::GameplayTraceBuffer>()
            .init_resource::<ambition_world::collision::MovingPlatformSet>()
            .init_resource::<ambition_actors::SandboxSimState>()
            // The session's movement-tuning authority. Engine-owned with a
            // neutral default so EVERY sim composition has one; content seeds
            // the authored values over it, and a developer build's inspector
            // edits reach it through `apply_editable_movement_tuning`. The
            // simulation never reads the dev-tools mirror.
            .init_resource::<ambition_engine_core::ActiveMovementTuning>()
            // Content-free default. Provider plugins replace/assemble this at
            // App build time; `init_resource` never clobbers their resource.
            .init_resource::<ambition_actors::features::CharacterRoster>()
            // The room-content staging seam: providers/content register pure
            // stagers into it; an app with none stages rooms as authored.
            .init_resource::<ambition_actors::features::RoomContentStagingRegistry>()
            // The construction recipe table (Phase 3). Installed with the
            // engine's own recipes below, and open for a provider to add its
            // own before the first room is planned.
            .init_resource::<ambition_actors::construction::ActorConstructionRegistry>()
            // App-local boss authority. Boss-free providers keep the explicit
            // empty resource; content plugins assemble provider fragments.
            .init_resource::<ambition_actors::boss_encounter::BossCatalog>()
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
            .init_resource::<ambition_actors::encounter::SwitchActivationQueue>()
            .init_resource::<ambition_actors::encounter::EncounterSwitchIndex>()
            // Victim-side hits staged in Combat, drained by the player resolver
            // NEXT frame — cross-frame combat truth, so a registered FIFO
            // rather than a message buffer (see `PendingPlayerHitEvents`).
            .init_resource::<ambition_combat::events::PendingPlayerHitEvents>()
            // Room and encounter music intent live as components on the exact
            // session-world root. Frontend routes therefore have no gameplay
            // music request authority to inherit or mutate.
            // Sandbox save (encounter defeat + switch state). Loaded from
            // disk by the presentation half only — headless/RL never touch
            // disk; mutated by encounter/switch systems.
            .init_resource::<ambition_persistence::save::SandboxSave>()
            // World-clock dt mirror — `WorldTime::scaled_dt` is the
            // bullet-time-respecting delta for gameplay timers.
            .init_resource::<ambition_time::WorldTime>()
            // The canonical timeline (N0.1): the index of the sim step now
            // running. Input streams and state hashes key on it.
            .init_resource::<ambition_time::SimTick>()
            // The per-tick input recorder (N0.2). Disarmed; a replay/RL/desync
            // driver arms it. Costs one resource read per tick while idle.
            .init_resource::<crate::InputStreamRecorder>()
            // Neutral runtime mirror of `WorldTime::sim_dt()`.
            .init_resource::<ambition_platformer_primitives::time::SimDt>()
            // Portal registry — per-portal lifecycle state machine.
            .init_resource::<ambition_actors::rooms::GatePortalRegistry>()
            .init_resource::<ambition_platformer_primitives::camera_ease::CameraEaseState>()
            .init_resource::<ambition_platformer_primitives::camera_ease::CameraEaseTuning>()
            .init_resource::<ambition_platformer_primitives::camera_ease::CameraShakeState>()
            .init_resource::<ambition_actors::session::reset::SandboxResetRequested>()
            // Track B: the rollback-registered slot a lifecycle op records into
            // under a rollback host, committed on a confirmed frame.
            .init_resource::<ambition_actors::session::lifecycle_commit::PendingLifecycleCommit>()
            // The always-available input seam (RL/headless writes it
            // programmatically; the windowed host's bridge fills it from
            // devices).
            .init_resource::<ambition_input::ControlFrame>()
            // Feel + physics tuning defaults; the game's assembly pre-inserts
            // its authored values (init never clobbers).
            .init_resource::<ambition_actors::time::feel::SandboxFeelTuning>()
            .init_resource::<ambition_actors::world::physics::PhysicsSandboxSettings>()
            // Engine-typed settings/inventory defaults; games pre-insert
            // their authored starters.
            .init_resource::<ambition_persistence::settings::UserSettings>()
            .init_resource::<ambition_actors::items::OwnedItems>()
            // The quest + boss-encounter registries are ENGINE vocabulary
            // read by the encounter/progression chains; content POPULATES
            // them (never owns the init).
            .init_resource::<ambition_persistence::quest::QuestRegistry>()
            .init_resource::<ambition_actors::boss_encounter::BossEncounterRegistry>();

        // The engine's own construction recipes. `init_resource` above never
        // clobbers a provider's pre-inserted registry, and registration is
        // idempotent, so composing this plugin twice is not an error.
        let mut recipes = app
            .world_mut()
            .resource_mut::<ambition_actors::construction::ActorConstructionRegistry>();
        ambition_actors::construction::install_actor_construction_recipes(&mut recipes)
            .expect("the engine's own construction recipes cannot conflict with each other");
    }
}
