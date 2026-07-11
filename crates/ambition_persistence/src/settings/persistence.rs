//! Disk persistence for `UserSettings`.
//!
//! Settings are user-global (not per-save) so they live alongside the
//! sandbox save file under the OS data dir, not inside any particular
//! save slot. The wire format is RON to match the rest of the
//! `assets/ambition/sandbox.ron` family — easy to read, easy to
//! hand-edit if a knob ends up out of range.
//!
//! All I/O failures are non-fatal: a missing file is "use defaults",
//! and a corrupt file logs a warning and falls back to defaults. The
//! goal is that the user can always launch the sandbox.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::log::{info, warn};
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
    let bytes = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return UserSettings::default();
        }
        Err(error) => {
            warn!(
                target: "ambition::settings",
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
                target: "ambition::settings",
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = ron::ser::to_string_pretty(settings, ron::ser::PrettyConfig::default())
        .map_err(|error| std::io::Error::other(format!("ron serialize: {error}")))?;
    let tmp = path.with_extension("ron.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Bevy startup system: load settings into `Res<UserSettings>` from
/// disk if a file exists. The default `UserSettings` is already
/// inserted in `init_sandbox_resources`, so this only overrides when
/// a file is found.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_settings_at_startup(mut settings: ResMut<UserSettings>) {
    let path = settings_path();
    if !path.exists() {
        return;
    }
    *settings = load_settings(&path);
    info!(
        target: "ambition::settings",
        "loaded user settings from {}",
        path.display()
    );
}

/// Bevy update system: when `UserSettings` changes (via the pause
/// menu), write the new state to disk. Throttled by checking
/// `Res::is_changed` so we don't write every frame.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_settings_on_change(settings: Res<UserSettings>) {
    if !settings.is_changed() {
        return;
    }
    let path = settings_path();
    if let Err(error) = save_settings(&path, &settings) {
        warn!(
            target: "ambition::settings",
            "failed to write settings file {}: {error}",
            path.display()
        );
    }
}

/// Wasm (browser) no-op for settings loading. First-pass web build does
/// not persist user settings; the in-memory `Res<UserSettings>` keeps
/// the defaults for the session. Browser persistence is a follow-up.
#[cfg(target_arch = "wasm32")]
pub fn load_settings_at_startup(_settings: ResMut<UserSettings>) {}

/// Wasm (browser) no-op for settings writing. See [`load_settings_at_startup`].
#[cfg(target_arch = "wasm32")]
pub fn save_settings_on_change(_settings: Res<UserSettings>) {}

#[cfg(test)]
mod tests;
