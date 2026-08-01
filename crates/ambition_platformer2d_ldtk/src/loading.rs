//! LDtk file-loading policy.
//!
//! Decides whether the live build reads a world's checked-in file from
//! disk, an env-override path, or the statically embedded copy — for
//! every world the caller's [`super::manifest::WorldManifest`] declares.
//! Pure I/O policy — no validation or runtime conversion lives here, and
//! no world is named here (the manifest rows carry identity, paths, and
//! embedded fallbacks).
//!
//! Per row, the path is selected by
//! [`crate::assets::sandbox_assets::SandboxAssetCatalog`] under the active
//! [`ambition_asset_manager::AssetProfile`]:
//! - `DesktopDevLoose` / `DesktopInstalled` / `SteamDeckInstalled` →
//!   `LocalPath` resolved against the canonical assets root.
//! - `AndroidBundle` / `IosBundle` / `WebStatic` / `BundledStatic` →
//!   the row's `embedded_text` (present when the authoring crate built
//!   with its static-embed feature).
//! - `WebHttp` → HTTP candidate if authored; currently falls back to
//!   embedded.
//! - `NoAssets` / `Headless` → `Disabled` → loader returns the
//!   required-asset error for the primary world.
//!
//! ## Multi-file world composition
//!
//! Secondary worlds are authored against the same project defs as the
//! primary (cloned via `python -m ambition_ldtk_tools world init`) so their
//! entity/layer uids match — the runtime simply appends their `levels`
//! arrays into the merged in-memory `LdtkProject` without remapping.
//! Every level keeps its own iid, activeArea, and LoadingZone targets, so
//! cross-file room transitions work via the standard target_room mechanism.

use std::fs;
use std::path::Path;

use ambition_asset_manager::sandbox_assets::{
    build_sandbox_catalog, SandboxAssetCatalog, SandboxAssetConfig, SandboxCatalogInputs,
    WorldCatalogRow,
};
use ambition_asset_manager::{AssetManifest, AssetProfile};

use super::manifest::{WorldManifest, WorldSource};
use super::project::LdtkProject;

impl LdtkProject {
    /// Load the manifest's primary LDtk world through the asset catalog and
    /// merge every secondary world onto it.
    ///
    /// - Filesystem-resident location: reads the file at the catalog's
    ///   `LocalPath` candidate, falling back to the row's embedded text if
    ///   disk IO fails. Hot reload remains armed via
    ///   [`crate::LdtkHotReloadState`].
    /// - Embedded / Bevy-path-only locations (Android, iOS, web, bundled):
    ///   parses the row's `embedded_text`.
    /// - `NoAssets` / `Headless`: returns the required-asset error
    ///   (matches [`ambition_asset_manager::MissingAssetPolicy::Error`]).
    pub fn load_default(
        catalog: &SandboxAssetCatalog,
        manifest: &WorldManifest,
    ) -> Result<Self, String> {
        let primary = manifest.primary();
        let resolved = catalog
            .resolve(&primary.id)
            .map_err(|err| format!("LDtk resolve failed: {err}"))?;

        // Disabled under NoAssets / Headless — the primary world is required
        // (MissingAssetPolicy::Error) so the catalog tolerance check
        // controls whether this is fatal.
        if resolved.location.is_disabled() {
            return Err(format!(
                "primary LDtk world '{}' is disabled under the active asset profile; \
                 this is fatal (MissingAssetPolicy::Error). Pick a profile that \
                 ships the LDtk world or build the world-owning crate with its \
                 static-embed feature.",
                primary.id
            ));
        }

        // Filesystem-resident location: read from disk; on IO failure,
        // fall back to the row's embedded copy when one is compiled in.
        if let Some(local) = resolved.location.as_local_path() {
            match Self::load_from_path(local) {
                Ok(mut project) => {
                    merge_secondary_worlds(&mut project, catalog, manifest);
                    return Ok(project);
                }
                Err(error) => {
                    let Some(text) = primary.embedded_text else {
                        return Err(format!(
                            "{error}. No statically embedded fallback is available in this \
                             build; restore the LDtk asset or build the world-owning crate \
                             with its static-embed feature."
                        ));
                    };
                    eprintln!(
                        "LDtk warning: {error}; falling back to the statically embedded '{}'",
                        primary.id
                    );
                    let mut project =
                        parse_world_text(text, primary).map_err(|fallback_error| {
                            format!("{error}; the embedded copy also failed: {fallback_error}")
                        })?;
                    merge_secondary_worlds(&mut project, catalog, manifest);
                    return Ok(project);
                }
            }
        }

        // Embedded / Bevy-path-only locations (Android, web, bundled).
        let Some(text) = primary.embedded_text else {
            return Err(format!(
                "primary LDtk world '{}' resolved to {:?} under {} profile, but the \
                 build embeds no world text. Either build the world-owning crate with \
                 its static-embed feature or pick a profile that resolves to a LocalPath.",
                primary.id,
                resolved.location,
                resolved.profile.label(),
            ));
        };
        let mut project = parse_world_text(text, primary)?;
        merge_secondary_worlds(&mut project, catalog, manifest);
        Ok(project)
    }

