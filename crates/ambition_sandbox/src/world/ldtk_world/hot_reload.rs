//! LDtk file-watch + transactional hot-reload state.
//!
//! `LdtkHotReloadState` tracks the watched `.ldtk` path and a pending/applied/
//! failed reload status; `poll_ldtk_file_changes` (Bevy system) debounces mtime
//! checks and marks pending reloads. `sandbox_ldtk_asset_path` /
//! `SANDBOX_LDTK_ASSET` give the bevy_ecs_ldtk asset path. Pure file/policy
//! state — actual parse + convert happens in `loading`/`conversion`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::{Res, ResMut, Resource, Time};

use crate::assets::sandbox_assets::{ids, SandboxAssetCatalog};

pub const SANDBOX_LDTK_ASSET: &str = "ambition/worlds/sandbox.ldtk";

/// `[ambition_asset_manager_transition]` Bevy `AssetPath` string for the
/// sandbox LDtk world. Used by the `bevy_ecs_ldtk` runtime spine to
/// spawn `LdtkWorldBundle`s. The catalog will eventually own this — for
/// now the path is the SANDBOX_LDTK_ASSET constant since
/// `bevy_ecs_ldtk` always loads via the default asset source.
pub fn sandbox_ldtk_asset_path() -> String {
    SANDBOX_LDTK_ASSET.to_string()
}

#[derive(Resource, Clone, Debug)]
pub struct LdtkHotReloadState {
    pub pending: bool,
    pub auto_apply: bool,
    pub poll_timer: f32,
    pub last_modified: Option<SystemTime>,
    pub last_status: String,
    pub last_errors: Vec<String>,
    pub applied_count: u32,
    /// Local filesystem path the watcher polls, when both the active
    /// asset profile and the resolved LDtk location support filesystem
    /// hot reload (resolved via
    /// `SandboxAssetCatalog::hot_reload_local_path`, private to
    /// `crate::assets::sandbox_assets`).
    /// `None` for bundled / web / embedded profiles — the watcher is
    /// effectively disabled there.
    pub watch_path: Option<PathBuf>,
}

impl Default for LdtkHotReloadState {
    fn default() -> Self {
        Self {
            pending: false,
            auto_apply: false,
            poll_timer: 0.0,
            last_modified: None,
            last_status: "LDtk hot reload idle".to_string(),
            last_errors: Vec::new(),
            applied_count: 0,
            watch_path: None,
        }
    }
}

impl LdtkHotReloadState {
    /// Build the hot-reload state from the active catalog. Arms the
    /// watcher if and only if the profile + resolved LDtk location
    /// both report `supports_hot_reload`. Otherwise the state stays
    /// idle and `poll_ldtk_file_changes` short-circuits.
    pub fn from_catalog(catalog: &SandboxAssetCatalog) -> Self {
        let mut state = Self::default();
        let watch_path = catalog.hot_reload_local_path(&ids::sandbox_ldtk());
        let Some(path) = watch_path else {
            state.last_status = format!(
                "LDtk hot reload inactive: profile {} does not support filesystem watching",
                catalog.profile().label(),
            );
            return state;
        };
        match modified_time_for(&path) {
            Ok(modified) => {
                state.last_modified = Some(modified);
                state.last_status = if cfg!(feature = "dev_hot_reload") {
                    "LDtk hot reload watching; press F11 to apply, F12 toggles auto-apply"
                        .to_string()
                } else {
                    "LDtk hot reload polling; run with --features dev_hot_reload for Bevy file watching too".to_string()
                };
            }
            Err(error) => {
                state.last_status = error;
            }
        }
        state.watch_path = Some(path);
        state
    }

    pub fn mark_pending(&mut self, modified: SystemTime) {
        self.last_modified = Some(modified);
        self.pending = true;
        self.last_errors.clear();
        self.last_status = "LDtk change detected; press F11 to apply".to_string();
    }

    pub fn mark_applied(&mut self, room: &str) {
        self.pending = false;
        self.applied_count = self.applied_count.saturating_add(1);
        self.last_errors.clear();
        self.last_status = format!("LDtk reload applied to '{room}' (#{})", self.applied_count);
    }

    pub fn mark_failed(&mut self, errors: Vec<String>) {
        self.pending = false;
        self.last_errors = errors;
        let first = self
            .last_errors
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown LDtk reload failure".to_string());
        self.last_status = format!("LDtk reload rejected: {first}");
    }
}

fn modified_time_for(path: &Path) -> Result<SystemTime, String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "could not read LDtk modified time for {}: {error}",
                path.display()
            )
        })
}

pub fn poll_ldtk_file_changes(time: Res<Time>, mut state: ResMut<LdtkHotReloadState>) {
    state.poll_timer -= time.delta_secs();
    if state.poll_timer > 0.0 {
        return;
    }
    state.poll_timer = 0.35;
    let Some(path) = state.watch_path.clone() else {
        return; // Profile doesn't support watching — stay idle.
    };
    let Ok(modified) = modified_time_for(&path) else {
        return;
    };
    let changed = state
        .last_modified
        .map(|last| modified > last)
        .unwrap_or(false);
    if changed {
        state.mark_pending(modified);
    } else if state.last_modified.is_none() {
        state.last_modified = Some(modified);
    }
}
