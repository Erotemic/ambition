//! Per-`AssetProfile` resolution: SFX bank load gates, LDtk
//! hot-reload availability, env-var override, required-vs-optional
//! load gates, and the `WebServedAssets` BevyPath fallback.

use super::super::*;
use crate::content::data::SandboxDataSpec;

use super::SFX_BANK_ENV_LOCK;

#[test]
fn sfx_bank_resolves_under_desktop_dev_loose() {
    // Serialize with the env-var test below; otherwise the
    // setter can race the reader (env vars are process-global).
    let _guard = SFX_BANK_ENV_LOCK.lock().unwrap();
    let prev = std::env::var("AMBITION_SFX_BANK_PATH").ok();
    // SAFETY: see SFX_BANK_ENV_LOCK note.
    unsafe { std::env::remove_var("AMBITION_SFX_BANK_PATH") };

    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let result = catalog.path_for(&ids::sfx_bank());

    // Restore prior value before asserting.
    match prev {
        Some(v) => unsafe { std::env::set_var("AMBITION_SFX_BANK_PATH", v) },
        None => unsafe { std::env::remove_var("AMBITION_SFX_BANK_PATH") },
    }
    assert_eq!(result.as_deref(), Some("audio/sfx.bank"));
}

#[test]
fn ldtk_resolves_to_local_path_under_desktop_dev_loose() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let resolved = catalog.resolve(&ids::sandbox_ldtk()).unwrap();
    // Explicit LooseFilesystem candidate -> LocalPath that the
    // hot-reload watcher can poll.
    assert!(resolved.location.as_local_path().is_some());
    assert!(resolved.supports_hot_reload());
    assert!(catalog
        .hot_reload_local_path(&ids::sandbox_ldtk())
        .is_some());
}

#[test]
fn ldtk_falls_back_to_embedded_under_web_static() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebStatic;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let path = catalog.path_for(&ids::sandbox_ldtk()).unwrap();
    // Authored EmbeddedBinary candidate carries the explicit URL
    // that AmbitionAssetSourcePlugin registers under
    // `EmbeddedAssetRegistry`. The catalog + the source plugin
    // must agree.
    assert_eq!(
        path,
        format!("embedded://{EMBEDDED_SANDBOX_LDTK_ASSET_PATH}"),
    );
    // Web static does NOT support hot reload.
    assert!(!catalog
        .resolve(&ids::sandbox_ldtk())
        .unwrap()
        .supports_hot_reload());
    // The catalog's load gate should now flip to `Some(path)`
    // because the embedded candidate is authored — the source
    // plugin actually serves these bytes under `static_map`.
    let resolved = catalog.resolve(&ids::sandbox_ldtk()).unwrap();
    assert!(resolved.authored_candidate);
    assert!(catalog.try_path_for_load(&ids::sandbox_ldtk()).is_some());
}

#[test]
fn bundled_static_does_not_support_hot_reload() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::BundledStatic;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    assert!(catalog
        .hot_reload_local_path(&ids::sandbox_ldtk())
        .is_none());
}

#[test]
fn no_assets_disables_optional_image_and_font_entries() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::NoAssets;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    assert!(catalog.path_for(&ids::font_dialog_regular()).is_none());
    assert!(catalog.path_for(&ids::sfx_bank()).is_none());
}

/// Required bootstrap assets resolve under every real profile —
/// `LdtkProject::load_default` and the sandbox RON loader both
/// depend on this. Disabling here would be fatal.
#[test]
fn required_bootstrap_assets_resolve_under_every_real_profile() {
    let spec = SandboxDataSpec::load_embedded();
    let real_profiles = [
        AssetProfile::DesktopDevLoose,
        AssetProfile::DesktopInstalled,
        AssetProfile::SteamDeckInstalled,
        AssetProfile::AndroidBundle,
        AssetProfile::IosBundle,
        AssetProfile::WebHttp,
        AssetProfile::WebStatic,
        AssetProfile::WebServedAssets,
        AssetProfile::BundledStatic,
    ];
    for profile in real_profiles {
        let mut config = GameAssetConfig::default();
        config.asset_profile = profile;
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        for id in [ids::sandbox_ldtk(), ids::sandbox_data()] {
            let resolved = catalog.resolve(&id).unwrap();
            assert!(
                !resolved.is_disabled(),
                "{} should not disable required {id} (got {:?})",
                profile.label(),
                resolved.location,
            );
        }
    }
}

