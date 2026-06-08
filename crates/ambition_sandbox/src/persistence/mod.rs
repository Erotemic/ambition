//! Save game I/O and the user-settings store.
//!
//! - `save` — autosave + load of the per-session `Save` resource.
//! - `settings` — typed model + persistence for audio / video /
//!   controls / gameplay preferences.

pub mod save;
pub mod settings;

/// Module-local Bevy plugin: schedules the user-settings + sandbox-save
/// persistence systems.
///
/// Two Startup loaders (settings, developer toggles, sandbox save) and
/// three Update writers (the same trio, gated on `Res::is_changed`).
/// Carved out of `app/plugins.rs::install_settings_and_save_systems`
/// per OVERNIGHT-TODO #6; visible builds register this through the
/// presentation install chain and headless / RL drivers omit it so a
/// `cargo run --bin headless` never reads or writes user files.
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
