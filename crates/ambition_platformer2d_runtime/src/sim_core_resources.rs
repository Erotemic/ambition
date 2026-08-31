//! The engine-generic simulation messages + resource defaults (E5 step 6).
//!
//! Everything here is `init_resource`/default semantics — a host overrides any of these by
//! `insert_resource` BEFORE adding the group (init never clobbers), which is the documented
//! host-override convention (`Platformer2dSimulationFoundationPlugin` docs).
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

use ambition_combat::death_rules::ActorDiedMessage;
use ambition_platformer2d_actor_monolith::session::data;
use ambition_vfx::vfx::DebrisBurstMessage;
use ambition_vfx::{FireworksRequest, FxRequest, VfxMessage};

/// Registers the engine sim messages and resource defaults (module docs).
/// Part of [`crate::PlatformerEnginePlugins`], right after the sets plugin.
pub struct SimCoreResourcesPlugin;

impl Plugin for SimCoreResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ambition_sfx::OwnedSfxMessage>()
            .add_message::<VfxMessage>()
            .add_message::<ambition_projectiles::ProjectileSpawnRequest>()
            .add_message::<FxRequest>()
            .add_message::<FireworksRequest>()
            .add_message::<DebrisBurstMessage>()
            .add_message::<ambition_vfx::vfx::KnockoutBeatRequested>()
            .add_message::<ActorDiedMessage>()
            .add_message::<ambition_damage::WalletShieldSpent>()
            .add_message::<ambition_combat::SetFlagRequested>()
            .add_message::<ambition_persistence::quest::QuestAdvanceRequested>()
            .add_message::<ambition_encounter::switches::SwitchActivated>()
            .add_message::<ambition_combat::GameplaySfxRequested>()
            .add_message::<ambition_combat::HitEvent>()
            // ⭐⭐ AND THE HIT'S RESULT, beside the hit itself. `HitEvent` and
            // `LandedBodyHit` are geometry; this is what the resolver DECIDED,
            // and the match freeze reads it. ⛔ REGISTERED HERE and not in a
            // schedule plugin: its writers are on the PLAYER road and the ACTOR
            // road, which different compositions install separately — a message
            // registered beside one of them panics the other, which is a crash
            // this repository has already shipped once.
            .add_message::<ambition_combat::hitbox::ResolvedBodyHit>()
            .add_message::<ambition_combat::hitbox::BlockedBodyHit>()
            // S4: the stocks loop. A KO of a body whose death a RULESET owns,
            // and the count that was spent for it.
            .add_message::<ambition_combat::stocks::BodyKnockedOut>()
            .add_message::<ambition_combat::stocks::FighterStockSpent>()
            .add_message::<ambition_combat::stocks::StocksMatchDecided>()
            // …and the third way one ends: somebody stopped it. Registered
            // beside the other two so `decide_stocks_match`'s reader can exist
            // in every composition that installs the loop.
            // Two attacks met and both were refused. Written by the arbitration
            // that runs ahead of the damage sweep; read by whatever a ruleset
            // wants a clank to cost (the rebound is not built yet).
            .add_message::<ambition_combat::clank::AttacksClanked>()
            .add_message::<ambition_combat::events::ActorStimulus>()
            .add_message::<ambition_combat::RoomReplayAdmitted>()
            .add_message::<ambition_combat::GameplayBannerRequested>()
            .add_message::<ambition_platformer2d_actor_monolith::avatar::PlayerHealRequested>()
            // Staging fact: a room's contents finished spawning (JD4).
            .add_message::<ambition_platformer2d_world::rooms::RoomLoaded>()
            // ADR 0010 — time-control vocabulary. Gameplay code writes
            // time-control messages instead of mutating time_scale directly.
            .add_message::<ambition_time::time_control::ClockScaleRequest>()
            .add_message::<ambition_time::time_control::ClockResetRequest>()
            .init_resource::<ambition_time::time_control::RegimePolicy>()
            .init_resource::<ambition_time::time_control::RequestedClockScale>()
            .init_resource::<ambition_time::ClockState>()
            .register_type::<ambition_platformer2d_shared_tangle::schedule::GameMode>()
            .init_resource::<ambition_gameplay_trace::GameplayTraceBuffer>()
            .init_resource::<ambition_platformer2d_world::collision::MovingPlatformSet>()
            .init_resource::<ambition_platformer2d_shared_tangle::safe_position::RoomTransitionCooldown>()
            // The session's movement-tuning authority. Engine-owned with a
            // neutral default so EVERY sim composition has one; content seeds
            // the authored values over it, and a developer build's inspector
            // edits reach it through `apply_editable_movement_tuning`. The
            // simulation never reads the dev-tools mirror.
            .init_resource::<ambition_platformer2d_core::ActiveMovementTuning>()
            // The room-content staging seam: providers/content register pure
            // stagers into it; an app with none stages rooms as authored.
            .init_resource::<ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry>()
            // The closed actor construction table, plus a descriptor-only
            // catalog that fingerprints every independently typed construction
            // domain installed by the composition.
            .init_resource::<ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry>()
            .init_resource::<ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog>()
            // App-local boss authority. Boss-free providers keep the explicit
            // empty resource; content plugins assemble provider fragments.
            .init_resource::<ambition_boss_encounter::BossCatalog>()
            .init_resource::<ambition_combat::GameplayBanner>()
            .init_resource::<ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay>()
            // A struck block flinches. Registered beside the world overlay because
            // it is the same kind of fact — something happened to authored
            // geometry — and because BOTH halves need the channel to exist: the
            // game emits it and the renderer consumes it, so a composition with
            // only one of them must still boot.
            .add_message::<ambition_platformer2d_shared_tangle::block_nudge::BlockStruck>()
            .init_resource::<ambition_sim_view::FeatureViewIndex>()
            .init_resource::<ambition_sim_view::ActorRenderIndex>()
            .init_resource::<ambition_sim_view::BossRenderIndex>()
            // Session data-spec RON loader (the engine's own asset format).
            .add_plugins(bevy_common_assets::ron::RonAssetPlugin::<
                data::Platformer2dGameplayDefaults,
            >::new(&["ron"]))
            // Every in-flight projectile is an ECS entity; one monotonic
            // spawn-id source orders the unified live-projectile population.
            .init_resource::<ambition_projectiles::ProjectileSeqCounter>()
            .init_resource::<ambition_encounter::switches::SwitchActivationQueue>()
            .init_resource::<ambition_encounter::switches::EncounterSwitchIndex>()
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
            .init_resource::<ambition_persistence::save::AmbitionGameSave>()
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
            .init_resource::<ambition_platformer2d_shared_tangle::time::SimDt>()
            // Portal registry — the AUTHORED per-portal configuration.
            .init_resource::<ambition_platformer2d_world::rooms::GatePortalRegistry>()
            // …and the live phase it drives, which is rollback state (registered
            // as `resource.gate_portal_phases`). Two resources because only one
            // of them rewinds.
            .init_resource::<ambition_platformer2d_world::rooms::GatePortalPhases>()
            // `CameraEaseState` is NOT here any more: it is per-VIEW state and
            // lives on the local view entity, spawned by `CameraObservationPlugin`.
            // The tuning below stays global — it is authored feel, one game-wide
            // answer, not something a second observer would disagree about.
            .init_resource::<ambition_platformer2d_shared_tangle::camera_ease::CameraEaseTuning>()
            .init_resource::<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeTuning>()
            .init_resource::<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeState>()
            .init_resource::<ambition_platformer2d_actor_monolith::session::reset::NewGameResetRequested>()
            // Track B: the rollback-registered slot a lifecycle op records into
            // under a rollback host, committed on a confirmed frame.
            .init_resource::<ambition_platformer2d_actor_monolith::session::lifecycle_commit::PendingLifecycleCommit>()
            // The always-available input seam (RL/headless writes it
            // programmatically; the windowed host's bridge fills it from
            // devices).
            .init_resource::<ambition_input::ControlFrame>()
            // Feel + physics tuning defaults; the game's assembly pre-inserts
            // its authored values (init never clobbers).
            .init_resource::<ambition_combat::feel::Platformer2dFeelTuningMonolith>()
            .init_resource::<ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings>()
            // Engine-typed settings/inventory defaults; games pre-insert
            // their authored starters.
            .init_resource::<ambition_persistence::settings::UserSettings>()
            .init_resource::<ambition_items::OwnedItems>()
            // The quest + boss-encounter registries are ENGINE vocabulary
            // read by the encounter/progression chains; content POPULATES
            // them (never owns the init).
            .init_resource::<ambition_persistence::quest::QuestRegistry>()
            .init_resource::<ambition_boss_encounter::BossEncounterRegistry>();

        // ── The world-state log ───────────────────────────────────────────
        //
        // `[game-mode]` + `[sim-clock]`: the two globals a "frozen game" report
        // has to distinguish between, stamped with the FRAME they changed on so
        // a deferred `NextState` transition can be ordered against the systems
        // that read the mode. Registered here, in the engine group, rather than
        // app-side beside `[frame-census]` — an Android freeze and a headless
        // repro must produce the same log, and only the engine group is common
        // to both.
        ambition_platformer2d_shared_tangle::world_log::install(app);
        app.add_systems(
            PostUpdate,
            ambition_platformer2d_actor_monolith::time::time_control::report_sim_clock_changes,
        );

        // The presentation half of the camera shake (P0.1). The simulation
        // publishes a `CameraShakeRequest`; this is the only thing that turns one
        // into a screen the player sees, which is what lets the confirmed-frame
        // quarantine hold a predicted hit's shake back until the host settles the
        // frame that produced it.
        //
        // in the ENGINE group, not the windowed host. `tick_camera_shake`
        // and `camera_follow` are the host's because a headless run has no camera
        // to move — but a headless run still ASKS for shakes and still has the
        // amplitude read off it (`app_it::hit_shakes_the_camera` watches a duel
        // through the sim harness). Registering the applier beside the resource
        // it writes keeps the seam wired in every composition that owns the
        // state, which is the same reason `shake_camera_on_landed_hits` lives in
        // the engine combat schedule rather than in `ambition_app`.
        app.add_message::<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>()
            .add_systems(
                Update,
                ambition_platformer2d_shared_tangle::camera_ease::apply_camera_shake_requests
                    .before(ambition_platformer2d_shared_tangle::camera_ease::tick_camera_shake),
            );

        // The engine's closed actor construction recipes. `init_resource` above
        // does not clobber a pre-inserted fixture/host registry, and registration
        // is idempotent, so composing this plugin twice is harmless. Optional
        // capabilities do not extend this table: they own typed construction
        // domains/lanes and contribute descriptor-only schema dumps separately.
        let actor_construction_dump = {
            let mut recipes = app
                .world_mut()
                .resource_mut::<ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry>();
            ambition_platformer2d_actor_monolith::construction::install_actor_construction_recipes(
                &mut recipes,
            )
            .expect("the engine's own construction recipes cannot conflict with each other");
            recipes.deterministic_dump()
        };
        app.world_mut()
            .resource_mut::<ambition_platformer2d_shared_tangle::construction::ConstructionSchemaCatalog>()
            .try_contribute(
                ambition_platformer2d_actor_monolith::construction::ACTOR_CONSTRUCTION_DOMAIN,
                actor_construction_dump,
            )
            .expect("the actor construction schema cannot conflict with itself");
    }
}
