//! Sandbox save game I/O + autosave.
//!
//! The data shape lives in `crate::save_data` (`AmbitionGameSaveData`,
//! `PersistedEncounter`, `PersistedSwitch`). This module is the
//! Bevy-side shim (`AmbitionGameSave` resource) that loads/saves to disk and
//! coordinates autosave.
//!
//! Convention: the save file lives next to `settings.ron` under the
//! OS-conventional data dir. One slot currently stores the mob-lab defeat state
//! and reset-switch position so a session is continuous across restarts.
//!
//! All I/O is non-fatal: a missing file means "fresh sandbox", a
//! corrupt file logs a warning and falls back to defaults. Save writes
//! go through a temp + rename so a crash mid-write can't corrupt the
//! live file.

// ⭐ THE NATIVE ROAD ONLY: `install_save`'s temp-file dance and the tests.
// The browser road goes through `crate::store`, which has no filesystem.
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::{Path, PathBuf};

use bevy::log::{info, warn};
use bevy::prelude::*;

use crate::save_data::{
    AmbitionGameSaveData, SaveCompatibility, CURRENT_SAVE_VERSION, PRE_VERSIONING_SAVE_VERSION,
};

pub const SANDBOX_SAVE_FILE: &str = "ambition/sandbox_save.ron";

/// Bevy resource holding the live save state. Mutated by the encounter
/// + switch systems; written to disk by `autosave_sandbox_save`.
#[derive(Resource, Clone, Debug, Default)]
pub struct AmbitionGameSave(pub AmbitionGameSaveData);

impl AmbitionGameSave {
    /// Canonical projection of the whole save, for the session checksum.
    ///
    /// ⭐ SERIALIZED RATHER THAN HAND-PROJECTED, DELIBERATELY. The save is an
    /// open set — every collection is `#[serde(default)]` so it can grow — and a
    /// hand-written field list would silently stop covering the field somebody
    /// adds next. Hashing the type's own serde form is exhaustive by
    /// construction, and it is the same form the file on disk already uses.
    ///
    /// Deterministic: `AmbitionGameSaveData` derives `Eq`, so it holds no
    /// floats, and every collection in it is an ordered `Vec`.
    ///
    /// ⛔ A FALLBACK OF `0` WOULD BE A CHECKSUM THAT CANNOT DISAGREE. RON
    /// serialization of this type does not fail, but if it ever did, hashing the
    /// error text keeps two peers that fail differently distinguishable.
    pub fn checksum(&self) -> u64 {
        use ambition_platformer2d_core::snapshot::checksum_bytes;
        match ron::ser::to_string(&self.0) {
            Ok(text) => checksum_bytes(text.as_bytes()),
            Err(error) => checksum_bytes(error.to_string().as_bytes()),
        }
    }

    pub fn data(&self) -> &AmbitionGameSaveData {
        &self.0
    }

