//! Tests for `game_assets`: CLI/`GameAssetConfig` arg parsing, stable
//! entity-sprite / parallax `AssetId` namespaces, sandbox image-manifest
//! contents, per-`AssetProfile` catalog path resolution, and the
//! `entity_sprite_for_kind` resolver.

use super::*;
use crate::features::FeatureVisualKind;

fn args(slice: &[&str]) -> Vec<String> {
    slice.iter().map(|s| s.to_string()).collect()
}

#[test]
fn default_config_loads_assets_from_sprites_folder() {
    let c = GameAssetConfig::default();
    assert!(!c.no_assets);
    assert_eq!(c.sprite_folder, "sprites");
}

#[test]
fn no_assets_flag_forces_placeholder_mode() {
    let c = GameAssetConfig::from_arg_slice(&args(&["--no-assets"]));
    assert!(c.no_assets);
    assert_eq!(c.sprite_folder, "sprites", "folder unaffected");
    // --no-assets must also flip the catalog profile so the resolver
    // returns Disabled for every entry. Otherwise a future call site
    // that consults the catalog (without the no_assets early-return)
    // would still get a Bevy path.
    assert_eq!(c.asset_profile, AssetProfile::NoAssets);
}

#[test]
fn default_config_uses_target_cfg_asset_profile() {
    // The default profile is cfg-driven (Desktop on dev hosts).
    // Tests run on the host architecture; assert against the same
    // expression to lock in the wiring.
    let c = GameAssetConfig::default();
    assert_eq!(c.asset_profile, default_asset_profile());
}

#[test]
fn sprite_folder_flag_overrides_default() {
    let c = GameAssetConfig::from_arg_slice(&args(&["--sprite-folder", "experimental"]));
    assert!(!c.no_assets);
    assert_eq!(c.sprite_folder, "experimental");
}

#[test]
fn unknown_flags_are_left_alone() {
    // Bevy may consume args itself; the parser ignores anything unknown.
    let c = GameAssetConfig::from_arg_slice(&args(&["--bevy-flag", "--no-assets"]));
    assert!(c.no_assets);
}

#[test]
fn sprite_folder_flag_without_value_is_a_noop() {
    // Trailing flag with no folder argument: keep the default.
    let c = GameAssetConfig::from_arg_slice(&args(&["--sprite-folder"]));
    assert_eq!(c.sprite_folder, "sprites");
}

/// Every `EntitySprite::ALL` variant has a stable, unique `AssetId`
/// in the `sprite.entity.*` namespace. Catches accidental id
/// collisions and namespace drift.
#[test]
fn every_entity_sprite_has_a_unique_asset_id_in_sprite_entity_namespace() {
    let mut seen = std::collections::HashSet::new();
    for &sprite in EntitySprite::ALL {
        let id = entity_sprite_asset_id(sprite);
        assert!(
            id.as_str().starts_with("sprite.entity."),
            "{sprite:?} id `{id}` not in sprite.entity.* namespace",
        );
        assert!(
            seen.insert(id.clone()),
            "duplicate asset id for {sprite:?}: `{id}`"
        );
    }
    // Lock the chest_closed variant by name so refactors that
    // change the snake_case scheme don't go unnoticed.
    assert_eq!(
        entity_sprite_asset_id(EntitySprite::ChestClosed).as_str(),
        "sprite.entity.chest_closed",
    );
}

/// Every parallax `(theme, layer)` pair has a stable, unique id in
/// the `background.parallax.<theme>.<layer>` namespace.
#[test]
fn every_parallax_layer_has_a_unique_asset_id() {
    let mut seen = std::collections::HashSet::new();
    for &theme in ParallaxTheme::ALL {
        for &layer in ParallaxLayerAsset::ALL {
            let id = parallax_layer_asset_id(theme, layer);
            assert!(
                id.as_str().starts_with("background.parallax."),
                "{:?}/{:?} id `{id}` not in background.parallax.* namespace",
                theme,
                layer,
            );
            assert!(seen.insert(id.clone()), "duplicate parallax id: `{id}`");
        }
    }
}

