//! Sandbox save game I/O + autosave.
//!
//! The data shape lives in `crate::engine_core::save` (`SandboxSaveData`,
//! `PersistedEncounter`, `PersistedSwitch`). This module is the
//! Bevy-side shim that loads/saves to disk and coordinates autosave.
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

use bevy::prelude::*;

use crate::persistence::save_data::SandboxSaveData;

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
    save_path_under(&crate::persistence::settings::platform_paths::data_dir_root())
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
pub fn load_save_at_startup(mut save: ResMut<SandboxSave>) {
    let path = save_path();
    if !path.exists() {
        return;
    }
    save.0 = load_save(&path);
    info!(
        target: "ambition::save",
        "loaded sandbox save from {}",
        path.display()
    );
}

/// Bevy update system: when the save resource changes, write to disk.
/// `Res::is_changed` is the throttle — change-detection only fires on
/// the frame the resource was mutated, not every frame.
#[cfg(not(target_arch = "wasm32"))]
pub fn autosave_sandbox_save(save: Res<SandboxSave>) {
    if !save.is_changed() {
        return;
    }
    let path = save_path();
    if let Err(error) = write_save(&path, &save.0) {
        warn!(
            target: "ambition::save",
            "failed to write save file {}: {error}",
            path.display()
        );
    }
}

/// Wasm (browser) no-op for save loading. First-pass web build does not
/// persist the sandbox save; the in-memory `Res<SandboxSave>` still
/// works for the session. Browser persistence (IndexedDB / LocalStorage
/// behind `web-sys`) is a follow-up.
#[cfg(target_arch = "wasm32")]
pub fn load_save_at_startup(_save: ResMut<SandboxSave>) {}

/// Wasm (browser) no-op for save writing. See [`load_save_at_startup`].
#[cfg(target_arch = "wasm32")]
pub fn autosave_sandbox_save(_save: Res<SandboxSave>) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::save_data::PersistedEncounterState;
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
