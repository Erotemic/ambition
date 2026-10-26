//! Compatibility facade for game-asset resources and loaders.
//!
//! It loads the art of every kind (characters, props, effects, bosses) into
//! `GameAssets`, so it sits above both the asset catalog (`crate::assets`)
//! and the sheet builders (`crate::character_sprites`), which name neither it
//! nor each other's loaders.
//!
//! TODO(compat-remove): move `load_game_assets` out of the actor monolith, migrate callers to
//! `ambition_sprite_sheet::game_assets`, then delete this module.


use bevy::prelude::*;
use std::collections::HashMap;

use ambition_sprite_sheet::boss as sprites;
use ambition_platformer2d_assets::character_sprites;
use ambition_platformer2d_world::rooms::RoomMetadata;
use ambition_persistence::settings::VisualQualityBudget;
use ambition_sprite_sheet::game_assets::GameAssetConfig;
use ambition_sprite_sheet::game_assets::GameAssets;
use ambition_sprite_sheet::game_assets::load_entity_sprites;
use ambition_sprite_sheet::game_assets::ParallaxTheme;
use ambition_sprite_sheet::game_assets::load_parallax_layers_for_theme;
use ambition_sprite_sheet::game_assets::EntitySprite;

/// Where one prop kind's sheet comes from: the sheet record `target` of the
/// current sheet generation, drawn from its page image in the sprite folder.
/// A `packed` sheet is drawn from the shared sprite pack first, at the
/// quality tier of the game.
#[derive(Clone, Debug)]
pub struct PropSheetSource {
    pub target: String,
    pub tuning: ambition_sprite_sheet::character::SheetTuning,
    pub packed: bool,
}

/// The prop sheets a composition's content registers, keyed by the
/// `PropSpec.kind` its rooms author.
///
/// Content adds rows when its plugin builds ([`PropSheetsAppExt`]). A sheet is
/// decoded on demand, by [`load_the_prop_sheets_in_reach`]: the scenery of a
/// room when that room is in reach, every other row when a session starts.
/// [`load_game_assets`] still loads every row it is handed; the compositions
/// hand it none, so a boot decodes no scenery.
#[derive(Resource, Clone, Debug, Default)]
pub struct PropSheets(Vec<(String, PropSheetSource)>);

impl PropSheets {
    /// Add the sheet for `kind`. A kind has one sheet, so a second row for
    /// the same kind is a composition error.
    pub fn register(&mut self, kind: impl Into<String>, source: PropSheetSource) {
        let kind = kind.into();
        assert!(
            self.0.iter().all(|(known, _)| *known != kind),
            "the prop kind `{kind}` has two sheets registered; a kind has one look"
        );
        self.0.push((kind, source));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &PropSheetSource)> {
        self.0.iter().map(|(kind, source)| (kind.as_str(), source))
    }

    /// Replace the rows one provider registered from its pack (`previous`,
    /// the kinds of its last registration) with `rows`, and return the kinds
    /// it now owns. A kind another provider registered is refused, and
    /// nothing changes: a reload must not take another provider's look.
    pub fn restate<'a>(
        &mut self,
        previous: impl IntoIterator<Item = &'a str>,
        rows: &ambition_sprite_sheet::prop_sheets::PropSheetRows,
    ) -> Result<Vec<String>, String> {
        let previous: Vec<&str> = previous.into_iter().collect();
        let taken: Vec<&String> = rows
            .keys()
            .filter(|kind| self.0.iter().any(|(known, _)| known == *kind) && !previous.contains(&kind.as_str()))
            .collect();
        if !taken.is_empty() {
            return Err(format!("prop sheet kind(s) {taken:?} belong to another provider"));
        }
        self.0.retain(|(kind, _)| !previous.contains(&kind.as_str()));
        self.0.extend(rows.iter().map(|(kind, row)| (kind.clone(), sprite_folder_row(kind, row))));
        Ok(rows.keys().cloned().collect())
    }
}

/// A pack row's source.
fn sprite_folder_row(kind: &str, row: &ambition_sprite_sheet::prop_sheets::AuthoredPropSheet) -> PropSheetSource {
    PropSheetSource { target: row.target(kind).to_string(), tuning: row.tuning(), packed: row.packed }
}

/// The realization of the prop sheet `source`: from the shared sprite pack
/// when it is `packed` and the pack has it, else from the record's page.
fn load_prop_sheet(
    source: &PropSheetSource,
    config: &GameAssetConfig,
    catalog: Option<&ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: Option<&VisualQualityBudget>,
) -> Option<ambition_sprite_sheet::character::CharacterSpriteAsset> {
    let packed = catalog.filter(|_| source.packed).and_then(|catalog| {
        let spec = ambition_sprite_sheet::character::try_load_spec_for_target(&source.target, &source.tuning)?;
        character_sprites::build_prop_sprite_asset_packed(catalog, asset_server, layouts, &source.target, &spec, quality)
    });
    packed.or_else(|| {
        character_sprites::load_prop_sheet_for_target(asset_server, layouts, &config.sprite_folder, &source.target, &source.tuning)
    })
}

