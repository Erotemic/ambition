//! Save game I/O and the user-settings store.
//!
//! - `save_data` — the pure save-game data shapes (was `crate::save`).
//! - `save` — autosave + load of the per-session `Save` resource.
//! - `settings` — typed model + persistence for audio / video /
//!   controls / gameplay preferences.

pub mod save;
/// Save-game *data shapes* (pure data + serde; was the root `crate::save`).
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
                settings::persistence::load_developer_at_startup,
                save::load_save_at_startup,
            ),
        )
        .add_systems(
            Update,
            (
                settings::persistence::save_settings_on_change,
                settings::persistence::save_developer_on_change,
                save::autosave_sandbox_save,
            ),
        );
    }
}