    pub fn data_mut(&mut self) -> &mut AmbitionGameSaveData {
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

/// A save read from disk, together with whether this build may write over it.
///
/// The second field is the point.
#[derive(Clone, Debug)]
pub struct LoadedSave {
    pub data: AmbitionGameSaveData,
    /// False when the file on disk holds something this build must not replace:
    /// a save from an incompatible schema, or bytes it could not parse at all.
    pub writable: bool,
    /// The caller must NOT record it as the persisted shadow — see [`load_save_at_startup`],
    /// where doing exactly that left upgraded files un-upgraded on disk.
    pub upgraded: bool,
}

impl LoadedSave {
    /// A save that came from nowhere (no file yet) — fresh, and writable.
    fn fresh() -> Self {
        Self {
            data: AmbitionGameSaveData::default(),
            writable: true,
            upgraded: false,
        }
    }

    /// A file exists and this build cannot safely replace it.
    fn preserve() -> Self {
        Self {
            data: AmbitionGameSaveData::default(),
            writable: false,
            upgraded: false,
        }
    }
}

pub fn load_save(path: &Path) -> LoadedSave {
    let bytes = match crate::store::read(path) {
        Ok(s) => s,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return LoadedSave::fresh();
        }
        Err(error) => {
            // The file EXISTS and could not be read (permissions, a bad mount, a transient I/O
            // error).
            warn!(
                target: "ambition_platformer2d::save",
                "could not read save file {}: {error}; playing on a fresh sandbox \
                 and NOT writing over the existing file",
                path.display()
            );
            return LoadedSave::preserve();
        }
    };
    match ron::from_str::<AmbitionGameSaveData>(&bytes) {
        Ok(mut save) => match save.migrate() {
            SaveCompatibility::Current => LoadedSave {
                data: save,
                writable: true,
                upgraded: false,
            },
            SaveCompatibility::Migrated { from } => {
                info!(
                    target: "ambition_platformer2d::save",
                    "migrated save file {} from version {from} to {CURRENT_SAVE_VERSION}",
                    path.display(),
                );
                LoadedSave {
                    data: save,
                    writable: true,
                    // Saying so is what gets them rewritten.
                    upgraded: true,
                }
            }
            SaveCompatibility::FromTheFuture { found } => {
                // A player who launches an older build once must not lose the
                // save they made in the newer one. Their progress is still on
                // disk after this session; it just is not loaded.
                warn!(
                    target: "ambition_platformer2d::save",
                    "save file {} is version {found}, newer than this build's \
                     {CURRENT_SAVE_VERSION}; playing on a fresh sandbox and \
                     leaving the file untouched",
                    path.display(),
                );
                LoadedSave::preserve()
            }
            SaveCompatibility::Unsupported { found } => {
                // A parsed structure is not automatically a schema we understand.
                // In particular, historical development files can contain an
                // explicit `version: 0`, while the first defined schema is v1.
                // Preserve those bytes exactly as we do a future-version save.
                warn!(
                    target: "ambition_platformer2d::save",
                    "save file {} declares unsupported version {found}; this build \
                     supports save versions {PRE_VERSIONING_SAVE_VERSION} through \
                     {CURRENT_SAVE_VERSION}. Starting with a fresh sandbox and \
                     leaving the file untouched. Rename or delete {} and restart \
                     to allow saving again",
                    path.display(),
                    path.display(),
                );
                LoadedSave::preserve()
            }
        },
        Err(error) => {
            warn!(
                target: "ambition_platformer2d::save",
                "could not parse save file {}: {error}; playing on a fresh sandbox \
                 and NOT writing over the existing file",
                path.display()
            );
            LoadedSave::preserve()
        }
    }
}

/// Write the save, replacing whatever is there.
///
/// ⛔⛔ A FAILED SAVE MUST LEAVE EITHER THE OLD OR THE NEW STATE INTACT, never
/// neither. That rule is the whole point of this function; how it is KEPT
/// differs by platform, so the two roads are whole functions rather than one
/// function wearing `#[cfg]`s on its statements. (The first draft did the
/// latter, and the wasm build caught it immediately: `tmp` was defined under a
/// `cfg` and used outside one.)
pub fn write_save(path: &Path, save: &AmbitionGameSaveData) -> std::io::Result<()> {
    let body = ron::ser::to_string_pretty(save, ron::ser::PrettyConfig::default())
        .map_err(|error| std::io::Error::other(format!("ron serialize: {error}")))?;
    install_save(path, &body)
}

/// ⭐ THE KEY/VALUE ROAD NEEDS NO DANCE. A `localStorage` set is ONE synchronous
/// call: it either replaced the value or it returned an error and left the old
/// one alone. There is no half-written state to keep a backup against, and no
/// `rename` to install one with — so the rule above is kept by the store itself.
#[cfg(target_arch = "wasm32")]
fn install_save(path: &Path, body: &str) -> std::io::Result<()> {
    crate::store::write(path, body)
}

/// ⭐ THE FILESYSTEM ROAD NEEDS THE DANCE, because a write is many syscalls and a
/// crash between them leaves a truncated file. Write to a temp name, then
/// install by rename; if the rename cannot replace the destination, move the old
/// file aside first and put it back if installing the new one fails.
#[cfg(not(target_arch = "wasm32"))]
fn install_save(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("ron.tmp");
    fs::write(&tmp, body)?;
    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            let backup = path.with_extension("ron.bak");
            let _ = fs::remove_file(&backup);
            fs::rename(path, &backup)?;
            match fs::rename(&tmp, path) {
                Ok(()) => {
                    let _ = fs::remove_file(&backup);
                    Ok(())
                }
                Err(error) => {
                    // Put the player's save back. Losing the new state is a
                    // recoverable annoyance; losing the old state is not.
                    let _ = fs::rename(&backup, path);
                    Err(error)
                }
            }
        }
        Err(error) => Err(error),
    }
}

