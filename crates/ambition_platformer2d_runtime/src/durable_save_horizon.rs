//! Host installation of the durable save horizon.
//!
//! A save is simulation/session state, not presentation, so every composition
//! that simulates a world installs this plugin. The concrete durable adapters
//! live with the actor/item domains that understand their values; this runtime
//! wrapper composes that typed offer without enumerating systems or checkpoint
//! baseline resources.
//!
//! The installed systems remain in top-level `Update`, outside rollback
//! resimulation. Their state is rewindable where required, but file/application
//! side effects themselves are not replayed as simulation ticks.

use bevy::prelude::{App, Plugin};

/// Install durable save/load participation for every platformer composition.
pub struct DurableSaveHorizonPlugin;

impl Plugin for DurableSaveHorizonPlugin {
    fn build(&self, app: &mut App) {
        ambition_platformer2d_session::durable_horizon::install_durable_save_horizon(
            app,
        );
        ambition_platformer2d_items::persist::install_item_durable_horizon(app);
        // The extension host's durable road, where the composition has the
        // host. Without this road the host refuses a durable module schema.
        if app.is_plugin_added::<ambition_extension_host::ExtensionHostPlugin>() {
            crate::module_save_road::install_module_save_road(app);
        }
    }
}
