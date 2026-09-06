//! A debounced mtime watch over the authored world file, and the
//! transactional reload it offers the developer controls.
//!
//! This watcher is format-agnostic. It lives here because its consumers are
//! the developer controls: the
//! Developer settings page's auto-apply row sits directly beside rows sourced
//! from [`DeveloperRuntimeState`](crate::dev_tools::DeveloperRuntimeState) and
//! [`DeveloperTools`](crate::dev_tools::DeveloperTools), both of which are this
//! crate's. The APPLY half — parse the file, validate the room graph, commit or
//! reject — stays with the game that knows the format.
//!
//!  the watcher does not resolve its own path. Resolution needs the asset
//! catalog and the world manifest, which are the composing game's; a
//! constructor that took both is what put an asset-profile decision inside a
//! format adapter. [`WorldSourceHotReload::watching`] takes the path the caller
//! already resolved, and [`WorldSourceHotReload::unavailable`] takes the
//! caller's reason for there not being one.

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
/// ⛔ THE COUNTDOWN LIVES IN A `Local`, NOT IN THE RESOURCE. `ResMut` marks its
/// resource changed the moment it is dereferenced, so ticking the timer through
/// it announced "the hot-reload watcher changed" on every frame of every run —
/// a lie that costs every `Res<WorldSourceHotReload>` reader its change
/// detection. The resource is now touched mutably only when something about the
/// WATCH actually moved.
///
/// ⚠ `fs::metadata` is a BLOCKING syscall on the main thread. Debounced to ~3Hz
/// it is invisible on a local disk, and it was measured at up to 3.9ms on
/// virtiofs. On a network mount, Android storage, or a slow card it is a frame
/// hitch.
///
/// ⛔⛔ **REGISTER THIS IN `Update`, NEVER IN THE SIMULATION SCHEDULE.** It ran in
/// `WorldPrep` — the sim's largest phase — until 2026-08-29, which put a blocking
/// stat on the deterministic tick. It is also unfit for that schedule on its own
/// terms: the debounce lives in a `Local`, which does NOT rewind, so a session
/// that actually rolled back would re-stat the file once per re-simulated tick.
/// Every reader of `WorldSourceHotReload` is a menu system in `Update` anyway.
///
/// ⛔ NOT a reason, though it reads like one: `Res<Time>` is FINE inside the sim.
/// `bevy_ggrs` swaps `Time<()>` for the rolled-back `Time<GgrsTime>` for the
/// duration of `GgrsSchedule`, and ADR 0023 rule 2 says so explicitly — the
/// wall-clock rule is about `std::time`, not `Res<Time>`.
///
/// ⇒ If the remaining ~3Hz stat ever shows up in a frame, move it off-thread —
/// do NOT poll less often. See `docs/planning/engine/performance-and-iteration.md`.
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
