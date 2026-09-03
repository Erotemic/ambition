//! Content-free platformer simulation assembly.
//!
//! [`PlatformerEnginePlugins`] installs shared simulation schedules, resources,
//! gameplay systems, and extension slots used by visible, headless, RL, and
//! demo hosts. Game content, windowing, presentation, audio, and host-specific
//! input policy are composed outside this crate.

use bevy::app::{App, FixedUpdate, Plugin, PluginGroup, PluginGroupBuilder, Update};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs as _;
use bevy::time::{Fixed, Time};

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

/// The reset horizon's composition: where checkpoint capture and restore sit in
/// the tick, and the ordering edges that make them one transaction.
pub mod checkpoint_horizon;
mod combat_schedule;
pub mod content_identity;
pub mod durable_save_horizon;
/// Holding external effects (audio, VFX) at the host's confirmed-frame boundary
/// so a rollback cannot duplicate one or leave a mispredicted one standing.
pub mod external_effects;
#[cfg(test)]
mod host_invariant_tests;
pub mod input_drive;
pub mod input_stream;
/// The opt-in LDtk world install: the format's runtime spine + its rollback row.
#[cfg(feature = "ldtk")]
pub mod ldtk_world;
mod mode_scope;
mod player_schedule;
#[cfg(feature = "portal")]
mod portal_schedule;
mod progression_schedule;
pub mod projectile_schedule;
/// Backend-neutral rollback schema composition and exact prepared-content identity.
pub mod rollback;
mod room_schedule;
pub mod room_transition;
/// The shared sandbox-reset authority (`reset_sandbox`) and the one
/// `RoomReplayRequested` consumer every host drains.
pub mod sandbox_reset;
pub mod session_world;
mod sim_core_resources;
/// Stable simulation identity maintenance shared by every host.
pub mod sim_identity;
#[cfg(test)]
mod sim_identity_tests;

// Re-export the shared finalization seam without moving its ownership up the dependency graph.
pub use ambition_platformer2d_shared_tangle::app_finalization::{finalize, finalize_and_update};
pub use combat_schedule::CombatSchedulePlugin;
pub use content_identity::{
    ContentDiagnostic, ContentEpoch, ContentEpochSequence, ContentFingerprint,
    ContentFingerprintSchemaVersion, ContentOwner, PreparedContent, PreparedContentBuildError,
    PreparedContentBuilder, PreparedContentIdentity, PreparedContentSection,
    SnapshotSchemaFingerprint,
};
/// The demo-hosting seam (D-C): gate a hosted ruleset on the active room's mode.
pub use mode_scope::{despawn_departed_mode_entities, in_base_mode, in_mode, ModeScopePlugin};
pub use player_schedule::PlayerSchedulePlugin;
#[cfg(feature = "portal")]
pub use portal_schedule::PortalSchedulePlugin;
pub use progression_schedule::ProgressionSchedulePlugin;
pub use room_schedule::RoomTransitionSchedulePlugin;
pub use room_transition::RoomTransitionComposerPlugin;
pub use sandbox_reset::{
    admit_room_replay, reset_sandbox, return_the_replay_subject_to_spawn, RoomReplayAdmission,
    RoomReplayConsequences, RoomReplaySchedulePlugin,
};
pub use sim_core_resources::SimCoreResourcesPlugin;

/// The canonical timeline (netcode N0.1). Re-exported here because the sim
/// schedule this crate assembles is what advances it.
pub use ambition_time::SimTick;
/// The per-tick input recorder (netcode N0.2).
pub use input_stream::{input_stream_recording, record_input_stream, InputStreamRecorder};
#[cfg(feature = "ldtk")]
pub use ldtk_world::LdtkWorldPlugin;

/// Host-facing input seams that are implemented by the simulation heart but
/// scheduled by a visible host. Keeping this tiny facade here lets
/// `ambition_platformer2d_host` wire leafwing/device input without depending directly on
/// `ambition_platformer2d_actor_monolith`.
#[cfg(feature = "causal")]
pub mod causal;

