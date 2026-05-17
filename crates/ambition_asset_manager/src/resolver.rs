//! Resolve `(AssetId, AssetProfile) -> ResolvedAsset`.
//!
//! The resolver is the seam between Ambition's logical catalog
//! ([`crate::manifest`]) and the runtime asset backend
//! (Bevy `AssetServer` for Bevy-native kinds, [`crate::sfx_integration`]
//! and similar tiny adapters for non-Bevy bytes).
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
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::manifest::{AssetEntry, AssetManifest, LocationCandidate};

    fn fixture_with_overrides() -> AssetManifest {
        AssetManifest::builder()
            .entry(
                AssetEntry::new(
                    "world.sandbox_ldtk",
                    AssetKind::LdtkProject,
                    "ambition/worlds/sandbox.ldtk",
                )
                .with_missing_policy(MissingAssetPolicy::Error),
            )
            .entry(
                // Embedded-only asset — only the WebStatic / BundledStatic
                // / DesktopDevLoose fallback path can reach it; HTTP +
                // filesystem-only profiles would never synthesize a
                // location for this id.
                AssetEntry::new(
                    "audio.sfx_bank",
                    AssetKind::AudioBank,
                    "ambition/audio/sfx.bank",
                )
                .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
                .with_location(
                    AssetSourceProfile::EmbeddedBinary,
                    AssetLocation::embedded("ambition/audio/sfx.bank"),
                )
                .with_location(
                    AssetSourceProfile::LooseFilesystem,
                    AssetLocation::LocalPath(PathBuf::from("/repo/assets/ambition/audio/sfx.bank")),
                ),
            )
            .entry(
                // HTTP-only asset (CDN-hosted boss-cinematic still).
                // The profile must declare HttpRemote support for this
                // to resolve to a URL.
                AssetEntry::new(
                    "sprite.cutscene.boss_intro",
                    AssetKind::Image,
                    "sprites/cutscene/boss_intro.png",
                )
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_location(
                    AssetSourceProfile::HttpRemote,
                    AssetLocation::HttpUrl(
                        "https://cdn.example.com/sprites/cutscene/boss_intro.png".into(),
                    ),
                ),
            )
            .entry(
                // IPFS-only asset (mod CID). Resolves only under the
                // placeholder profile.
                AssetEntry::new(
                    "sprite.mod.crystal",
                    AssetKind::Image,
                    "sprites/mod/crystal.png",
                )
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_location(
                    AssetSourceProfile::IpfsGateway,
                    AssetLocation::IpfsGateway {
                        gateway: "https://w3s.link".into(),
                        cid: "bafycrystal".into(),
                        path: "crystal.png".into(),
                    },
                ),
            )
            .build()
    }

    #[test]
    fn unknown_id_returns_error() {
        let m = AssetManifest::new();
        let err = resolve(&m, &AssetId::new("nope"), AssetProfile::DesktopDevLoose).unwrap_err();
        assert_eq!(err, AssetResolutionError::UnknownId(AssetId::new("nope")));
    }

    #[test]
    fn desktop_dev_loose_synthesizes_bevy_path_for_filesystem_assets() {
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("world.sandbox_ldtk"),
            AssetProfile::DesktopDevLoose,
        )
        .unwrap();
        assert_eq!(
            r.bevy_asset_path().as_deref(),
            Some("ambition/worlds/sandbox.ldtk"),
        );
        assert_eq!(r.source_used, Some(AssetSourceProfile::LooseFilesystem));
        assert!(r.supports_hot_reload());
        assert!(r.missing_policy.is_required());
        assert!(!r.required_but_missing());
    }

    #[test]
    fn web_static_prefers_embedded_override() {
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("audio.sfx_bank"),
            AssetProfile::WebStatic,
        )
        .unwrap();
        assert_eq!(
            r.bevy_asset_path().as_deref(),
            Some("embedded://ambition/audio/sfx.bank"),
        );
        assert_eq!(r.source_used, Some(AssetSourceProfile::EmbeddedBinary));
        // Embedded sources don't support hot reload.
        assert!(!r.supports_hot_reload());
    }

    #[test]
    fn desktop_dev_loose_picks_loose_override_over_embedded() {
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("audio.sfx_bank"),
            AssetProfile::DesktopDevLoose,
        )
        .unwrap();
        // LooseFilesystem is preferred and has an explicit LocalPath
        // override, so the resolver picks that.
        assert_eq!(r.source_used, Some(AssetSourceProfile::LooseFilesystem));
        assert_eq!(
            r.location.as_local_path().map(|p| p.to_string_lossy().to_string()),
            Some("/repo/assets/ambition/audio/sfx.bank".to_string()),
        );
    }

    #[test]
    fn web_http_resolves_explicit_url_for_http_only_asset() {
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("sprite.cutscene.boss_intro"),
            AssetProfile::WebHttp,
        )
        .unwrap();
        assert_eq!(
            r.bevy_asset_path().as_deref(),
            Some("https://cdn.example.com/sprites/cutscene/boss_intro.png"),
        );
        assert_eq!(r.source_used, Some(AssetSourceProfile::HttpRemote));
    }

    #[test]
    fn no_assets_profile_disables_everything() {
        let m = fixture_with_overrides();
        for id in [
            "world.sandbox_ldtk",
            "audio.sfx_bank",
            "sprite.cutscene.boss_intro",
        ] {
            let r = resolve(&m, &AssetId::new(id), AssetProfile::NoAssets).unwrap();
            assert!(r.is_disabled(), "{id} should be Disabled under NoAssets");
            assert!(r.source_used.is_none());
        }
    }

    #[test]
    fn no_assets_marks_required_assets_as_required_but_missing() {
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("world.sandbox_ldtk"),
            AssetProfile::NoAssets,
        )
        .unwrap();
        assert!(r.required_but_missing());
    }

    #[test]
    fn ipfs_profile_renders_gateway_url_for_ipfs_only_asset() {
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("sprite.mod.crystal"),
            AssetProfile::IpfsGatewayPlaceholder,
        )
        .unwrap();
        assert_eq!(
            r.location.http_url().as_deref(),
            Some("https://w3s.link/ipfs/bafycrystal/crystal.png"),
        );
        assert_eq!(r.source_used, Some(AssetSourceProfile::IpfsGateway));
        // Not bevy-pathable without a registered IPFS source.
        assert!(r.bevy_asset_path().is_none());
    }

    #[test]
    fn web_http_falls_back_to_embedded_when_http_unavailable() {
        // An asset that has only an embedded override should still
        // resolve under WebHttp because the profile's preferred sources
        // include EmbeddedBinary as the fallback.
        let m = fixture_with_overrides();
        let r = resolve(&m, &AssetId::new("audio.sfx_bank"), AssetProfile::WebHttp).unwrap();
        assert_eq!(r.source_used, Some(AssetSourceProfile::EmbeddedBinary));
    }

    #[test]
    fn authored_candidate_is_true_only_when_resolver_picks_an_explicit_candidate() {
        let m = fixture_with_overrides();
        // sfx_bank has an explicit Embedded candidate; WebStatic picks it.
        let r = resolve(
            &m,
            &AssetId::new("audio.sfx_bank"),
            AssetProfile::WebStatic,
        )
        .unwrap();
        assert!(r.authored_candidate, "explicit Embedded candidate should be authored");

        // world.sandbox_ldtk has no candidate — DesktopDevLoose synthesizes
        // BevyPath from logical_path.
        let r = resolve(
            &m,
            &AssetId::new("world.sandbox_ldtk"),
            AssetProfile::DesktopDevLoose,
        )
        .unwrap();
        assert!(!r.authored_candidate, "synthesized default must not be authored");
    }

    #[test]
    fn android_bundle_synthesizes_from_logical_path_when_no_override() {
        // No explicit AndroidApk candidate — the resolver should
        // synthesize a Bevy-relative path from logical_path.
        let m = fixture_with_overrides();
        let r = resolve(
            &m,
            &AssetId::new("sprite.cutscene.boss_intro"),
            AssetProfile::AndroidBundle,
        )
        .unwrap();
        assert_eq!(
            r.bevy_asset_path().as_deref(),
            Some("sprites/cutscene/boss_intro.png"),
        );
        assert_eq!(r.source_used, Some(AssetSourceProfile::AndroidApk));
    }

    #[test]
    fn resolve_all_returns_one_record_per_entry() {
        let m = fixture_with_overrides();
        let resolved = resolve_all(&m, AssetProfile::DesktopDevLoose);
        assert_eq!(resolved.len(), m.len());
    }

    #[test]
    fn explicit_candidate_construction_via_struct() {
        // LocationCandidate is publicly constructible for users who
        // want to author the manifest by hand (round-trip from RON/JSON).
        let candidate = LocationCandidate {
            source: AssetSourceProfile::EmbeddedBinary,
            location: AssetLocation::embedded("ambition/audio/sfx.bank"),
        };
        assert_eq!(candidate.source, AssetSourceProfile::EmbeddedBinary);
    }
}