/// Whether this session may commit its save to disk.
///
/// Default TRUE, so any app that never loads a file (every test fixture, every
/// headless harness) keeps saving exactly as it did. It is only ever cleared by
/// [`load_save_at_startup`] finding something on disk it must not destroy.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SaveFileWritable(pub bool);

impl Default for SaveFileWritable {
    fn default() -> Self {
        Self(true)
    }
}

pub fn load_save_at_startup(
    mut save: ResMut<AmbitionGameSave>,
    mut last: ResMut<LastPersistedSave>,
    mut writable: ResMut<SaveFileWritable>,
    root: Res<crate::PersistenceRoot>,
) {
    let path = save_path_under(&root.0);
    if !path.exists() {
        return;
    }
    let loaded = load_save(&path);
    let upgraded = loaded.upgraded;
    save.0 = loaded.data;
    writable.0 = loaded.writable;
    // The shadow represents disk state. After an in-memory migration, leave it
    // empty so the first autosave commits the upgraded schema.
    last.0 = if upgraded { None } else { Some(save.0.clone()) };
    if loaded.writable {
        info!(
            target: "ambition_platformer2d::save",
            "loaded sandbox save from {}",
            path.display()
        );
    } else {
        // Said once, at the point of decision. The `warn!` inside `load_save`
        // explains WHY; this says what it costs the player for the rest of the
        // session, which is the part they need.
        warn!(
            target: "ambition_platformer2d::save",
            "this session will not write to {} — progress made now is NOT being saved",
            path.display()
        );
    }
}

/// What was last committed to disk. The autosave compares against this
/// instead of asking Bevy whether the resource was touched.
///
/// Change detection is the wrong throttle under a rollback host, in both
/// directions. It fires when nothing meaningful changed — GGRS's own restore
/// writes `AmbitionGameSave` on every rewind, so `is_changed()` is true almost
/// constantly — and it is consumed by a system that ran and declined to write,
/// so a genuine change can be dropped by any guard placed in front of it. A
/// value comparison has neither problem and is the honest question anyway:
/// *is what is on disk still correct?*
#[derive(Resource, Clone, Debug, Default)]
//  see `LastPersistedSettings`: the type is platform-identical because the wasm
// no-op systems take it as a parameter; only the native writer reads the value.
pub struct LastPersistedSave(
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))] Option<AmbitionGameSaveData>,
);