pub mod host_input {
    pub use ambition_platformer2d_actor_monolith::schedule::{
        apply_menu_frame_to_cutscene_request, commit_seat_raw_frames,
        declare_gameplay_input_context, declare_in_session_input_contexts,
        freeze_local_seating_for_the_decided_match, mirror_primary_slot_to_control_frame,
        populate_menu_control_frame_from_actions, populate_seat_control_frames,
        populate_seat_menu_frames, publish_latched_slot_controls,
        publish_seat_controls_when_nobody_else_does, seat_input_participants_for_roster,
        spawn_primary_input_participant, sync_primary_recipe_from_settings,
        toggle_player_trail_emission_from_actions, MenuFrameConsume, MenuFrameCutsceneSkip,
        MenuFramePopulate, MenuNavConsume, SeatBurstTriggerState,
    };
    pub use ambition_platformer2d_shared_tangle::schedule::SimulationSetupSet;
    // Publication boundary re-exported with the host-facing shaping systems.
    pub use ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit;
    // Frame-to-tick latch re-exported through the host-facing input seam.
    pub use ambition_characters::control::SlotControlLatches;
    pub use ambition_dialog::dialog_pointer_input;
}

pub mod host_seams {
    pub use ambition_dev_tools::DeveloperRuntimeState;
}

/// Fixture/demo support re-exported from the runtime composition tier so the
/// `ambition_platformer2d_host` smoke shell can assemble a tiny content plugin without taking
/// a direct `ambition_platformer2d_actor_monolith` dependency.
pub mod demo_fixture {
    pub use ambition_boss_encounter::BossCatalog;
    pub use ambition_characters::prepared::PreparedCharacterRegistry;
    pub use ambition_dev_tools::dev_tools::EditableAbilitySet;
    pub use ambition_platformer2d_actor_monolith::avatar::{InitialBodyPolicy, StartingCharacter};
    pub use ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry;
    pub use ambition_platformer2d_actor_monolith::features::ActorConstructionContext;
    pub use ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry;
    // Demo fixtures are RON-authored consumers and intentionally do not expose LDtk runtime state.
    pub use ambition_platformer2d_actor_monolith::session::setup::{
        simulation_world, SimulationSetup,
    };
    pub use ambition_platformer2d_actor_monolith::world::placements::PlacementLoweringRegistry;
    pub use ambition_platformer2d_world::rooms::{ActiveRoomMetadata, RoomSet, RoomSpec};
    // Demo simulation reads the neutral movement-tuning authority, not dev-tools mirror state.
    pub use ambition_platformer2d_core::ActiveMovementTuning;
    pub use ambition_platformer2d_shared_tangle::schedule::SimulationSetupSet;
}

/// The sim tick rate under [`PlatformerEnginePlugins::fixed_tick`]. 60 Hz.
pub const SIM_TICK_HZ: f64 = 60.0;

/// Construction-time owner of the authoritative simulation schedule.
///
/// This is deliberately not a runtime toggle: Bevy systems register into one
/// concrete schedule while plugins build. A game that does not need rollback
/// chooses [`Fixed60Hz`](Self::Fixed60Hz) or [`RenderFrame`](Self::RenderFrame)
/// and does not install rollback-backend snapshot/session machinery at all.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SimulationHost {
    #[default]
    RenderFrame,
    Fixed60Hz,
    Rollback,
}

impl SimulationHost {
    pub fn is_rollback(self) -> bool {
        matches!(self, Self::Rollback)
    }
}

/// Marker installed by a concrete rollback backend once it has selected its
/// schedule and installed the host machinery the generic engine relies on.
///
/// This is composition state, not simulation state.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RollbackHostReady;

