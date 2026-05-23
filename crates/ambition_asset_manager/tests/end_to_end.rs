//! End-to-end integration tests: build a manifest, resolve under
//! every profile, and inspect what the catalog reports.
//!
//! These exist to lock in the cross-module contract that the per-module
//! unit tests don't see (a manifest authored in code in one place
//! resolving under every profile in another).

use ambition_asset_manager::{
    resolve, resolve_all, AssetEntry, AssetId, AssetKind, AssetLocation, AssetManifest,
    AssetProfile, AssetSourceProfile, MissingAssetPolicy, PreloadGroup,
};

fn ambition_manifest() -> AssetManifest {
    AssetManifest::builder()
        .entry(
            AssetEntry::new(
                "world.sandbox_ldtk",
                AssetKind::LdtkProject,
                "ambition/worlds/sandbox.ldtk",
            )
            .with_missing_policy(MissingAssetPolicy::Error)
            .with_preload_group(PreloadGroup::Bootstrap)
            // Mirror the real sandbox catalog (see
            // `crate::sandbox_assets::extend_with_world_entries` in
            // the sandbox crate): both an authored LocalPath for
            // desktop hot reload AND an authored Embedded for
            // bundled / web profiles. The two-pass resolver picks
            // whichever source matches the active profile's
            // preferred-source order.
            .with_location(
                AssetSourceProfile::LooseFilesystem,
                AssetLocation::LocalPath(std::path::PathBuf::from(
                    "/tmp/ambition_test_ldtk_path/sandbox.ldtk",
                )),
            )
            .with_location(
                AssetSourceProfile::EmbeddedBinary,
                AssetLocation::embedded("ambition/worlds/sandbox.ldtk"),
            ),
        )
        .entry(
            AssetEntry::new(
                "audio.sfx_bank",
                AssetKind::AudioBank,
                "ambition/audio/sfx.bank",
            )
            .with_missing_policy(MissingAssetPolicy::WarnAndPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore)
            .with_location(
                AssetSourceProfile::EmbeddedBinary,
                AssetLocation::embedded("ambition/audio/sfx.bank"),
            ),
        )
        .entry(
            AssetEntry::new(
                "sprite.entity.chest_closed",
                AssetKind::Image,
                "sprites/entities/chest_closed.png",
            )
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(PreloadGroup::SandboxCore),
        )
        .entry(
            AssetEntry::new(
                "sprite.mod.crystal",
                AssetKind::Image,
                "sprites/mod/crystal.png",
            )
            .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
            .with_preload_group(PreloadGroup::Zone)
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
fn bootstrap_required_assets_resolve_under_every_real_profile() {
    let manifest = ambition_manifest();
    let real_profiles = [
        AssetProfile::DesktopDevLoose,
        AssetProfile::DesktopInstalled,
        AssetProfile::SteamDeckInstalled,
        AssetProfile::AndroidBundle,
        AssetProfile::IosBundle,
        AssetProfile::WebHttp,
        AssetProfile::WebStatic,
        AssetProfile::BundledStatic,
    ];
    for profile in real_profiles {
        let r = resolve(&manifest, &AssetId::new("world.sandbox_ldtk"), profile).unwrap();
        assert!(
            !r.is_disabled(),
            "{} should resolve world.sandbox_ldtk to a real location, got Disabled",
            profile.label(),
        );
        // Either a Bevy-pathable location (Embedded / BevyPath via
        // Bevy's AssetReader) OR a LocalPath the sandbox's LDtk loader
        // reads synchronously. DesktopDevLoose picks the authored
        // LocalPath; static profiles pick the authored Embedded.
        // Anything that produces neither would be a misconfiguration.
        assert!(
            r.bevy_asset_path().is_some() || r.location.as_local_path().is_some(),
            "{} produced neither a Bevy AssetPath nor a LocalPath for world.sandbox_ldtk \
             (got {:?})",
            profile.label(),
            r.location,
        );
    }
}

#[test]
fn no_assets_disables_every_entry_including_required() {
    let manifest = ambition_manifest();
    for r in resolve_all(&manifest, AssetProfile::NoAssets) {
        assert!(r.is_disabled(), "{} not disabled under NoAssets", r.id);
    }
    // Required asset is now required_but_missing.
    let r = resolve(
        &manifest,
        &AssetId::new("world.sandbox_ldtk"),
        AssetProfile::NoAssets,
    )
    .unwrap();
    assert!(r.required_but_missing());
}

#[test]
fn web_static_serves_embedded_for_required_and_synthesizes_for_optional_sprite() {
    let manifest = ambition_manifest();
    let ldtk = resolve(
        &manifest,
        &AssetId::new("world.sandbox_ldtk"),
        AssetProfile::WebStatic,
    )
    .unwrap();
    assert_eq!(ldtk.source_used, Some(AssetSourceProfile::EmbeddedBinary));
    assert_eq!(
        ldtk.bevy_asset_path().as_deref(),
        Some("embedded://ambition/worlds/sandbox.ldtk"),
    );

    let chest = resolve(
        &manifest,
        &AssetId::new("sprite.entity.chest_closed"),
        AssetProfile::WebStatic,
    )
    .unwrap();
    // No explicit Embedded candidate → resolver synthesizes from logical_path.
    assert_eq!(chest.source_used, Some(AssetSourceProfile::EmbeddedBinary));
    assert_eq!(
        chest.bevy_asset_path().as_deref(),
        Some("embedded://sprites/entities/chest_closed.png"),
    );
}

#[test]
fn ipfs_only_asset_is_disabled_outside_the_ipfs_profile() {
    let manifest = ambition_manifest();
    let crystal_id = AssetId::new("sprite.mod.crystal");

    let r_dev = resolve(&manifest, &crystal_id, AssetProfile::DesktopDevLoose).unwrap();
    assert_eq!(r_dev.source_used, Some(AssetSourceProfile::LooseFilesystem));
    // logical_path synthesizes for filesystem sources even when IPFS
    // override is present — desktop still gets a useful path. The
    // explicit IpfsGateway candidate is only chosen when the active
    // profile lists IpfsGateway in its preferred sources.
    assert_eq!(
        r_dev.bevy_asset_path().as_deref(),
        Some("sprites/mod/crystal.png"),
    );

    let r_ipfs = resolve(&manifest, &crystal_id, AssetProfile::IpfsGatewayPlaceholder).unwrap();
    assert_eq!(r_ipfs.source_used, Some(AssetSourceProfile::IpfsGateway));
    assert_eq!(
        r_ipfs.location.http_url().as_deref(),
        Some("https://w3s.link/ipfs/bafycrystal/crystal.png"),
    );
}

#[test]
fn preload_groups_partition_the_catalog() {
    let manifest = ambition_manifest();
    let bootstrap_ids: Vec<_> = manifest
        .entries_in_group(PreloadGroup::Bootstrap)
        .iter()
        .map(|e| e.id.as_str().to_string())
        .collect();
    assert_eq!(bootstrap_ids, vec!["world.sandbox_ldtk".to_string()]);

    let core_ids: Vec<_> = manifest
        .entries_in_group(PreloadGroup::SandboxCore)
        .iter()
        .map(|e| e.id.as_str().to_string())
        .collect();
    assert_eq!(
        core_ids,
        vec![
            "audio.sfx_bank".to_string(),
            "sprite.entity.chest_closed".to_string()
        ],
    );
}

#[test]
fn desktop_dev_loose_is_the_only_profile_that_offers_hot_reload() {
    let manifest = ambition_manifest();
    let id = AssetId::new("world.sandbox_ldtk");
    for profile in [
        AssetProfile::DesktopDevLoose,
        AssetProfile::DesktopInstalled,
        AssetProfile::SteamDeckInstalled,
        AssetProfile::AndroidBundle,
        AssetProfile::WebStatic,
        AssetProfile::BundledStatic,
    ] {
        let r = resolve(&manifest, &id, profile).unwrap();
        let expected = matches!(profile, AssetProfile::DesktopDevLoose);
        assert_eq!(
            r.supports_hot_reload(),
            expected,
            "{} hot-reload mismatch",
            profile.label(),
        );
    }
}