/// The system set of [`load_the_prop_sheets_in_reach`]. A system that reads
/// which prop sheets are loaded to decide what a room waits for (a room's
/// asset manifest, the neighbour prefetch) orders after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropSheetDemandSet;

/// The prop kinds `room` draws as scenery: its props and the looks of its
/// gated walls. Nothing else draws these kinds, so their sheets are needed
/// only while the room is in reach.
pub fn scenery_prop_kinds(room: &ambition_platformer2d_world::rooms::RoomSpec) -> impl Iterator<Item = &str> {
    room.props
        .iter()
        .map(|prop| prop.kind.as_str())
        .chain(room.lock_walls.iter().filter_map(|wall| wall.look.as_deref()))
}

/// Decode the prop sheets the session can show soon, and no others.
///
/// A sheet is wanted when its kind is the scenery ([`scenery_prop_kinds`]) of
/// a room in reach (a live room, a door's neighbour of one, or the far room of
/// an open gate link), or when no room's scenery names its kind (a drop's
/// look, a pickup, a platform's sheet: art that is not one room's). A wanted
/// sheet that is not loaded is loaded; a loaded one stays, as character
/// sheets do. It does the work only when the rooms in reach, the loaded art or
/// the rows changed, so a session that stays in one room pays one comparison
/// a frame.
///
/// Before this, every row decoded at boot, the scenery of every district
/// with it (MEASURED 2026-10-24: 63.7 MP at boot against the 40 MP budget,
/// 16 MP of it the Pirate Port's and the Ninja Village's scenery).
#[allow(clippy::too_many_arguments)]
pub fn load_the_prop_sheets_in_reach(
    rooms: ambition_platformer2d_world::rooms::LiveRoomSpecs,
    network: Option<Res<ambition_platformer2d_world::rooms::GateNetworkState>>,
    sheets: Res<PropSheets>,
    config: Res<GameAssetConfig>,
    catalog: Res<ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>,
    asset_server: Res<AssetServer>,
    (quality, settings): (
        Option<Res<ambition_persistence::settings::ResolvedVisualQuality>>,
        Option<Res<ambition_persistence::settings::UserSettings>>,
    ),
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut assets: ResMut<GameAssets>,
    mut seen: Local<Option<(Vec<usize>, Vec<String>)>>,
) {
    let set = rooms.rooms();
    let index_of = |id: &str| set.rooms.iter().position(|room| room.id == id);
    let mut reach: Vec<usize> = rooms.live_specs().into_iter().filter_map(|(_, spec)| index_of(&spec.id)).collect();
    reach.sort_unstable();
    reach.dedup();
    let mut far: Vec<String> = network
        .iter()
        .flat_map(|network| network.links.values().map(|link| link.far_room.clone()))
        .collect();
    far.sort();
    far.dedup();
    let signature = (reach, far);
    if seen.as_ref() == Some(&signature) && !assets.is_changed() && !sheets.is_changed() && !rooms.set_is_changed() {
        return;
    }
    let (live, far) = signature.clone();
    *seen = Some(signature);
    let mut near: Vec<usize> = live.clone();
    for &room in &live {
        near.extend(set.neighboring_room_indices_of(room));
    }
    near.extend(far.iter().filter_map(|room| index_of(room)));
    let scenery: std::collections::BTreeSet<&str> = set.rooms.iter().flat_map(scenery_prop_kinds).collect();
    let in_reach: std::collections::BTreeSet<&str> =
        near.iter().filter_map(|&room| set.rooms.get(room)).flat_map(scenery_prop_kinds).collect();
    let missing: Vec<(&str, &PropSheetSource)> = sheets
        .iter()
        .filter(|(kind, _)| !scenery.contains(kind) || in_reach.contains(kind))
        .filter(|(kind, _)| !assets.characters.props.contains_key(*kind))
        .collect();
    if missing.is_empty() {
        return;
    }
    let quality = ambition_persistence::settings::ResolvedVisualQuality::current(quality.as_deref(), settings.as_deref())
        .budget
        .clone();
    for (kind, source) in missing {
        match load_prop_sheet(source, &config, Some(&catalog), &asset_server, &mut layouts, Some(&quality)) {
            Some(asset) => {
                assets.characters.props.insert(kind.to_string(), asset);
            }
            None => bevy::log::warn_once!(
                "the prop sheet for `{kind}` does not load under the {} profile; it draws as a placeholder",
                catalog.profile().label(),
            ),
        }
    }
}

