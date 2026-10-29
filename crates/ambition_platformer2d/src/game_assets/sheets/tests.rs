//! Tests for `game_assets`: CLI/`GameAssetConfig` arg parsing, stable
//! entity-sprite / parallax `AssetId` namespaces, sandbox image-manifest
//! contents, per-`AssetProfile` catalog path resolution, and the
//! `entity_sprite_for_kind` resolver.

use super::*;
use ambition_asset_manager::AssetProfile;
use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
use ambition_sprite_sheet::game_assets::ParallaxLayerAsset;

use ambition_sprite_sheet::game_assets::default_asset_profile;
use ambition_sprite_sheet::game_assets::entity_sprite_asset_id;
use ambition_sprite_sheet::game_assets::entity_sprite_for_kind;
use ambition_sprite_sheet::game_assets::parallax_layer_asset_id;


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
    let manifest = ambition_sprite_sheet::test_support::shared_libraries().asset_manifest().unwrap();
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



/// `NoAssets` profile reports every entry as Disabled, so
/// `path_for` returns `None` and the loaders insert no handles —
/// the same outcome the `--no-assets` early-return produces.
#[test]
fn no_assets_profile_disables_every_image_in_the_manifest() {
    let catalog = ambition_asset_manager::AmbitionAssetCatalog::new(ambition_sprite_sheet::test_support::shared_libraries().asset_manifest().unwrap());
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

/// A registered prop kind is in the first `GameAssets`, so no game waits for
/// `GameAssets` and inserts its own sheets. The control is the same build with
/// no rows: the kind is absent, so its presence is the registration's.
#[test]
fn registered_prop_sheets_load_with_the_first_game_assets() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    const KIND: &str = "sanic_ring_prop";
    assert!(
        ambition_sprite_sheet::character::sheets::available_sheet_keys(&sprite_content).contains(&KIND),
        "fixture: the baked sheets must include `{KIND}`"
    );
    let mut app = App::new();
    app.insert_resource(sprite_content.clone());
    app.add_plugins(bevy::app::TaskPoolPlugin::default());
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    app.init_asset::<TextureAtlasLayout>();
    app.register_prop_sheet(
        KIND,
        PropSheetSource {
            target: KIND.to_string(),
            tuning: ambition_sprite_sheet::character::SheetTuning::new(1.0, 2),
            packed: false,
        },
    );

    let config = GameAssetConfig::default();
    let characters = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
    let bosses = ambition_boss_encounter::BossCatalog::default();
    let catalog = ambition_platformer2d_assets::platformer_assets::build_platformer2d_asset_catalog(
        &ambition_sprite_sheet::test_support::shared_libraries(),
        &config,
        &characters,
        &bosses,
        &ambition_audio::spec::MusicRegistry {
            default_track: String::new(),
            tracks: Vec::new(),
        },
        &ambition_platformer2d_world::world_manifest::WorldManifest::default(),
    );
    let registered = app.world().resource::<PropSheets>().clone();
    let world = app.world_mut();
    let server = world.resource::<AssetServer>().clone();
    let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
    let mut build = |sheets: &PropSheets| {
        load_game_assets(&sprite_content,
            &config,
            &characters,
            &Default::default(),
            &bosses,
            &catalog,
            &server,
            &mut layouts,
            &RoomMetadata::default(),
            None,
            sheets,
        )
    };
    assert!(
        !build(&PropSheets::default()).characters.props.contains_key(KIND),
        "control: with no rows the kind must be absent"
    );
    assert!(
        build(&registered).characters.props.contains_key(KIND),
        "a registered prop kind is missing from the first `GameAssets`"
    );
}

#[test]
#[should_panic(expected = "has two sheets registered")]
fn a_prop_kind_has_one_sheet() {
    let source = || PropSheetSource {
        target: "a".to_string(),
        tuning: ambition_sprite_sheet::character::SheetTuning::new(1.0, 0),
        packed: false,
    };
    let mut sheets = PropSheets::default();
    sheets.register("a", source());
    sheets.register("a", source());
}

/// A prop row draws the record it names from that record's own page (the
/// creator lab's props are records of one sheet), and a `packed` row draws
/// from the shared sprite pack.
#[test]
fn a_prop_row_draws_its_records_page_or_the_pack() {
    let sprite_content = ambition_sprite_sheet::test_support::shared_sprites();
    let mut app = App::new();
    app.insert_resource(sprite_content.clone());
    app.add_plugins(bevy::app::TaskPoolPlugin::default());
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    app.init_asset::<TextureAtlasLayout>();
    let tuning = ambition_sprite_sheet::character::SheetTuning::new(1.0, 2);
    app.register_prop_sheet("lab_genesis_vat", PropSheetSource { target: "genesis_vat".to_string(), tuning, packed: false });
    app.register_prop_sheet("intro_cart", PropSheetSource { target: "intro_cart".to_string(), tuning, packed: true });
    app.register_prop_sheet("intro_cart_unpacked", PropSheetSource { target: "intro_cart".to_string(), tuning, packed: false });

    let config = GameAssetConfig::default();
    let characters = ambition_characters::actor::character_catalog::CharacterCatalog::empty();
    let bosses = ambition_boss_encounter::BossCatalog::default();
    let catalog = ambition_platformer2d_assets::platformer_assets::build_platformer2d_asset_catalog(
        &ambition_sprite_sheet::test_support::shared_libraries(),
        &config,
        &characters,
        &bosses,
        &ambition_audio::spec::MusicRegistry { default_track: String::new(), tracks: Vec::new() },
        &ambition_platformer2d_world::world_manifest::WorldManifest::default(),
    );
    let registered = app.world().resource::<PropSheets>().clone();
    let world = app.world_mut();
    let server = world.resource::<AssetServer>().clone();
    let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
    let assets = load_game_assets(&sprite_content,
        &config,
        &characters,
        &Default::default(),
        &bosses,
        &catalog,
        &server,
        &mut layouts,
        &RoomMetadata::default(),
        None,
        &registered,
    );
    let page = |kind: &str| {
        let asset = assets.characters.props.get(kind).unwrap_or_else(|| panic!("`{kind}` did not load"));
        server.get_path(asset.texture.id()).map(|path| path.to_string()).unwrap_or_default()
    };
    assert!(page("lab_genesis_vat").ends_with("/creator_lab_props_spritesheet.png"), "the vat is not drawn from its record's page: {}", page("lab_genesis_vat"));
    assert!(page("intro_cart_unpacked").ends_with("/intro_cart_spritesheet.png"), "control: an unpacked cart: {}", page("intro_cart_unpacked"));
    assert!(page("intro_cart").contains("sprite_packs/"), "the packed cart is not drawn from the pack: {}", page("intro_cart"));
}
