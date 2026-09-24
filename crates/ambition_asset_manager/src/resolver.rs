//! Resolve `(AssetId, AssetProfile)` into a runtime asset location.
//!
//! Resolution follows the profile's preferred source order, using an authored
//! candidate when present and otherwise synthesizing supported default
//! locations from `logical_path`. Profiles with no enabled sources, or entries
//! with no matching source, resolve to `Disabled`. The resolver performs no I/O;
//! callers apply [`crate::policy::MissingAssetPolicy`]. Hot reload requires both
//! the active profile and resolved location to support it.

use thiserror::Error;

use crate::id::AssetId;
use crate::kind::AssetKind;
use crate::location::AssetLocation;
use crate::manifest::AssetManifest;
use crate::policy::{CachePolicy, MissingAssetPolicy};
use crate::preload::PreloadGroup;
use crate::profile::{AssetProfile, AssetSourceProfile};

/// Result of resolving one `(id, profile)` pair.
///
/// `location.is_disabled()` is the "no source matched / NoAssets" case;
/// inspect [`Self::missing_policy`] to choose error / warn / silent
/// handling at the call site.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedAsset {
    pub id: AssetId,
    pub kind: AssetKind,
    pub profile: AssetProfile,
    pub location: AssetLocation,
    pub missing_policy: MissingAssetPolicy,
    pub cache_policy: CachePolicy,
    pub preload_group: Option<PreloadGroup>,
    /// Source kind that produced `location`. `None` when the profile
    /// has no enabled sources (resolved to `Disabled`).
    pub source_used: Option<AssetSourceProfile>,
    /// `true` when the location came from an authored
    /// [`crate::manifest::LocationCandidate`] for [`Self::source_used`];
    /// `false` when the resolver synthesized it from `logical_path`.
    ///
    /// Consumers use this to tell a speculative synthesized `embedded://` or
    /// bundle path (skip the load, use the fallback) from a promise that the
    /// bytes are packaged (call `AssetServer::load`).
    pub authored_candidate: bool,
}

impl ResolvedAsset {
    /// Whether this asset's source can fire file-change notifications
    /// under the active profile. Used by LDtk hot-reload to know
    /// whether to arm the file watcher.
    pub fn supports_hot_reload(&self) -> bool {
        self.profile.supports_hot_reload() && self.location.supports_hot_reload()
    }

    /// Bevy `AssetPath` string form. None for non-Bevy locations
    /// (LocalPath, IpfsGateway, Disabled).
    pub fn bevy_asset_path(&self) -> Option<String> {
        self.location.bevy_asset_path()
    }

    /// Whether the resolver returned `Disabled` for the active profile.
    pub fn is_disabled(&self) -> bool {
        self.location.is_disabled()
    }

    /// True when the asset is both `Error`-policy and resolved to
    /// `Disabled`. Required asset that the active profile cannot
    /// deliver — fatal unless the profile [`AssetProfile::tolerates_missing_required`].
    pub fn required_but_missing(&self) -> bool {
        self.missing_policy.is_required() && self.is_disabled()
    }
}

/// What can go wrong during resolution.
#[derive(Debug, Error, PartialEq)]
pub enum AssetResolutionError {
    /// The manifest has no entry for `id`. Likely a typo or a stale
    /// reference to a removed asset.
    #[error("unknown asset id: {0}")]
    UnknownId(AssetId),
}

/// Resolve one `(id, profile)` against `manifest`.
pub fn resolve(
    manifest: &AssetManifest,
    id: &AssetId,
    profile: AssetProfile,
) -> Result<ResolvedAsset, AssetResolutionError> {
    let entry = manifest
        .get(id)
        .ok_or_else(|| AssetResolutionError::UnknownId(id.clone()))?;

    let mut chosen_location = AssetLocation::Disabled;
    let mut chosen_source: Option<AssetSourceProfile> = None;
    let mut authored_candidate = false;

    if !profile.preferred_sources().is_empty() {
        // Two passes, so authored candidates always beat synthesized
        // defaults. Otherwise a synthesized `embedded://` URL could shadow an
        // `InstalledFilesystem` `BevyPath` later in the list, which breaks
        // `WebServedAssets`.
        //
        // Pass 1: the first authored candidate in the profile's preferred
        // source order.
        'outer: for &source in profile.preferred_sources() {
            if let Some(candidate) = entry.locations.iter().find(|c| c.source == source) {
                if !candidate.location.is_disabled() {
                    chosen_location = candidate.location.clone();
                    chosen_source = Some(source);
                    authored_candidate = true;
                    break 'outer;
                }
            }
        }
        // Pass 2: nothing authored matched, so synthesize a default in
        // preferred-source order. These have `authored_candidate = false`, so
        // the load gate can skip them (WebStatic skips unpackaged embedded
        // URLs).
        if chosen_source.is_none() {
            for &source in profile.preferred_sources() {
                if let Some(loc) = synthesize_default_location(source, &entry.logical_path) {
                    chosen_location = loc;
                    chosen_source = Some(source);
                    authored_candidate = false;
                    break;
                }
            }
        }
    }

    Ok(ResolvedAsset {
        id: entry.id.clone(),
        kind: entry.kind,
        profile,
        location: chosen_location,
        missing_policy: entry.missing_policy,
        cache_policy: entry.cache_policy,
        preload_group: entry.preload_group,
        source_used: chosen_source,
        authored_candidate,
    })
}

/// Resolve every entry in `manifest` under `profile`. Convenience for
/// preload group expansion and content-validation passes. Order matches
/// `manifest.iter()` (hash map order — not stable). Pair with
/// [`crate::manifest::AssetManifest::entries_in_group`] if you need a
/// reproducible order.
pub fn resolve_all(manifest: &AssetManifest, profile: AssetProfile) -> Vec<ResolvedAsset> {
    manifest
        .iter()
        .map(|(id, _)| {
            // unwrap is safe — id came from manifest.iter()
            resolve(manifest, id, profile).expect("id present in manifest must resolve")
        })
        .collect()
}

fn synthesize_default_location(
    source: AssetSourceProfile,
    logical_path: &str,
) -> Option<AssetLocation> {
    match source {
        AssetSourceProfile::LooseFilesystem
        | AssetSourceProfile::InstalledFilesystem
        | AssetSourceProfile::AndroidApk
        | AssetSourceProfile::IosBundle => {
            // Bevy's default `AssetSource` resolves this relative path against
            // its own root on each platform.
            Some(AssetLocation::BevyPath(logical_path.to_string()))
        }
        AssetSourceProfile::EmbeddedBinary => {
            Some(AssetLocation::embedded(logical_path.to_string()))
        }
        AssetSourceProfile::HttpRemote | AssetSourceProfile::IpfsGateway => {
            // HTTP and IPFS have no known base URL. Entries for these sources
            // must author an explicit `LocationCandidate`.
            None
        }
    }
}

#[cfg(test)]
mod tests;