/// Register prop sheets from a content plugin's `build`.
pub trait PropSheetsAppExt {
    fn register_prop_sheet(&mut self, kind: impl Into<String>, source: PropSheetSource) -> &mut Self;
    /// Register each row of a pack's `prop_sheets`, and return the kinds, for
    /// a reload to restate ([`PropSheets::restate`]).
    fn register_prop_sheet_rows(&mut self, rows: &ambition_sprite_sheet::prop_sheets::PropSheetRows) -> Vec<String>;
}

impl PropSheetsAppExt for App {
    fn register_prop_sheet(&mut self, kind: impl Into<String>, source: PropSheetSource) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<PropSheets>()
            .register(kind, source);
        self
    }

    fn register_prop_sheet_rows(&mut self, rows: &ambition_sprite_sheet::prop_sheets::PropSheetRows) -> Vec<String> {
        for (kind, row) in rows {
            self.register_prop_sheet(kind.clone(), sprite_folder_row(kind, row));
        }
        rows.keys().cloned().collect()
    }
}

/// Restate one provider's rows (`previous` are the kinds it owned), load
/// their sheets again and forget the sheets of the kinds it dropped. Returns
/// the kinds it owns now; on a refusal (a kind of another provider) nothing
/// changes. Each content reload that restates prop sheets calls this.
pub fn restate_prop_sheet_rows(
    world: &mut World,
    previous: &[String],
    rows: &ambition_sprite_sheet::prop_sheets::PropSheetRows,
) -> Result<Vec<String>, String> {
    let kinds = world
        .get_resource_or_insert_with(PropSheets::default)
        .restate(previous.iter().map(String::as_str), rows)?;
    let dropped: Vec<String> = previous.iter().filter(|kind| !kinds.contains(kind)).cloned().collect();
    reload_prop_sheets(world, &dropped, &kinds);
    Ok(kinds)
}

/// Load again each prop sheet made from a sheet of `targets`, after a sheet
/// generation is published: a prop's realization holds the frame rects of
/// the records it was made from.
pub fn reload_prop_sheets_of_targets(world: &mut World, targets: &std::collections::BTreeSet<String>) {
    let Some(sheets) = world.get_resource::<PropSheets>() else {
        return;
    };
    let kinds: Vec<String> = sheets
        .iter()
        .filter(|(_, source)| targets.contains(&source.target))
        .map(|(kind, _)| kind.to_owned())
        .collect();
    if !kinds.is_empty() {
        reload_prop_sheets(world, &[], &kinds);
    }
}

