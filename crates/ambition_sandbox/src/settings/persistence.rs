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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Difficulty, FlashIntensity};
    use std::cell::Cell;
    use std::sync::Mutex;

    static TEST_DIR_LOCK: Mutex<()> = Mutex::new(());
    thread_local!(static UNIQUE: Cell<u64> = const { Cell::new(0) });

    fn temp_root(name: &str) -> PathBuf {
        let counter = UNIQUE.with(|c| {
            let next = c.get() + 1;
            c.set(next);
            next
        });
        let mut p = std::env::temp_dir();
        p.push(format!(
            "ambition_settings_{name}_{}_{}",
            std::process::id(),
            counter
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn missing_file_returns_defaults() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("missing");
        let path = settings_path_under(&root);
        let s = load_settings(&path);
        assert_eq!(s, UserSettings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("round_trip");
        let path = settings_path_under(&root);
        let mut s = UserSettings::default();
        s.audio.master_volume = 0.42;
        s.gameplay.difficulty = Difficulty::Hard;
        s.video.flashes = FlashIntensity::Off;
        save_settings(&path, &s).unwrap();
        let restored = load_settings(&path);
        assert_eq!(restored, s);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("corrupt");
        let path = settings_path_under(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"this is not valid RON ::: garbage").unwrap();
        let s = load_settings(&path);
        assert_eq!(s, UserSettings::default());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn save_clamps_values_back_into_range_on_load() {
        // Settings clamp on load even if the on-disk value is out of
        // range; this protects the sandbox from a hand-edit that puts
        // master_volume = 5.0.
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("clamp");
        let path = settings_path_under(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let raw = r#"(
            video: (
                display_mode: Windowed,
                camera_zoom: Normal,
                flashes: On,
                colorblind: Off,
            ),
            audio: (
                master_volume: 5.0,
                music_volume: -0.4,
                sfx_volume: 0.5,
                muted: false,
                muted_snapshot_master: 0.85,
            ),
            controls: (
                keyboard_preset_index: 0,
                controller_profile: Default,
                left_stick_deadzone: 0.18,
                right_stick_deadzone: 0.20,
                trigger_release_threshold: 0.30,
                trigger_press_threshold: 0.55,
                dpad_menu_navigation: true,
                invert_aim_y: false,
                dash_input_mode: Trigger,
                menu_repeat_initial_delay: 0.32,
                menu_repeat_interval: 0.12,
            ),
            gameplay: (
                difficulty: Medium,
                assist: Off,
                player_damage_multiplier: 1.0,
                trace_auto_dump: true,
            ),
        )"#;
        fs::write(&path, raw).unwrap();
        let s = load_settings(&path);
        assert!(s.audio.master_volume <= 1.0);
        assert!(s.audio.music_volume >= 0.0);
        let _ = fs::remove_dir_all(&root);
    }
}
