//! Resolve `(AssetId, AssetProfile) -> ResolvedAsset`.
//!
//! The resolver is the seam between Ambition's logical catalog
//! ([`crate::manifest`]) and the runtime asset backend. Bevy callers feed
//! resolved locations to `AssetServer`; non-Bevy consumers feed them to their
//! owning subsystem's byte/provider loader.
//!
//! ## Resolution algorithm
//!
//! For one `(id, profile)` pair:
//!
//! 1. Look up the [`crate::manifest::AssetEntry`] for `id`. Missing →
//!    [`AssetResolutionError::UnknownId`].
//! 2. If the profile has no preferred sources (`NoAssets`, `Headless`),
//!    return a `Disabled` location immediately.
//! 3. Otherwise walk the profile's
//!    [`crate::profile::AssetProfile::preferred_sources`] order; for
//!    each source kind:
//!    - If the entry has an explicit
//!      [`crate::manifest::LocationCandidate`] for that source, use its
//!      [`crate::location::AssetLocation`].
//!    - Otherwise, if the source is one of the filesystem / embedded
//!      kinds, synthesize a default location from the entry's
//!      `logical_path` (Bevy-relative for filesystem, `embedded://` for
//!      EmbeddedBinary, etc.).
//!    - Return the first non-`Disabled` location.
//! 4. If no source produced a location, return `Disabled`.
//!
//! The resolver does NOT read any bytes, does NOT inspect the
//! filesystem, and does NOT panic on missing required assets — the
//! caller consults [`crate::policy::MissingAssetPolicy`].
//!
//! ## Hot-reload
//!
//! [`ResolvedAsset::supports_hot_reload`] is `true` only when both the
//! active profile and the resolved location report hot-reload support.
//! Today that means: `DesktopDevLoose` profile + filesystem-backed
//! location.

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
    /// `true` when the resolved location came from an **authored**
    /// [`crate::manifest::LocationCandidate`] for [`Self::source_used`];
    /// `false` when the resolver *synthesized* a default location from
    /// the entry's `logical_path` because no candidate was authored
    /// for that source.
    ///
    /// This is the seam consumers use to decide whether a synthesized
    /// `embedded://` / bundle path is *speculative* (packaging hasn't
    /// happened yet → skip the load and rely on the fallback) versus
    /// an explicit promise that the bytes are actually packaged
    /// (proceed with `AssetServer::load`).
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

    /// Whether the asset is missing under the active profile (i.e. the
    /// resolver returned `Disabled`). Equivalent to
    /// `self.location.is_disabled()`; here as an explicit accessor for
    /// call sites that read like English.
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
        // Two-pass resolution so authored candidates always beat
        // synthesized defaults, regardless of source order. Otherwise
        // an entry with no authored EmbeddedBinary candidate would
        // synthesize an `embedded://...` URL from `logical_path` and
        // shadow a perfectly good InstalledFilesystem BevyPath later
        // in the source list — that breaks `WebServedAssets`, where
        // we want the synthesized BevyPath (HTTP-fetched) for
        // out-of-set art.
        //
        // Pass 1: pick the first authored candidate whose source is
        // in the profile's preferred order. Walk preferred_sources to
        // preserve the priority between authored candidates (e.g.
        // `[HttpRemote, EmbeddedBinary]` should prefer an authored
        // HttpRemote when both exist).
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
        // Pass 2: nothing authored matched — synthesize a default in
        // preferred-source order. Synthesized defaults are flagged
        // `authored_candidate = false` so the per-profile load gate
        // can choose to skip them (e.g. WebStatic skips speculative
        // embedded URLs whose bytes aren't actually packaged).
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
            // Bevy's default `AssetSource` resolves these against its
            // own root (host filesystem on desktop, app bundle on
            // Android/iOS). The catalog hands Bevy the relative path
            // and lets each platform's AssetReader do the work.
            Some(AssetLocation::BevyPath(logical_path.to_string()))
        }
        AssetSourceProfile::EmbeddedBinary => {
            Some(AssetLocation::embedded(logical_path.to_string()))
        }
        AssetSourceProfile::HttpRemote | AssetSourceProfile::IpfsGateway => {
            // Synthesizing an HTTP URL or an IPFS gateway URL without
            // an explicit candidate is ambiguous — neither has a known
            // base. Skip; entries that target these sources must
            // author an explicit `LocationCandidate`.
            None
        }
    }
}

#[cfg(test)]
mod tests;
