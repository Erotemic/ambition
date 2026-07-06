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
//! ## What is deliberately NOT here (yet)
//!
//! Still assembled app-side because they either wrap app-local systems or
//! carry presentation deps: the sandbox resource plugin (blocked on the E4
//! vfx-message inversion), the progression schedule (needs its engine/content
//! split), the per-frame player input/simulation/room-transition registrations
//! and portal schedule wiring (destined for [the windowed host],
//! `ambition_host`). Those tighten as the E-track carves land — "assemble with
//! what exists; tighten as carves land". The group grows; its consumers don't
//! change.
//!
//! Presentation, audio, windowing, dev tools, and CONTENT are never in this
//! group — they are the game's / host's responsibility.

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};

mod combat_schedule;
pub use combat_schedule::CombatSchedulePlugin;

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
        ambition_gameplay_core::schedule::configure_sandbox_sets(app);
        // Shrine activation pulse (interaction → save flash).
        app.init_resource::<ambition_gameplay_core::shrine::ShrineActivationPulse>();
        // Slot-keyed gesture/buffer authority (double-tap, interact buffer).
        // Local input publishes it; body mode / interaction / transitions
        // consume it for the controlled body's slot.
        app.init_resource::<ambition_gameplay_core::player::SlotInteractionState>();
        // Which character the local player spawns as (empty = the
        // content-installed default). Hosts pre-insert to override.
        app.init_resource::<ambition_gameplay_core::player::StartingCharacter>();
    }
}

/// The engine's content-free simulation plugin group (see module docs).
pub struct PlatformerEnginePlugins;

impl PluginGroup for PlatformerEnginePlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            // Sets + engine resources FIRST (see SandboxSetsPlugin docs).
            .add(SandboxSetsPlugin)
            // The world-prep phase (body integration, gravity collection, etc.).
            .add(ambition_gameplay_core::features::WorldPrepSchedulePlugin)
            // Universal-brain messages/resources (player/NPC/enemy/boss).
            .add(ambition_characters::brain::BrainPlugin)
            // Traversal ability/weapon kit + shared app state.
            .add(ambition_gameplay_core::abilities::AmbitionAbilitiesPlugin)
            // The emitted player trail substrate.
            .add(ambition_gameplay_core::player::trail::PlayerTrailPlugin)
            // Gravity zones/switches + the ambient-gravity snapshot.
            .add(ambition_gameplay_core::gravity::GravityPlugin)
            // Item pickup simulation.
            .add(ambition_gameplay_core::items::pickup::ItemPickupSimulationPlugin)
            // Feature (room-entity) collection + interaction schedules.
            .add(ambition_gameplay_core::features::FeatureCollectionSchedulePlugin)
            .add(ambition_gameplay_core::features::FeatureInteractionSchedulePlugin)
            // LDtk runtime spine (room load/transition spine).
            .add(ambition_gameplay_core::ldtk_world::LdtkRuntimeSpinePlugin)
            // Encounter + cutscene simulation schedules.
            .add(ambition_gameplay_core::encounter::EncounterSimulationSchedulePlugin)
            .add(ambition_gameplay_core::cutscene::CutsceneSchedulePlugin)
            // Gameplay effects + feature view-sync schedules.
            .add(ambition_gameplay_core::features::GameplayEffectsSchedulePlugin)
            .add(ambition_gameplay_core::features::FeatureViewSyncSchedulePlugin)
            // Sandbox reset schedule.
            .add(ambition_gameplay_core::session::reset::SandboxResetSchedulePlugin)
            // Deterministic sim traces.
            .add(ambition_gameplay_core::trace::TraceSchedulePlugin)
            // Per-frame affordance table (what would each verb do right now).
            .add(ambition_gameplay_core::player::affordances::AffordancesPlugin)
            // The camera OBSERVATION seam (E4-17): the sim resolves ONE
            // follow-camera snapshot per tick (the only CameraEaseState
            // writer); presentation consumes it. Headless/RL readers too.
            .add(ambition_gameplay_core::camera_snapshot::CameraObservationPlugin)
            // The combat-phase chain + the content extension slots
            // (CombatSet::ContentSpecials / ContentFlavor).
            .add(CombatSchedulePlugin);
        builder
    }
}

/// Engine states every entry point must initialize after Bevy's `StatesPlugin`
/// exists and before the sim plugins build (their run conditions read the
/// state). One call site per app instead of a copy-pasted `init_state`.
pub fn init_engine_states(app: &mut App) {
    use bevy::state::app::AppExtStates as _;
    app.init_state::<ambition_gameplay_core::game_mode::GameMode>();
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
