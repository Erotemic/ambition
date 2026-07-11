//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
            camera_zoom: Combat,
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
