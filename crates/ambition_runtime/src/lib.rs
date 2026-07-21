//! The platformer ENGINE face — [the sim assembly] (decomposition E5):
//! [`PlatformerEnginePlugins`], a Bevy [`PluginGroup`] that assembles the
//! **content-free simulation plugins** shared by every platformer built on
//! this engine, plus the shared app-foundation helpers every entry point
//! (visible, headless, RL, demo) composes with.
//!
//! ## Why this crate
//!
//! A game — Ambition, or a demo (`demos/…`) — builds its simulation App by
//! adding this group plus its OWN content crate:
//!
//! ```ignore
//! let mut app = App::new();
//! ambition_runtime::add_headless_foundation(&mut app); // or DefaultPlugins + init_engine_states
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins::default())
//!    .add_plugins(my_content::MyGameContentPlugin);
//! ```
//!
//! This is **the demo gate**: a demo app depends on `ambition_runtime`, never
//! on `ambition_app`. The group carries only plugins that name no content and
//! reach for no app-local system — the sim schedule SETS + engine resources
//! ([`SandboxSetsPlugin`]), the universal brain, gravity, traversal abilities,
//! item pickups, encounters/cutscenes, feature collection/interaction/effects/
//! view-sync, room reset, traces, affordances, and the combat-phase chain
//! ([`CombatSchedulePlugin`]) with its content extension slots.
//!
//! ## What is deliberately NOT here
//!
//! The app-LOCAL residue the E5 carve deliberately left behind: the Ambition
//! reset-INPUT consumer (its button binding is Ambition's), the home-reset
//! policy + player presentation sync, the room-transition APPLY composer
//! (`load_room` + render spawns), and the catalog/roster content installs. Each
//! pins itself into a documented ordering SLOT between engine systems (see
//! `player_schedule` / `room_schedule` module docs).
//!
//! The room-REPLAY consumer used to be on that list and no longer is (see
//! [`sandbox_reset`]): content in every host emits `RoomReplayRequested`, so
//! leaving the only consumer in `ambition_app` meant the standalone demo
//! binaries drained nothing.
//!
//! Presentation, audio, windowing, dev tools, and CONTENT are never in this
//! group — [the windowed host] (`ambition_host`) and the game's app own those.

use bevy::app::{App, FixedUpdate, Plugin, PluginGroup, PluginGroupBuilder, Update};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs as _;
use bevy::time::{Fixed, Time};

use ambition_platformer_primitives::schedule::SimScheduleExt as _;

mod combat_schedule;
pub mod content_identity;
pub mod input_stream;
mod mode_scope;
mod player_schedule;
#[cfg(feature = "portal")]
mod portal_schedule;
mod progression_schedule;
pub mod projectile_schedule;
/// GGRS rollback integration: typed state registration, input/session bridge, and exact schema identity.
pub mod rollback;
mod room_schedule;
/// The shared sandbox-reset authority (`reset_sandbox`) and the one
/// `RoomReplayRequested` consumer every host drains.
pub mod sandbox_reset;
pub mod session_world;
mod sim_core_resources;

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
pub use sandbox_reset::{
    apply_room_replay_request_system, reset_sandbox, RoomReplaySchedulePlugin,
};
pub use sim_core_resources::SimCoreResourcesPlugin;

/// The canonical timeline (netcode N0.1). Re-exported here because the sim
/// schedule this crate assembles is what advances it.
pub use ambition_time::SimTick;
/// The per-tick input recorder (netcode N0.2).
pub use input_stream::{input_stream_recording, record_input_stream, InputStreamRecorder};

/// Host-facing input seams that are implemented by the simulation heart but
/// scheduled by a visible host. Keeping this tiny facade here lets
/// `ambition_host` wire leafwing/device input without depending directly on
/// `ambition_actors`.
pub mod host_input {
    pub use ambition_actors::schedule::{
        apply_menu_frame_to_cutscene_request, declare_gameplay_input_context,
        populate_control_frame_from_actions, populate_menu_control_frame_from_actions,
        spawn_primary_input_participant, toggle_player_trail_emission_from_actions,
        SimulationSetupSet,
    };
    pub use ambition_dialog::dialog_pointer_input;
}

/// Host-facing presentation seams that still originate in lower sim/foundation
/// crates. This records the intentional runtime facade used to keep the windowed
/// host out of the actor-systems crate.
pub mod host_seams {
    pub use ambition_dev_tools::SandboxDevState;
}