/// WebStatic / BundledStatic should actually attempt to load the
/// LDtk world (the source plugin registers the bytes under
/// `static_map`). Catches a regression where the authored Embedded
/// candidate gets stripped or the load gate stops honoring it.
#[test]
fn web_static_attempts_to_load_embedded_sandbox_ldtk() {
    let spec = SandboxDataSpec::load_embedded();
    for profile in [AssetProfile::WebStatic, AssetProfile::BundledStatic] {
        let mut config = GameAssetConfig::default();
        config.asset_profile = profile;
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        assert!(
            catalog.try_path_for_load(&ids::sandbox_ldtk()).is_some(),
            "{} should attempt LDtk load via authored Embedded candidate",
            profile.label(),
        );
    }
}

/// LDtk hot reload is available only under `DesktopDevLoose`. The
/// hot-reload watcher consults this via
/// `SandboxAssetCatalog::hot_reload_local_path`; any new profile
/// that pretends to support filesystem watching would silently
/// break here.
#[test]
fn ldtk_hot_reload_only_under_desktop_dev_loose() {
    let spec = SandboxDataSpec::load_embedded();
    let profiles = [
        (AssetProfile::DesktopDevLoose, true),
        (AssetProfile::DesktopInstalled, false),
        (AssetProfile::SteamDeckInstalled, false),
        (AssetProfile::AndroidBundle, false),
        (AssetProfile::IosBundle, false),
        (AssetProfile::WebHttp, false),
        (AssetProfile::WebStatic, false),
        (AssetProfile::WebServedAssets, false),
        (AssetProfile::BundledStatic, false),
        (AssetProfile::NoAssets, false),
        (AssetProfile::Headless, false),
        (AssetProfile::IpfsGatewayPlaceholder, false),
    ];
    for (profile, expected) in profiles {
        let mut config = GameAssetConfig::default();
        config.asset_profile = profile;
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        let supports = catalog
            .hot_reload_local_path(&ids::sandbox_ldtk())
            .is_some();
        assert_eq!(
            supports,
            expected,
            "{}: expected hot-reload support = {} but got {}",
            profile.label(),
            expected,
            supports,
        );
    }
}

/// `AMBITION_SFX_BANK_PATH` shows up as an authored
/// `LooseFilesystem` candidate on the SFX bank entry when set.
/// Guards against accidentally re-introducing the env-var probe
/// somewhere outside the catalog.
#[test]
fn sfx_bank_env_override_is_authored_local_path_candidate() {
    let _guard = SFX_BANK_ENV_LOCK.lock().unwrap();
    // SAFETY: tests touching env vars are intrinsically global;
    // use a recognizable temp path and clean up after.
    let probe = "/tmp/__ambition_sfx_bank_override_probe__.bank";
    let prev = std::env::var("AMBITION_SFX_BANK_PATH").ok();
    // SAFETY: `set_var` is unsafe in Rust 2024 because it mutates
    // process-global state; this test acknowledges that risk and
    // restores the prior value below.
    unsafe { std::env::set_var("AMBITION_SFX_BANK_PATH", probe) };

    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let entry = catalog.catalog().manifest().get(&ids::sfx_bank()).unwrap();
    let has_override = entry.locations.iter().any(|c| {
        matches!(
            &c.location,
            AssetLocation::LocalPath(p) if p == std::path::Path::new(probe)
        )
    });

    match prev {
        // SAFETY: see above.
        Some(value) => unsafe { std::env::set_var("AMBITION_SFX_BANK_PATH", value) },
        // SAFETY: see above.
        None => unsafe { std::env::remove_var("AMBITION_SFX_BANK_PATH") },
    }

    assert!(
        has_override,
        "AMBITION_SFX_BANK_PATH must be reflected as a LooseFilesystem LocationCandidate on the SFX bank entry",
    );
}