/// The full sandbox image manifest has exactly one entry per
/// `EntitySprite::ALL` variant + one per `(theme, layer)`, no
/// duplicates, and every entry has the `Image` kind.
#[test]
fn sandbox_image_manifest_registers_every_entity_and_parallax_entry() {
    let manifest = sandbox_image_manifest("sprites");
    // Each base image also registers one entry per generated resolution variant
    // (see `insert_scaled_image_entry` + `TextureResolutionScale::MANIFEST_VARIANTS`),
    // so every base contributes 1 + N_variants entries.
    const ENTRIES_PER_BASE: usize =
        1 + crate::persistence::settings::TextureResolutionScale::MANIFEST_VARIANTS.len();
    let base_count =
        EntitySprite::ALL.len() + ParallaxTheme::ALL.len() * ParallaxLayerAsset::ALL.len();
    let expected = base_count * ENTRIES_PER_BASE;
    assert_eq!(
        manifest.len(),
        expected,
        "manifest len mismatch (entity={} parallax={}x{}={}, x{ENTRIES_PER_BASE} variants)",
        EntitySprite::ALL.len(),
        ParallaxTheme::ALL.len(),
        ParallaxLayerAsset::ALL.len(),
        ParallaxTheme::ALL.len() * ParallaxLayerAsset::ALL.len(),
    );
    for &sprite in EntitySprite::ALL {
        let id = entity_sprite_asset_id(sprite);
        let entry = manifest
            .get(&id)
            .unwrap_or_else(|| panic!("manifest missing {sprite:?}"));
        assert!(matches!(
            entry.kind,
            ambition_asset_manager::AssetKind::Image
        ));
    }
    for &theme in ParallaxTheme::ALL {
        for &layer in ParallaxLayerAsset::ALL {
            let id = parallax_layer_asset_id(theme, layer);
            let entry = manifest
                .get(&id)
                .unwrap_or_else(|| panic!("manifest missing {theme:?}/{layer:?}"));
            assert!(matches!(
                entry.kind,
                ambition_asset_manager::AssetKind::Image
            ));
        }
    }
}

/// The catalog under `DesktopDevLoose` produces the exact path
/// strings the prior raw-path loader would have built. Locks in
/// migration parity for every entity sprite + every parallax layer.
#[test]
fn catalog_paths_match_legacy_loader_paths_under_desktop_dev_loose() {
    let catalog = build_sandbox_image_catalog("sprites");
    let profile = AssetProfile::DesktopDevLoose;

    for &sprite in EntitySprite::ALL {
        let id = entity_sprite_asset_id(sprite);
        let path = catalog
            .path_for(&id, profile)
            .unwrap_or_else(|| panic!("desktop catalog missing path for {sprite:?}"));
        assert_eq!(
            path,
            format!("sprites/{}", sprite.relative_path()),
            "{sprite:?} path drift",
        );
    }
    for &theme in ParallaxTheme::ALL {
        for &layer in ParallaxLayerAsset::ALL {
            let id = parallax_layer_asset_id(theme, layer);
            let path = catalog
                .path_for(&id, profile)
                .unwrap_or_else(|| panic!("desktop catalog missing {theme:?}/{layer:?}"));
            assert_eq!(
                path,
                layer.relative_path(theme),
                "{theme:?}/{layer:?} path drift"
            );
        }
    }
}

/// `--sprite-folder custom_sprites` propagates through the catalog
/// so the resolved entity-sprite paths point at the override
/// directory. Parallax layers live under a fixed `backgrounds/`
/// path independent of `--sprite-folder` and must not move.
#[test]
fn sprite_folder_flag_propagates_through_catalog() {
    let catalog = build_sandbox_image_catalog("custom_sprites");
    let id = entity_sprite_asset_id(EntitySprite::ChestClosed);
    let path = catalog
        .path_for(&id, AssetProfile::DesktopDevLoose)
        .unwrap();
    assert_eq!(
        path,
        format!(
            "custom_sprites/{}",
            EntitySprite::ChestClosed.relative_path()
        ),
    );
    // Parallax layers don't follow --sprite-folder; verify the
    // override doesn't accidentally remap their path.
    let parallax_id = parallax_layer_asset_id(ParallaxTheme::Hub, ParallaxLayerAsset::Sky);
    assert_eq!(
        catalog
            .path_for(&parallax_id, AssetProfile::DesktopDevLoose)
            .unwrap(),
        ParallaxLayerAsset::Sky.relative_path(ParallaxTheme::Hub),
    );
}

