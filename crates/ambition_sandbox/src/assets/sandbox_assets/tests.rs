//! Tests for the sandbox asset catalog builder + source plugin.
//!
//! Extracted from `sandbox_assets/mod.rs` to keep the implementation
//! file focused on catalog construction. The fixtures (`fixture_catalog`,
//! `SFX_BANK_ENV_LOCK`) live here too — they're only used by these
//! tests.

use super::*;
use crate::content::data::SandboxDataSpec;
use std::collections::HashSet;
use std::sync::Mutex;

/// Shared lock for tests that mutate `AMBITION_SFX_BANK_PATH`.
/// Env-var mutations are process-global; rust tests run in
/// parallel by default, so the lock keeps the
/// `sfx_bank_env_override_is_authored_local_path_candidate` setter
/// from racing the `sfx_bank_resolves_under_desktop_dev_loose`
/// reader.
static SFX_BANK_ENV_LOCK: Mutex<()> = Mutex::new(());

fn fixture_catalog() -> SandboxAssetCatalog {
    let config = GameAssetConfig::default();
    let spec = SandboxDataSpec::load_embedded();
    build_sandbox_catalog(&config, &spec.audio)
}

#[test]
fn every_well_known_id_resolves_to_an_entry() {
    let catalog = fixture_catalog();
    let inner = catalog.catalog();
    for id_str in [
        ids::SANDBOX_LDTK,
        ids::SANDBOX_DATA,
        ids::SFX_BANK,
        ids::FONT_DIALOG_REGULAR,
        ids::FONT_DIALOG_SEMIBOLD,
        ids::FONT_DEBUG_MONO,
    ] {
        assert!(
            inner.manifest().get(&AssetId::new(id_str)).is_some(),
            "manifest missing well-known id `{id_str}`",
        );
    }
}

#[test]
fn sandbox_ldtk_is_required_and_bootstrap() {
    let catalog = fixture_catalog();
    let entry = catalog
        .catalog()
        .manifest()
        .get(&ids::sandbox_ldtk())
        .unwrap();
    assert_eq!(entry.kind, AssetKind::LdtkProject);
    assert_eq!(entry.missing_policy, MissingAssetPolicy::Error);
    assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
}

#[test]
fn sandbox_data_is_required_and_bootstrap() {
    let catalog = fixture_catalog();
    let entry = catalog
        .catalog()
        .manifest()
        .get(&ids::sandbox_data())
        .unwrap();
    assert_eq!(entry.kind, AssetKind::RonData);
    assert_eq!(entry.missing_policy, MissingAssetPolicy::Error);
    assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
}

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

#[test]
fn music_track_ids_match_audio_spec() {
    let catalog = fixture_catalog();
    let spec = SandboxDataSpec::load_embedded();
    for track in &spec.audio.music_tracks {
        let id = ids::music_track(&track.id);
        if track.asset_path.is_some() {
            let entry = catalog
                .catalog()
                .manifest()
                .get(&id)
                .unwrap_or_else(|| panic!("missing music catalog entry for {id}"));
            assert_eq!(entry.kind, AssetKind::AudioClip);
        }
    }
}

#[test]
fn all_catalog_ids_are_unique() {
    let catalog = fixture_catalog();
    let mut seen = HashSet::new();
    for (id, _) in catalog.catalog().manifest().iter() {
        assert!(seen.insert(id.clone()), "duplicate id: {id}");
    }
}

// ─────────────────────────────────────────────────────────────────
// Guardrail tests — these fail loud when the catalog migration
// regresses (legacy `asset_exists` re-appears, embedded source
// breaks the WebStatic flip, etc.). Add to this section, don't
// delete.
// ─────────────────────────────────────────────────────────────────

/// No `fn asset_exists` / `fn desktop_asset_exists` *definitions*
/// live anywhere under `crates/ambition_sandbox/src/`. The only
/// host-filesystem probe is `desktop_candidate_roots` in this
/// file. Catching a regression here means someone re-added a
/// per-target existence walker; collapse it back through
/// [`SandboxAssetCatalog::resolve_local_file_path`].
///
/// Matches at line start (`^[ \t]*`) so the test's own doc-comment
/// mentioning the function names doesn't trip the guard.
#[test]
fn no_legacy_asset_exists_copies_in_sandbox_src() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let output = Command::new("grep")
        .args([
            "-rln",
            "-E",
            "^[[:space:]]*(pub(\\([^)]*\\))?[[:space:]]+)?fn[[:space:]]+(asset_exists|desktop_asset_exists)\\b",
        ])
        .arg(&src)
        .output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return, // `grep` missing → skip the guard rather than spuriously failing.
    };
    let offenders: Vec<&str> = stdout.lines().filter(|line| !line.is_empty()).collect();
    assert!(
        offenders.is_empty(),
        "legacy asset_exists / desktop_asset_exists copies re-appeared:\n  {}\n\
         Collapse the candidate-roots walk back through \
         SandboxAssetCatalog::resolve_local_file_path.",
        offenders.join("\n  "),
    );
}