#[test]
fn should_attempt_required_load_only_disabled_for_no_assets_profiles() {
    for (profile, expected) in [
        (AssetProfile::DesktopDevLoose, true),
        (AssetProfile::DesktopInstalled, true),
        (AssetProfile::AndroidBundle, true),
        (AssetProfile::WebStatic, true),
        (AssetProfile::WebServedAssets, true),
        (AssetProfile::BundledStatic, true),
        (AssetProfile::IpfsGatewayPlaceholder, true),
        (AssetProfile::NoAssets, false),
        (AssetProfile::Headless, false),
    ] {
        let mut config = GameAssetConfig::default();
        config.asset_profile = profile;
        let spec = SandboxDataSpec::load_embedded();
        let catalog = build_sandbox_catalog(&config, &spec.audio);
        assert_eq!(
            catalog.should_attempt_required_load("foo.png"),
            expected,
            "{}",
            profile.label(),
        );
    }
}

/// `WebServedAssets` is the "same game in the browser" profile.
/// Catalog entries with no authored Embedded candidate must
/// still produce a Bevy `AssetPath` (so the wasm HTTP reader
/// fetches `/assets/<path>`), and `try_path_for_load` must
/// return `Some(...)` for those entries — the gate is permissive
/// because we cannot pre-check the host filesystem from the
/// browser. Locks in the contract that lets optional sprites /
/// fonts / music load over HTTP without per-asset packaging.
#[test]
fn web_served_assets_attempts_optional_sprites_via_bevy_path() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebServedAssets;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);

    // An out-of-set entity sprite (no Embedded candidate). Under
    // `WebStatic` this returns None; under `WebServedAssets` it
    // should produce a synthesized BevyPath the wasm HTTP reader
    // fetches from `/assets/sprites/entities/breakable_intact.png`.
    let id = crate::assets::game_assets::entity_sprite_asset_id(
        crate::assets::game_assets::EntitySprite::BreakableIntact,
    );
    let path = catalog
        .try_path_for_load(&id)
        .expect("WebServedAssets must attempt out-of-set sprites via synthesized BevyPath");
    assert!(
        !path.starts_with("embedded://"),
        "WebServedAssets should NOT route out-of-set sprites through embedded:// (path = {path})",
    );
    assert!(
        path.contains("breakable_intact.png"),
        "synthesized path missing filename: {path}",
    );

    // Parallax layers also resolve to BevyPath under WebServedAssets.
    let parallax_id = crate::assets::game_assets::parallax_layer_asset_id(
        crate::assets::game_assets::ParallaxTheme::Hub,
        crate::assets::game_assets::ParallaxLayerAsset::Sky,
    );
    let parallax_path = catalog
        .try_path_for_load(&parallax_id)
        .expect("WebServedAssets must attempt parallax layers via synthesized BevyPath");
    assert!(parallax_path.contains("backgrounds/parallax_layers/hub_sky.png"));

    // Authored Embedded candidates still take priority on WebServedAssets
    // (sandbox.ldtk is required + embedded).
    let ldtk_path = catalog.try_path_for_load(&ids::sandbox_ldtk()).unwrap();
    assert!(ldtk_path.starts_with("embedded://"));
}

/// `WebServedAssets` resolves every music track that has an
/// `asset_path` in `sandbox.ron` to a `music.track.<id>` BevyPath
/// — Bevy's wasm HTTP reader can fetch the OGGs from the served
/// `/assets/` tree. Catches a regression where music tracks stop
/// being added to the catalog or the WebServedAssets gate
/// excludes them.
#[test]
fn web_served_assets_resolves_music_track_paths_via_bevy_path() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebServedAssets;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);

    let mut attempted = 0;
    for track in &spec.audio.music_tracks {
        if track.asset_path.is_none() {
            continue;
        }
        let id = ids::music_track(&track.id);
        let path = catalog.try_path_for_load(&id).unwrap_or_else(|| {
            panic!(
                "music track `{}` (id {id}) missing under WebServedAssets",
                track.id
            )
        });
        assert!(!path.starts_with("embedded://"));
        attempted += 1;
    }
    assert!(
        attempted > 0,
        "expected at least one music track with asset_path"
    );
}
