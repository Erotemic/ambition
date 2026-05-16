//! LDtk file-loading policy.
//!
//! Decides whether the live build reads the checked-in external
//! `assets/ambition/worlds/sandbox.ldtk`, an env-override path, or
//! the statically packed copy embedded into the binary on Android /
//! Web. Pure I/O policy — no validation or runtime conversion lives
//! here.
//!
//! The path itself is selected by
//! [`crate::sandbox_assets::SandboxAssetCatalog`] under the active
//! [`ambition_asset_manager::AssetProfile`]:
//! - `DesktopDevLoose` / `DesktopInstalled` / `SteamDeckInstalled` →
//!   `LocalPath` resolved against the canonical assets root.
//! - `AndroidBundle` / `IosBundle` / `WebStatic` / `BundledStatic` →
//!   embedded fallback (the `static_map` feature provides the bytes
//!   via `include_str!`).
//! - `WebHttp` → HTTP candidate if authored; currently falls back to
//!   embedded.
//! - `NoAssets` / `Headless` → `Disabled` → loader returns the
//!   required-asset error.
//!
//! ## Multi-file world composition
//!
//! Story-content zones (intro, future real-game-map zones) live in
//! separate `.ldtk` source files next to sandbox.ldtk. They are
//! authored against the same project defs (cloned via
//! `python -m ambition_ldtk_tools world init`) so their entity/layer
//! uids match the sandbox's — that means the runtime can simply
//! append their `levels` arrays into the merged in-memory
//! `LdtkProject` without remapping anything. Every level keeps its
//! own iid, its own activeArea, and its own LoadingZone targets, so
//! cross-file room transitions work via the standard target_room
//! mechanism.

use std::fs;
use std::path::Path;

use crate::sandbox_assets::{ids, SandboxAssetCatalog};

use super::project::LdtkProject;

/// Story-content world files appended into the runtime project on top
/// of `sandbox.ldtk`. Paths are relative to the same `worlds/`
/// directory the sandbox map lives in. New zones land here as their
/// `.ldtk` source files get authored. Missing files are tolerated —
/// the project still boots from sandbox.ldtk alone — so a partial
/// checkout doesn't crash startup.
const SECONDARY_WORLD_FILES: &[&str] = &["intro.ldtk"];

impl LdtkProject {
    /// Load the sandbox LDtk project through the asset catalog.
    ///
    /// Resolves [`crate::sandbox_assets::ids::sandbox_ldtk`] under the
    /// active [`ambition_asset_manager::AssetProfile`]:
    ///
    /// - Desktop loose/installed: reads the file at the catalog's
    ///   `LocalPath` candidate and falls back to the embedded static
    ///   map (when the `static_map` feature is enabled) if disk IO
    ///   fails. Hot reload remains armed via
    ///   [`crate::ldtk_world::LdtkHotReloadState`].
    /// - Android / iOS / Web / Bundled static: uses the embedded
    ///   `include_str!` byte stream from
    ///   [`Self::load_static_map`].
    /// - `NoAssets` / `Headless`: returns the required-asset error
    ///   (matches [`ambition_asset_manager::MissingAssetPolicy::Error`]).
    pub fn load_default(catalog: &SandboxAssetCatalog) -> Result<Self, String> {
        let resolved = catalog
            .resolve(&ids::sandbox_ldtk())
            .map_err(|err| format!("LDtk resolve failed: {err}"))?;

        // Disabled under NoAssets / Headless — the asset is required
        // (MissingAssetPolicy::Error) so the catalog tolerance check
        // controls whether this is fatal.
        if resolved.location.is_disabled() {
            return Err(
                "sandbox LDtk world is disabled under the active asset profile; \
                 this is fatal (MissingAssetPolicy::Error). Pick a profile that \
                 ships the LDtk world or rebuild with `--features static_map`."
                    .to_string(),
            );
        }

        // Filesystem-resident location: read from disk; on IO failure,
        // fall back to the embedded copy when `static_map` is enabled.
        if let Some(local) = resolved.location.as_local_path() {
            match Self::load_from_path(local) {
                Ok(mut project) => {
                    merge_secondary_worlds(&mut project, local);
                    return Ok(project);
                }
                Err(error) => {
                    #[cfg(feature = "static_map")]
                    {
                        eprintln!(
                            "LDtk warning: {error}; falling back to statically packed sandbox.ldtk"
                        );
                        let mut project =
                            Self::load_static_map().map_err(|fallback_error| {
                                format!(
                                    "{error}; statically packed sandbox.ldtk also failed: \
                                     {fallback_error}"
                                )
                            })?;
                        merge_static_secondary_worlds(&mut project);
                        return Ok(project);
                    }
                    #[cfg(not(feature = "static_map"))]
                    {
                        return Err(format!(
                            "{error}. No statically packed fallback is available in this build; \
                             restore the LDtk asset or rebuild with `--features static_map`."
                        ));
                    }
                }
            }
        }

        // Embedded / Bevy-path-only locations (Android, web, bundled).
        // The `static_map` feature provides the bytes via include_str!.
        #[cfg(feature = "static_map")]
        {
            let mut project = Self::load_static_map()?;
            merge_static_secondary_worlds(&mut project);
            Ok(project)
        }
        #[cfg(not(feature = "static_map"))]
        {
            Err(format!(
                "sandbox LDtk world resolved to {:?} under {} profile, but the build \
                 has no `static_map` feature to read the embedded bytes. Either build with \
                 `--features static_map` or pick a profile that resolves to a LocalPath.",
                resolved.location,
                resolved.profile.label(),
            ))
        }
    }

