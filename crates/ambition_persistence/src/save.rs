//! Sandbox save game I/O + autosave.
//!
//! The data shape lives in `crate::save_data` (`SandboxSaveData`,
//! `PersistedEncounter`, `PersistedSwitch`). This module is the
//! Bevy-side shim (`SandboxSave` resource) that loads/saves to disk and
//! coordinates autosave.
//!
//! Convention: the save file lives next to `settings.ron` under the
//! OS-conventional data dir. One slot for now ("sandbox") because the
//! sandbox itself isn't a campaign — the save just remembers the mob
//! lab defeat state and the reset-switch position so a session is
//! continuous across restarts. A future story crate adds named slots.
//!
//! All I/O is non-fatal: a missing file means "fresh sandbox", a
//! corrupt file logs a warning and falls back to defaults. Save writes
//! go through a temp + rename so a crash mid-write can't corrupt the
//! live file.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::log::{info, warn};
use bevy::prelude::*;

use crate::save_data::SandboxSaveData;

pub const SANDBOX_SAVE_FILE: &str = "ambition/sandbox_save.ron";

/// Bevy resource holding the live save state. Mutated by the encounter
/// + switch systems; written to disk by `autosave_sandbox_save`.
#[derive(Resource, Clone, Debug, Default)]
pub struct SandboxSave(pub SandboxSaveData);

impl SandboxSave {
    pub fn data(&self) -> &SandboxSaveData {
        &self.0
    }

    pub fn data_mut(&mut self) -> &mut SandboxSaveData {
        &mut self.0
    }
}

/// Where the sandbox save lives. Reuses the same data-dir resolution
/// as the settings persistence module so both files end up alongside
/// each other.
pub fn save_path() -> PathBuf {
    save_path_under(&crate::settings::platform_paths::data_dir_root())
}

pub fn save_path_under(root: &Path) -> PathBuf {
    root.join(SANDBOX_SAVE_FILE)
}

pub fn load_save(path: &Path) -> SandboxSaveData {
    let bytes = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return SandboxSaveData::default();
        }
        Err(error) => {
            warn!(
                target: "ambition::save",
                "could not read save file {}: {error}; using fresh sandbox",
                path.display()
            );
            return SandboxSaveData::default();
        }
    };
    match ron::from_str::<SandboxSaveData>(&bytes) {
        Ok(save) => save,
        Err(error) => {
            warn!(
                target: "ambition::save",
                "could not parse save file {}: {error}; using fresh sandbox",
                path.display()
            );
            SandboxSaveData::default()
        }
    }
}

pub fn write_save(path: &Path, save: &SandboxSaveData) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|error| std::io::Error::other(format!("ron serialize: {error}")))?;
    let tmp = path.with_extension("ron.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_save_at_startup(mut save: ResMut<SandboxSave>, mut last: ResMut<LastPersistedSave>) {
    let path = save_path();
    if !path.exists() {
        return;
    }
    save.0 = load_save(&path);
    // What we just read IS what is on disk, so the autosave has nothing to do
    // until something actually changes. Without this the first frame rewrites
    // the file it just finished reading.
    last.0 = Some(save.0.clone());
    info!(
        target: "ambition::save",
        "loaded sandbox save from {}",
        path.display()
    );
}

/// What was last committed to disk. The autosave compares against this
/// instead of asking Bevy whether the resource was touched.
///
/// Change detection is the wrong throttle under a rollback host, in both
/// directions. It fires when nothing meaningful changed — GGRS's own restore
/// writes `SandboxSave` on every rewind, so `is_changed()` is true almost
/// constantly — and it is consumed by a system that ran and declined to write,
/// so a genuine change can be dropped by any guard placed in front of it. A
/// value comparison has neither problem and is the honest question anyway:
/// *is what is on disk still correct?*
#[derive(Resource, Clone, Debug, Default)]
pub struct LastPersistedSave(Option<SandboxSaveData>);

/// Bevy update system: commit the save to disk when it no longer matches what
/// is there, and only while the simulation holds no predicted state.
///
/// The confirmation gate is the load-bearing half. A rollback host advances
/// frames using a guess at what a remote peer did; the world therefore holds
/// state that may be rewound and recomputed. Writing that to disk records a
/// guess as history — and unlike a sound, which is merely heard once and wrong,
/// a save file outlives the session that produced it.
///
/// `world_state_is_confirmed` is true whenever nothing is predicted, which on
/// every non-rollback host is *always*, so a fixed-tick or headless game keeps
/// writing exactly when it did before. Under a rollback session it means the
/// autosave waits for a moment with no outstanding predictions rather than
/// racing them; if that moment never comes, not autosaving is the correct
/// outcome, not a missed one.
#[cfg(not(target_arch = "wasm32"))]
pub fn autosave_sandbox_save(save: Res<SandboxSave>, mut last: ResMut<LastPersistedSave>) {
    if last.0.as_ref() == Some(&save.0) {
        return;
    }
    let path = save_path();
    match write_save(&path, &save.0) {
        Ok(()) => last.0 = Some(save.0.clone()),
        Err(error) => warn!(
            target: "ambition::save",
            "failed to write save file {}: {error}",
            path.display()
        ),
    }
}

/// Wasm (browser) no-op for save loading. First-pass web build does not
/// persist the sandbox save; the in-memory `Res<SandboxSave>` still
/// works for the session. Browser persistence (IndexedDB / LocalStorage
/// behind `web-sys`) is a follow-up.
#[cfg(target_arch = "wasm32")]
pub fn load_save_at_startup(_save: ResMut<SandboxSave>, _last: ResMut<LastPersistedSave>) {}

