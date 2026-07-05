//! The platformer ENGINE face (demo plan E5): [`PlatformerEnginePlugins`], a
//! Bevy [`PluginGroup`] that assembles the **content-free simulation plugins**
//! shared by every platformer built on this engine.
//!
//! ## Why this crate
//!
//! A game — Ambition, or a demo (`demos/demo_sanic`, `demos/demo_smb`) — builds
//! its simulation App by adding this group plus its OWN content crate:
//!
//! ```ignore
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins)
//!    .add_plugins(my_content::MyGameContentPlugin);
//! ```
//!
//! This is **the demo gate**: a demo app depends on `ambition_runtime`, never on
//! `ambition_app`. The group carries only plugins that name no content and reach
//! for no app-local system — the sim schedule, the universal brain, gravity,
//! traversal abilities, item pickups, encounters/cutscenes, feature collection/
//! interaction/effects/view-sync, room reset, traces, and affordances.
//!
//! ## What is deliberately NOT here (yet)
//!
//! The first assembly (E5) migrates the unconditional, unentangled engine
//! plugins. Still assembled app-side because they either wrap app-local systems
//! or need host-specific ordering: the sandbox resource plugin, combat/
//! progression schedules, the per-frame player input/simulation/room-transition/
//! presentation-sync system registrations, and the portal schedule wiring.
//! Those tighten into this group (or a thin host adapter) as the E-track carves
//! land — "assemble with what exists; tighten as carves land". The group grows;
//! its consumers don't change.
//!
//! Presentation, audio, windowing, dev tools, and CONTENT are never in this
//! group — they are the game's / host's responsibility.

use bevy::app::{PluginGroup, PluginGroupBuilder};

/// The engine's content-free simulation plugin group (see module docs).
pub struct PlatformerEnginePlugins;

impl PluginGroup for PlatformerEnginePlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
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
            .add(ambition_gameplay_core::player::affordances::AffordancesPlugin);
        builder
    }
}
