//! Compatibility facade for game-asset resources and loaders.
//!
//! TODO(compat-remove): move `load_game_assets` out of the actor monolith, migrate callers to
//! `ambition_sprite_sheet::game_assets`, then delete this module.


use bevy::prelude::*;
use std::collections::HashMap;

use ambition_boss_encounter::sprites;
use crate::character_sprites;
use ambition_platformer2d_world::rooms::RoomMetadata;
use ambition_persistence::settings::VisualQualityBudget;
use ambition_sprite_sheet::game_assets::GameAssetConfig;
use ambition_sprite_sheet::game_assets::GameAssets;
use ambition_sprite_sheet::game_assets::load_entity_sprites;
use ambition_sprite_sheet::game_assets::ParallaxTheme;
use ambition_sprite_sheet::game_assets::load_parallax_layers_for_theme;
use ambition_sprite_sheet::game_assets::EntitySprite;

/// Build a fresh `GameAssets`, honoring `config` + the shared catalog resource.
pub fn load_game_assets(
    config: &GameAssetConfig,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    // Provider-authored sheets (U1).
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    catalog: &crate::assets::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    active_room_metadata: &RoomMetadata,
    quality: Option<&VisualQualityBudget>,
) -> GameAssets {
    if config.no_assets {
        eprintln!("[game_assets] --no-assets in effect: rendering with colored-rectangle placeholders only");
        return GameAssets::default();
    }

    let characters = character_sprites::load_character_sprites_in(
        authored_sheets,
        character_catalog,
        catalog,
        asset_server,
        layouts,
        quality,
    );
    let entities = load_entity_sprites(catalog, asset_server, quality);
    // The engine's own effect art. Not a character, not an LDtk prop, and not
    // content's job to declare — see `ambition_sprite_sheet::fx`.
    let fx = character_sprites::load_fx_sheets(asset_server, layouts, &config.sprite_folder);
    let fallback_sheet_key = boss_catalog.fallback_sheet_key();
    let boss = fallback_sheet_key.and_then(|key| {
        sprites::load_boss_sprite_in(
            catalog,
            asset_server,
            layouts,
            key,
            boss_catalog.sheet_for_key(key),
            quality,
        )
    });
    // ⛔ NO DEDICATED BOSS SHEET IS DECODED HERE ANY MORE (asset open work 2,
    // 2026-09-02). Every boss in the catalog used to load at boot — seven
    // sheets, 30 MP, resident in rooms with no boss in them, measured by the
    // image stage ledger in the hall as the single largest owner. A room that
    // authors a `BossSpawn` demands them through `ensure_boss_sheets_loaded`
    // when it is prepared, and the reveal barrier waits on them there. The
    // fallback body above stays eager: it is one sheet and every boss may need
    // it.
    let boss_sprites: HashMap<String, sprites::BossSpriteAsset> = HashMap::new();
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
        fx,
        boss,
        boss_sprites,
        parallax_layers,
    }
}

#[cfg(test)]
mod tests;

/// Decode every dedicated boss sheet the catalog names that `assets` does not
/// hold yet. THE seam a boss's art comes into residency through: called when a
/// room that authors a `BossSpawn` is prepared (transition, prefetch, direct
/// startup), never at boot.
///
/// `keys` are the render keys the room's placements resolve to
/// ([`boss_sheet_keys_for_room`]); `None` means every dedicated sheet the
/// catalog names (a fixture with no room in hand). Sheets already resident are
/// skipped, so a second boss room decodes only what it adds.
///
/// Returns how many sheets this call decoded.
pub fn ensure_boss_sheets_loaded(
    assets: &mut GameAssets,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    keys: Option<&std::collections::BTreeSet<String>>,
    catalog: &crate::assets::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: Option<&VisualQualityBudget>,
) -> usize {
    let fallback_sheet_key = boss_catalog.fallback_sheet_key();
    let mut loaded = 0usize;
    let mut missed: Vec<String> = Vec::new();
    for (key, _filename) in boss_catalog
        .sprite_filenames()
        .filter(|(key, _)| Some(*key) != fallback_sheet_key)
        .filter(|(key, _)| keys.is_none_or(|wanted| wanted.contains(*key)))
    {
        if assets.boss_sprites.contains_key(key) {
            continue;
        }
        let spec = boss_catalog.sheet_for_key(key);
        match sprites::load_named_boss_sprite_via_catalog(
            catalog,
            asset_server,
            layouts,
            key,
            spec,
            quality,
        ) {
            Some(sheet) => {
                assets.boss_sprites.insert(key.to_string(), sheet);
                loaded += 1;
            }
            None => missed.push(key.to_string()),
        }
    }
    if loaded > 0 || !missed.is_empty() {
        // A boss renders the provider-selected fallback body exactly when its
        // `boss_key` (its lowercased behavior id) is absent from this map —
        // `upgrade_boss_sprites` warns once per such boss. Listing the keys says
        // whether a key was never LOADED (below) or never LOOKED UP under that
        // name (the render keys on `behavior.id`).
        let mut keys: Vec<&str> = assets.boss_sprites.keys().map(String::as_str).collect();
        keys.sort_unstable();
        eprintln!(
            "[boss_sprites] {loaded} dedicated sheet(s) decoded for a boss room; resident: {}",
            keys.join(", ")
        );
        if !missed.is_empty() {
            eprintln!(
                "[boss_sprites] {} FAILED to load (these bosses draw the generic body): {}",
                missed.len(),
                missed.join(", ")
            );
        }
    }
    loaded
}

/// The dedicated-sheet render keys a room's `BossSpawn`s resolve to — the same
/// derivation the renderer makes for a LIVE boss (`upgrade_boss_sprites`:
/// `behavior.id`, lowercased, `-` → `_`), run over the authored placements
/// before anything spawns, so a boss room demands its own bosses' art and no
/// other's. A placement whose profile has no dedicated sheet contributes
/// nothing (it draws the fallback body by design).
pub fn boss_sheet_keys_for_room(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
) -> std::collections::BTreeSet<String> {
    use ambition_boss_encounter::BossBehaviorProfileExt as _;
    room.boss_spawns
        .iter()
        .map(|spawn| {
            let canonical = ambition_boss_encounter::behavior::canonical_boss_id_from(
                &spawn.name,
                &spawn.payload,
            );
            let profile =
                ambition_boss_encounter::pattern::profile::BossBehaviorProfile::for_authored_boss(
                    boss_catalog,
                    &canonical,
                );
            profile.id.to_ascii_lowercase().replace('-', "_")
        })
        .filter(|key| boss_catalog.has_authored_sheet(key))
        .collect()
}