/// Fixture/demo support re-exported from the runtime composition tier so the
/// `ambition_host` smoke shell can assemble a tiny content plugin without taking
/// a direct `ambition_actors` dependency.
pub mod demo_fixture {
    pub use ambition_actors::avatar::StartingCharacter;
    pub use ambition_actors::boss_encounter::BossCatalog;
    pub use ambition_actors::features::CharacterRoster;
    pub use ambition_actors::features::RoomContentStagingRegistry;
    pub use ambition_actors::ldtk_world::LdtkRuntimeIndex;
    pub use ambition_actors::rooms::{ActiveRoomMetadata, RoomSet, RoomSpec};
    pub use ambition_actors::session::setup::{simulation_world, SimulationSetup};
    pub use ambition_actors::world::placements::PlacementLoweringRegistry;
    pub use ambition_dev_tools::dev_tools::EditableAbilitySet;
    // The neutral movement-tuning authority a demo's simulation reads. The
    // dev-tools mirror is deliberately NOT re-exported here any more: a demo is
    // a shipping-shaped consumer and must not read the inspector's state.
    pub use ambition_engine_core::ActiveMovementTuning;
    pub use ambition_platformer_primitives::schedule::SimulationSetupSet;
}

/// The sim tick rate under [`PlatformerEnginePlugins::fixed_tick`]. 60 Hz.
pub const SIM_TICK_HZ: f64 = 60.0;

/// Construction-time owner of the authoritative simulation schedule.
///
/// This is deliberately not a runtime toggle: Bevy systems register into one
/// concrete schedule while plugins build. A game that does not need rollback
/// chooses [`Fixed60Hz`](Self::Fixed60Hz) or [`RenderFrame`](Self::RenderFrame)
/// and does not install GGRS snapshot/session machinery at all.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SimulationHost {
    #[default]
    RenderFrame,
    Fixed60Hz,
    Ggrs,
}

impl SimulationHost {
    pub fn is_ggrs(self) -> bool {
        matches!(self, Self::Ggrs)
    }
}

/// Choose [`SimulationHost`] before any content or simulation plugin builds.
pub trait SimulationHostAppExt {
    fn set_simulation_host(&mut self, host: SimulationHost) -> &mut Self;
}

impl SimulationHostAppExt for App {
    fn set_simulation_host(&mut self, host: SimulationHost) -> &mut Self {
        use ambition_platformer_primitives::schedule::SimScheduleExt as _;

        let same_host = self
            .world()
            .get_resource::<SimulationHost>()
            .is_some_and(|current| *current == host);
        let same_schedule = match host {
            SimulationHost::RenderFrame => self.sim_is(Update),
            SimulationHost::Fixed60Hz => self.sim_is(FixedUpdate),
            SimulationHost::Ggrs => self.sim_is(bevy_ggrs::GgrsSchedule),
        };
        if !same_host || !same_schedule {
            match host {
                SimulationHost::RenderFrame => self.set_sim_schedule(Update),
                SimulationHost::Fixed60Hz => self.set_sim_schedule(FixedUpdate),
                SimulationHost::Ggrs => self.set_sim_schedule(bevy_ggrs::GgrsSchedule),
            };
        }
        self.insert_resource(host);
        self
    }
}

/// The canonical simulation-phase SETS + the engine resources every consumer
/// needs before any `.in_set(SandboxSet::…)` registration or host override.
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
/// [`SimSchedule`]: ambition_platformer_primitives::schedule::SimSchedule
#[derive(Default)]
pub struct SandboxSetsPlugin {
    pub host: SimulationHost,
}

