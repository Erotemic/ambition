//! Sandbox-side aggregator for the [`ambition_asset_manager`] catalog.
//!
//! This module builds the single [`SandboxAssetCatalog`] resource used by visible
//! sandbox systems to resolve Bevy asset paths: sprites, parallax, character and
//! boss sheets, fonts, LDtk world/data, SFX bank, and music tracks.
//!
//! Construction starts from the image manifest in
//! [`crate::assets::game_assets::sandbox_image_manifest`], extends it with domain
//! builders, and wraps the result with the active [`AssetProfile`]. Consumers ask
//! the catalog for a path and pass it to Bevy's `AssetServer`; the catalog itself
//! performs no IO.

use std::path::PathBuf;

use bevy::prelude::Resource;

use ambition_asset_manager::{
    AmbitionAssetCatalog, AssetId, AssetManifest, AssetProfile, AssetResolutionError,
    AssetSourceProfile, ResolvedAsset,
};

// The `tests` module reaches for several `ambition_asset_manager` types
// through `use super::*`. Re-import them under `#[cfg(test)]` so the
// prod `lib` build doesn't flag them as unused, while the test build
// still sees them at this module's path.
#[cfg(test)]
use ambition_asset_manager::{AssetKind, AssetLocation, MissingAssetPolicy, PreloadGroup};

use crate::assets::game_assets::{sandbox_image_manifest, GameAssetConfig};
use crate::runtime::data::AudioSpec;

mod builders;
mod embedded;
pub mod ids;

use builders::{
    extend_with_boss_entries, extend_with_character_entries, extend_with_data_entries,
    extend_with_font_entries, extend_with_music_entries, extend_with_sfx_bank_entry,
    extend_with_world_entries,
};
pub use embedded::AmbitionAssetSourcePlugin;
pub(crate) use embedded::{
    embedded_core, EMBEDDED_CUT_ROPE_LDTK_ASSET_PATH, EMBEDDED_INTRO_LDTK_ASSET_PATH,
    EMBEDDED_SANDBOX_LDTK_ASSET_PATH,
};

/// Wrapped [`AmbitionAssetCatalog`] + active [`AssetProfile`].
///
/// One instance per app session, installed as a Bevy `Resource` by
/// [`crate::schedule::init_sandbox_resources`]. Every subsystem that loads
/// an asset goes through this; nothing else owns asset-source policy.
///
/// Cheap to clone (the underlying manifest is wrapped in an `Arc`-like
/// shared shape inside [`AmbitionAssetCatalog`]'s `Clone` impl).
#[derive(Resource, Clone, Debug)]
pub struct SandboxAssetCatalog {
    catalog: AmbitionAssetCatalog,
    profile: AssetProfile,
}

impl SandboxAssetCatalog {
    /// Construct from a fully-built [`AmbitionAssetCatalog`] + the
    /// active profile. Prefer [`build_sandbox_catalog`] from
    /// production code; this is the seam for unit tests that author
    /// a partial manifest.
    pub fn new(catalog: AmbitionAssetCatalog, profile: AssetProfile) -> Self {
        Self { catalog, profile }
    }

    /// Convenience: build a desktop-dev catalog from the embedded
    /// sandbox spec, suitable for headless / RL / test fixtures that
    /// don't have a [`crate::assets::game_assets::GameAssetConfig`] resource
    /// in hand. Production startup builds the catalog from the live
    /// `GameAssetConfig` via [`build_sandbox_catalog`].
    pub fn for_desktop_dev_default() -> Self {
        let config = crate::assets::game_assets::GameAssetConfig {
            asset_profile: AssetProfile::DesktopDevLoose,
            ..Default::default()
        };
        let spec = crate::runtime::data::SandboxDataSpec::load_embedded();
        build_sandbox_catalog(&config, &spec.audio)
    }

    pub fn catalog(&self) -> &AmbitionAssetCatalog {
        &self.catalog
    }

    pub fn profile(&self) -> AssetProfile {
        self.profile
    }

    pub fn path_for(&self, id: &AssetId) -> Option<String> {
        self.catalog.path_for(id, self.profile)
    }

    pub fn resolve(&self, id: &AssetId) -> Result<ResolvedAsset, AssetResolutionError> {
        self.catalog.resolve(id, self.profile)
    }

