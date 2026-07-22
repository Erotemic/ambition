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
        use bevy::prelude::{IntoScheduleConfigs as _, Startup, Update};

        app.init_resource::<save::LastPersistedSave>()
            .init_resource::<settings::persistence::LastPersistedSettings>()
            .add_systems(
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
                    // The sandbox save IS rollback state, so a speculating host
                    // must not commit it to disk while anything is predicted.
                    // On every non-rollback host this condition is always true
                    // and the behaviour is unchanged. See `autosave_sandbox_save`.
                    save::autosave_sandbox_save
                        .run_if(ambition_engine_core::world_state_is_confirmed),
                ),
            );
    }
}

/// Serializes every test that repoints `AMBITION_DATA_DIR`.
///
/// That variable is process-global, and both the save and the settings suites
/// resolve real on-disk paths through it. One lock per module is not mutual
/// exclusion — the suites raced, and each other's scratch directory looked like
/// a missing or unexpected file. A poisoned lock is deliberately tolerated:
/// otherwise the first genuine assertion failure cascades into every later test
/// reporting `PoisonError` instead of its own result.
#[cfg(test)]
pub(crate) fn lock_data_dir() -> std::sync::MutexGuard<'static, ()> {
    static DATA_DIR: std::sync::Mutex<()> = std::sync::Mutex::new(());
    DATA_DIR
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
