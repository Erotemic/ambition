use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bevy::prelude::{Res, ResMut, Resource, Time};

pub const SANDBOX_LDTK_ASSET: &str = "ambition/worlds/sandbox.ldtk";
pub const AMBITION_LDTK_ENV: &str = "AMBITION_LDTK";

/// Return the default checked-in sandbox LDtk path on disk.
///
/// Normal sandbox builds load this external file at runtime so LDtk edits and
/// modded maps do not require recompiling Rust. Build with `--features
/// static_map` to also embed a fallback copy in the binary.
pub fn default_sandbox_ldtk_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(SANDBOX_LDTK_ASSET)
}

fn absolute_user_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn cli_ldtk_arg() -> Option<PathBuf> {
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--ldtk" || arg == "--map" {
            return args.next().map(PathBuf::from).map(absolute_user_path);
        }
        if let Some(raw) = arg.strip_prefix("--ldtk=") {
            return Some(absolute_user_path(PathBuf::from(raw)));
        }
        if let Some(raw) = arg.strip_prefix("--map=") {
            return Some(absolute_user_path(PathBuf::from(raw)));
        }
    }
    None
}

/// User-selected LDtk file, if one was provided on the command line or in the
/// environment.
///
/// Command-line flags win over `AMBITION_LDTK`:
///
/// ```text
/// ambition_sandbox --ldtk mods/my_world.ldtk
/// AMBITION_LDTK=mods/my_world.ldtk ambition_sandbox
/// ```
pub fn configured_ldtk_path() -> Option<PathBuf> {
    cli_ldtk_arg().or_else(|| {
        env::var_os(AMBITION_LDTK_ENV)
            .map(PathBuf::from)
            .map(absolute_user_path)
    })
}

pub fn sandbox_ldtk_path() -> PathBuf {
    configured_ldtk_path().unwrap_or_else(default_sandbox_ldtk_path)
}

/// Convert a filesystem LDtk path into a Bevy asset path when the file lives
/// under the sandbox asset root. The LDtk runtime-spine asset loader can only
/// load files from Bevy asset sources; arbitrary external mod paths are still
/// parsed by Ambition's direct JSON loader, but cannot currently be mirrored
/// through `bevy_ecs_ldtk` unless they are placed under `assets/`.
pub fn ldtk_asset_path_for(path: &Path) -> Option<String> {
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let relative = path.strip_prefix(&asset_root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        parts.push(component.as_os_str().to_string_lossy().to_string());
    }
    Some(parts.join("/"))
}

pub fn sandbox_ldtk_asset_path() -> String {
    ldtk_asset_path_for(&sandbox_ldtk_path()).unwrap_or_else(|| SANDBOX_LDTK_ASSET.to_string())
}

pub fn sandbox_ldtk_modified_time() -> Result<SystemTime, String> {
    let path = sandbox_ldtk_path();
    fs::metadata(&path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "could not read LDtk modified time for {}: {error}",
                path.display()
            )
        })
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
        }
    }
}

impl LdtkHotReloadState {
    pub fn from_current_file() -> Self {
        let mut state = Self::default();
        match sandbox_ldtk_modified_time() {
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

pub fn poll_ldtk_file_changes(time: Res<Time>, mut state: ResMut<LdtkHotReloadState>) {
    state.poll_timer -= time.delta_secs();
    if state.poll_timer > 0.0 {
        return;
    }
    state.poll_timer = 0.35;
    let Ok(modified) = sandbox_ldtk_modified_time() else {
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
