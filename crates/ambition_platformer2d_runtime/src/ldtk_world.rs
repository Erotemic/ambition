//! **The LDtk world's runtime install — offered by the engine, accepted by a
//! game that HAS an LDtk world.**
//!
//! ⭐ **a format installs its own spine.** Until 2026-08-16
//! [`crate::PlatformerEnginePlugins`] added
//! `ambition_platformer2d_ldtk::LdtkRuntimeSpinePlugin` unconditionally and
//! `register_engine_rollback_state` registered `root.ldtk_runtime_index` in the
//! actors domain, so five RON-authored games — Sanic, Mary-O, Twintrack, Smash,
//! the versus stage — installed six LDtk index resources, a six-system sim
//! chain, and one LDtk row in their wire format for an authoring format none of
//! them uses.
//!
//! D135 got half of this: the index became optional session state, which made
//! `run_if(ldtk_world_installed)` statable and stopped the chain EXECUTING in
//! those games. ⛔ but a plugin that is added and then declines to run is still
//! added — its resources are still initialized, its systems are still in the
//! schedule graph, and its component is still in the snapshot schema. This is
//! the other half: the composition never mentions LDtk unless the game has one.
//!
//! ⚠ **why this plugin lives in the runtime crate and not in
//! `ambition_platformer2d_ldtk`.** The rollback registration vocabulary
//! (`AmbitionRollbackApp::rollback_component_clone_checksum`) is this crate's;
//! `ambition_platformer2d_core::snapshot::RollbackRegistrar` — the floor that
//! lets a domain register itself — carries only the RESOURCE method today, and
//! the LDtk index is a COMPONENT on the session root. So the format's rollback
//! row is registered here, next to every other domain adapter, and this plugin
//! is the single thing a game adds to say *"I have an LDtk world"*.

use bevy::app::{App, Plugin};

/// **A game with an LDtk world adds this; nothing else does.**
///
/// Installs the LDtk runtime spine (index rebuild chain + its index resources)
/// and registers the LDtk runtime index's rollback row. Deliberately NOT part
/// of [`crate::PlatformerEnginePlugins`] — see the module doc.
///
/// ⚠ add it AFTER the engine group. The rollback registrar reads
/// [`crate::SimulationHost`] to decide whether a recorded descriptor also
/// becomes live `bevy_ggrs` machinery, and the group is what sets it.
pub struct LdtkWorldPlugin;

impl Plugin for LdtkWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ambition_platformer2d_ldtk::LdtkRuntimeSpinePlugin);
        crate::rollback::domains::ldtk::register(app);
    }
}
