//! Intro sprite catalog identity: the content extension's NPC/prop
//! entries resolve through the prebuilt sandbox catalog.

use ambition_asset_manager::{AssetKind, AssetProfile};
use ambition_gameplay_core::assets::game_assets::GameAssetConfig;
use ambition_gameplay_core::session::data::load_embedded_music_registry;

#[test]
fn intro_npc_and_prop_sprite_ids_resolve_through_the_catalog() {
    use ambition_content::intro::sprites::{
        intro_npc_asset_id, intro_npc_sprite_rows, intro_prop_asset_id, intro_prop_sprite_rows,
    };

    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let music = load_embedded_music_registry();
    // The intro entries are a CONTENT extension (the app assembly wires
    // them through `build_sandbox_catalog_with`); mirror that wiring here.
    let catalog = ambition_gameplay_core::assets::sandbox_assets::build_sandbox_catalog_with(
        &config,
        &music,
        |manifest| {
            ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                manifest,
                &config.sprite_folder,
            );
        },
    );

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