/// No raw `env::var_os("BEVY_ASSET_ROOT"` / `env::var("BEVY_ASSET_ROOT"`
/// outside `sandbox_assets.rs`. The catalog owns the only probe.
/// Catches regressions where a new loader re-implements the
/// candidate-roots dance instead of calling
/// [`SandboxAssetCatalog::resolve_local_file_path`]. Doc-comments
/// mentioning the env var by name are allowed.
#[test]
fn no_unauthorized_bevy_asset_root_probes() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let output = Command::new("grep")
        .args([
            "-rln",
            "-E",
            "env::(var|var_os)\\([[:space:]]*\"BEVY_ASSET_ROOT\"",
        ])
        .arg(&src)
        .output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return,
    };
    let allowed = ["sandbox_assets/mod.rs", "sandbox_assets/tests.rs"];
    let offenders: Vec<String> = stdout
        .lines()
        .filter(|line| {
            !line.is_empty()
                && !allowed
                    .iter()
                    .any(|a| line.ends_with(a) || line.contains(&format!("/{a}:")))
        })
        .map(String::from)
        .collect();
    assert!(
        offenders.is_empty(),
        "unauthorized BEVY_ASSET_ROOT probe(s) re-appeared:\n  {}\n\
         Approved sites are `sandbox_assets/mod.rs` and \
         `sandbox_assets/tests.rs` only. Route new host-filesystem \
         reads through SandboxAssetCatalog::resolve_local_file_path.",
        offenders.join("\n  "),
    );
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

/// SFX bank byte resolution goes through the catalog. No ad-hoc
/// candidate walker should exist in `setup.rs`. The catalog's
/// `AMBITION_SFX_BANK_PATH` env override is an authored
/// `LooseFilesystem` `LocationCandidate` — visible policy, not a
/// side path.
#[test]
fn no_setup_resolve_to_disk_path_helper() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let setup = manifest_dir.join("src/setup.rs");
    let output = Command::new("grep")
        .args(["-n", "fn resolve_to_disk_path"])
        .arg(&setup)
        .output();
    if let Ok(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout).to_string();
        assert!(
            stdout.is_empty(),
            "setup.rs::resolve_to_disk_path re-appeared:\n  {stdout}\n\
             Route SFX bank disk reads through \
             SandboxAssetCatalog::resolve_local_file_path instead.",
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

/// `world.intro_ldtk` exists in the catalog and resolves under
/// `DesktopDevLoose` (LocalPath) and `WebStatic` (Embedded).
/// Catches a regression where the secondary-world list goes back
/// to a hard-coded loader-local constant.
/// Every URL in `embedded_core::ALL_URLS` is unique. Catches a
/// copy-paste typo where two const URLs collide and would silently
/// overwrite each other in `EmbeddedAssetRegistry`.
#[test]
fn embedded_core_urls_are_unique() {
    let mut seen = HashSet::new();
    for url in embedded_core::ALL_URLS {
        assert!(seen.insert(*url), "duplicate embedded core URL: {url}");
    }
}

/// Smoke: the `AmbitionAssetSourcePlugin` build completes without
/// panicking under `static_core_assets`. The macro-emitted
/// `register_embedded_core_assets` calls `EmbeddedAssetRegistry::
/// insert_asset(...)` once per row; if any `include_bytes!` path
/// is wrong, that's a compile-time error inside the macro
/// expansion (so this test wouldn't even build).
///
/// The macro keeps three pieces in sync structurally:
/// 1. URL constants in `embedded_core` (emitted from `$name`)
/// 2. `embedded_core::ALL_URLS` slice (emitted from `$name`)
/// 3. `register_embedded_core_assets` call (emitted from
///    `$name` + `$rel_path`)
/// They cannot diverge at the macro-call site. The catalog-side
/// pairing (the `with_location(EmbeddedBinary, ...)` on the
/// `AssetEntry`) is the only piece a contributor can forget; the
/// `embedded_core_urls_have_authored_catalog_candidates` test
/// below catches that.
#[cfg(feature = "static_core_assets")]
#[test]
fn ambition_asset_source_plugin_installs_under_static_core_assets() {
    use bevy::prelude::App;
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    // Should not panic. `register_embedded_core_assets` runs
    // once per `embedded_core::ALL_URLS` row.
    app.add_plugins(AmbitionAssetSourcePlugin::for_profile(
        AssetProfile::WebStatic,
    ));
}

/// Every embedded-core URL is reachable from at least one catalog
/// entry as an authored `EmbeddedBinary` candidate. Catches the
/// case where someone adds a row to `embed_core_assets!` without
/// adding the matching catalog candidate (would burn binary size
/// without enabling the load).
///
/// Only meaningful when the `static_core_assets` feature is on
/// (otherwise the candidates are intentionally absent).
#[cfg(feature = "static_core_assets")]
#[test]
fn embedded_core_urls_have_authored_catalog_candidates() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebStatic;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let mut authored_urls = HashSet::<String>::new();
    for (_, entry) in catalog.catalog().manifest().iter() {
        for candidate in &entry.locations {
            if candidate.source != AssetSourceProfile::EmbeddedBinary {
                continue;
            }
            if let AssetLocation::Embedded(url) = &candidate.location {
                authored_urls.insert(url.clone());
            }
        }
    }
    // The LDtk URLs are intentionally outside the embedded_core
    // set — they're embedded under `static_map` not
    // `static_core_assets`. Skip them here.
    let ldtk_urls = [
        EMBEDDED_SANDBOX_LDTK_ASSET_PATH.to_string(),
        EMBEDDED_INTRO_LDTK_ASSET_PATH.to_string(),
    ];
    let core_urls_authored: HashSet<String> = authored_urls
        .into_iter()
        .filter(|u| !ldtk_urls.contains(u))
        .collect();
    for url in embedded_core::ALL_URLS {
        assert!(
            core_urls_authored.contains(*url),
            "embedded_core URL `{url}` has no matching catalog candidate. \
             Add `with_embedded_core_candidate(entry, {url})` to the right \
             `extend_with_*` helper.",
        );
    }
}

