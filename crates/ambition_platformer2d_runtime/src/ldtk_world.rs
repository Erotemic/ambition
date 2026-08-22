//! Opt-in host composition for games that use an LDtk world. Installs the LDtk
//! runtime spine and its format-owned rollback registration; it is not part of
//! the default engine plugin group.

use bevy::app::{App, Plugin};

/// A game with an LDtk world adds this; nothing else does.
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
