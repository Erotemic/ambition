//! Compatibility adapter for persistence paths that still sit inside the
//! gameplay-core UI/dev surface.
//!
//! The stored save/settings/quest shapes moved to `ambition_persistence` in
//! E1a; `DeveloperTools` disk persistence moved to `ambition_dev_tools` in E1d.
//! The local residue is the settings/menu IR, still tied to gameplay-core state
//! until E1e.

pub use ambition_persistence::{save, save_data, PersistenceSchedulePlugin};

pub mod settings;

/// Schedules developer-tool persistence. The user settings and sandbox save
/// schedule lives in `ambition_persistence::PersistenceSchedulePlugin`.
pub struct DeveloperPersistenceSchedulePlugin;

impl bevy::prelude::Plugin for DeveloperPersistenceSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{Startup, Update};
        app.add_systems(
            Startup,
            ambition_dev_tools::persistence::load_developer_at_startup,
        )
        .add_systems(
            Update,
            ambition_dev_tools::persistence::save_developer_on_change,
        );
    }
}
