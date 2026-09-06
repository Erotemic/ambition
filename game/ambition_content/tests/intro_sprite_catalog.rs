//! Intro sprite catalog identity: the content extension's PROP entries
//! resolve through the prebuilt sandbox catalog.
//!
//! ⚠ The NPC half is gone with the preload table it checked — see the note at
//! the loop below.

use ambition_asset_manager::AssetProfile;
use ambition_content::audio_registries::load_music_registry;
use ambition_sprite_sheet::game_assets::GameAssetConfig;

#[test]
fn intro_npc_and_prop_sprite_ids_resolve_through_the_catalog() {
    use ambition_content::intro::sprites::{
        intro_prop_asset_id, intro_prop_sprite_rows,
    };

    // Catalog building resolves character sprite rows through the explicit
    // App-local catalog supplied by the composition root.
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let music = load_music_registry();
    let character_catalog =
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(
                ambition_content::character_catalog::CHARACTER_CATALOG_RON,
            ),
        );
    let boss_catalog = ambition_content::bosses::authored_boss_catalog();
    // The intro entries are a CONTENT extension (the app assembly wires
    // them through `build_sandbox_catalog_with`); mirror that wiring here.
    let catalog = ambition_platformer2d_actor_monolith::assets::platformer_assets::build_sandbox_catalog_with(
        &config,
        &character_catalog,
        &boss_catalog,
        &music,
        &ambition_content::worlds::world_manifest(),
        |manifest| {
            ambition_content::intro::sprites::extend_with_intro_sprite_entries(
                manifest,
                &config.sprite_folder,
            );
        },
    );

    // ⚠ THE NPC HALF OF THIS TEST IS GONE WITH THE TABLE IT CHECKED. It asserted
    // that every intro NPC sprite row resolved in the catalog under
    // `sprite.character.intro_<name>` — ids nothing ever looked up, because the
    // world keys its `NpcSpawn`s by `character_id` and never by the display name
    // those rows published under. The property that replaced it is
    // `every_intro_npc_spawn_names_a_character_the_catalog_knows` in the lib.
    // Props keep their half below: a `Prop` IS keyed by the `kind` its row uses.
    for (kind, filename, _spec, _pack) in intro_prop_sprite_rows() {
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
