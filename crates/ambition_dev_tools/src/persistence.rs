//! Disk persistence for the [`DeveloperTools`] resource (developer.ron).
//!
//! User settings and sandbox save I/O live in `ambition_persistence`; the
//! developer-only switches live here (E1d) beside the `DeveloperTools`
//! resource this crate owns. The on-disk root is resolved through
//! `ambition_persistence::settings::platform_paths`.

#[cfg(any(test, not(target_arch = "wasm32")))]
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
    load_existing_developer(path).unwrap_or_default()
}

fn load_existing_developer(path: &Path) -> Option<DeveloperTools> {
    let bytes = match ambition_persistence::store::read(path) {
        Ok(s) => s,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return None;
        }
        Err(error) => {
            warn!(
                target: "ambition_platformer2d::settings",
                "could not read developer file {}: {error}; using defaults",
                path.display()
            );
            return Some(DeveloperTools::default());
        }
    };
    match ron::from_str::<DeveloperTools>(&bytes) {
        Ok(mut developer) => {
            developer.normalize_debug_modes();
            Some(developer)
        }
        Err(error) => {
            warn!(
                target: "ambition_platformer2d::settings",
                "could not parse developer file {}: {error}; using defaults",
                path.display()
            );
            Some(DeveloperTools::default())
        }
    }
}

/// Atomic write of the developer-tools file.
pub fn save_developer(path: &Path, developer: &DeveloperTools) -> std::io::Result<()> {
    let body = ron::ser::to_string_pretty(developer, ron::ser::PrettyConfig::default())
        .map_err(|error| std::io::Error::other(format!("ron serialize: {error}")))?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("ron.tmp");
        fs::write(&tmp, body)?;
        fs::rename(tmp, path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        ambition_persistence::store::write(path, &body)
    }
}

pub fn load_developer_at_startup(
    mut developer: ResMut<DeveloperTools>,
    // Use the same App-owned root as settings and saves, not the environment, so
    // a windowless host writes its own directory, not the user's.
    root: Res<ambition_persistence::PersistenceRoot>,
) {
    let path = developer_path_under(&root.0);
    let Some(loaded) = load_existing_developer(&path) else {
        return;
    };
    *developer = loaded;
    info!(
        target: "ambition_platformer2d::settings",
        "loaded developer tools from {}",
        path.display()
    );
}

pub fn save_developer_on_change(
    developer: Res<DeveloperTools>,
    root: Res<ambition_persistence::PersistenceRoot>,
) {
    if !developer.is_changed() {
        return;
    }
    let path = developer_path_under(&root.0);
    if let Err(error) = save_developer(&path, &developer) {
        warn!(
            target: "ambition_platformer2d::settings",
            "failed to write developer file {}: {error}",
            path.display()
        );
    }
}

/// Schedules developer-tool persistence. User-facing settings and sandbox save
/// I/O stay in `ambition_persistence::PersistenceSchedulePlugin`; this plugin
/// owns only the developer-only `developer.ron` resource this crate defines.
pub struct DeveloperPersistenceSchedulePlugin;

impl Plugin for DeveloperPersistenceSchedulePlugin {
    fn build(&self, app: &mut App) {
        // Idempotent: whichever plugin gets here first supplies the default, and
        // an App that declared its own root before adding either keeps it.
        app.init_resource::<ambition_persistence::PersistenceRoot>()
            .add_systems(Startup, load_developer_at_startup)
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
    #[test]
    fn god_blink_survives_restart_and_still_requires_mechanical_admission() {
        use crate::dev_tools::{ActiveEditableAbilityMask, EditableAbilitySet};
        use ambition_platformer2d_core::{MechanicalEditAdmission, PendingMechanicalEdits};
        use ambition_platformer2d_shared_tangle::lifecycle::{
            SessionScopeActivated, SessionScopeId,
        };
        fn app(root: &Path) -> App {
            let mut app = App::new();
            app.insert_resource(ambition_persistence::PersistenceRoot::at(root))
                .init_resource::<DeveloperTools>()
                .init_resource::<EditableAbilitySet>()
                .init_resource::<ActiveEditableAbilityMask>()
                .init_resource::<PendingMechanicalEdits>()
                .init_resource::<ambition_persistence::save::SaveOwner>()
                .add_message::<SessionScopeActivated>()
                .add_plugins(DeveloperPersistenceSchedulePlugin)
                .add_systems(
                    PreUpdate,
                    (
                        crate::propose_editable_abilities,
                        crate::admit_editable_abilities,
                    )
                        .chain(),
                )
                .add_systems(
                    Update,
                    crate::reset_editable_abilities_on_activation.before(save_developer_on_change),
                );
            app.update();
            app.world_mut()
                .write_message(SessionScopeActivated(SessionScopeId(1)));
            app.update();
            app.update();
            app
        }
        let root = temp_root("god_blink_restart");
        let mut first = app(&root);
        let owner = first
            .world()
            .resource::<ambition_persistence::save::SaveOwner>()
            .current()
            .to_owned();
        first
            .world_mut()
            .resource_mut::<EditableAbilitySet>()
            .unrestricted_blink = true;
        first.update();
        assert!(load_developer(&developer_path_under(&root))
            .god_blink_experiences
            .contains(&owner));
        drop(first);

        let mut restarted = app(&root);
        assert!(
            restarted
                .world()
                .resource::<EditableAbilitySet>()
                .unrestricted_blink
        );
        assert!(
            restarted
                .world()
                .resource::<ActiveEditableAbilityMask>()
                .0
                .unwrap()
                .unrestricted_blink
        );
        let mut refused = app(&root);
        refused.insert_resource(MechanicalEditAdmission::Refuse);
        refused
            .world_mut()
            .write_message(SessionScopeActivated(SessionScopeId(2)));
        refused.update();
        refused.update();
        assert!(
            refused
                .world()
                .resource::<EditableAbilitySet>()
                .unrestricted_blink
        );
        assert!(
            !refused
                .world()
                .resource::<ActiveEditableAbilityMask>()
                .0
                .unwrap()
                .unrestricted_blink
        );
        drop(refused);

        restarted
            .world_mut()
            .resource_mut::<EditableAbilitySet>()
            .unrestricted_blink = false;
        restarted.update();
        drop(restarted);
        let disabled = app(&root);
        assert!(
            !disabled
                .world()
                .resource::<EditableAbilitySet>()
                .unrestricted_blink
        );
        assert!(!load_developer(&developer_path_under(&root))
            .god_blink_experiences
            .contains(&owner));
        drop(disabled);
        fs::remove_dir_all(root).unwrap();
    }
}
