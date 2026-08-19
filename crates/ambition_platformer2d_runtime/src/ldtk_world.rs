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
//! ⚠ **why this plugin still lives in the runtime crate.** The LDtk crate now
//! owns the rollback declaration itself through the backend-neutral
//! `RollbackRegistrar`; this plugin owns only host composition. A game adds one
//! thing to say *"I have an LDtk world"*: install the runtime spine, borrow the
//! host's rollback registrar, and hand it to the format-owned declaration.

use bevy::app::{App, Plugin};

/// **A game with an LDtk world adds this; nothing else does.**
///
/// Installs the LDtk runtime spine (index rebuild chain + its index resources)
/// and registers the LDtk runtime index's rollback row. Deliberately NOT part
/// of [`crate::PlatformerEnginePlugins`] — see the module doc.
///
/// Add it after the engine group so its schema contribution joins the same
/// prepared-content identity assembled by the runtime.
pub struct LdtkWorldPlugin;

impl Plugin for LdtkWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ambition_platformer2d_ldtk::LdtkRuntimeSpinePlugin);
        let mut registrar = crate::rollback::SchemaRollbackRegistrar::new(app);
        ambition_platformer2d_ldtk::register_rollback_state(&mut registrar);
    }
}