/// Wasm (browser) no-op for save writing. See [`load_save_at_startup`].
#[cfg(target_arch = "wasm32")]
pub fn autosave_sandbox_save(_save: Res<SandboxSave>, _last: ResMut<LastPersistedSave>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::PersistedEncounterState;
    use std::sync::Mutex;

    static TEST_DIR_LOCK: Mutex<()> = Mutex::new(());

    fn temp_root(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("ambition_save_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn missing_file_returns_default_save() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("missing");
        let path = save_path_under(&root);
        let s = load_save(&path);
        assert_eq!(s, SandboxSaveData::default());
    }

    #[test]
    fn save_then_load_preserves_encounter_and_switch() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("round_trip");
        let path = save_path_under(&root);
        let mut save = SandboxSaveData::default();
        save.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
        save.set_switch("reset_switch", true);
        write_save(&path, &save).unwrap();
        let restored = load_save(&path);
        assert_eq!(
            restored.encounter("goblin_encounter"),
            PersistedEncounterState::Cleared
        );
        assert!(restored.switch("reset_switch"));
        let _ = fs::remove_dir_all(&root);
    }

    /// Runs the REAL plugin against a scratch data dir, so the confirmation
    /// gate and the value comparison are exercised exactly as they ship.
    fn autosave_app(root: &Path) -> App {
        std::env::set_var("AMBITION_DATA_DIR", root);
        let mut app = App::new();
        app.init_resource::<SandboxSave>()
            .init_resource::<crate::settings::UserSettings>()
            .add_plugins(crate::PersistenceSchedulePlugin);
        // Run startup + the first autosave, which commits the fresh default
        // save exactly as the shipping app does. Clearing the file afterwards
        // leaves the shadow agreeing with an absent file, so every assertion
        // below reads as "did THIS update write?" rather than tripping over
        // boot behaviour.
        app.update();
        let _ = fs::remove_file(save_path_under(root));
        app
    }

    fn speculating(app: &mut App, current: i32, confirmed: i32) {
        app.insert_resource(ambition_engine_core::ConfirmedFrameBoundary {
            current,
            confirmed,
            session: 0,
        });
    }

    fn touch_save(app: &mut App, flag: &str) {
        app.world_mut()
            .resource_mut::<SandboxSave>()
            .data_mut()
            .set_flag(flag, true);
    }

    /// The core hazard: a rollback host advances frames on a guess, so the
    /// world may hold state that is about to be rewound. A save file outlives
    /// the session, so committing a guess writes it into history.
    #[test]
    fn a_predicted_world_is_never_committed_to_disk() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("predicted");
        let mut app = autosave_app(&root);

        speculating(&mut app, 10, 6);
        touch_save(&mut app, "reached_the_vault");
        app.update();

        assert!(
            !save_path_under(&root).exists(),
            "the world still holds four predicted frames; nothing may be written yet"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The half change detection would have lost. A guard in front of an
    /// `is_changed()` system consumes the change: the system ran, declined to
    /// write, and the flag is gone. Comparing values instead means the pending
    /// write survives however long confirmation takes.
    #[test]
    fn a_change_made_while_predicting_is_written_once_it_confirms() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("deferred");
        let mut app = autosave_app(&root);

        speculating(&mut app, 10, 6);
        touch_save(&mut app, "reached_the_vault");
        for _ in 0..5 {
            app.update();
        }
        assert!(!save_path_under(&root).exists());

        // The peer's real input arrives and everything settles.
        speculating(&mut app, 10, 10);
        app.update();

        let written = load_save(&save_path_under(&root));
        assert!(
            written.flag("reached_the_vault"),
            "the change made during the predicted window must not be lost"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// GGRS writes `SandboxSave` on every restore, so under change detection
    /// the autosave would rewrite an identical file on every rewind. Deleting
    /// the file and proving it does not come back is an exact "no write
    /// happened" probe.
    #[test]
    fn a_restore_that_changes_nothing_does_not_rewrite_the_file() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("no_churn");
        let mut app = autosave_app(&root);

        speculating(&mut app, 4, 4);
        touch_save(&mut app, "reached_the_vault");
        app.update();
        assert!(save_path_under(&root).exists(), "the first commit happens");

        fs::remove_file(save_path_under(&root)).unwrap();
        // A rewind restores the same value: Bevy marks it changed, but nothing
        // about it is actually different.
        let restored = app.world().resource::<SandboxSave>().0.clone();
        app.world_mut().resource_mut::<SandboxSave>().0 = restored;
        app.update();

        assert!(
            !save_path_under(&root).exists(),
            "an identical save was rewritten — the autosave is still keying on \
             change detection rather than on what is on disk"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// No rollback host: the absent boundary must mean "write normally", or
    /// every fixed-tick and headless game silently stops saving.
    #[test]
    fn without_a_rollback_host_the_save_is_written_immediately() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("no_host");
        let mut app = autosave_app(&root);

        touch_save(&mut app, "reached_the_vault");
        app.update();

        assert!(
            load_save(&save_path_under(&root)).flag("reached_the_vault"),
            "a game that never speculates must save exactly as it always did"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_save_falls_back_to_default() {
        let _g = TEST_DIR_LOCK.lock().unwrap();
        let root = temp_root("corrupt");
        let path = save_path_under(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"garbage not ron").unwrap();
        let s = load_save(&path);
        assert_eq!(s, SandboxSaveData::default());
        let _ = fs::remove_dir_all(&root);
    }
}