/// Under `static_core_assets`, every WebStatic font + every primary
/// character spritesheet (`player` / `robot` / `goblin` / `sandbag`)
/// + every core entity sprite resolves to its embedded URL via
/// `try_path_for_load`. The plain `path_for` also resolves; the
/// gate confirms WebStatic actually attempts to load them.
#[cfg(feature = "static_core_assets")]
#[test]
fn web_static_loads_core_fonts_and_sprites_under_static_core_assets() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebStatic;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);

    // Fonts.
    for id in [
        ids::font_dialog_regular(),
        ids::font_dialog_semibold(),
        ids::font_debug_mono(),
    ] {
        let path = catalog
            .try_path_for_load(&id)
            .unwrap_or_else(|| panic!("WebStatic + static_core_assets must load font {id}"));
        assert!(
            path.starts_with("embedded://ambition_sandbox/fonts/"),
            "font {id} resolved to non-embedded path: {path}",
        );
    }

    // Primary character sheets.
    for label in ["player", "robot", "goblin", "sandbag"] {
        let id = ids::character_sprite(label);
        let path = catalog.try_path_for_load(&id).unwrap_or_else(|| {
            panic!("WebStatic + static_core_assets must load primary character sheet {label}")
        });
        assert!(path.starts_with("embedded://ambition_sandbox/sprites/"));
    }

    // Core entity sprites — pull from the EntitySprite enum so the
    // contract list is the same one `entity_sprite_embedded_core_url`
    // owns.
    for sprite in [
        crate::assets::game_assets::EntitySprite::ChestClosed,
        crate::assets::game_assets::EntitySprite::ChestOpen,
        crate::assets::game_assets::EntitySprite::PickupHealth,
        crate::assets::game_assets::EntitySprite::PickupCurrency,
        crate::assets::game_assets::EntitySprite::PickupAbility,
        crate::assets::game_assets::EntitySprite::DoorZone,
        crate::assets::game_assets::EntitySprite::EdgeExit,
        crate::assets::game_assets::EntitySprite::ProjectileEnergy,
        crate::assets::game_assets::EntitySprite::SolidTile,
        crate::assets::game_assets::EntitySprite::OneWayTile,
        crate::assets::game_assets::EntitySprite::HazardTile,
        crate::assets::game_assets::EntitySprite::BossCore,
    ] {
        let id = crate::assets::game_assets::entity_sprite_asset_id(sprite);
        let path = catalog.try_path_for_load(&id).unwrap_or_else(|| {
            panic!("WebStatic + static_core_assets must load core entity sprite {sprite:?}")
        });
        assert!(path.starts_with("embedded://ambition_sandbox/sprites/entities/"));
    }
}