/// The confirmation gate is the load-bearing half. A rollback host advances
/// frames using a guess at what a remote peer did; the world therefore holds
/// state that may be rewound and recomputed. Writing that to disk records a
/// guess as history — and unlike a sound, which is merely heard once and wrong,
/// a save file outlives the session that produced it.
///
/// Under a rollback session it means the autosave waits for a moment with no outstanding
/// predictions rather than racing them; if that moment never comes, not autosaving is the
/// correct outcome, not a missed one.
pub fn autosave_sandbox_save(
    save: Res<AmbitionGameSave>,
    mut last: ResMut<LastPersistedSave>,
    writable: Res<SaveFileWritable>,
    root: Res<crate::PersistenceRoot>,
) {
    // Startup found a file this build must not replace — a save from a newer
    // build, or bytes it could not parse. Refusing to write is the whole
    // protection; without this the first flag the player sets destroys it.
    if !writable.0 {
        return;
    }
    if last.0.as_ref() == Some(&save.0) {
        return;
    }
    let path = save_path_under(&root.0);
    match write_save(&path, &save.0) {
        Ok(()) => last.0 = Some(save.0.clone()),
        Err(error) => warn!(
            target: "ambition_platformer2d::save",
            "failed to write save file {}: {error}",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save_data::PersistedEncounterState;

    fn temp_root(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("ambition_save_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn missing_file_returns_default_save() {
        let _g = crate::lock_data_dir();
        let root = temp_root("missing");
        let path = save_path_under(&root);
        let s = load_save(&path);
        assert_eq!(s.data, AmbitionGameSaveData::default());
        assert!(s.writable, "no file at all means a fresh, writable sandbox");
    }

    #[test]
    fn save_then_load_preserves_encounter_and_switch() {
        let _g = crate::lock_data_dir();
        let root = temp_root("round_trip");
        let path = save_path_under(&root);
        let mut save = AmbitionGameSaveData::default();
        save.set_encounter("goblin_encounter", PersistedEncounterState::Cleared);
        save.set_switch("reset_switch", true);
        write_save(&path, &save).unwrap();
        let restored = load_save(&path).data;
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
        app.init_resource::<AmbitionGameSave>()
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
        app.insert_resource(ambition_platformer2d_core::ConfirmedFrameBoundary {
            current,
            confirmed,
            session: 0,
        });
    }

    fn touch_save(app: &mut App, flag: &str) {
        app.world_mut()
            .resource_mut::<AmbitionGameSave>()
            .data_mut()
            .set_flag(flag, true);
    }

    /// The core hazard: a rollback host advances frames on a guess, so the
    /// world may hold state that is about to be rewound. A save file outlives
    /// the session, so committing a guess writes it into history.
    #[test]
    fn a_predicted_world_is_never_committed_to_disk() {
        let _g = crate::lock_data_dir();
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
        let _g = crate::lock_data_dir();
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

        let written = load_save(&save_path_under(&root)).data;
        assert!(
            written.flag("reached_the_vault"),
            "the change made during the predicted window must not be lost"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// GGRS writes `AmbitionGameSave` on every restore, so under change detection
    /// the autosave would rewrite an identical file on every rewind. Deleting
    /// the file and proving it does not come back is an exact "no write
    /// happened" probe.
    #[test]
    fn a_restore_that_changes_nothing_does_not_rewrite_the_file() {
        let _g = crate::lock_data_dir();
        let root = temp_root("no_churn");
        let mut app = autosave_app(&root);

        speculating(&mut app, 4, 4);
        touch_save(&mut app, "reached_the_vault");
        app.update();
        assert!(save_path_under(&root).exists(), "the first commit happens");

        fs::remove_file(save_path_under(&root)).unwrap();
        // A rewind restores the same value: Bevy marks it changed, but nothing
        // about it is actually different.
        let restored = app.world().resource::<AmbitionGameSave>().0.clone();
        app.world_mut().resource_mut::<AmbitionGameSave>().0 = restored;
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
        let _g = crate::lock_data_dir();
        let root = temp_root("no_host");
        let mut app = autosave_app(&root);

        touch_save(&mut app, "reached_the_vault");
        app.update();

        assert!(
            load_save(&save_path_under(&root))
                .data
                .flag("reached_the_vault"),
            "a game that never speculates must save exactly as it always did"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A save written by a NEWER build survives being opened by an older one.
    ///
    /// Playing on a fresh sandbox is fine. Committing that sandbox is not.
    #[test]
    fn an_older_build_never_writes_over_a_newer_builds_save() {
        let _g = crate::lock_data_dir();
        let root = temp_root("from_the_future");
        let path = save_path_under(&root);
        let mut future = AmbitionGameSaveData::default();
        future.version = crate::save_data::CURRENT_SAVE_VERSION + 1;
        future.set_flag("beat_the_final_boss", true);
        write_save(&path, &future).unwrap();
        let on_disk_before = fs::read_to_string(&path).unwrap();

        std::env::set_var("AMBITION_DATA_DIR", &root);
        let mut app = App::new();
        app.init_resource::<AmbitionGameSave>()
            .init_resource::<crate::settings::UserSettings>()
            .add_plugins(crate::PersistenceSchedulePlugin);
        app.update();
        // The session is playable, and playable means changes happen.
        touch_save(&mut app, "wandered_around");
        for _ in 0..3 {
            app.update();
        }

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            on_disk_before,
            "an older build overwrote a save it could not understand — the \
             player's progress in the newer build is gone"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A syntactically valid file with an unsupported schema is no more reason
    /// to abort startup than a corrupt file. The session gets a fresh save, the
    /// unknown bytes stay untouched, and writes remain disabled until the user
    /// moves the incompatible file out of the way.
    #[test]
    fn an_unsupported_save_never_blocks_startup_or_gets_overwritten() {
        let _g = crate::lock_data_dir();
        let root = temp_root("unsupported_version");
        let path = save_path_under(&root);
        let mut unsupported = AmbitionGameSaveData::default();
        unsupported.version = 0;
        unsupported.set_flag("old_progress", true);
        write_save(&path, &unsupported).unwrap();
        let on_disk_before = fs::read_to_string(&path).unwrap();

        std::env::set_var("AMBITION_DATA_DIR", &root);
        let mut app = App::new();
        app.init_resource::<AmbitionGameSave>()
            .init_resource::<crate::settings::UserSettings>()
            .add_plugins(crate::PersistenceSchedulePlugin);

        app.update();

        assert_eq!(
            app.world().resource::<AmbitionGameSave>().data(),
            &AmbitionGameSaveData::default(),
            "an unsupported save should yield a playable fresh session"
        );
        assert!(
            !app.world().resource::<SaveFileWritable>().0,
            "the fresh fallback must not overwrite bytes from an unknown schema"
        );

        touch_save(&mut app, "wandered_around");
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            on_disk_before,
            "the unsupported save was overwritten after startup fallback"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The same protection for bytes that are not a save at all. A corrupt file
    /// might be recoverable by hand, or might be the only copy of something; a
    /// session that cannot read it has no business replacing it.
    #[test]
    fn an_unreadable_save_is_left_on_disk_rather_than_replaced() {
        let _g = crate::lock_data_dir();
        let root = temp_root("keep_corrupt");
        let path = save_path_under(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"garbage not ron").unwrap();

        std::env::set_var("AMBITION_DATA_DIR", &root);
        let mut app = App::new();
        app.init_resource::<AmbitionGameSave>()
            .init_resource::<crate::settings::UserSettings>()
            .add_plugins(crate::PersistenceSchedulePlugin);
        app.update();
        touch_save(&mut app, "wandered_around");
        for _ in 0..3 {
            app.update();
        }

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "garbage not ron",
            "the unreadable file was replaced; whatever it was is now unrecoverable"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A migrated save is committed to disk without requiring a gameplay change.
    #[test]
    fn a_migration_is_committed_without_waiting_for_unrelated_gameplay() {
        let _g = crate::lock_data_dir();
        let root = temp_root("migrate_commits");
        let path = save_path_under(&root);
        let mut old = AmbitionGameSaveData::default();
        old.version = 1;
        old.set_flag("found_the_shrine", true);
        write_save(&path, &old).unwrap();

        std::env::set_var("AMBITION_DATA_DIR", &root);
        let mut app = App::new();
        app.init_resource::<AmbitionGameSave>()
            .init_resource::<crate::settings::UserSettings>()
            .add_plugins(crate::PersistenceSchedulePlugin);
        app.update();

        let on_disk = load_save(&path);
        assert!(
            !on_disk.upgraded,
            "the file on disk is STILL the old version: the migration lives only \
             in memory and will run again on every startup"
        );
        assert_eq!(on_disk.data.version, crate::save_data::CURRENT_SAVE_VERSION);
        assert!(on_disk.data.flag("found_the_shrine"));
        let _ = fs::remove_dir_all(&root);
    }

    /// Saving TWICE has to work. `fs::rename` replaces the destination on Unix
    /// and not on Windows, so a writer that only ever ran once in a test would
    /// pass here and fail for half the players on the second save.
    #[test]
    fn writing_the_save_repeatedly_replaces_the_previous_file() {
        let _g = crate::lock_data_dir();
        let root = temp_root("repeat_write");
        let path = save_path_under(&root);
        for round in 0..4 {
            let mut save = AmbitionGameSaveData::default();
            save.set_flag(&format!("round_{round}"), true);
            write_save(&path, &save)
                .unwrap_or_else(|error| panic!("save #{round} failed to commit: {error}"));
            let reread = load_save(&path);
            assert!(
                reread.data.flag(&format!("round_{round}")),
                "save #{round} did not reach the file"
            );
        }
        assert!(
            !path.with_extension("ron.bak").exists(),
            "the replacement fallback left its backup behind"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// An old file is migrated on load AND written back at the current version,
    /// so the tag on disk describes the shape that is actually there. Without
    /// the rewrite, a v1 file stays labelled v1 forever no matter how many
    /// current-shape saves are committed over it.
    #[test]
    fn a_migrated_save_is_written_back_at_the_current_version() {
        let _g = crate::lock_data_dir();
        let root = temp_root("migrate_write_back");
        let path = save_path_under(&root);
        let mut old = AmbitionGameSaveData::default();
        old.version = 1;
        old.set_flag("found_the_shrine", true);
        write_save(&path, &old).unwrap();

        std::env::set_var("AMBITION_DATA_DIR", &root);
        let mut app = App::new();
        app.init_resource::<AmbitionGameSave>()
            .init_resource::<crate::settings::UserSettings>()
            .add_plugins(crate::PersistenceSchedulePlugin);
        app.update();
        touch_save(&mut app, "kept_going");
        for _ in 0..3 {
            app.update();
        }

        let reread = load_save(&path);
        assert!(reread.writable);
        assert_eq!(reread.data.version, crate::save_data::CURRENT_SAVE_VERSION);
        assert!(
            reread.data.flag("found_the_shrine"),
            "the migration must not cost the player what the old file held"
        );
        assert!(reread.data.flag("kept_going"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_save_falls_back_to_default() {
        let _g = crate::lock_data_dir();
        let root = temp_root("corrupt");
        let path = save_path_under(&root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"garbage not ron").unwrap();
        let s = load_save(&path);
        assert_eq!(s.data, AmbitionGameSaveData::default());
        let _ = fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod save_checksum_tests {
    use super::AmbitionGameSave;
    use crate::save_data::PersistedFlag;

    /// ⭐ THE POSITIVE CONTROL FOR `track_room_visits`, whose `Local`
    /// edge-detector does not rewind: a resimulation can skip the save write, so
    /// a lost `room_visited_*` flag has to move the session checksum or the
    /// sync test cannot report it.
    #[test]
    fn a_lost_room_visited_flag_moves_the_checksum() {
        let base = AmbitionGameSave::default();
        let mut visited = base.clone();
        visited
            .data_mut()
            .flags
            .push(PersistedFlag::new("room_visited_hall", true));
        assert_ne!(
            base.checksum(),
            visited.checksum(),
            "a dropped visited-room flag must be visible to the session checksum"
        );
    }

    /// ⛔ The arm that would catch a checksum that never agrees.
    #[test]
    fn equal_saves_agree() {
        let mut a = AmbitionGameSave::default();
        a.data_mut()
            .flags
            .push(PersistedFlag::new("room_visited_hall", true));
        let b = a.clone();
        assert_eq!(a.checksum(), b.checksum());
    }
}
