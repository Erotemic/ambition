//! Bevy plugin / resource / helper layer.
//!
//! Wraps [`crate::manifest::AssetManifest`] + [`crate::profile::AssetProfile`]
//! in two Bevy resources and adds load helpers that route through
//! Bevy's `AssetServer` and `AssetPath` machinery. The crate does NOT
//! re-implement async loading, handles, dependencies, or hot reload —
//! those live in Bevy itself.
//!
//! ## Wiring
//!
//! ```ignore
//! use ambition_asset_manager::{
//!     bevy_integration::{
//!         AmbitionAssetCatalog, AmbitionAssetManagerPlugin, AmbitionAssetProfile,
//!     },
//!     AssetManifest, AssetProfile,
//! };
//! use bevy::prelude::*;
//!
//! fn build_app() -> App {
//!     let mut app = App::new();
//!     let manifest: AssetManifest = build_my_manifest();
//!     app.add_plugins(MinimalPlugins)
//!         .add_plugins(AmbitionAssetManagerPlugin {
//!             manifest,
//!             profile: AssetProfile::DesktopDevLoose,
//!         });
//!     app
//! }
//! ```
//!
//! The plugin inserts [`AmbitionAssetCatalog`] and
//! [`AmbitionAssetProfile`] resources. Callers query the catalog through
//! its [`AmbitionAssetCatalog::path_for`] / [`AmbitionAssetCatalog::load`]
//! helpers.
//!
//! ## Registering custom Bevy `AssetSource`s
//!
//! Source registrations (`embedded`, `http`, `https`, custom IPFS) live
//! in the consuming app. Bevy's `AssetPlugin::source` (Bevy 0.18+) is
//! the canonical hook. This crate intentionally does not auto-register
//! sources — the consumer knows which features it compiled with.
//!
//! See `docs/asset_manager.md` for the integration recipe per profile
//! (`bevy_embedded_assets` for BundledStatic, Bevy's `http` source for
//! WebHttp, etc.).

#![cfg(feature = "bevy")]

use bevy::asset::Asset;
use bevy::prelude::{App, AssetServer, Handle, Plugin, Res, Resource};

use crate::id::AssetId;
use crate::manifest::AssetManifest;
use crate::profile::AssetProfile;
use crate::resolver::{resolve, AssetResolutionError, ResolvedAsset};

/// Bevy plugin that installs the asset catalog resources. Construct
/// with the active manifest + profile; both are cloned into the
/// `World` so the plugin instance can be discarded after `add_plugins`.
pub struct AmbitionAssetManagerPlugin {
    pub manifest: AssetManifest,
    pub profile: AssetProfile,
}

impl Plugin for AmbitionAssetManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AmbitionAssetCatalog::new(self.manifest.clone()))
            .insert_resource(AmbitionAssetProfile(self.profile));
    }
}

/// Catalog resource: the merged manifest + helpers.
///
/// Keep the resource for the lifetime of the App; the manifest is
/// `Clone` but cheap to share through `Res<AmbitionAssetCatalog>`.
#[derive(Resource, Clone, Debug)]
pub struct AmbitionAssetCatalog {
    manifest: AssetManifest,
}

impl AmbitionAssetCatalog {
    pub fn new(manifest: AssetManifest) -> Self {
        Self { manifest }
    }

    /// Borrow the underlying manifest. Useful for diagnostics; most
    /// call sites should reach for [`Self::resolve`] or
    /// [`Self::path_for`] instead.
    pub fn manifest(&self) -> &AssetManifest {
        &self.manifest
    }

    /// Resolve `id` under the active `profile`. Wraps
    /// [`crate::resolver::resolve`] for ergonomics from systems.
    pub fn resolve(
        &self,
        id: &AssetId,
        profile: AssetProfile,
    ) -> Result<ResolvedAsset, AssetResolutionError> {
        resolve(&self.manifest, id, profile)
    }

    /// Return the Bevy `AssetPath` string for `id` under `profile`, or
    /// `None` if the asset is disabled / not Bevy-pathable.
    pub fn path_for(&self, id: &AssetId, profile: AssetProfile) -> Option<String> {
        self.resolve(id, profile).ok()?.bevy_asset_path()
    }

