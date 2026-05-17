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

use ambition_asset_manager::AssetId;

use crate::sandbox_assets::{ids, SandboxAssetCatalog};

use super::project::LdtkProject;

/// Story-content world ids appended into the runtime project on top of
/// `world.sandbox_ldtk`. Each id is looked up through
/// [`SandboxAssetCatalog`]; the resolved location decides whether to
/// read from disk (DesktopDevLoose) or fall back to the embedded
/// static copy (Web / Android / Bundled).
///
/// Adding a new secondary world entails (1) a catalog id constructor
/// under [`crate::sandbox_assets::ids`], (2) a manifest entry in
/// `crate::sandbox_assets::extend_with_world_entries`, and (3) one
/// row here. Missing entries are tolerated so a partial checkout
/// still boots `sandbox.ldtk` alone.
fn secondary_world_ids() -> Vec<AssetId> {
    vec![ids::intro_ldtk()]
}

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
                    merge_secondary_worlds_via_catalog(&mut project, catalog);
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
        serde_json::from_str(include_str!("../../../assets/ambition/worlds/sandbox.ldtk"))
            .map_err(|error| format!("could not parse statically packed sandbox.ldtk: {error}"))
    }

    /// Hot-reload re-parse helper: read the LDtk file the watcher
    /// discovered at startup, then re-merge secondary worlds via the
    /// shared catalog. Catalog is passed by the caller because the
    /// hot-reload system has both resources in hand.
    pub fn load_from_disk_at(
        path: &Path,
        catalog: &SandboxAssetCatalog,
    ) -> Result<Self, String> {
        let mut project = Self::load_from_path(path)?;
        merge_secondary_worlds_via_catalog(&mut project, catalog);
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

/// Walk the catalog-driven [`secondary_world_ids`] list and append
/// each present file's levels into `project`. Missing or disabled
/// entries are skipped silently; malformed files log a warning and
/// the sandbox keeps booting. Only `LocalPath` resolutions (desktop
/// profiles) are read here — embedded/static secondary worlds flow
/// through [`merge_static_secondary_worlds`].
fn merge_secondary_worlds_via_catalog(
    project: &mut LdtkProject,
    catalog: &SandboxAssetCatalog,
) {
    for id in secondary_world_ids() {
        let Ok(resolved) = catalog.resolve(&id) else {
            continue;
        };
        let Some(local) = resolved.location.as_local_path() else {
            // Embedded / disabled / bevy-only locations don't read
            // through disk IO. Skip — `merge_static_secondary_worlds`
            // handles the embedded path on static profiles.
            continue;
        };
        if !local.exists() {
            continue;
        }
        match LdtkProject::load_from_path(local) {
            Ok(secondary) => append_levels(project, secondary, id.as_str()),
            Err(error) => {
                eprintln!(
                    "LDtk warning: could not load secondary world '{id}' from {}: {error}; \
                     continuing without it",
                    local.display()
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
        include_str!("../../../assets/ambition/worlds/intro.ldtk");
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
