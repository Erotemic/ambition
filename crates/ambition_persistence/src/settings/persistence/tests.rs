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
    let _g = TEST_DIR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let root = temp_root("missing");
    let path = settings_path_under(&root);
    let s = load_settings(&path);
    assert_eq!(s, UserSettings::default());
}

#[test]
fn save_then_load_round_trips() {
    let _g = TEST_DIR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
    let _g = TEST_DIR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
    let _g = TEST_DIR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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

#[test]
fn loading_an_existing_file_seeds_the_persisted_value_shadow() {
    let _g = TEST_DIR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let root = temp_root("seed_shadow");
    let path = settings_path_under(&root);
    let mut expected = UserSettings::default();
    expected.audio.master_volume = 0.37;
    save_settings(&path, &expected).unwrap();

    let mut settings = UserSettings::default();
    let mut last = LastPersistedSettings::default();
    assert!(load_existing_settings(&path, &mut settings, &mut last));

    assert_eq!(settings, expected);
    assert_eq!(
        last.0.as_ref(),
        Some(&expected),
        "the first Update must see that the loaded file is already current"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The BEHAVIOUR the shadow-seeding exists for, not just the mechanism.
///
/// `loading_an_existing_file_seeds_the_persisted_value_shadow` asserts the
/// shadow is populated, which is the thing the fix writes. This asserts what a
/// user observes, by running the REAL startup and save systems: booting with a
/// settings file already on disk must not rewrite it. Deleting the file between
/// the two systems and proving it does not reappear is an exact "no write
/// happened" probe.
#[test]
fn startup_with_an_unchanged_file_does_not_rewrite_it() {
    // The SHARED lock: this test repoints the process-global data dir, which
    // the save suite also resolves through.
    let _g = crate::lock_data_dir();
    let root = temp_root("no_startup_rewrite");
    std::env::set_var("AMBITION_DATA_DIR", &root);

    let path = settings_path_under(&root);
    let mut stored = UserSettings::default();
    stored.audio.master_volume = 0.37;
    save_settings(&path, &stored).unwrap();

    let mut world = World::new();
    world.init_resource::<UserSettings>();
    world.init_resource::<LastPersistedSettings>();
    world
        .run_system_cached(load_settings_at_startup)
        .expect("startup load runs");
    assert_eq!(
        world.resource::<UserSettings>().audio.master_volume,
        0.37,
        "the stored file must actually have been loaded, or this proves nothing"
    );

    fs::remove_file(&path).unwrap();
    world
        .run_system_cached(save_settings_on_change)
        .expect("save pass runs");

    assert!(
        !path.exists(),
        "startup rewrote a settings file that nothing had changed"
    );
    std::env::remove_var("AMBITION_DATA_DIR");
    let _ = fs::remove_dir_all(&root);
}