    /// Test / headless / RL shortcut: build a desktop-dev catalog and
    /// load the LDtk world through it. Production startup builds the
    /// catalog from the live `GameAssetConfig` resource and threads it
    /// in via [`Self::load_default`]; this helper is the equivalent
    /// for entry points that don't have a Bevy `World` yet.
    pub fn load_default_for_dev() -> Result<Self, String> {
        let catalog = SandboxAssetCatalog::for_desktop_dev_default();
        Self::load_default(&catalog)
    }

    #[cfg(feature = "static_map")]
    pub fn load_static_map() -> Result<Self, String> {
        serde_json::from_str(include_str!("../../assets/ambition/worlds/sandbox.ldtk"))
            .map_err(|error| format!("could not parse statically packed sandbox.ldtk: {error}"))
    }

    /// `[ambition_asset_manager_transition]` Disk-only load helper, kept
    /// for the LDtk hot-reload path that uses
    /// [`crate::ldtk_world::LdtkHotReloadState::watch_path`] +
    /// [`Self::load_from_path`] to re-parse after a file-change event.
    /// Once hot reload also consults the catalog for fresh paths this
    /// helper can be inlined.
    pub fn load_from_disk_at(path: &Path) -> Result<Self, String> {
        let mut project = Self::load_from_path(path)?;
        merge_secondary_worlds(&mut project, path);
        Ok(project)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("could not read LDtk project {}: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("could not parse LDtk project {}: {error}", path.display()))
    }
}

/// Walk [`SECONDARY_WORLD_FILES`] and append each present file's levels
/// into `project`. Missing files are skipped with a warning; malformed
/// files are skipped with a warning and the sandbox keeps booting.
fn merge_secondary_worlds(project: &mut LdtkProject, sandbox_path: &Path) {
    let Some(parent) = sandbox_path.parent() else {
        return;
    };
    for name in SECONDARY_WORLD_FILES {
        let path = parent.join(name);
        if !path.exists() {
            continue;
        }
        match LdtkProject::load_from_path(&path) {
            Ok(secondary) => append_levels(project, secondary, name),
            Err(error) => {
                eprintln!(
                    "LDtk warning: could not load secondary world '{name}' from {}: {error}; \
                     continuing without it",
                    path.display()
                );
            }
        }
    }
}

#[cfg(feature = "static_map")]
fn merge_static_secondary_worlds(project: &mut LdtkProject) {
    // Android `static_map` builds embed the sandbox map at compile
    // time. Secondary worlds need the same treatment — `include_str!`
    // each known file when it exists in the workspace. We currently
    // hard-code the intro because there's exactly one secondary
    // file; a build-time codegen pass can replace this when the list
    // grows.
    const INTRO_LDTK_STATIC: &str =
        include_str!("../../assets/ambition/worlds/intro.ldtk");
    match serde_json::from_str::<LdtkProject>(INTRO_LDTK_STATIC) {
        Ok(secondary) => append_levels(project, secondary, "intro.ldtk"),
        Err(error) => eprintln!(
            "LDtk warning: could not parse statically packed intro.ldtk: {error}; \
             continuing without it"
        ),
    }
}

#[cfg(not(feature = "static_map"))]
#[allow(dead_code)]
fn merge_static_secondary_worlds(_project: &mut LdtkProject) {}

fn append_levels(project: &mut LdtkProject, secondary: LdtkProject, source_label: &str) {
    let added = secondary.levels.len();
    project.levels.extend(secondary.levels);
    eprintln!("LDtk: merged {added} level(s) from secondary world '{source_label}'");
}
