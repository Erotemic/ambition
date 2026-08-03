//! Saved game, quest, and settings shapes for Ambition.
//!
//! This crate owns the data that can be stored, loaded, and mirrored into
//! Bevy resources. Menu/UI policy stays above this crate and reads these typed
//! settings instead of owning their serialized shape.

pub mod host;
pub mod quest;
pub mod save;

/// The directory this App reads and writes its settings, save and developer
/// files in. Defaults to the platform data dir, so production is unchanged.
///
/// ⚠ **it exists so that "where my files are" is an APP fact.** As a global it
/// was shared by every test in a binary and by every process on the machine.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct PersistenceRoot(pub std::path::PathBuf);

impl Default for PersistenceRoot {
    fn default() -> Self {
        Self(settings::platform_paths::data_dir_root())
    }
}

impl PersistenceRoot {
    /// A private directory nobody else writes — for an App that is not a
    /// player's session.
    ///
    /// ⭐ **the symmetry to keep in mind**: a windowless host already redirects
    /// AUDIO away from the user's speakers (`AudioOutputMode::Recording`). This
    /// is the same rule for the other side effect a non-session App should not
    /// have — writing the user's settings and save.
    ///
    /// Unique per call: process id plus a counter, so two Apps in one test
    /// binary do not share a root either. Nothing cleans these up, deliberately
    /// — a few empty directories under the temp dir are cheaper than a harness
    /// that deletes paths, and the OS reclaims them.
    pub fn isolated() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        Self(
            std::env::temp_dir()
                .join("ambition-app-state")
                .join(format!("{}-{unique}", std::process::id())),
        )
    }
}
pub mod save_data;
pub mod settings;

/// Schedules user-settings and sandbox-save persistence for visible builds.
/// Headless / RL drivers omit this plugin so they never read or write user files.
pub struct PersistenceSchedulePlugin;

impl bevy::prelude::Plugin for PersistenceSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs as _, Startup, Update};

        // ⭐ **WHERE THIS APP KEEPS ITS FILES, as app state rather than as an
        // ambient process fact.** The path used to be read from the environment
        // at every call, which made it PER-USER: every `app_it` test shared one
        // settings file, one save and one developer file — with each other, and
        // with every other checkout and session on the machine, concurrently.
        // Nothing declared that, and a test cannot opt out of a global.
        //
        // `init_resource` keeps production behaviour exactly (the default IS the
        // platform dir) while letting any App — a test, a tool, a second
        // instance — declare its own root before adding this plugin.
        app.init_resource::<PersistenceRoot>();
        app.init_resource::<save::SaveFileWritable>()
            .init_resource::<save::LastPersistedSave>()
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
                        .run_if(ambition_platformer2d_core::world_state_is_confirmed),
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