    /// Test / headless / RL shortcut: build a desktop-dev catalog and
    /// load the LDtk world through it. Production startup builds the
    /// catalog from the live `GameAssetConfig` resource and threads it
    /// in via [`Self::load_default`]; this helper is the equivalent
    /// for entry points that don't have a Bevy `World` yet.
    pub fn load_default_for_dev(manifest: &WorldManifest) -> Result<Self, String> {
        let config = SandboxAssetConfig {
            sprite_folder: "sprites".to_string(),
            asset_profile: AssetProfile::DesktopDevLoose,
        };
        let inputs = SandboxCatalogInputs {
            worlds: manifest
                .worlds
                .iter()
                .map(|source| WorldCatalogRow {
                    id: source.id.clone(),
                    asset_path: source.asset_path.clone(),
                    required: source.required,
                    loose_path: source.loose_path.clone(),
                    embedded_bevy_path: source.embedded_bevy_path,
                })
                .collect(),
            ..Default::default()
        };
        let catalog = build_sandbox_catalog(&config, AssetManifest::new(), &inputs);
        Self::load_default(&catalog, manifest)
    }

    /// Hot-reload re-parse helper: read the LDtk file the watcher
    /// discovered at startup, then re-merge secondary worlds via the
    /// shared catalog. Catalog and manifest are passed by the caller because
    /// the hot-reload system has both in hand.
    pub fn load_from_disk_at(
        path: &Path,
        catalog: &SandboxAssetCatalog,
        manifest: &WorldManifest,
    ) -> Result<Self, String> {
        let mut project = Self::load_from_path(path)?;
        merge_secondary_worlds(&mut project, catalog, manifest);
        Ok(project)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)
            .map_err(|error| format!("could not read LDtk project {}: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("could not parse LDtk project {}: {error}", path.display()))
    }

    /// Parse an in-memory LDtk project JSON string — the entry point for a
    /// GAME crate that ships its own `include_str!`-embedded world instead of
    /// registering a row in the engine's world manifest (a demo's standalone
    /// level file). No secondary-world merging: the caller owns exactly one
    /// file.
    pub fn from_json_str(text: &str) -> Result<Self, String> {
        serde_json::from_str(text)
            .map_err(|error| format!("could not parse LDtk project JSON: {error}"))
    }
}

fn parse_world_text(text: &str, source: &WorldSource) -> Result<LdtkProject, String> {
    serde_json::from_str(text).map_err(|error| {
        format!(
            "could not parse statically embedded LDtk world '{}': {error}",
            source.id
        )
    })
}

/// Walk the manifest's secondary worlds and append each resolvable one's
/// levels into `project`. Per row: prefer the catalog's on-disk copy;
/// fall back to the row's embedded text; skip (secondaries are optional)
/// when neither is available. Malformed files log a warning and the
/// primary keeps booting.
fn merge_secondary_worlds(
    project: &mut LdtkProject,
    catalog: &SandboxAssetCatalog,
    manifest: &WorldManifest,
) {
    for source in manifest.secondaries() {
        if let Ok(resolved) = catalog.resolve(&source.id) {
            if let Some(local) = resolved.location.as_local_path() {
                if local.exists() {
                    match LdtkProject::load_from_path(local) {
                        Ok(secondary) => {
                            append_levels(project, secondary, source.id.as_str());
                        }
                        Err(error) => {
                            eprintln!(
                                "LDtk warning: could not load secondary world '{}' from {}: \
                                 {error}; continuing without it",
                                source.id,
                                local.display()
                            );
                        }
                    }
                    continue;
                }
            }
        }
        // No usable on-disk copy — embedded fallback (static profiles),
        // else skip: secondaries are optional so a partial checkout boots.
        let Some(text) = source.embedded_text else {
            continue;
        };
        match parse_world_text(text, source) {
            Ok(secondary) => append_levels(project, secondary, source.id.as_str()),
            Err(error) => eprintln!("LDtk warning: {error}; continuing without it"),
        }
    }
}

fn append_levels(project: &mut LdtkProject, secondary: LdtkProject, source_label: &str) {
    let added = secondary.levels.len();
    project.levels.extend(secondary.levels);
    eprintln!("LDtk: merged {added} level(s) from secondary world '{source_label}'");
}
