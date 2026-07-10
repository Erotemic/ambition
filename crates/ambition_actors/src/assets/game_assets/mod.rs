//! Compatibility facade for game-asset resources and loaders.
//!
//! Canonical render-facing asset vocabulary now lives in
//! `ambition_sprite_sheet::game_assets` so `ambition_render` can consume
//! `GameAssets` without depending on `ambition_actors`. The full loader remains
//! here because it joins the content-installed character roster before building
//! the shared resource.

pub use ambition_sprite_sheet::game_assets::*;

use bevy::prelude::*;
use std::collections::HashMap;

use crate::boss_encounter::sprites;
use crate::character_sprites;
use crate::rooms::RoomMetadata;
use ambition_persistence::settings::VisualQualityBudget;

/// Build a fresh `GameAssets`, honoring `config` + the shared catalog resource.
pub fn load_game_assets(
    config: &GameAssetConfig,
    catalog: &crate::assets::sandbox_assets::SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    active_room_metadata: &RoomMetadata,
    quality: Option<&VisualQualityBudget>,
) -> GameAssets {
    if config.no_assets {
        eprintln!("[game_assets] --no-assets in effect: rendering with colored-rectangle placeholders only");
        return GameAssets::default();
    }

    let characters =
        character_sprites::load_character_sprites_in(catalog, asset_server, layouts, quality);
    let entities = load_entity_sprites(catalog, asset_server, quality);
    let boss = sprites::load_boss_sprite_in(catalog, asset_server, layouts, quality);
    let mut boss_sprites: HashMap<&'static str, sprites::BossSpriteAsset> = HashMap::new();
    let mut boss_sheets_missed: Vec<&'static str> = Vec::new();
    for (key, spec) in sprites::dedicated_boss_sheets() {
        match sprites::load_named_boss_sprite_via_catalog(
            catalog,
            asset_server,
            layouts,
            key,
            spec,
            quality,
        ) {
            Some(sheet) => {
                boss_sprites.insert(key, sheet);
            }
            None => boss_sheets_missed.push(key),
        }
    }
    // The diagnostic tracks.md's boss-sprite bug asked for, made permanent. A boss
    // renders the GENERIC gradient-sentinel body exactly when its `boss_key` (its
    // lowercased behavior id) is absent from this map — `upgrade_boss_sprites`
    // warns once per such boss. Printing the map's contents here says whether the
    // key was never LOADED (an asset/catalog problem, listed below) or never
    // LOOKED UP under that name (a key-agreement problem, and the disproven
    // `sprite_target` dispatch is not the fix — the render keys on `behavior.id`).
    {
        let mut keys: Vec<&str> = boss_sprites.keys().copied().collect();
        keys.sort_unstable();
        eprintln!(
            "[boss_sprites] {} dedicated sheet(s) loaded: {}",
            boss_sprites.len(),
            keys.join(", ")
        );
        if !boss_sheets_missed.is_empty() {
            eprintln!(
                "[boss_sprites] {} FAILED to load (these bosses draw the generic body): {}",
                boss_sheets_missed.len(),
                boss_sheets_missed.join(", ")
            );
        }
    }
    let active_parallax_theme = ParallaxTheme::from_room_metadata(active_room_metadata);
    let parallax_layers =
        load_parallax_layers_for_theme(catalog, asset_server, active_parallax_theme, quality);

    let missing = EntitySprite::ALL.len() - entities.len();
    if missing > 0 {
        eprintln!(
            "[game_assets] {missing}/{} entity sprites missing under assets/{}/ — those entities use colored rectangles. Drop matching files in to enable them.",
            EntitySprite::ALL.len(),
            config.sprite_folder,
        );
    }

    GameAssets {
        characters,
        entities,
        boss,
        boss_sprites,
        parallax_layers,
    }
}

#[cfg(test)]
mod tests;
