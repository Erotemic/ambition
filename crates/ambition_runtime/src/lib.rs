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
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins)
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

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};

mod combat_schedule;
mod player_schedule;
#[cfg(feature = "portal")]
mod portal_schedule;
mod progression_schedule;
mod room_schedule;
mod sim_core_resources;

pub use combat_schedule::CombatSchedulePlugin;
pub use player_schedule::PlayerSchedulePlugin;
#[cfg(feature = "portal")]
pub use portal_schedule::PortalSchedulePlugin;
pub use progression_schedule::ProgressionSchedulePlugin;
pub use room_schedule::RoomTransitionSchedulePlugin;
pub use sim_core_resources::SimCoreResourcesPlugin;

/// Host-facing input seams that are implemented by the simulation heart but
/// scheduled by a visible host. Keeping this tiny facade here lets
/// `ambition_host` wire leafwing/device input without depending directly on
/// `ambition_actors`.
pub mod host_input {
    pub use ambition_actors::dialog::dialog_pointer_input;
    pub use ambition_actors::schedule::{
        apply_menu_frame_to_cutscene_request, attach_player_input_components,
        populate_control_frame_from_actions, populate_menu_control_frame_from_actions,
        toggle_player_trail_emission_from_actions, SimulationSetupSet,
    };
}

/// Host-facing presentation seams that still originate in lower sim/foundation
/// crates. This records the intentional runtime facade used to keep the windowed
/// host out of the actor-systems crate.
pub mod host_seams {
    pub use ambition_actors::SandboxDevState;
}

/// Fixture/demo support re-exported from the runtime composition tier so the
/// `ambition_host` smoke shell can assemble a tiny content plugin without taking
/// a direct `ambition_actors` dependency.
pub mod demo_fixture {
    pub use ambition_actors::character_roster::install_character_catalog;
    pub use ambition_actors::dev::dev_tools::{EditableAbilitySet, EditableMovementTuning};
    pub use ambition_actors::ldtk_world::LdtkRuntimeIndex;
    pub use ambition_actors::player::StartingCharacter;
    pub use ambition_actors::rooms::{ActiveRoomMetadata, RoomSet, RoomSpec};
    pub use ambition_actors::session::setup::{simulation_world, SimulationSetup};
    pub use ambition_platformer_primitives::schedule::SimulationSetupSet;
}

/// The canonical simulation-phase SETS + the engine resources every consumer
/// needs before any `.in_set(SandboxSet::…)` registration or host override.
///
/// First plugin in [`PlatformerEnginePlugins`]. Hosts override the resources
/// by `insert_resource` BEFORE `add_plugins` (Bevy's `init_resource` never
/// clobbers an existing value) — the `StartingCharacter` env-var override in
/// the Ambition CLI relies on exactly that.
pub struct SandboxSetsPlugin;

impl Plugin for SandboxSetsPlugin {
    fn build(&self, app: &mut App) {
        // Declare the canonical simulation-phase ordering. System
        // registrations elsewhere only need `.in_set(SandboxSet::X)`.
        ambition_actors::schedule::configure_sandbox_sets(app);
        // Shrine activation pulse (interaction → save flash).
        app.init_resource::<ambition_actors::shrine::ShrineActivationPulse>();
        // Slot-keyed gesture/buffer authority (double-tap, interact buffer).
        // Local input publishes it; body mode / interaction / transitions
        // consume it for the controlled body's slot.
        app.init_resource::<ambition_actors::player::SlotInteractionState>();
        // Which character the local player spawns as (empty = the
        // content-installed default). Hosts pre-insert to override.
        app.init_resource::<ambition_actors::player::StartingCharacter>();
    }
}

/// The engine's content-free simulation plugin group (see module docs).
pub struct PlatformerEnginePlugins;

impl PluginGroup for PlatformerEnginePlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            // Sets + engine resources FIRST (see SandboxSetsPlugin docs).
            .add(SandboxSetsPlugin)
            // The engine sim messages + resource defaults (E5 step 6) —
            // hosts override by insert-before-add (init never clobbers).
            .add(SimCoreResourcesPlugin)
            // The world-prep phase (body integration, gravity collection, etc.).
            .add(ambition_actors::features::WorldPrepSchedulePlugin)
            // Universal-brain messages/resources (player/NPC/enemy/boss).
            .add(ambition_characters::brain::BrainPlugin)
            // Traversal ability/weapon kit + shared app state.
            .add(ambition_actors::abilities::AmbitionAbilitiesPlugin)
            // The emitted player trail substrate.
            .add(ambition_actors::player::trail::PlayerTrailPlugin)
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
            .add(ambition_actors::player::affordances::AffordancesPlugin)
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
            .add(ProgressionSchedulePlugin);
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
