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
        last.persisted.as_ref(),
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
    // The root is APP state now; these systems read it instead of the process env.
    world.init_resource::<crate::PersistenceRoot>();
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

/// A settings file written before the BURST rename still loads, whole.
///
/// It has to be: the field carries no `#[serde(default)]` and `ControlSettings` has no
/// container default, so a key this struct cannot find is a deserialize error for the WHOLE
/// struct — and `load_settings` answers a parse error by discarding the entire file and
/// returning `UserSettings::default()`. Video, audio, gameplay, the keyboard preset and every
/// binding override would go with it, on a warning line nobody reads.
///
///  the assertions below are why `save_clamps_values_back_into_range_on_load`
/// does not cover this. That test feeds a file of the same vintage and then
/// asserts only `master_volume <= 1.0` and `music_volume >= 0.0` — both of which
/// the DEFAULTS satisfy. It reports green on a total wipe. So this one asserts
/// the poison too: a value that is not the default, read back as itself.
#[test]
fn a_pre_burst_settings_file_keeps_its_saved_preferences() {
    use crate::settings::controls::BurstInputMode;

    let _g = TEST_DIR_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let root = temp_root("pre_burst_wire");
    let path = settings_path_under(&root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    // Exactly what a shipped build wrote before the rename, down to the key
    // spelling. Every value chosen to differ from its default.
    let raw = r#"(
        controls: (
            keyboard_preset_index: 1,
            controller_profile: Default,
            left_stick_deadzone: 0.33,
            right_stick_deadzone: 0.22,
            trigger_release_threshold: 0.30,
            trigger_press_threshold: 0.55,
            dpad_menu_navigation: false,
            invert_aim_y: true,
            dash_input_mode: Both,
            menu_repeat_initial_delay: 0.40,
            menu_repeat_interval: 0.20,
        ),
    )"#;
    fs::write(&path, raw).unwrap();

    let s = load_settings(&path);

    assert_ne!(
        s,
        UserSettings::default(),
        "the file did not parse at all: `load_settings` swallowed the error and \
         handed back defaults, which is a SILENT wipe of the player's settings"
    );
    assert_eq!(
        s.controls.burst_input_mode,
        BurstInputMode::Both,
        "the saved burst-input preference did not survive the rename; the \
         `#[serde(rename = \"dash_input_mode\")]` pin on `burst_input_mode` is \
         what carries it"
    );
    // The neighbours prove the whole struct came through, not just the one key.
    assert_eq!(s.controls.keyboard_preset_index, 1);
    assert!(s.controls.invert_aim_y);
    assert!(!s.controls.dpad_menu_navigation);
    let _ = fs::remove_dir_all(&root);
}

/// A settings file written before a knob existed must still load — ALL of it.
///
/// ⛔⛔ A MISSING KEY DISCARDS THE WHOLE FILE. `ControlSettings` has no container
/// default, so a field without `#[serde(default)]` turns every older save into a
/// deserialize error, and `load_settings` answers a parse error by returning
/// `UserSettings::default()` — throwing away video, audio, gameplay, presets and
/// every binding override the player had. `burst_input_mode`'s own doc records
/// that hazard from the time it was nearly caused by a rename; this is the test
/// that catches the next instance, which was `right_stick_mode` on 2026-08-31.
///
/// ⭐ THE FIXTURE IS A REAL SAVE WITH ONE LINE DELETED, not a hand-written stub.
/// A stub goes stale the moment a field is added and then proves nothing about
/// the file players actually have.
#[test]
fn a_settings_file_predating_a_knob_still_loads_everything_else() {
    let _guard = TEST_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_root("forward_compat");
    let path = root.join("settings.ron");

    // Something non-default in a DIFFERENT section, so a discarded file is
    // visible as more than a defaulted knob.
    let mut written = UserSettings::default();
    written.gameplay.difficulty = Difficulty::Hard;
    save_settings(&path, &written).expect("settings save");

    let body = std::fs::read_to_string(&path).expect("read back");
    assert!(
        body.contains("right_stick_mode"),
        "the knob is not in the saved form, so removing it proves nothing"
    );
    let older: String = body
        .lines()
        .filter(|line| !line.contains("right_stick_mode"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, older).expect("write the older file");

    let loaded = load_settings(&path);
    assert_eq!(
        loaded.gameplay.difficulty,
        Difficulty::Hard,
        "a settings file predating `right_stick_mode` was DISCARDED — the player \
         lost every other setting they had. The field needs `#[serde(default)]`"
    );
    assert_eq!(
        loaded.controls.right_stick_mode,
        crate::settings::RightStickMode::Aim,
        "the absent knob should read as its default"
    );
}

/// ⛔⛔ **ONE REFUSED WRITE USED TO BECOME A 60 Hz RETRY.** The writer advances
/// its shadow only on success, so a store that says no — a full disk, a browser
/// with site data blocked — leaves `persisted != settings` permanently true.
/// Every Update then re-serialized, re-wrote and re-warned, for the rest of the
/// session, about a condition that had not changed.
///
/// The three arms are the whole contract: it attempts once, it does NOT attempt
/// again while nothing has changed, and it DOES attempt again the moment
/// something does — a latch that never lifts would fail a transient outage
/// closed and silently stop persisting.
#[test]
fn a_refused_write_is_attempted_once_and_retried_only_when_the_value_changes() {
    let _g = crate::lock_data_dir();
    let root = temp_root("refused_write");
    std::env::set_var("AMBITION_DATA_DIR", &root);
    let path = settings_path_under(&root);

    // Block the write: put a regular FILE where the settings DIRECTORY must go,
    // so `create_dir_all` fails. Nothing about this is browser-specific — it is
    // the same `Err` road `localStorage` takes when the origin is blocked.
    fs::create_dir_all(root.parent().unwrap()).unwrap();
    let blocker = path.parent().unwrap();
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(blocker, b"not a directory").unwrap();

    let mut world = World::new();
    world.init_resource::<UserSettings>();
    world.init_resource::<LastPersistedSettings>();
    world.init_resource::<crate::PersistenceRoot>();
    world.resource_mut::<UserSettings>().audio.master_volume = 0.11;

    world
        .run_system_cached(save_settings_on_change)
        .expect("the write pass runs");
    assert!(
        world.resource::<LastPersistedSettings>().refused.is_some(),
        "the store refused; without remembering WHICH value was refused there is \
         nothing to stop the next Update trying the identical write"
    );
    assert!(
        world
            .resource::<LastPersistedSettings>()
            .persisted
            .is_none(),
        "and a refusal is not a persist — the shadow must NOT advance"
    );

    // Clear the obstruction. A writer that retried every Update would now
    // succeed, which is exactly what makes this observable.
    fs::remove_file(blocker).unwrap();
    world
        .run_system_cached(save_settings_on_change)
        .expect("the second pass runs");
    assert!(
        !path.exists(),
        "nothing changed, so nothing was owed; a file here means the writer is \
         retrying a refused write every frame"
    );

    // Something changed. The latch was about a VALUE, not about the session.
    world.resource_mut::<UserSettings>().audio.master_volume = 0.22;
    world
        .run_system_cached(save_settings_on_change)
        .expect("the third pass runs");
    assert!(
        path.exists(),
        "a new value must be attempted; a latch that never lifts fails a \
         transient outage closed and stops persisting for good"
    );
    assert_eq!(load_settings(&path).audio.master_volume, 0.22);

    std::env::remove_var("AMBITION_DATA_DIR");
    let _ = fs::remove_dir_all(&root);
}