    /// Load `id` through Bevy's `AssetServer` and return a typed
    /// `Handle<T>`. Panics with a descriptive message when the manifest
    /// has no entry for `id` (programmer error). Returns `None` when
    /// the asset is `Disabled` under `profile` — the call site decides
    /// whether to honor `MissingAssetPolicy::Error` or fall back to a
    /// placeholder.
    pub fn load_optional<T: Asset>(
        &self,
        asset_server: &AssetServer,
        id: &AssetId,
        profile: AssetProfile,
    ) -> Option<Handle<T>> {
        let resolved = self
            .resolve(id, profile)
            .unwrap_or_else(|err| panic!("ambition_asset_manager: {err}"));
        let path = resolved.bevy_asset_path()?;
        Some(asset_server.load(path))
    }

    /// Same as [`Self::load_optional`] but returns a placeholder
    /// handle (`Handle::default()`) instead of `None` when the asset is
    /// disabled. Useful for call sites that always want a `Handle<T>`
    /// to slot into a `Sprite` / `Mesh` component.
    pub fn load_with_default<T: Asset>(
        &self,
        asset_server: &AssetServer,
        id: &AssetId,
        profile: AssetProfile,
    ) -> Handle<T> {
        self.load_optional::<T>(asset_server, id, profile)
            .unwrap_or_default()
    }
}

/// Active asset profile. Read by [`AmbitionAssetCatalog::path_for`] /
/// [`AmbitionAssetCatalog::load_optional`] callers via
/// `Res<AmbitionAssetProfile>`.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AmbitionAssetProfile(pub AssetProfile);

impl Default for AmbitionAssetProfile {
    fn default() -> Self {
        // The most-conservative default: a desktop dev session with
        // loose files. Real apps should set this from CLI / cfg in
        // their plugin construction call.
        Self(AssetProfile::DesktopDevLoose)
    }
}

/// Bevy-flavored helper: resolve via the catalog using whatever
/// profile the app currently has. Wraps the typical
/// `Res<AmbitionAssetCatalog>` + `Res<AmbitionAssetProfile>` query so
/// systems can read both with one parameter pack.
pub fn path_for_active(
    catalog: &Res<AmbitionAssetCatalog>,
    profile: &Res<AmbitionAssetProfile>,
    id: &AssetId,
) -> Option<String> {
    catalog.path_for(id, profile.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::AssetKind;
    use crate::manifest::{AssetEntry, AssetManifest};
    use crate::policy::MissingAssetPolicy;

    fn fixture() -> AssetManifest {
        AssetManifest::builder()
            .entry(
                AssetEntry::new(
                    "world.sandbox_ldtk",
                    AssetKind::LdtkProject,
                    "ambition/worlds/sandbox.ldtk",
                )
                .with_missing_policy(MissingAssetPolicy::Error),
            )
            .build()
    }

    #[test]
    fn path_for_returns_bevy_path_under_desktop_dev_loose() {
        let catalog = AmbitionAssetCatalog::new(fixture());
        let path = catalog
            .path_for(&AssetId::new("world.sandbox_ldtk"), AssetProfile::DesktopDevLoose)
            .unwrap();
        assert_eq!(path, "ambition/worlds/sandbox.ldtk");
    }

    #[test]
    fn path_for_returns_none_under_no_assets() {
        let catalog = AmbitionAssetCatalog::new(fixture());
        assert!(catalog
            .path_for(&AssetId::new("world.sandbox_ldtk"), AssetProfile::NoAssets)
            .is_none());
    }

    #[test]
    fn path_for_returns_none_for_unknown_id() {
        let catalog = AmbitionAssetCatalog::new(fixture());
        assert!(catalog
            .path_for(&AssetId::new("missing"), AssetProfile::DesktopDevLoose)
            .is_none());
    }

    #[test]
    fn web_static_synthesizes_embedded_path() {
        let catalog = AmbitionAssetCatalog::new(fixture());
        let path = catalog
            .path_for(&AssetId::new("world.sandbox_ldtk"), AssetProfile::WebStatic)
            .unwrap();
        assert_eq!(path, "embedded://ambition/worlds/sandbox.ldtk");
    }
}
