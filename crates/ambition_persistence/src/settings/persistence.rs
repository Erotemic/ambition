//! Disk persistence for `UserSettings`.
//!
//! Settings are user-global (not per-save) so they live alongside the
//! sandbox save file under the OS data dir, not inside any particular
//! save slot. The wire format is RON to match the rest of the
//! `assets/ambition/platformer_defaults.ron` family — easy to read, easy to
//! hand-edit if a knob ends up out of range.
//!
//! All I/O failures are non-fatal: a missing file is "use defaults",
//! and a corrupt file logs a warning and falls back to defaults. The
//! goal is that the user can always launch the sandbox.

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};

use bevy::log::warn;
use bevy::prelude::*;

use super::platform_paths::data_dir_root;
use super::UserSettings;

/// Where the settings file lives on disk relative to the user's data
/// dir. The sandbox passes this through `data_dir().join(SETTINGS_FILE)`.
pub const SETTINGS_FILE: &str = "ambition/settings.ron";

/// Resolve the absolute path of the settings file for the live build.
pub fn settings_path() -> PathBuf {
    settings_path_under(&data_dir_root())
}

pub fn settings_path_under(root: &Path) -> PathBuf {
    root.join(SETTINGS_FILE)
}

/// Load `UserSettings` from `path`. Returns defaults if the file is
/// missing or unreadable; logs a warning on parse failure and returns
/// defaults.
pub fn load_settings(path: &Path) -> UserSettings {
    let bytes = match crate::store::read(path) {
        Ok(s) => s,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return UserSettings::default();
        }
        Err(error) => {
            warn!(
                target: "ambition_platformer2d::settings",
                "could not read settings file {}: {error}; using defaults",
                path.display()
            );
            return UserSettings::default();
        }
    };
    match ron::from_str::<UserSettings>(&bytes) {
        Ok(mut settings) => {
            settings.clamp_all();
            settings
        }
        Err(error) => {
            warn!(
                target: "ambition_platformer2d::settings",
                "could not parse settings file {}: {error}; using defaults",
                path.display()
            );
            UserSettings::default()
        }
    }
}

/// Save `UserSettings` to `path`. Creates the parent directory if
/// needed; writes via temp file + rename so a crash mid-write cannot
/// corrupt the live file. Returns the IO error on failure so the caller
/// can decide whether to surface it (most callers log + continue).
pub fn save_settings(path: &Path, settings: &UserSettings) -> std::io::Result<()> {
    let body = ron::ser::to_string_pretty(settings, ron::ser::PrettyConfig::default())
        .map_err(|error| std::io::Error::other(format!("ron serialize: {error}")))?;
    install_settings(path, &body)
}

/// ⭐ ONE SYNCHRONOUS CALL, SO NOTHING TO MAKE ATOMIC. See `save::install_save`
/// for the same split and the same reason.
#[cfg(target_arch = "wasm32")]
fn install_settings(path: &Path, body: &str) -> std::io::Result<()> {
    crate::store::write(path, body)
}

/// ⭐ TEMP FILE PLUS RENAME, because a filesystem write is many syscalls and a
/// crash between them leaves a truncated settings file.
#[cfg(not(target_arch = "wasm32"))]
fn install_settings(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("ron.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Bevy startup system: load settings into `Res<UserSettings>` from
/// disk if a file exists. The default `UserSettings` is already
/// inserted in `init_sandbox_resources`, so this only overrides when
/// a file is found.
fn load_existing_settings(
    path: &Path,
    settings: &mut UserSettings,
    last: &mut LastPersistedSettings,
) -> bool {
    if !crate::store::exists(path) {
        return false;
    }
    *settings = load_settings(path);
    // The file we just loaded is already the persisted value. Seeding the
    // comparison shadow prevents the first Update from rewriting an unchanged
    // file merely because `LastPersistedSettings` started at `None`.
    last.0 = Some(settings.clone());
    true
}

pub fn load_settings_at_startup(
    mut settings: ResMut<UserSettings>,
    mut last: ResMut<LastPersistedSettings>,
    root: Res<crate::PersistenceRoot>,
) {
    let path = settings_path_under(&root.0);
    if !load_existing_settings(&path, &mut settings, &mut last) {
        return;
    }
    info!(
        target: "ambition_platformer2d::settings",
        "loaded user settings from {}",
        path.display()
    );
}

/// What was last committed to disk, so the writer can ask whether the file is
/// still correct rather than whether Bevy saw a mutation.
#[derive(Resource, Clone, Debug, Default)]
// the TYPE must exist on every platform — the wasm no-op systems take it as a
// parameter so the schedule is identical — but only the native writer READS the
// value. `cfg`-ing the field away would change the type per platform; this says
// the truth instead: unread here, not unused.
pub struct LastPersistedSettings(
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))] Option<UserSettings>,
);

/// Bevy update system: write `UserSettings` to disk when it no longer matches
/// what is there.
///
/// # Why this is NOT gated on confirmed simulation state
///
/// Its sibling `autosave_sandbox_save` is, because `AmbitionGameSave` is registered
/// rollback state that a speculating host will rewind — writing it mid-
/// prediction records a guess as history. `UserSettings` is not simulation
/// state at all: it is not rollback-registered, and every writer is menu or
/// pause-UI side. Delaying a settings write until nothing is predicted would
/// buy no correctness and could drop a preference the player just set while a
/// session happens to be running.
///
/// The half that IS shared is dropping change detection. `Res::is_changed` is
/// consumed by a system that ran and declined to write, so any guard placed in
/// front of it can silently swallow a real change; a value comparison cannot.
///
/// If a simulation system ever writes `UserSettings`, this reasoning expires
/// and the confirmation gate becomes required — the settings would then be
/// speculative like anything else the sim touches.
pub fn save_settings_on_change(
    settings: Res<UserSettings>,
    mut last: ResMut<LastPersistedSettings>,
    root: Res<crate::PersistenceRoot>,
) {
    if last.0.as_ref() == Some(&*settings) {
        return;
    }
    let path = settings_path_under(&root.0);
    match save_settings(&path, &settings) {
        Ok(()) => last.0 = Some(settings.clone()),
        Err(error) => warn!(
            target: "ambition_platformer2d::settings",
            "failed to write settings file {}: {error}",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests;
