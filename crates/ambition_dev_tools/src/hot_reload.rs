//! A debounced mtime watch over the authored world file, and the
//! transactional reload it offers the developer controls.
//!
//! This watcher is format-agnostic. It lives here because its consumers are
//! the developer controls: the Developer settings page's auto-apply row sits
//! beside rows sourced from
//! [`DeveloperRuntimeState`](crate::dev_tools::DeveloperRuntimeState) and
//! [`DeveloperTools`](crate::dev_tools::DeveloperTools). The apply half (parse,
//! validate the room graph, commit or reject) stays with the game that knows
//! the format.
//!
//! The watcher does not resolve its own path; that needs the composing game's
//! asset catalog and world manifest. [`WorldSourceHotReload::watching`] takes
//! a resolved path, and [`WorldSourceHotReload::unavailable`] takes the reason
//! there is none.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::ecs::change_detection::DetectChangesMut;
use bevy::prelude::{Local, Res, ResMut, Resource, Time};

/// Watch state for the authored world file the developer controls can reload.
#[derive(Resource, Clone, Debug)]
pub struct WorldSourceHotReload {
    pub pending: bool,
    pub auto_apply: bool,
    pub last_modified: Option<SystemTime>,
    pub last_status: String,
    pub last_errors: Vec<String>,
    pub applied_count: u32,
    /// Local filesystem path the watcher polls, when both the active asset
    /// profile and the resolved world location support filesystem hot reload.
    /// `None` for bundled / web / embedded profiles — the watcher is
    /// effectively disabled there.
    pub watch_path: Option<PathBuf>,
}

impl Default for WorldSourceHotReload {
    fn default() -> Self {
        Self {
            pending: false,
            auto_apply: false,
            last_modified: None,
            last_status: "world hot reload idle".to_string(),
            last_errors: Vec::new(),
            applied_count: 0,
            watch_path: None,
        }
    }
}

impl WorldSourceHotReload {
    /// Arm the watcher on an already-resolved local path, taking its current
    /// mtime as the baseline. A path that cannot be stat'd arms nothing and
    /// reports why.
    pub fn watching(path: PathBuf) -> Self {
        let mut state = Self::default();
        match modified_time_for(&path) {
            Ok(modified) => {
                state.last_modified = Some(modified);
                state.last_status = format!("world hot reload watching {}", path.display());
            }
            Err(error) => {
                state.last_status = error;
            }
        }
        state.watch_path = Some(path);
        state
    }

    /// No path to watch, and the caller's reason — an asset profile that does
    /// not support filesystem watching, most often.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            last_status: reason.into(),
            ..Self::default()
        }
    }

    pub fn mark_pending(&mut self, modified: SystemTime) {
        self.last_modified = Some(modified);
        self.pending = true;
        self.last_errors.clear();
        self.last_status =
            "world file change detected; use Apply Reload from the developer controls".to_string();
    }

    pub fn mark_applied(&mut self, room: &str) {
        self.pending = false;
        self.applied_count = self.applied_count.saturating_add(1);
        self.last_errors.clear();
        self.last_status = format!("world reload applied to '{room}' (#{})", self.applied_count);
    }

    pub fn mark_failed(&mut self, errors: Vec<String>) {
        self.pending = false;
        self.last_errors = errors;
        let first = self
            .last_errors
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown world reload failure".to_string());
        self.last_status = format!("world reload rejected: {first}");
    }
}

fn modified_time_for(path: &Path) -> Result<SystemTime, String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "could not read modified time for {}: {error}",
                path.display()
            )
        })
}

/// Debounced mtime poll. Short-circuits when no path is armed.
///
/// The countdown is in a `Local`, not the resource: dereferencing `ResMut`
/// marks the resource changed, which would defeat change detection for every
/// reader on every frame. The resource is written only when the watch changes.
///
/// `fs::metadata` is a blocking syscall on the main thread (up to 3.9 ms on
/// virtiofs). If it shows up in a frame, move it off-thread; do not poll less
/// often. See `docs/planning/engine/performance-and-iteration.md`.
///
/// Register this in `Update`, never in the simulation schedule: the blocking
/// stat would sit in the deterministic tick, and the `Local` debounce does not
/// rewind. (`Res<Time>` alone would be fine in the sim; see ADR 0023 rule 2.)
/// All readers of `WorldSourceHotReload` are menu systems in `Update`.
pub fn poll_world_source_changes(
    time: Res<Time>,
    mut state: ResMut<WorldSourceHotReload>,
    mut poll_timer: Local<f32>,
) {
    *poll_timer -= time.delta_secs();
    if *poll_timer > 0.0 {
        return;
    }
    *poll_timer = 0.35;
    // Reading the armed path is not a change to it.
    let Some(path) = state.bypass_change_detection().watch_path.clone() else {
        return; // Profile doesn't support watching — stay idle.
    };
    let Ok(modified) = modified_time_for(&path) else {
        return;
    };
    let last_modified = state.bypass_change_detection().last_modified;
    if last_modified.is_some_and(|last| modified > last) {
        // A pending reload IS a change, and readers should see it.
        state.mark_pending(modified);
    } else if last_modified.is_none() {
        // Seeding the baseline is bookkeeping, not news.
        state.bypass_change_detection().last_modified = Some(modified);
    }
}