/// Make again each resident effect sheet of `targets`, at the tier it was made
/// at, after a sheet generation is published: an effect drawn after it reads
/// the frame rects of the new records.
pub fn reload_fx_sheets_of_targets(world: &mut World, targets: &std::collections::BTreeSet<String>) {
    let Some(asset_server) = world.get_resource::<AssetServer>().cloned() else {
        return;
    };
    if !world.contains_resource::<GameAssets>() || !world.contains_resource::<Assets<TextureAtlasLayout>>() {
        return;
    }
    world.resource_scope(|world, mut layouts: Mut<Assets<TextureAtlasLayout>>| {
        let fx = &mut world.resource_mut::<GameAssets>().into_inner().fx;
        let stale: Vec<(&'static str, ambition_persistence::settings::TextureResolutionScale)> = fx
            .targets()
            .into_iter()
            .filter(|target| targets.contains(*target))
            .filter_map(|target| fx.get(target).map(|asset| (target, asset.requested_tier)))
            .collect();
        for (target, tier) in stale {
            fx.remove(target);
            character_sprites::ensure_fx_sheet_loaded(fx, target, &asset_server, &mut layouts, tier);
        }
    });
}

/// Load the sheets of `kinds` again into the loaded art, after a reload
/// restated their rows, and forget the sheets of `dropped`. The room the
/// reload builds draws its props from the new sheets. A world with no loaded
/// art (a headless one) has nothing to load again.
fn reload_prop_sheets(world: &mut World, dropped: &[String], kinds: &[String]) {
    let (Some(config), Some(asset_server)) =
        (world.get_resource::<GameAssetConfig>().cloned(), world.get_resource::<AssetServer>().cloned())
    else {
        return;
    };
    if !world.contains_resource::<GameAssets>() || !world.contains_resource::<Assets<TextureAtlasLayout>>() {
        return;
    }
    let sheets = world.get_resource::<PropSheets>().cloned().unwrap_or_default();
    let catalog = world.get_resource::<ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog>().cloned();
    // The tier the materializer reads (`converge_character_residency_to_active_quality`).
    let quality = ambition_persistence::settings::ResolvedVisualQuality::current(
        world.get_resource::<ambition_persistence::settings::ResolvedVisualQuality>(),
        world.get_resource::<ambition_persistence::settings::UserSettings>(),
    )
    .budget
    .clone();
    let loaded_kinds: Vec<String> = world.resource::<GameAssets>().characters.props.keys().cloned().collect();
    let mut loaded = Vec::new();
    world.resource_scope(|_, mut layouts: Mut<Assets<TextureAtlasLayout>>| {
        // Only the sheets that are loaded now: the others load when a room
        // in reach shows them (`load_the_prop_sheets_in_reach`).
        let loaded_now = |kind: &str| loaded_kinds.iter().any(|known| known == kind);
        for (kind, source) in sheets.iter().filter(|(kind, _)| kinds.iter().any(|wanted| wanted == kind) && loaded_now(kind)) {
            let asset = load_prop_sheet(source, &config, catalog.as_ref(), &asset_server, &mut layouts, Some(&quality));
            loaded.push((kind.to_string(), asset));
        }
    });
    let mut game_assets = world.resource_mut::<GameAssets>();
    let props = &mut game_assets.characters.props;
    for kind in dropped {
        props.remove(kind);
    }
    for (kind, asset) in loaded {
        match asset {
            Some(asset) => {
                props.insert(kind, asset);
            }
            None => {
                props.remove(&kind);
                bevy::log::warn!(
                    target: "ambition_platformer2d::character_sprites",
                    "the prop sheet for `{kind}` does not load; it draws as a placeholder"
                );
            }
        }
    }
}

/// Load every registered prop sheet. A row that does not load draws as the
/// placeholder quad, as an absent sheet always has.
fn load_prop_sheets(
    sheets: &PropSheets,
    config: &GameAssetConfig,
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    quality: Option<&VisualQualityBudget>,
    props: &mut HashMap<String, ambition_sprite_sheet::character::CharacterSpriteAsset>,
) {
    for (kind, source) in sheets.iter() {
        let asset = load_prop_sheet(source, config, Some(catalog), asset_server, layouts, quality);
        match asset {
            Some(asset) => {
                props.insert(kind.to_string(), asset);
            }
            None => bevy::log::warn!(
                target: "ambition_platformer2d::character_sprites",
                "the prop sheet for `{kind}` does not load under the {} profile; it draws as a placeholder",
                catalog.profile().label(),
            ),
        }
    }
}

/// Build a fresh `GameAssets`, honoring `config` + the shared catalog resource.
#[allow(clippy::too_many_arguments)]
pub fn load_game_assets(
    config: &GameAssetConfig,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    // Provider-authored sheets (U1).
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    active_room_metadata: &RoomMetadata,
    quality: Option<&VisualQualityBudget>,
    prop_sheets: &PropSheets,
) -> GameAssets {
    if config.no_assets {
        eprintln!("[game_assets] --no-assets in effect: rendering with colored-rectangle placeholders only");
        return GameAssets::default();
    }

    let mut characters = character_sprites::load_character_sprites_in(
        authored_sheets,
        character_catalog,
        catalog,
        asset_server,
        layouts,
        quality,
    );
    load_prop_sheets(
        prop_sheets,
        config,
        catalog,
        asset_server,
        layouts,
        quality,
        &mut characters.props,
    );
    let entities = load_entity_sprites(catalog, asset_server, quality);
    // The engine's own effect art. Not a character, not an LDtk prop, and not
    // content's job to declare — see `ambition_sprite_sheet::fx`.
    let fx = character_sprites::load_fx_sheets(asset_server, layouts, &config.sprite_folder, quality);
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

    let mut assets = GameAssets {
        characters,
        entities,
        fx,
        boss,
        boss_sprites,
        parallax_layers,
        room_dressing: Default::default(),
    };
    // The rest of the art of the start room's theme (the terrain skin, a
    // corrupted state of the sky) comes by the road each later room takes.
    ambition_sprite_sheet::game_assets::ensure_parallax_layers_for_room(
        &mut assets,
        catalog,
        asset_server,
        active_room_metadata,
        quality,
    );
    assets
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
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
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
    room.boss_spawns
        .iter()
        .map(|spawn| {
            let profile = ambition_boss_encounter::behavior::authored_boss_behavior(
                boss_catalog,
                &spawn.name,
                &spawn.payload,
            );
            profile.id.to_ascii_lowercase().replace('-', "_")
        })
        .filter(|key| boss_catalog.has_authored_sheet(key))
        .collect()
}