impl Plugin for SandboxSetsPlugin {
    fn build(&self, app: &mut App) {
        app.set_simulation_host(self.host);
        if self.host == SimulationHost::Fixed60Hz {
            // `set_simulation_host` committed FixedUpdate before
            // `configure_sandbox_sets` can seal the schedule. Bevy's default
            // `Time<Fixed>` is 64 Hz; the sim
            // timeline is 60. `run_fixed_main_schedule` swaps the generic
            // `Time` to this clock for the duration of each tick, which is why
            // `refresh_world_time` needs no fixed-tick special case: inside the
            // tick it reads TICK_DT, and `scaled_dt = TICK_DT × time_scale`
            // falls out. Bullet-time therefore composes INSIDE the tick and
            // never touches the accumulator.
            app.insert_resource(Time::<Fixed>::from_hz(SIM_TICK_HZ));
            // NOTE: the frame→tick input LATCH is NOT installed here. It is the
            // DEVICE layer's bridge (`ambition_host`), because only a device
            // samples on the feel clock. Headless, RL, and replay drivers
            // author the per-tick `ControlFrame` directly, and a latch
            // publisher would overwrite it at the head of every tick.
        }
        // Declare the canonical simulation-phase ordering. System
        // registrations elsewhere only need `.in_set(SandboxSet::X)`.
        ambition_actors::schedule::configure_sandbox_sets(app);
        // The Class-B transit ledger (`collision-and-ccd.md` §3.2). Frame-scoped:
        // cleared at the head of the sim, appended to by portal transit, room
        // transitions, death/respawn, and the teleport abilities. It belongs to
        // THIS plugin because it is a property of the sim FRAME, not of any one
        // mechanic — and every Class-B writer lives downstream of `CoreSimulation`'s
        // leading edge, `ResetProcessing` (a tail set) included.
        let sim = app.sim_schedule();
        app.init_resource::<ambition_platformer_primitives::class_b::ClassBRemapLog>();
        app.add_systems(
            sim,
            ambition_platformer_primitives::class_b::clear_class_b_remap_log
                .in_set(ambition_platformer_primitives::schedule::GameplaySimulationRoot)
                .before(ambition_platformer_primitives::schedule::SandboxSet::CoreSimulation),
        );
        // N3.1's identity vocabulary. Every body the sim can identify from an
        // authored fact gets its `SimId` at the head of the frame, before anything
        // reads identity — rollback, replay, and the sync-test canary all key on it.
        app.add_systems(
            sim,
            (
                rollback::ensure_sim_id,
                rollback::mint_spawned_sim_ids,
                rollback::heal_projectile_owners,
            )
                .chain()
                .in_set(ambition_platformer_primitives::schedule::GameplaySimulationRoot)
                .before(ambition_platformer_primitives::schedule::SandboxSet::CoreSimulation),
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
                rollback::ensure_sim_id,
                rollback::mint_spawned_sim_ids,
                rollback::heal_projectile_owners,
            )
                .chain()
                .in_set(ambition_platformer_primitives::schedule::GameplaySimulationRoot)
                .after(ambition_platformer_primitives::schedule::SandboxSet::ResetProcessing)
                .before(ambition_platformer_primitives::schedule::SandboxSet::FeatureViewSync),
        );
        // Shrine activation pulse (interaction → save flash).
        app.init_resource::<ambition_actors::shrine::ShrineActivationPulse>();
        // Slot-keyed gesture/buffer authority (double-tap, interact buffer).
        // Local input publishes it; body mode / interaction / transitions
        // consume it for the controlled body's slot.
        app.init_resource::<ambition_actors::control::SlotInteractionState>();
    }
}

/// The engine's content-free simulation plugin group (see module docs).
///
/// # Simulation host (netcode N0.1)
///
/// The default [`SimulationHost::RenderFrame`] advances once per rendered frame
/// in `Update`. [`Self::fixed_tick`] advances at [`SIM_TICK_HZ`] in
/// `FixedUpdate`. [`Self::rollback`] advances only through GGRS requests and is
/// the only mode that installs GGRS schedules, snapshots, checksums, and session
/// machinery.
///
/// Every member plugin registers into
/// [`SimSchedule`](ambition_platformer_primitives::schedule::SimSchedule) rather
/// than naming a schedule, so the host choice threads through the whole group
/// and through content plugins that ask `app.sim_schedule()` the same way.
///
/// Content plugins are sometimes added before this group. Such an app must set
/// the construction-time host before its first simulation/content plugin:
///
/// ```ignore
/// app.set_simulation_host(SimulationHost::Fixed60Hz);
/// app.add_plugins(MyContentPlugin);
/// app.add_plugins(PlatformerEnginePlugins::fixed_tick());
/// ```
///
/// Getting that order wrong panics at startup rather than silently splitting
/// the simulation across schedules.
#[derive(Default)]
pub struct PlatformerEnginePlugins {
    pub host: SimulationHost,
}

impl PlatformerEnginePlugins {
    pub fn new(host: SimulationHost) -> Self {
        Self { host }
    }

    /// The fixed-tick engine: `Time<Fixed>` at [`SIM_TICK_HZ`], presentation
    /// interpolating in `Update`. See the type docs for the ordering rule.
    pub fn fixed_tick() -> Self {
        Self::new(SimulationHost::Fixed60Hz)
    }

    /// The GGRS-driven engine. The sim advances only through GGRS requests.
    pub fn rollback() -> Self {
        Self::new(SimulationHost::Ggrs)
    }
}