/// The rollback authority and its lifetime model. See
/// [`rollback::authority`] for why confirmation health is owned by a
/// [`SessionScopeId`](ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId)
/// rather than by the process.
pub use rollback::authority::{
    ActiveRollbackAuthority, RollbackConfirmationState, RollbackDiagnostic,
    RollbackDiagnosticHistory, RollbackTimelineContract, RollbackTimelineGeneration,
    RollbackTimelineStatus, SessionRollbackConfirmation,
};

/// Choose [`SimulationHost`] before any content or simulation plugin builds.
pub trait SimulationHostAppExt {
    fn set_simulation_host(&mut self, host: SimulationHost) -> &mut Self;
}

impl SimulationHostAppExt for App {
    fn set_simulation_host(&mut self, host: SimulationHost) -> &mut Self {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

        let same_host = self
            .world()
            .get_resource::<SimulationHost>()
            .is_some_and(|current| *current == host);
        let rollback_backend_ready = self.world().contains_resource::<RollbackHostReady>();
        let same_schedule = match host {
            SimulationHost::RenderFrame => self.sim_is(Update),
            SimulationHost::Fixed60Hz => self.sim_is(FixedUpdate),
            // A concrete rollback backend owns its schedule label.
            SimulationHost::Rollback => rollback_backend_ready,
        };
        if !same_host || !same_schedule {
            match host {
                SimulationHost::RenderFrame => {
                    self.set_sim_schedule(Update);
                }
                SimulationHost::Fixed60Hz => {
                    self.set_sim_schedule(FixedUpdate);
                }
                // Declaring rollback host semantics may happen before the backend
                // plugin is added. The foundation below is the deadline: by then
                // the backend must have selected its concrete schedule.
                SimulationHost::Rollback => {}
            }
        }
        self.insert_resource(host);
        self
    }
}

/// The canonical simulation-phase SETS + the engine resources every consumer
/// needs before any `.in_set(Platformer2dSimulationPhaseMonolith::…)` registration or host override.
///
/// First plugin in [`PlatformerEnginePlugins`]. Hosts may override ordinary
/// engine configuration resources before `add_plugins` (Bevy's
/// `init_resource` never clobbers an existing value). Live room/world state is
/// not configured this way: providers publish it as components on the exact
/// session root, while direct apps create the same root during composition.
///
/// It is also where the group's [`SimulationHost`] choice becomes real: this
/// plugin commits the [`SimSchedule`] label before any other plugin can read one.
///
/// [`SimSchedule`]: ambition_platformer2d_shared_tangle::schedule::SimSchedule
#[derive(Default)]
pub struct Platformer2dSimulationFoundationPlugin {
    pub host: SimulationHost,
}

