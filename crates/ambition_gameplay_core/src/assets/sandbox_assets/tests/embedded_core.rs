//! `embedded_core::*` URL constants + `AmbitionAssetSourcePlugin`
//! registration tests. Verifies the `embed_core_assets!` macro keeps
//! URL constants unique, the source plugin installs without panic,
//! and every URL has both an authored catalog candidate and (for the
//! WebStatic profile) loads through the embedded source under the
//! `static_core_assets` feature.

use super::super::*;
use crate::session::data::authored_music_registry;
use std::collections::HashSet;

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
    let music = authored_music_registry().clone();
    let catalog = build_sandbox_catalog(&config, &music);
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
    let ldtk_urls: Vec<String> = crate::ldtk_world::world_manifest()
        .worlds
        .iter()
        .filter_map(|world| world.embedded_bevy_path.map(str::to_string))
        .collect();
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
    let music = authored_music_registry().clone();
    let catalog = build_sandbox_catalog(&config, &music);

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
            path.starts_with("embedded://ambition_gameplay_core/fonts/"),
            "font {id} resolved to non-embedded path: {path}",
        );
    }

    // Primary character sheets.
    for label in ["player", "robot", "goblin", "sandbag"] {
        let id = ids::character_sprite(label);
        let path = catalog.try_path_for_load(&id).unwrap_or_else(|| {
            panic!("WebStatic + static_core_assets must load primary character sheet {label}")
        });
        assert!(path.starts_with("embedded://ambition_gameplay_core/sprites/"));
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
        assert!(path.starts_with("embedded://ambition_gameplay_core/sprites/entities/"));
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
    let music = authored_music_registry().clone();
    let catalog = build_sandbox_catalog(&config, &music);

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