/// `NoAssets` profile reports every entry as Disabled, so
/// `path_for` returns `None` and the loaders insert no handles —
/// the same outcome the `--no-assets` early-return produces.
#[test]
fn no_assets_profile_disables_every_image_in_the_manifest() {
    let catalog = build_sandbox_image_catalog("sprites");
    for &sprite in EntitySprite::ALL {
        let id = entity_sprite_asset_id(sprite);
        assert!(
            catalog.path_for(&id, AssetProfile::NoAssets).is_none(),
            "{sprite:?} not disabled under NoAssets",
        );
    }
    for &theme in ParallaxTheme::ALL {
        for &layer in ParallaxLayerAsset::ALL {
            let id = parallax_layer_asset_id(theme, layer);
            assert!(
                catalog.path_for(&id, AssetProfile::NoAssets).is_none(),
                "{theme:?}/{layer:?} not disabled under NoAssets",
            );
        }
    }
}

/// WebStatic resolves to `embedded://...` for core sprites that
/// have an authored Embedded candidate (chest, pickups, doors,
/// tiles, boss core, etc.). Out-of-set sprites (breakable
/// variants, lock-wall tile, soft/hard blink walls) resolve to a
/// synthesized `embedded://` URL but `try_path_for_load` returns
/// `None` because the candidate is not authored.
///
/// Locks in the slice-13 packaging contract:
/// `static_core_assets` packages ChestClosed but not BreakableIntact.
#[test]
fn web_static_loads_core_sprites_and_skips_out_of_set_sprites() {
    use crate::assets::sandbox_assets::SandboxAssetCatalog;

    let inner = build_sandbox_image_catalog("sprites");
    let catalog = SandboxAssetCatalog::new(inner, AssetProfile::WebStatic);

    let chest_id = entity_sprite_asset_id(EntitySprite::ChestClosed);
    // ChestClosed IS in the embedded core set IFF the build
    // enabled `static_core_assets`. Tests run with default
    // features, which include desktop_dev → no static_core_assets,
    // so the candidate is absent here.
    if cfg!(feature = "static_core_assets") {
        let path = catalog.try_path_for_load(&chest_id).unwrap();
        assert_eq!(
            path,
            format!(
                "embedded://{}",
                crate::assets::sandbox_assets::embedded_core::SPRITE_CHEST_CLOSED_URL
            ),
        );
    } else {
        assert!(
            catalog.try_path_for_load(&chest_id).is_none(),
            "without static_core_assets the WebStatic load gate must skip optional images",
        );
    }

    // Out-of-set sprite: no embedded candidate ever, regardless
    // of feature. Always skip on WebStatic.
    let breakable_id = entity_sprite_asset_id(EntitySprite::BreakableIntact);
    assert!(
        catalog.try_path_for_load(&breakable_id).is_none(),
        "WebStatic must skip optional images without an authored Embedded candidate",
    );
}

#[test]
fn entity_sprite_for_kind_handles_all_visual_kinds() {
    // Sanity: every FeatureVisualKind variant returns something
    // (either Some(sprite) or an explicit None for animated/dynamic
    // visuals). Ensures a new variant doesn't silently reach the
    // pattern-match catch-all and break the visual layer.
    for kind in [
        FeatureVisualKind::Hazard,
        FeatureVisualKind::Breakable,
        FeatureVisualKind::Chest,
        FeatureVisualKind::Pickup,
    ] {
        assert!(
            entity_sprite_for_kind(kind).is_some(),
            "static sprite expected for {kind:?}"
        );
    }
    // Actors resolve their sprite via the name-first + state-keyed upgrade path
    // (`upgrade_actor_sprites`), not a static per-kind sprite; switches render as
    // colored blocks.
    assert!(entity_sprite_for_kind(FeatureVisualKind::Actor).is_none());
    assert!(entity_sprite_for_kind(FeatureVisualKind::Switch).is_none());
}