impl Plugin for Platformer2dSimulationFoundationPlugin {
    fn build(&self, app: &mut App) {
        app.set_simulation_host(self.host);
        if self.host == SimulationHost::Rollback {
            assert!(
                app.world().contains_resource::<RollbackHostReady>(),
                "SimulationHost::Rollback requires a concrete rollback backend before the engine foundation builds"
            );
        }
        if self.host == SimulationHost::Fixed60Hz {
            // Bullet-time therefore composes INSIDE the tick and never touches the accumulator.
            app.insert_resource(Time::<Fixed>::from_hz(SIM_TICK_HZ));
            // NOTE: the frame→tick input LATCH is NOT installed here. It is the
            // DEVICE layer's bridge (`ambition_platformer2d_host`), because only a device
            // samples on the feel clock. Headless, RL, and replay drivers
            // author the per-tick `ControlFrame` directly, and a latch
            // publisher would overwrite it at the head of every tick.
        }
        // Declare the canonical simulation-phase ordering. System
        // registrations elsewhere only need `.in_set(Platformer2dSimulationPhaseMonolith::X)`.
        ambition_platformer2d_actor_monolith::schedule::configure_platformer2d_simulation_phases(
            app,
        );
        // The Class-B transit ledger (`docs/concepts/movement-collision.md`). Frame-scoped:
        // cleared at the head of the sim, appended to by portal transit, room
        // transitions, death/respawn, and the teleport abilities. It belongs to
        // THIS plugin because it is a property of the sim FRAME, not of any one
        // mechanic — and every Class-B writer lives downstream of `CoreSimulation`'s
        // leading edge, `ResetProcessing` (a tail set) included.
        let sim = app.sim_schedule();
        app.init_resource::<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>();
        app.add_systems(
            sim,
            ambition_platformer2d_shared_tangle::class_b::clear_class_b_remap_log
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .before(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation),
        );
        // N3.1's identity vocabulary. Every body the sim can identify from an
        // authored fact gets its `SimId` at the head of the frame, before anything
        // reads identity — rollback, replay, and the sync-test canary all key on it.
        app.add_systems(
            sim,
            (
                sim_identity::ensure_sim_id,
                sim_identity::mint_spawned_sim_ids,
                sim_identity::heal_projectile_owners,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .before(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::CoreSimulation),
        );
        // ...and again at the TAIL, after the last in-tick spawner (room
        // transition lowering, wave spawns, summons, sandbox reset), so identity
        // is synchronous with the tick that spawned the body. Without this, a
        // GGRS save at the boundary of a transition tick captures the
        // freshly-lowered bodies WITHOUT identity — invisible to the roster and
        // unreproducible after rollback entity recreation. Same canonical systems,
        // second scheduling; the `Without<SimId>` guard makes the pair idempotent.
        app.add_systems(
            sim,
            (
                sim_identity::ensure_sim_id,
                sim_identity::mint_spawned_sim_ids,
                sim_identity::heal_projectile_owners,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::ResetProcessing)
                .before(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
        // Shrine activation pulse (interaction → save flash).
        app.init_resource::<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>();
        // Slot-keyed gesture/buffer authority (double-tap, interact buffer).
        // Local input publishes it; body mode / interaction / transitions
        // consume it for the controlled body's slot.
        app.init_resource::<ambition_characters::control::SlotInteractionState>();
    }
}

/// Content-free simulation plugin group.
///
/// [`SimulationHost::RenderFrame`] uses `Update`; [`Self::fixed_tick`] uses
/// [`SIM_TICK_HZ`] in `FixedUpdate`; rollback backends install their own session
/// machinery. Members register through `SimSchedule`, so content and engine
/// systems share the selected host. Set the host before adding simulation or
/// content plugins; changing it after registration is rejected at startup.
#[derive(Default)]
pub struct PlatformerEnginePlugins {
    pub host: SimulationHost,
}

impl PlatformerEnginePlugins {
    pub fn new(host: SimulationHost) -> Self {
        Self { host }
    }

    /// See the type docs for the ordering rule.
    pub fn fixed_tick() -> Self {
        Self::new(SimulationHost::Fixed60Hz)
    }
}

impl PluginGroup for PlatformerEnginePlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            // Sets + engine resources FIRST (see Platformer2dSimulationFoundationPlugin docs).
            .add(Platformer2dSimulationFoundationPlugin { host: self.host });
        let builder = builder
            // Prepared content always carries the exact typed rollback-schema
            // fingerprint. This plugin is metadata-only for every host; a
            // concrete rollback backend invokes the same declarations through
            // its own registrar to install executable snapshot machinery.
            .add(crate::rollback::AmbitionRollbackSchemaPlugin)
            // The engine sim messages + resource defaults (E5 step 6) —
            // hosts override by insert-before-add (init never clobbers).
            .add(SimCoreResourcesPlugin)
            // Domain-owned sim resources + dev live-edit sets (decision #9:
            // the dev/dialog/encounter/menu domains install their own local
            // state; the assembly below only ORDERS their public sets).
            .add(ambition_dev_tools::DevToolsSimPlugin)
            .add(ambition_dialog::DialogSimStatePlugin)
            .add(ambition_encounter::EncounterRegistryPlugin)
            .add(ambition_menu::map::MapStatePlugin)
            // The world-prep phase (body integration, gravity collection, etc.).
            .add(ambition_platformer2d_actor_monolith::features::WorldPrepSchedulePlugin)
            // A sheet-authored body adopts the box for the pose it is showing, in
            // `WorldPrepSet::BeforeIntegrate`.
            .add(ambition_character_sprites::SpritePosedBodyPlugin)
            .add(ambition_platformer2d_actor_monolith::character_runtime::CharacterRuntimePlugin)
            // Universal-brain messages/resources (player/NPC/enemy/boss).
            .add(ambition_characters::brain::BrainPlugin)
            // Traversal ability/weapon kit + shared app state.
            .add(ambition_platformer2d_actor_monolith::abilities::AmbitionAbilitiesPlugin)
            // The emitted player trail substrate.
            .add(ambition_platformer2d_actor_monolith::avatar::trail::PlayerTrailPlugin)
            // Gravity zones/switches + the ambient-gravity snapshot.
            .add(ambition_platformer2d_actor_monolith::gravity::GravityPlugin)
            // The PRESSED collectible: a held weapon taken with `Attack`, its
            // use, throw, ground physics and custody-driven residency, and
            // the schedule they own (`ItemPickupSet::CoreHeldItems`, configured
            // end to end by this plugin — D33, 2026-09-03).
            .add(ambition_held_items::HeldItemSimulationPlugin)
            // The kernel's residue of that domain: the two sibling variants
            // (thrown-item effects, wielded abilities), the three-variant
            // chain, and the shrine / gun / match-spawn systems that attach to
            // the domain's steps. ⛔ Composed beside the domain's plugin, not
            // adding it: a composition with only this plugin gets the chain and
            // no core; with only the domain's, a core and no chain. Order
            // between the two does not matter — each configures its own sets.
            .add(ambition_platformer2d_actor_monolith::items::pickup::ItemPickupSimulationPlugin)
            // ⭐ ITS SIBLING, AND THE TWO ARE COMPOSED HERE RATHER THAN ONE
            // ADDING THE OTHER: the TOUCHED collectible — where it is, whether
            // it is moving, and that walking into it collects it. Split out of
            // the actor kernel in D33 (2026-09-02) along the collect TRIGGER.
            // ⚠ Order between the two plugins does not matter; the order that
            // does is INSIDE this one (step before collect), and it says so.
            .add(ambition_world_items::WorldItemSimulationPlugin)
            // Feature (room-entity) collection + interaction schedules.
            .add(ambition_platformer2d_actor_monolith::features::FeatureCollectionSchedulePlugin)
            .add(ambition_platformer2d_actor_monolith::features::FeatureInteractionSchedulePlugin)
            .add(ambition_platformer2d_actor_monolith::encounter::EncounterSimulationSchedulePlugin)
            // Every writer of `gate_solids`, in one place: the encounter-phase
            // seal walls and the authored-condition ones. Their adjacency is the
            // point — see the plugin's module doc.
            .add(ambition_platformer2d_actor_monolith::world::gating::WorldGatingSchedulePlugin)
            .add(ambition_platformer2d_actor_monolith::cutscene::CutsceneSchedulePlugin)
            // Gameplay effects + feature view-sync schedules.
            .add(ambition_platformer2d_actor_monolith::features::GameplayEffectsSchedulePlugin)
            // Reward chests react to the encounter domain's published cleared
            // list; composed beside its siblings so no registration for it
            // lands back in the encounter adapter.
            .add(ambition_platformer2d_actor_monolith::features::EncounterRewardSyncPlugin)
            // Runtime brain-switch authority (BrainCommand) + actor-directive routing.
            .add(ambition_platformer2d_actor_monolith::features::BrainCommandPlugin)
            .add(ambition_sim_view::FeatureViewSyncSchedulePlugin)
            // The observation-boundary view resources (E4): HUD facts, held
            // items/shots, marks, shrines, gravity switches, gun-swords.
            .add(ambition_sim_view::SimViewPlugin)
            // Sandbox reset schedule.
            .add(ambition_platformer2d_actor_monolith::session::reset::NewGameResetPlugin)
            // Deterministic sim traces.
            .add(ambition_platformer2d_actor_monolith::trace::TraceSchedulePlugin)
            // Per-frame affordance table (what would each verb do right now).
            .add(ambition_sim_view::affordances::AffordancesPlugin)
            // Per-body derived action scheme (slot → action) — the source the
            // control-prompt read-model (P2) and the input→action seam (P3)
            // read. Reconciled from live AbilitySet + moveset.
            .add(ambition_platformer2d_actor_monolith::action_scheme::ActionSchemePlugin)
            // The camera OBSERVATION seam (E4-17): ONE follow-camera
            // snapshot per rendered frame (the only CameraEaseState
            // writer); presentation consumes it. Headless/RL readers too.
            //
            // Per frame rather than per tick because where the camera
            // looks is presentation state, not a sim fact: it depends on
            // the physical viewport and video settings and eases on the
            // render clock. A headless composition that wants camera
            // observation gets it by running Update; it is not implied by
            // advancing the sim.
            // Resamples the per-tick pose read-models onto the RENDER clock,
            // ahead of both consumers below. The camera and the sprite must
            // frame the same presented position; when they sampled different
            // clocks, a moving subject shuddered horizontally against a world
            // that looked perfectly stable.
            .add(ambition_sim_view::presented_pose::PresentedPosePlugin)
            .add(ambition_sim_view::camera_snapshot::CameraObservationPlugin)
            // The combat-phase chain + the content extension slots
            // (CombatSet::ContentSpecials / ContentFlavor).
            .add(CombatSchedulePlugin)
            // The per-frame player lifecycle (E5 step 5): time control →
            // input → controlled subject → brains → possession → hit events
            // → presentation write-back. Headless/RL runs all of it.
            .add(PlayerSchedulePlugin)
            // Room-transition detection + per-room feature reset; the host's
            // transition APPLY (the composition tier) slots in between.
            .add(ambition_platformer2d_actor_monolith::features::transform_beat::TransformBeatPlugin)
            // The empowerment LIFECYCLE — the clock that ends a timed grant and
            // the release that follows the component out of the world. Both are
            // invariants of what `Empowered::for_seconds(…, 2.0)` means, not
            // rules a game gets an opinion about; what stays each game's choice
            // is the ORDER (against `EmpowermentExpiry`) and whether it wants
            // `apply_contact_harm` at all.
            .add(
                ambition_platformer2d_actor_monolith::features::empowerment::EmpowermentLifecyclePlugin,
            )
            .add(RoomTransitionSchedulePlugin)
            .add(RoomTransitionComposerPlugin)
            // The one `RoomReplayRequested` consumer + the two content slots
            // that must precede it. In the group because content in EVERY host
            // emits the request: without a consumer here, a standalone demo
            // binary writes the message into a channel nothing drains.
            .add(RoomReplaySchedulePlugin)
            // The reset horizon (checkpoint baseline capture + restore). Added
            // immediately after the replay consumer because its restore set is
            // ordered against that consumer's set, and the two are one
            // transaction: put the world back, then rebuild the room from it.
            .add(checkpoint_horizon::CheckpointHorizonPlugin)
            // The DURABLE horizon, immediately after the checkpoint one because it is a
            // serialization of the same three values and its load is a checkpoint resume.
            .add(durable_save_horizon::DurableSaveHorizonPlugin)
            // The world-fact domain's authored-condition provider. Added here
            // because composition is where plugins are chosen — NOT because
            // anything central knows what conditions exist. The item domain
            // publishes its own from `ItemPickupSimulationPlugin`, and neither
            // names the other.
            // The authored-COMMAND machinery: the request channel, the one set
            // authored verbs happen in, and the one system that performs them.
            // it publishes no command — a domain adds its verbs from its own
            // plugin, exactly as it adds its questions.
            .add(ambition_platformer2d_shared_tangle::authored_logic::AuthoredCommandPlugin)
            // The first authored-command CONSUMER: a `Switch` that names a verb
            // in the level instead of in a const table. it publishes nothing
            // and names no domain — it performs whatever the composed catalog
            // happens to know, which is why it can live here beside the
            // machinery rather than inside whichever domain a level asks for.
            .add(ambition_platformer2d_actor_monolith::world::authored_switch_commands::AuthoredSwitchCommandPlugin)
            .add(ambition_platformer2d_actor_monolith::world_facts::WorldFactConditionsPlugin)
            // The inventory domain's provider, added for the same reason and in
            // the same way. it is the THIRD provider and it cost one line of
            // composition — which is the acceptance clause, restated as code.
            .add(ambition_platformer2d_actor_monolith::items::conditions::InventoryConditionsPlugin)
            // The engine progression chain (boss encounters, save mirrors,
            // quest pump, room metadata/music, portal phases) + its content
            // slots.
            .add(ProgressionSchedulePlugin)
            // The demo-hosting seam (D-C): retire a departed game mode's
            // entities once the active room's mode changes. Reads the metadata
            // ProgressionSchedulePlugin just published, so it is added after it.
            .add(ModeScopePlugin);
        #[cfg(feature = "portal")]
        let builder = builder
            // PortalPlugin + the portal-set schedule placement (the three
            // ordering landmines documented on the plugin).
            .add(PortalSchedulePlugin);
        builder
    }
}

/// Engine states every entry point must initialize after Bevy's `StatesPlugin`
/// exists and before the sim plugins build (their run conditions read the
/// state). One call site per app instead of a copy-pasted `init_state`.
pub fn init_engine_states(app: &mut App) {
    use bevy::state::app::AppExtStates as _;
    app.init_state::<ambition_platformer2d_shared_tangle::schedule::GameMode>();
}

/// The minimal Bevy foundation for a HEADLESS engine app (tests, RL, trace
/// replay, demo smoke shells): schedules/time via `MinimalPlugins`, asset +
/// image registries (bevy_ecs_ldtk touches `Image` handles even with no
/// renderer), transforms, states, and the engine states.
pub fn add_headless_foundation(app: &mut App) {
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::image::ImagePlugin::default());
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(bevy::state::app::StatesPlugin);
    serialize_frame_schedules(app);
    init_engine_states(app);
}