    /// Local filesystem path the LDtk hot-reload watcher should poll,
    /// when both the active profile and the resolved location support
    /// it. `None` everywhere else (bundled / web / no-assets).
    pub fn hot_reload_local_path(&self, id: &AssetId) -> Option<PathBuf> {
        let resolved = self.resolve(id).ok()?;
        if !resolved.supports_hot_reload() {
            return None;
        }
        resolved.location.as_local_path().map(|p| p.to_path_buf())
    }

    /// Resolve `id` and apply the per-profile load gate in one call.
    ///
    /// Returns `Some(path)` when the loader should hand the path to
    /// Bevy's `AssetServer::load`; `None` when the loader should fall
    /// back (colored rectangle, silent SFX, Bevy default font, etc.).
    ///
    /// This is the **only** function loaders need to call — it combines:
    /// - `path_for(id)` (resolver),
    /// - the per-profile "is this asset actually available?" gate
    ///   ([`Self::should_attempt_resolved_load`]).
    ///
    /// Consumers that need the local on-disk path (the SFX bank byte
    /// loader, the LDtk hot-reload watcher) go through
    /// [`Self::resolve_local_file_path`] / [`Self::hot_reload_local_path`].
    pub fn try_path_for_load(&self, id: &AssetId) -> Option<String> {
        let resolved = self.resolve(id).ok()?;
        let path = resolved.bevy_asset_path()?;
        if self.should_attempt_resolved_load(&resolved, &path) {
            Some(path)
        } else {
            None
        }
    }

    /// Per-profile load gate keyed on a fully-resolved entry.
    ///
    /// - Desktop (DevLoose / Installed / SteamDeck): pre-check the host
    ///   filesystem via the candidate-roots walker
    ///   ([`desktop_candidate_roots`]) so missing optional art falls back
    ///   to colored rectangles / Bevy's default font before Bevy logs a
    ///   load failure. Required assets always attempt the load so the
    ///   `MissingAssetPolicy::Error` path can surface a useful error.
    /// - Android / iOS bundle: trust the packager; let Bevy's platform
    ///   `AssetReader` try the load.
    /// - Web / BundledStatic: attempt the load when the entry has an
    ///   **authored** embedded candidate (the bytes are packaged via
    ///   `embedded_asset!`); skip otherwise to preserve colored-rectangle
    ///   fallback.
    /// - WebHttp: attempt only when the entry has an authored
    ///   `HttpRemote` candidate. Optional images today have none, so they
    ///   fall back to placeholders.
    /// - IpfsGatewayPlaceholder: attempt when an authored `IpfsGateway`
    ///   candidate is present.
    /// - NoAssets / Headless: never attempt (catalog already returned
    ///   None for `path_for`; this is exhaustive-match insurance).
    pub fn should_attempt_resolved_load(&self, resolved: &ResolvedAsset, path: &str) -> bool {
        match self.profile {
            AssetProfile::DesktopDevLoose
            | AssetProfile::DesktopInstalled
            | AssetProfile::SteamDeckInstalled => {
                resolved.missing_policy.is_required()
                    || self.resolve_local_file_path(path).is_some()
            }
            AssetProfile::AndroidBundle | AssetProfile::IosBundle => true,
            AssetProfile::WebStatic | AssetProfile::BundledStatic => {
                resolved.authored_candidate
                    && matches!(
                        resolved.source_used,
                        Some(AssetSourceProfile::EmbeddedBinary)
                    )
            }
            // WebServedAssets attempts every resolution that produces
            // a Bevy-pathable URL: either an authored `Embedded`
            // candidate (delivered from `EmbeddedAssetRegistry`) or
            // the synthesized `BevyPath` from `logical_path` (which
            // Bevy's wasm HTTP reader fetches from `/assets/<path>`).
            // Missing files surface as Bevy load-failure logs + the
            // renderer's existing placeholder fallbacks; we cannot
            // pre-check the host filesystem from the browser, so the
            // "trust Bevy to fetch" stance matches Android/iOS.
            AssetProfile::WebServedAssets => {
                matches!(
                    resolved.source_used,
                    Some(AssetSourceProfile::EmbeddedBinary)
                        | Some(AssetSourceProfile::InstalledFilesystem)
                )
            }
            AssetProfile::WebHttp => {
                resolved.authored_candidate
                    && matches!(resolved.source_used, Some(AssetSourceProfile::HttpRemote))
            }
            AssetProfile::IpfsGatewayPlaceholder => {
                resolved.authored_candidate
                    && matches!(resolved.source_used, Some(AssetSourceProfile::IpfsGateway))
            }
            AssetProfile::NoAssets | AssetProfile::Headless => false,
        }
    }