/// Out-of-set sprites and parallax layers do NOT load on WebStatic
/// even with `static_core_assets` on — they have no authored
/// `EmbeddedBinary` candidate and `try_path_for_load` returns
/// `None`. This is the seam slice 17+ will eventually flip.
#[test]
fn web_static_skips_out_of_set_visuals_even_with_static_core_assets() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebStatic;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);

    // Breakable variants are not in the embedded core set.
    for sprite in [
        crate::assets::game_assets::EntitySprite::BreakableIntact,
        crate::assets::game_assets::EntitySprite::BreakableCracked,
        crate::assets::game_assets::EntitySprite::BreakableBroken,
    ] {
        let id = crate::assets::game_assets::entity_sprite_asset_id(sprite);
        assert!(
            catalog.try_path_for_load(&id).is_none(),
            "WebStatic should not attempt out-of-set sprite {sprite:?} (no Embedded candidate)",
        );
    }
    // Parallax layers are not in the embedded core set either.
    let parallax_id = crate::assets::game_assets::parallax_layer_asset_id(
        crate::assets::game_assets::ParallaxTheme::Hub,
        crate::assets::game_assets::ParallaxLayerAsset::Sky,
    );
    assert!(catalog.try_path_for_load(&parallax_id).is_none());
}

/// Intro NPC + prop catalog entries exist in the prebuilt
/// catalog. The intro plugin's load systems query them via
/// `try_path_for_load`; missing entries would silently fall
/// through to colored rectangles.
#[test]
fn intro_npc_and_prop_sprite_ids_resolve_through_the_catalog() {
    use crate::intro::sprites::{
        intro_npc_asset_id, intro_npc_sprite_rows, intro_prop_asset_id, intro_prop_sprite_rows,
    };

    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);

    for (name, filename, _spec) in intro_npc_sprite_rows() {
        let id = intro_npc_asset_id(name);
        let resolved = catalog.resolve(&id).unwrap_or_else(|err| {
            panic!("intro NPC `{name}` (id {id}) missing from catalog: {err}")
        });
        assert_eq!(resolved.kind, AssetKind::Image);
        // The logical path should end with the registered filename.
        assert!(
            resolved
                .bevy_asset_path()
                .map(|p| p.ends_with(filename))
                .unwrap_or(false),
            "intro NPC `{name}` resolved to path that doesn't end with {filename}",
        );
    }
    for (kind, filename, _spec) in intro_prop_sprite_rows() {
        let id = intro_prop_asset_id(kind);
        let resolved = catalog.resolve(&id).unwrap_or_else(|err| {
            panic!("intro prop `{kind}` (id {id}) missing from catalog: {err}")
        });
        assert!(resolved
            .bevy_asset_path()
            .map(|p| p.ends_with(filename))
            .unwrap_or(false));
    }
}

/// Guardrail: `SandboxAssetCatalog::should_attempt_optional_load(path: &str)`
/// has been removed. Catches a regression where someone adds the
/// gate back to satisfy a new dynamic-path consumer instead of
/// authoring a catalog id.
#[test]
fn no_should_attempt_optional_load_method_definition() {
    use std::process::Command;
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let output = Command::new("grep")
        .args([
            "-rln",
            "-E",
            "fn[[:space:]]+should_attempt_optional_load[[:space:]]*\\(",
        ])
        .arg(&src)
        .output();
    let stdout = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return,
    };
    let offenders: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        offenders.is_empty(),
        "should_attempt_optional_load(...) reappeared:\n  {}\n\
         Author a catalog id + `with_embedded_core_candidate` if needed; \
         loaders should use `try_path_for_load` only.",
        offenders.join("\n  "),
    );
}

#[test]
fn intro_ldtk_is_in_the_catalog_under_world_namespace() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let entry = catalog
        .catalog()
        .manifest()
        .get(&ids::intro_ldtk())
        .expect("world.intro_ldtk catalog entry missing");
    assert_eq!(entry.kind, AssetKind::LdtkProject);
    let r_desktop = catalog.resolve(&ids::intro_ldtk()).unwrap();
    assert!(r_desktop.location.as_local_path().is_some());

    config.asset_profile = AssetProfile::WebStatic;
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let path = catalog.try_path_for_load(&ids::intro_ldtk()).unwrap();
    assert_eq!(path, format!("embedded://{EMBEDDED_INTRO_LDTK_ASSET_PATH}"));
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
