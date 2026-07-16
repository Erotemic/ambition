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
//! reset/replay consumers, the home-reset policy + player presentation sync,
//! the room-transition APPLY composer (`load_room` + render spawns), and the
//! catalog/roster content installs. Each pins itself into a documented
//! ordering SLOT between engine systems (see `player_schedule` /
//! `room_schedule` module docs).
//!
//! Presentation, audio, windowing, dev tools, and CONTENT are never in this
//! group — [the windowed host] (`ambition_host`) and the game's app own those.

use bevy::app::{App, FixedUpdate, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::ecs::schedule::IntoScheduleConfigs as _;
use bevy::time::{Fixed, Time};

use ambition_platformer_primitives::schedule::SimScheduleExt as _;

mod combat_schedule;
pub mod input_stream;
mod mode_scope;
mod player_schedule;
#[cfg(feature = "portal")]
mod portal_schedule;
mod progression_schedule;
pub mod projectile_schedule;
mod room_schedule;
pub mod session_world;
mod sim_core_resources;
/// N3.1's opt-in sim-state registration seam, and N0.4's desync-canary hash.
pub mod snapshot;

pub use combat_schedule::CombatSchedulePlugin;
/// The demo-hosting seam (D-C): gate a hosted ruleset on the active room's mode.
pub use mode_scope::{despawn_departed_mode_entities, in_base_mode, in_mode, ModeScopePlugin};
pub use player_schedule::PlayerSchedulePlugin;
#[cfg(feature = "portal")]
pub use portal_schedule::PortalSchedulePlugin;
pub use progression_schedule::ProgressionSchedulePlugin;
pub use room_schedule::RoomTransitionSchedulePlugin;
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
        apply_menu_frame_to_cutscene_request, attach_player_input_components,
        populate_control_frame_from_actions, populate_menu_control_frame_from_actions,
        toggle_player_trail_emission_from_actions, SimulationSetupSet,
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
    pub use ambition_actors::ldtk_world::LdtkRuntimeIndex;
    pub use ambition_actors::rooms::{ActiveRoomMetadata, RoomSet, RoomSpec};
    pub use ambition_actors::session::setup::{simulation_world, SimulationSetup};
    pub use ambition_actors::world::placements::PlacementLoweringRegistry;
    pub use ambition_dev_tools::dev_tools::{EditableAbilitySet, EditableMovementTuning};
    pub use ambition_platformer_primitives::schedule::SimulationSetupSet;
}

/// The sim tick rate under [`PlatformerEnginePlugins::fixed_tick`]. 60 Hz.
pub const SIM_TICK_HZ: f64 = 60.0;

/// The canonical simulation-phase SETS + the engine resources every consumer
/// needs before any `.in_set(SandboxSet::…)` registration or host override.
///
/// First plugin in [`PlatformerEnginePlugins`]. Hosts may override ordinary
/// engine configuration resources before `add_plugins` (Bevy's
/// `init_resource` never clobbers an existing value). Live room/world state is
/// not configured this way: providers publish it as components on the exact
/// session root, while direct apps create the same root during composition.
///
/// It is also where the group's `fixed_tick` choice becomes real: this plugin
/// commits the [`SimSchedule`] label before any other plugin can read one.
///
/// [`SimSchedule`]: ambition_platformer_primitives::schedule::SimSchedule
#[derive(Default)]
pub struct SandboxSetsPlugin {
    /// Host the sim in `FixedUpdate` on `Time<Fixed>` instead of `Update`.
    pub fixed_tick: bool,
}

impl Plugin for SandboxSetsPlugin {
    fn build(&self, app: &mut App) {
        if self.fixed_tick {
            // Commit the label FIRST — `configure_sandbox_sets` reads it, and
            // reading seals it. A sim plugin added before this group would have
            // already sealed `Update`, and `set_sim_schedule` panics rather
            // than let half the sim land in the wrong schedule.
            app.set_sim_schedule(FixedUpdate);
            // The tick cadence. Bevy's default `Time<Fixed>` is 64 Hz; the sim
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
        // reads identity — snapshot, replay, and the N0.4 canary all key on it.
        app.add_systems(
            sim,
            (snapshot::ensure_sim_id, snapshot::mint_spawned_sim_ids)
                .chain()
                .in_set(ambition_platformer_primitives::schedule::GameplaySimulationRoot)
                .before(ambition_platformer_primitives::schedule::SandboxSet::CoreSimulation),
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
/// # The two clocks (netcode N0.1)
///
/// By default the group is **frame-stepped**: the sim advances once per
/// rendered frame, in `Update`. This is Ambition today.
///
/// `PlatformerEnginePlugins::fixed_tick()` switches it to **fixed-tick**: the
/// sim advances on `Time<Fixed>` at [`SIM_TICK_HZ`], hosted in `FixedUpdate`,
/// while presentation, device sampling, and feel-time effects stay on the
/// render frame in `Update`. Demos, Super Smash Siblings, deterministic replay,
/// lockstep, and rollback all want this.
///
/// Every member plugin registers its systems into
/// [`SimSchedule`](ambition_platformer_primitives::schedule::SimSchedule) rather
/// than naming a schedule, so the choice threads through the whole group — and
/// through any CONTENT plugin, which asks `app.sim_schedule()` the same way.
///
/// Content plugins are frequently added BEFORE this group (Ambition's app does
/// exactly that). Such an app must therefore choose the mode itself, before its
/// first sim plugin:
///
/// ```ignore
/// app.set_sim_schedule(FixedUpdate);          // ← before any sim plugin
/// app.add_plugins(MyContentPlugin);
/// app.add_plugins(PlatformerEnginePlugins::fixed_tick());
/// ```
///
/// Getting that order wrong panics at startup with both labels named, rather
/// than silently splitting the sim across two schedules.
#[derive(Default)]
pub struct PlatformerEnginePlugins {
    /// Host the sim in `FixedUpdate` on `Time<Fixed>` at [`SIM_TICK_HZ`].
    pub fixed_tick: bool,
}

impl PlatformerEnginePlugins {
    /// The fixed-tick engine: `Time<Fixed>` at [`SIM_TICK_HZ`], presentation
    /// interpolating in `Update`. See the type docs for the ordering rule.
    pub fn fixed_tick() -> Self {
        Self { fixed_tick: true }
    }
}

impl PluginGroup for PlatformerEnginePlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            // Sets + engine resources FIRST (see SandboxSetsPlugin docs).
            .add(SandboxSetsPlugin {
                fixed_tick: self.fixed_tick,
            })
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
            // N3.1's snapshot registry, with the engine's own state registered.
            // EARLY, so every plugin after it — including a downstream game's
            // content plugins — can `resource_mut::<SnapshotRegistry>()` and add
            // the sim state it owns. Registration order is a function of plugin
            // build order, hence of the binary, hence identical across two sims.
            .add(crate::snapshot::SnapshotRegistryPlugin)
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
            // The camera OBSERVATION seam (E4-17): the sim resolves ONE
            // follow-camera snapshot per tick (the only CameraEaseState
            // writer); presentation consumes it. Headless/RL readers too.
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
};
