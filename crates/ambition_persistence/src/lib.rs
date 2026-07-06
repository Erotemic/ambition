//! Saved game, quest, and settings shapes for Ambition.
//!
//! This crate owns the data that can be stored, loaded, and mirrored into
//! Bevy resources. Menu/UI policy stays above this crate and reads these typed
//! settings instead of owning their serialized shape.

pub mod host;
pub mod quest;
pub mod save;
pub mod save_data;
pub mod settings;

/// Schedules user-settings and sandbox-save persistence for visible builds.
/// Headless / RL drivers omit this plugin so they never read or write user files.
pub struct PersistenceSchedulePlugin;

impl bevy::prelude::Plugin for PersistenceSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{Startup, Update};
        app.add_systems(
            Startup,
            (
                settings::persistence::load_settings_at_startup,
                save::load_save_at_startup,
            ),
        )
        .add_systems(
            Update,
            (
                settings::persistence::save_settings_on_change,
                save::autosave_sandbox_save,
            ),
        );
    }
}
