//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

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
    let r = resolve(&m, &AssetId::new("audio.sfx_bank"), AssetProfile::WebStatic).unwrap();
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
        r.location
            .as_local_path()
            .map(|p| p.to_string_lossy().to_string()),
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
    let r = resolve(&m, &AssetId::new("audio.sfx_bank"), AssetProfile::WebStatic).unwrap();
    assert!(
        r.authored_candidate,
        "explicit Embedded candidate should be authored"
    );

    // world.sandbox_ldtk has no candidate — DesktopDevLoose synthesizes
    // BevyPath from logical_path.
    let r = resolve(
        &m,
        &AssetId::new("world.sandbox_ldtk"),
        AssetProfile::DesktopDevLoose,
    )
    .unwrap();
    assert!(
        !r.authored_candidate,
        "synthesized default must not be authored"
    );
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