/// Run the main-world frame schedules serially instead of on the
/// multithreaded executor.
///
/// Profiling (headless boss room, 3600 ticks) measured ~1.5M voluntary
/// context switches per run — hundreds per tick — with gameplay systems at
/// <2% of CPU while executor bookkeeping + thread parking took ~40%+. With
/// system bodies this small, cross-thread dispatch costs far more than it
/// buys; rollback backends likewise own deterministic execution policy for
/// their concrete simulation schedules.
pub fn serialize_frame_schedules(app: &mut App) {
    use bevy::app::{First, Last, PostUpdate, PreUpdate, Update};
    use bevy::ecs::schedule::SingleThreadedExecutor;
    app.edit_schedule(First, |s| {
        s.set_executor(SingleThreadedExecutor::new());
    });
    app.edit_schedule(PreUpdate, |s| {
        s.set_executor(SingleThreadedExecutor::new());
    });
    app.edit_schedule(Update, |s| {
        s.set_executor(SingleThreadedExecutor::new());
    });
    app.edit_schedule(PostUpdate, |s| {
        s.set_executor(SingleThreadedExecutor::new());
    });
    app.edit_schedule(Last, |s| {
        s.set_executor(SingleThreadedExecutor::new());
    });
}

pub use session_world::{
    PlatformerSessionCatalogs, PlatformerSessionRequests, PlatformerSessionWorld,
    PreparedPlatformerSource,
};
