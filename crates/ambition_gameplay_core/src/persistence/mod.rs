//! Compatibility adapter for persistence paths that still sit inside the
//! gameplay-core UI/dev surface.
//!
//! The stored save/settings/quest shapes moved to `ambition_persistence` in
//! E1a. The local residue is the settings/menu IR and dev-tool persistence,
//! both still tied to gameplay-core state until E1d/E1e.

pub use ambition_persistence::{save, save_data, PersistenceSchedulePlugin};

pub mod settings;

/// Schedules developer-tool persistence. The user settings and sandbox save
/// schedule lives in `ambition_persistence::PersistenceSchedulePlugin`.
pub struct DeveloperPersistenceSchedulePlugin;

impl bevy::prelude::Plugin for DeveloperPersistenceSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{Startup, Update};
        app.add_systems(Startup, settings::persistence::load_developer_at_startup)
            .add_systems(Update, settings::persistence::save_developer_on_change);
    }
}