impl PluginGroup for PlatformerEnginePlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            // Sets + engine resources FIRST (see SandboxSetsPlugin docs).
            .add(SandboxSetsPlugin { host: self.host });
        // Non-rollback games do not pay for GGRS schedules, snapshot storage,
        // checksums, entity recreation, or session/request handling.
        let builder = if self.host.is_ggrs() {
            builder.add(crate::rollback::AmbitionRollbackPlugin)
        } else {
            builder
        };
        let builder = builder
            // Prepared content always carries the exact typed rollback-schema
            // fingerprint, even when this composition does not execute GGRS.
            // The schema plugin is metadata-only on non-GGRS hosts because the
            // registration vocabulary gates runtime installation on host kind.
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
            .add(ambition_actors::features::WorldPrepSchedulePlugin)
            // Universal-brain messages/resources (player/NPC/enemy/boss).
            .add(ambition_characters::brain::BrainPlugin)
            // Traversal ability/weapon kit + shared app state.
            .add(ambition_actors::abilities::AmbitionAbilitiesPlugin)
            // The emitted player trail substrate.
            .add(ambition_actors::avatar::trail::PlayerTrailPlugin)
            // Gravity zones/switches + the ambient-gravity snapshot.
            .add(ambition_actors::gravity::GravityPlugin)
            // Item pickup simulation.
            .add(ambition_actors::items::pickup::ItemPickupSimulationPlugin)
            // Feature (room-entity) collection + interaction schedules.
            .add(ambition_actors::features::FeatureCollectionSchedulePlugin)
            .add(ambition_actors::features::FeatureInteractionSchedulePlugin)
            // LDtk runtime spine (room load/transition spine).
            .add(ambition_actors::ldtk_world::LdtkRuntimeSpinePlugin)
            // Encounter + cutscene simulation schedules.
            .add(ambition_actors::encounter::EncounterSimulationSchedulePlugin)
            .add(ambition_actors::cutscene::CutsceneSchedulePlugin)
            // Gameplay effects + feature view-sync schedules.
            .add(ambition_actors::features::GameplayEffectsSchedulePlugin)
            // Runtime brain-switch authority (BrainCommand) + actor-directive routing.
            .add(ambition_actors::features::BrainCommandPlugin)
            .add(ambition_sim_view::FeatureViewSyncSchedulePlugin)
            // The observation-boundary view resources (E4): HUD facts, held
            // items/shots, marks, shrines, gravity switches, gun-swords.
            .add(ambition_sim_view::SimViewPlugin)
            // Sandbox reset schedule.
            .add(ambition_actors::session::reset::SandboxResetSchedulePlugin)
            // Deterministic sim traces.
            .add(ambition_actors::trace::TraceSchedulePlugin)
            // Per-frame affordance table (what would each verb do right now).
            .add(ambition_actors::affordances::AffordancesPlugin)
            // Per-body derived action scheme (slot → action) — the source the
            // control-prompt read-model (P2) and the input→action seam (P3)
            // read. Reconciled from live AbilitySet + moveset.
            .add(ambition_actors::action_scheme::ActionSchemePlugin)
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
            .add(RoomTransitionSchedulePlugin)
            // The one `RoomReplayRequested` consumer + the two content slots
            // that must precede it. In the group because content in EVERY host
            // emits the request: without a consumer here, a standalone demo
            // binary writes the message into a channel nothing drains.
            .add(RoomReplaySchedulePlugin)
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
    app.init_state::<ambition_platformer_primitives::schedule::GameMode>();
}

/// The minimal Bevy foundation for a HEADLESS engine app (tests, RL, trace
/// replay, demo smoke shells): schedules/time via `MinimalPlugins`, asset +
/// image registries (bevy_ecs_ldtk touches `Image` handles even with no
/// renderer), transforms, states, and the engine states.
///
/// Visible apps use `DefaultPlugins` instead and call [`init_engine_states`]
/// themselves; everything else converges here (this block was previously
/// copy-pasted across the headless binary, its tests, and the RL runtime).
pub fn add_headless_foundation(app: &mut App) {
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.add_plugins(bevy::image::ImagePlugin::default());
    app.add_plugins(bevy::transform::TransformPlugin);
    app.add_plugins(bevy::state::app::StatesPlugin);
    init_engine_states(app);
}

pub use session_world::{
    PlatformerSessionCatalogs, PlatformerSessionRequests, PlatformerSessionWorld,
    PreparedPlatformerSource,
};
