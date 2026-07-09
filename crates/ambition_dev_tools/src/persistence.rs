//! Disk persistence for the [`DeveloperTools`] resource (developer.ron).
//!
//! User settings and sandbox save I/O live in `ambition_persistence`; the
//! developer-only switches live here (E1d) beside the `DeveloperTools`
//! resource this crate owns. The on-disk root is resolved through
//! `ambition_persistence::settings::platform_paths`.

use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;

use crate::dev_tools::DeveloperTools;

/// Companion file holding developer-only switches. Kept separate from
/// `settings.ron` so clearing dev knobs does not reset player-facing tuning.
pub const DEVELOPER_FILE: &str = "ambition/developer.ron";

pub fn developer_path() -> PathBuf {
    developer_path_under(&ambition_persistence::settings::platform_paths::data_dir_root())
}

pub fn developer_path_under(root: &Path) -> PathBuf {
    root.join(DEVELOPER_FILE)
}

/// Load `DeveloperTools` from disk. Missing file -> defaults, parse error ->
/// log + defaults.
pub fn load_developer(path: &Path) -> DeveloperTools {
    let bytes = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DeveloperTools::default();
        }
        Err(error) => {
            warn!(
                target: "ambition::settings",
                "could not read developer file {}: {error}; using defaults",
                path.display()
            );
            return DeveloperTools::default();
        }
    };
    match ron::from_str::<DeveloperTools>(&bytes) {
        Ok(mut developer) => {
            developer.normalize_debug_modes();
            developer
        }
        Err(error) => {
            warn!(
                target: "ambition::settings",
                "could not parse developer file {}: {error}; using defaults",
                path.display()
            );
            DeveloperTools::default()
        }
    }
}

/// Atomic write of the developer-tools file.
pub fn save_developer(path: &Path, developer: &DeveloperTools) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = ron::ser::to_string_pretty(developer, ron::ser::PrettyConfig::default())
        .map_err(|error| std::io::Error::other(format!("ron serialize: {error}")))?;
    let tmp = path.with_extension("ron.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_developer_at_startup(mut developer: ResMut<DeveloperTools>) {
    let path = developer_path();
    if !path.exists() {
        return;
    }
    *developer = load_developer(&path);
    info!(
        target: "ambition::settings",
        "loaded developer tools from {}",
        path.display()
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_developer_on_change(developer: Res<DeveloperTools>) {
    if !developer.is_changed() {
        return;
    }
    let path = developer_path();
    if let Err(error) = save_developer(&path, &developer) {
        warn!(
            target: "ambition::settings",
            "failed to write developer file {}: {error}",
            path.display()
        );
    }
}

#[cfg(target_arch = "wasm32")]
pub fn load_developer_at_startup(_developer: ResMut<DeveloperTools>) {}

#[cfg(target_arch = "wasm32")]
pub fn save_developer_on_change(_developer: Res<DeveloperTools>) {}

/// Schedules developer-tool persistence. User-facing settings and sandbox save
/// I/O stay in `ambition_persistence::PersistenceSchedulePlugin`; this plugin
/// owns only the developer-only `developer.ron` resource this crate defines.
pub struct DeveloperPersistenceSchedulePlugin;

impl Plugin for DeveloperPersistenceSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_developer_at_startup)
            .add_systems(Update, save_developer_on_change);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local!(static UNIQUE: Cell<u64> = const { Cell::new(0) });

    fn temp_root(name: &str) -> PathBuf {
        let counter = UNIQUE.with(|c| {
            let next = c.get() + 1;
            c.set(next);
            next
        });
        let mut p = std::env::temp_dir();
        p.push(format!(
            "ambition_developer_{name}_{}_{}",
            std::process::id(),
            counter
        ));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn developer_load_normalizes_legacy_art_toggles() {
        let root = temp_root("legacy_art");
        let path = root.join("developer.ron");
        let mut developer = DeveloperTools::default();
        developer.debug_art_mode = crate::dev_tools::DebugArtMode::Normal;
        developer.hide_sprites = true;
        developer.placeholder_sprites = true;
        save_developer(&path, &developer).unwrap();

        let restored = load_developer(&path);
        assert_eq!(
            restored.debug_art_mode,
            crate::dev_tools::DebugArtMode::Placeholder
        );
        assert!(restored.placeholder_sprites);
        assert!(!restored.hide_sprites);
        let _ = fs::remove_dir_all(&root);
    }
}