    /// Same gate, but for **required** assets. Required entries with
    /// no host-filesystem precheck always attempt the load; the
    /// resolver's `Disabled` path is what consults
    /// [`MissingAssetPolicy::Error`].
    pub fn should_attempt_required_load(&self, _path: &str) -> bool {
        !matches!(
            self.profile,
            AssetProfile::NoAssets | AssetProfile::Headless
        )
    }

    /// Locate the absolute on-disk path for a Bevy-relative asset path
    /// under the current profile, when one is available. Returns
    /// `None` for non-desktop profiles or when the file simply isn't
    /// there. Walks the same candidate roots Bevy's file `AssetReader`
    /// consults at runtime, in this order:
    ///
    /// 1. `$BEVY_ASSET_ROOT/assets/<rel>`
    /// 2. `$BEVY_ASSET_ROOT/<rel>`
    /// 3. `$CWD/assets/<rel>`
    /// 4. `$CWD/<rel>`
    /// 5. `$CARGO_MANIFEST_DIR/assets/<rel>` (dev fallback)
    ///
    /// This is the **only** host-filesystem probe in the sandbox. The
    /// LDtk hot-reload watcher and the SFX bank byte loader both call
    /// through here — there is no duplicate candidate walk anywhere
    /// else in `crates/ambition_gameplay_core/src/`.
    pub fn resolve_local_file_path(&self, rel: &str) -> Option<std::path::PathBuf> {
        if !matches!(
            self.profile,
            AssetProfile::DesktopDevLoose
                | AssetProfile::DesktopInstalled
                | AssetProfile::SteamDeckInstalled
        ) {
            return None;
        }
        desktop_candidate_roots(rel)
            .into_iter()
            .find(|p| p.exists())
    }
}

/// Build the ordered candidate roots for `rel_path` on desktop / Steam
/// Deck profiles. The only candidate-roots walker in the sandbox;
/// [`SandboxAssetCatalog::resolve_local_file_path`] (and through it
/// `should_attempt_optional_load` / `try_path_for_load`) are the sole
/// callers.
fn desktop_candidate_roots(rel_path: &str) -> Vec<std::path::PathBuf> {
    let rel = std::path::Path::new(rel_path);
    let mut candidates = Vec::with_capacity(5);
    if let Some(root) = std::env::var_os("BEVY_ASSET_ROOT") {
        let root = std::path::PathBuf::from(root);
        candidates.push(root.join("assets").join(rel));
        candidates.push(root.join(rel));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets").join(rel));
        candidates.push(cwd.join(rel));
    }
    candidates.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel),
    );
    candidates
}

/// Build the full sandbox catalog: every visible-sandbox asset id +
/// the active profile. Called once during `init_sandbox_resources`.
///
/// `audio` is borrowed from the already-loaded [`crate::runtime::data::SandboxDataSpec`]
/// so music-track ids land in the catalog at startup; the spec itself
/// is loaded via `include_str!`, so the catalog doesn't depend on
/// disk-resident files for bootstrap.
pub fn build_sandbox_catalog(config: &GameAssetConfig, audio: &AudioSpec) -> SandboxAssetCatalog {
    build_sandbox_catalog_with(config, audio, |_| {})
}

/// [`build_sandbox_catalog`] with a content-extension hook: the app
/// assembly passes the content layer's extra manifest entries (e.g.
/// the intro sprite rows) so this machinery module names no content.
pub fn build_sandbox_catalog_with(
    config: &GameAssetConfig,
    audio: &AudioSpec,
    extend: impl FnOnce(&mut AssetManifest),
) -> SandboxAssetCatalog {
    let mut manifest = sandbox_image_manifest(&config.sprite_folder);
    extend_with_world_entries(&mut manifest);
    extend_with_data_entries(&mut manifest);
    extend_with_sfx_bank_entry(&mut manifest);
    extend_with_font_entries(&mut manifest);
    extend_with_character_entries(&mut manifest, &config.sprite_folder);
    extend_with_boss_entries(&mut manifest, &config.sprite_folder);
    extend_with_music_entries(&mut manifest, audio);
    extend(&mut manifest);
    SandboxAssetCatalog::new(AmbitionAssetCatalog::new(manifest), config.asset_profile)
}

#[cfg(test)]
mod tests;
