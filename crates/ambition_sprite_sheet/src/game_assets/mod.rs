//! Game asset wiring for character sheets, entity sprites, and parallax art.
//!
//! Missing assets remain optional so rendering can fall back to placeholders, and
//! callers resolve through `GameAssets` rather than depending on source paths.

use bevy::prelude::*;
use std::collections::HashMap;

use ambition_asset_manager::AssetProfile;

use crate::boss::BossSpriteAsset;
use crate::character::{CharacterSpriteAsset, CharacterSpriteAssets};
use ambition_persistence::settings::VisualQualityBudget;
use ambition_platformer2d_world::rooms::RoomMetadata;

/// Load one sheet or layer image. This is the one place that decides whether
/// the decoded pixels stay in the main world.
///
/// Bevy's default `RenderAssetUsages::MAIN_WORLD | RENDER_WORLD` keeps a CPU
/// copy of each image and clones it into the render world on extract. These
/// images load `RENDER_WORLD` only, so extract is a move and the CPU copy is
/// freed: Bevy 0.19 takes the pixels and leaves the `Image` in `Assets<Image>`
/// with `data == None` and its size intact. This is safe because readiness
/// checks use the asset server's load state (`texture_is_ready`, the room
/// manifest), the image census gets the byte count from the descriptor, and no
/// production reader indexes a sheet's pixels.
/// `AMBITION_IMAGES_RENDER_WORLD_ONLY=0` keeps the CPU copy for comparison. The
/// value is read once, and the visual-quality census records it.
///
/// `source` names the caller road (`"character-sheet"`, `"parallax"`,
/// `"fx-sheet"`, `"boss-sheet"`) for the [`image_stages`] ledger. The ledger
/// stamps the demand time here, so a late image can report its delay and stage.
pub fn load_sheet_image(content: &crate::SpriteContent,
    asset_server: &AssetServer,
    source: &'static str,
    path: impl Into<bevy::asset::AssetPath<'static>>,
) -> Handle<Image> {
    let path = path.into();
    let path = crate::baked_sheet_rons::current(content).image_path(&path.to_string())
        .map(|address| bevy::asset::AssetPath::from(address.to_owned())).unwrap_or(path);
    let label = path.to_string();
    let handle = if images_render_world_only() {
        asset_server
            .load_builder()
            .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
                settings.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD;
            })
            .load(path)
    } else {
        asset_server.load(path)
    };
    image_stages::note_demand(handle.id().untyped(), source, label);
    handle
}

pub const IMAGES_RENDER_WORLD_ONLY_ENV: &str = "AMBITION_IMAGES_RENDER_WORLD_ONLY";

/// Whether sheet images skip the main-world copy: on unless the environment
/// says `0`. Read once per process.
pub fn images_render_world_only() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var(IMAGES_RENDER_WORLD_ONLY_ENV)
            .ok()
            .is_none_or(|v| v != "0")
    })
}

/// Select the default [`AssetProfile`] for the build target.
///
/// - wasm32: [`AssetProfile::WebStatic`], or `WebServedAssets` with the
///   `web_served` feature. The static build embeds the LDtk bootstrap only;
///   optional PNGs are skipped and the renderer paints colored rectangles.
/// - Android: [`AssetProfile::AndroidBundle`] (the Android AssetReader reads
///   from the APK).
/// - Other targets: [`AssetProfile::DesktopDevLoose`] (a workspace-relative
///   `assets/` directory, with hot reload).
///
/// `--no-assets` changes this to [`AssetProfile::NoAssets`] through
/// [`GameAssetConfig::from_arg_slice`].
pub fn default_asset_profile() -> AssetProfile {
    if cfg!(target_arch = "wasm32") {
        if cfg!(feature = "web_served") {
            // Build composed `--features web_served_assets` →
            // browser fetches `/assets/*` over HTTP.
            AssetProfile::WebServedAssets
        } else {
            // Build composed `--features web` → only assets with an
            // authored `EmbeddedBinary` candidate are attempted.
            AssetProfile::WebStatic
        }
    } else if cfg!(target_os = "android") {
        AssetProfile::AndroidBundle
    } else {
        AssetProfile::DesktopDevLoose
    }
}

/// CLI/runtime configuration for asset loading. Inserted as a Bevy resource
/// before the presentation startup system runs.
#[derive(Resource, Clone, Debug)]
pub struct GameAssetConfig {
    /// When true, skip every disk asset load and force colored-rectangle
    /// placeholders everywhere. Set via the `--no-assets` CLI flag.
    /// Equivalent to setting `asset_profile = AssetProfile::NoAssets`;
    /// kept as a separate flag so the existing "rendering with
    /// placeholders only" log line is preserved.
    pub no_assets: bool,
    /// Directory under `assets/` that holds character + entity sprites.
    /// Default `"sprites"`. Lets designers point at experimental sets
    /// without recompiling.
    pub sprite_folder: String,
    /// Active [`AssetProfile`] for catalog resolution. Defaults from
    /// [`default_asset_profile`] (per-target cfg). `--no-assets` flips
    /// it to [`AssetProfile::NoAssets`].
    pub asset_profile: AssetProfile,
}

impl Default for GameAssetConfig {
    fn default() -> Self {
        Self {
            no_assets: false,
            sprite_folder: "sprites".into(),
            asset_profile: default_asset_profile(),
        }
    }
}

impl GameAssetConfig {
    /// Parse the supported flags out of process args. Unknown args are left
    /// alone (Bevy may consume some itself).
    pub fn from_args() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        Self::from_arg_slice(&args)
    }

    /// Parse the supported flags out of an explicit arg slice. Unit-testable
    /// counterpart of `from_args` that doesn't read `env::args`.
    pub fn from_arg_slice(args: &[String]) -> Self {
        let mut config = Self::default();
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--no-assets" => {
                    config.no_assets = true;
                    // Keep the asset profile in sync so the catalog
                    // resolver reports every entry as Disabled too.
                    config.asset_profile = AssetProfile::NoAssets;
                }
                "--sprite-folder" => {
                    if let Some(folder) = args.get(i + 1) {
                        config.sprite_folder = folder.clone();
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        config
    }
}

mod entity_sprite;
mod resolvers;
mod room_dressing;

/// The per-image stage ledger the funnel below feeds; see its module docs.
pub use ambition_asset_manager::image_stages;
pub use entity_sprite::*;
pub use resolvers::*;
pub use room_dressing::*;

/// A theme key supplied by authored content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParallaxTheme(&'static str);

impl ParallaxTheme {
    pub const fn new(key: &'static str) -> Self { Self(key) }
    pub const fn key(self) -> &'static str { self.0 }

    pub fn from_key(value: &str) -> Option<Self> {
        let value = value.trim();
        (!value.is_empty()).then(|| Self(crate::published_tree::kept(value)))
    }

    pub fn from_room_metadata(metadata: &RoomMetadata) -> Self {
        Self::named_by_room_metadata(metadata).unwrap_or(Self::new("default"))
    }

    /// Only the explicit theme field selects a background and terrain skin.
    pub fn named_by_room_metadata(metadata: &RoomMetadata) -> Option<Self> {
        metadata.visual_profile.parallax_theme.as_deref().and_then(Self::from_key)
    }

    /// The themes whose layers have bindings in this catalog.
    pub fn in_manifest(manifest: &ambition_asset_manager::AssetManifest) -> Vec<Self> {
        manifest.iter().filter_map(|(id, _)| {
            let (key, layer) = id.as_str().strip_prefix("background.parallax.")?.rsplit_once('.')?;
            ParallaxLayerAsset::ALL.iter().any(|entry| entry.key() == layer)
                .then(|| Self::from_key(key)).flatten()
        }).collect::<std::collections::BTreeSet<_>>().into_iter().collect()
    }
}

/// How the motes of a theme are, and how they move. Lengths are in world
/// units and times in seconds. `velocity` has y up.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoteStyle {
    /// How many motes a view has.
    pub count: usize,
    /// The steady motion of each mote: up for an ember, along for spray.
    pub velocity: Vec2,
    /// How far a mote goes from its line, each way.
    pub wander: f32,
    /// How many times a second a mote goes round its wander.
    pub wander_rate: f32,
    /// The smallest and the largest side of a mote.
    pub size: (f32, f32),
    /// The alpha of a mote at its most.
    pub alpha: f32,
    /// How much of its alpha a mote loses at the low point of its twinkle
    /// (0 is a steady mote).
    pub twinkle: f32,
    pub twinkle_rate: f32,
    /// The share of the motes that are in front of the play. The others are
    /// behind the blocks.
    pub front: f32,
}

/// Alternate states of room art, supplied by the selected libraries.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomThemes {
    #[serde(default)]
    pub alternates: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub motes: std::collections::BTreeMap<String, MoteStyle>,
}

impl RoomThemes {
    pub fn alternate(&self, theme: ParallaxTheme) -> Option<ParallaxTheme> {
        self.alternates.get(theme.key()).and_then(|key| ParallaxTheme::from_key(key))
    }

    pub(crate) fn merge(&mut self, other: Self) -> Result<(), String> {
        for (key, style) in other.motes {
            if self.motes.insert(key.clone(), style).is_some() { return Err(format!("Two libraries define motes for {key:?}")); }
        }
        for (key, alternate) in other.alternates {
            if key.trim().is_empty() || alternate.trim().is_empty() || key == alternate {
                return Err(format!("Invalid room theme alternate: {key:?} -> {alternate:?}"));
            }
            if self.alternates.insert(key.clone(), alternate).is_some() {
                return Err(format!("Two libraries define the alternate of room theme {key:?}"));
            }
        }
        Ok(())
    }
}

/// Generated background/parallax image key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParallaxLayerAsset {
    /// Mostly opaque biome sky/backdrop. This is what prevents the room from
    /// falling back to a transparent-on-grid-only look.
    Sky,
    FarBackplate,
    NearBackground,
    /// The air between the scene and the play: mist, beams, dust. It is behind
    /// the play, in spite of its name.
    ForegroundAtmosphere,
    /// A few dark things between the eye and the play: the one layer that is
    /// drawn in front of the play. A theme with no foreground publishes an
    /// empty picture for it.
    Foreground,
    /// The one picture a tier that draws no parallax shows behind the play,
    /// fixed to the view: the layers behind the play as a view at the middle
    /// of the room shows them, made small (`scenes.flat` in the art
    /// submodule).
    Flat,
}

impl ParallaxLayerAsset {
    pub const ALL: &'static [Self] = &[
        Self::Sky,
        Self::FarBackplate,
        Self::NearBackground,
        Self::ForegroundAtmosphere,
        Self::Foreground,
        Self::Flat,
    ];

    /// The layers a tier that draws parallax draws, back to front.
    pub const STACK: &'static [Self] = &[
        Self::Sky,
        Self::FarBackplate,
        Self::NearBackground,
        Self::ForegroundAtmosphere,
        Self::Foreground,
    ];

    /// Whether a tier with `budget` draws this layer: the stack where parallax
    /// is drawn, the flat backdrop where it is not. With no budget, the stack.
    pub fn drawn_by(self, budget: Option<&VisualQualityBudget>) -> bool {
        let parallax = budget.is_none_or(|budget| budget.parallax.enabled);
        (self == Self::Flat) != parallax
    }

    pub const fn key(self) -> &'static str {
        match self {
            Self::Sky => "sky",
            Self::FarBackplate => "far_backplate",
            Self::NearBackground => "near_background",
            Self::ForegroundAtmosphere => "foreground_atmosphere",
            Self::Foreground => "foreground",
            Self::Flat => "flat",
        }
    }

    pub fn relative_path(self, theme: ParallaxTheme) -> String {
        format!(
            "backgrounds/parallax_layers/{}_{}.png",
            theme.key(),
            self.key()
        )
    }
}

/// Map from generated background/parallax layer keys to loaded image handles.
#[derive(Default, Clone)]
pub struct ParallaxLayerSet {
    handles: HashMap<(ParallaxTheme, ParallaxLayerAsset), Handle<Image>>,
}

impl ParallaxLayerSet {
    pub fn get(&self, theme: ParallaxTheme, layer: ParallaxLayerAsset) -> Option<&Handle<Image>> {
        self.handles.get(&(theme, layer))
    }

    /// Ensure the generated parallax stack for `theme` has been requested from
    /// Bevy's [`AssetServer`]. Repeated calls are cheap: already-present layer
    /// handles are left alone, and missing optional assets continue to fall back
    /// to the room renderer's clear color / grid visuals.
    pub fn ensure_theme_loaded(
        &mut self, content: &crate::SpriteContent,
        catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
        asset_server: &AssetServer,
        theme: ParallaxTheme,
        quality: Option<&VisualQualityBudget>,
    ) -> usize {
        let mut added = 0usize;
        // Only what the tier draws: a tier with no parallax decoded each layer
        // of the stack and drew none of them.
        for &layer in ParallaxLayerAsset::ALL.iter().filter(|layer| layer.drawn_by(quality)) {
            if self.handles.contains_key(&(theme, layer)) {
                continue;
            }
            let id = parallax_layer_asset_id(theme, layer);
            let Some(path) = quality
                .and_then(|q| {
                    catalog.try_quality_path_for_load(
                        &id,
                        q.backgrounds.resolution_scale.asset_id_suffix(),
                        q.backgrounds.prefer_scaled_variants,
                    )
                })
                .or_else(|| catalog.try_path_for_load(&id))
            else {
                continue;
            };
            self.handles.insert(
                (theme, layer),
                load_sheet_image(&content, asset_server, "parallax", path),
            );
            added += 1;
        }
        added
    }

    /// Whether a layer of `theme` that a tier with `quality` draws is resident.
    pub fn has_drawn_layers(&self, theme: ParallaxTheme, quality: Option<&VisualQualityBudget>) -> bool {
        ParallaxLayerAsset::ALL
            .iter()
            .filter(|layer| layer.drawn_by(quality))
            .any(|layer| self.handles.contains_key(&(theme, *layer)))
    }

    /// Drop every layer handle whose theme `keep` rejects; returns how many
    /// handles were dropped.
    ///
    /// This is the only eviction API, and it has no policy. `GameAssets` is built
    /// once at startup, so without eviction each visited theme stays resident for
    /// the life of the process.
    ///
    /// The caller decides which themes to keep. That rule needs the active room,
    /// its neighbours, and transition timing, which this crate must not know.
    ///
    /// Dropping a handle is not sufficient. Bevy frees the pixels when the last
    /// `Handle<Image>` drops, so a clone held elsewhere (for example a spawned
    /// `ParallaxLayerVisual`) keeps the image alive. The caller must retire those
    /// visuals. The app-side guard checks that the image leaves `Assets<Image>`.
    pub fn retain_themes(&mut self, keep: impl Fn(ParallaxTheme) -> bool) -> usize {
        let before = self.handles.len();
        self.handles.retain(|(theme, _), _| keep(*theme));
        before - self.handles.len()
    }

    /// The resident themes in key order.
    pub fn resident_themes(&self) -> Vec<ParallaxTheme> {
        self.handles.keys().map(|(theme, _)| *theme)
            .collect::<std::collections::BTreeSet<_>>().into_iter().collect()
    }

    pub fn len(&self) -> usize {
        self.handles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

/// All image-handle assets the sandbox knows about. Inserted as a Bevy
/// resource by the presentation startup system; consumed by rendering
/// systems via [`get`]-style accessors that always tolerate `None`.
#[derive(Resource, Default, Clone)]
pub struct GameAssets {
    pub room_themes: RoomThemes,
    pub characters: CharacterSpriteAssets,
    pub entities: EntitySpriteSet,
    /// The effect sheets the engine draws from, keyed by sheet manifest target
    /// (`generic_exotic_fx`).
    ///
    /// An FX sheet is neither a character nor an LDtk prop, so it has its own
    /// slot. Without it, `spawn_effect` falls back to its no-asset particle
    /// branch. The engine's `load_game_assets` fills this from
    /// [`crate::fx::fx_sheets`] with no content involved.
    pub fx: FxSheetAssets,
    /// Generic boss spritesheet — the fallback the renderer uses for any boss
    /// without a dedicated sheet in `boss_sprites`. Separate from `characters`
    /// because the boss generator emits its own animation rows
    /// (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/death) that don't fit
    /// `CharacterAnim`. `None` falls back to the static `EntitySprite::BossCore`.
    pub boss: Option<BossSpriteAsset>,
    /// Dedicated per-boss spritesheets, keyed by the boss catalog's sheet key
    /// (`BossCatalog::sprite_filenames`) — the renderer looks up
    /// `boss_sprites.get(&boss_key)` and falls back to `boss`.
    pub boss_sprites: HashMap<String, BossSpriteAsset>,
    /// Optional generated biome sky/background/parallax layers. Missing PNGs
    /// are fine: room rendering simply skips the extra layers and keeps the
    /// existing clear-color/grid/block visuals.
    pub parallax_layers: ParallaxLayerSet,
    /// The terrain skin of each resident theme: the art a room lays on its
    /// blocks. Loaded and retired with the parallax layers of the theme.
    pub room_dressing: RoomDressingSet,
}

/// Decoded FX spritesheets, keyed by manifest target.
///
/// Keys are interned metadata identities. The selected content supplies
/// the effect vocabulary.
#[derive(Default, Clone)]
pub struct FxSheetAssets {
    sheets: HashMap<&'static str, CharacterSpriteAsset>,
}

impl FxSheetAssets {
    pub fn insert(&mut self, target: &'static str, asset: CharacterSpriteAsset) {
        self.sheets.insert(target, asset);
    }

    pub fn contains(&self, target: &str) -> bool {
        self.sheets.contains_key(target)
    }

    /// Forget the realization of `target`, so that the next ensure makes it
    /// again.
    pub fn remove(&mut self, target: &str) -> Option<CharacterSpriteAsset> {
        self.sheets.remove(target)
    }

    pub fn get(&self, target: &str) -> Option<&CharacterSpriteAsset> {
        self.sheets.get(target)
    }

    pub fn len(&self) -> usize {
        self.sheets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    /// The targets that decoded, sorted — for the startup tally.
    pub fn targets(&self) -> Vec<&'static str> {
        let mut targets: Vec<&'static str> = self.sheets.keys().copied().collect();
        targets.sort_unstable();
        targets
    }
}

impl GameAssets {
    /// Dedicated boss spritesheet for `key` (a boss catalog sheet key), if one
    /// was loaded. The render layer falls back to [`Self::boss`] when this is
    /// `None`.
    pub fn boss_sprite(&self, key: &str) -> Option<&BossSpriteAsset> {
        self.boss_sprites.get(key)
    }
}

// Full `load_game_assets` remains in `ambition_platformer2d::game_assets`
// because it joins the content-installed character roster. The render-facing
// data/resource types and static image loaders live here.

pub fn load_entity_sprites(content: &crate::SpriteContent,
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    quality: Option<&VisualQualityBudget>,
) -> EntitySpriteSet {
    let mut handles = HashMap::with_capacity(EntitySprite::ALL.len());
    // Report one tally, not one warning per key. A headless fixture with no
    // asset root misses every sprite, and many warnings would hide the signal.
    let mut missing: Vec<String> = Vec::new();
    for &key in EntitySprite::ALL {
        let id = entity_sprite_asset_id(key);
        let Some(path) = quality
            .and_then(|q| {
                catalog.try_quality_path_for_load(
                    &id,
                    q.sprites.resolution_scale.asset_id_suffix(),
                    q.sprites.prefer_scaled_variants,
                )
            })
            .or_else(|| catalog.try_path_for_load(&id))
        else {
            missing.push(id.to_string());
            continue;
        };
        // Use the `"entity-sprite"` road, not `"fx-sheet"`. Door zones, tiles,
        // and terminals are not effects, and residency by road must count
        // each population separately.
        handles.insert(key, load_sheet_image(&content, asset_server, "entity-sprite", path));
    }
    // This catches only one failure: `try_path_for_load` returns `None` when
    // the catalog refuses an id (no manifest entry, or a quality profile that
    // excludes it).
    if !missing.is_empty() {
        bevy::log::warn!(
            target: "crate::entity_sprites",
            "entity sprites: {}/{} resolved; the catalog refused {} of them, so \
             the features that use them draw as a colour fallback or not at all: \
             {:?}",
            handles.len(),
            EntitySprite::ALL.len(),
            missing.len(),
            missing
        );
    }
    EntitySpriteSet { handles }
}

pub fn load_parallax_layers_for_theme(content: &crate::SpriteContent,
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    theme: ParallaxTheme,
    quality: Option<&VisualQualityBudget>,
) -> ParallaxLayerSet {
    let mut set = ParallaxLayerSet::default();
    let added = set.ensure_theme_loaded(content, catalog, asset_server, theme, quality);
    if added > 0 {
        eprintln!(
            "[game_assets] loaded {added}/{} generated background/parallax layers for '{}' under assets/backgrounds/parallax_layers/ (other themes lazy-load on room transition)",
            ParallaxLayerAsset::ALL.len(),
            theme.key(),
        );
    }
    set
}

/// Load the generated parallax stack for the room that is about to become
/// active. Visual room transitions call this before spawning parallax sprites so
/// startup only pays for the first room's zone art, while later rooms still get
/// their authored background the first time the player visits them.
pub fn ensure_parallax_layers_for_room(content: &crate::SpriteContent,
    assets: &mut GameAssets,
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    metadata: &RoomMetadata,
    quality: Option<&VisualQualityBudget>,
) {
    assets.room_themes = crate::baked_sheet_rons::current(content).room_themes().clone();
    let theme = ParallaxTheme::from_room_metadata(metadata);
    let mut added = assets
        .parallax_layers
        .ensure_theme_loaded(content, catalog, asset_server, theme, quality);
    if let Some(corrupted) = assets.room_themes.alternate(theme) {
        added += assets
            .parallax_layers
            .ensure_theme_loaded(content, catalog, asset_server, corrupted, quality);
    }
    // The skin of the room's blocks comes with its sky, and is kept as long.
    // A room that names no theme has no skin: see `named_by_room_metadata`.
    if let Some(named) = ParallaxTheme::named_by_room_metadata(metadata) {
        assets.room_dressing.ensure_theme_loaded(content, catalog, asset_server, named);
        // A theme with two states lays both skins on its blocks.
        if let Some(corrupted) = assets.room_themes.alternate(named) {
            assets.room_dressing.ensure_theme_loaded(content, catalog, asset_server, corrupted);
        }
    }
    if added > 0 {
        bevy::log::debug!(
            target: "ambition_platformer2d::assets",
            "lazy-loaded {added}/{} parallax layers for '{}'",
            ParallaxLayerAsset::ALL.len(),
            theme.key(),
        );
    }
}

// Optional-image load policy lives in `Platformer2dAssetCatalog`:
// desktop profiles pre-check loose files, bundled/mobile profiles trust
// packaging, web/static profiles skip optional PNGs, and headless/no-assets have
// already returned `None` upstream.

// Splitting sprite-side coverage into this crate is a separate opportunity
// (dev/journals/code_smells.md).

#[cfg(test)]
mod parallax_residency_tests {
    use super::*;
    use ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog;
    use ambition_asset_manager::AssetProfile;

    fn packaged_catalog() -> Platformer2dAssetCatalog {
        Platformer2dAssetCatalog::new(
            ambition_asset_manager::AmbitionAssetCatalog::new({
                let mut manifest = ambition_asset_manager::AssetManifest::default();
                for theme in [ParallaxTheme::new("hub"), ParallaxTheme::new("lab"), ParallaxTheme::new("cave"), ParallaxTheme::new("boss")] {
                    for layer in ParallaxLayerAsset::ALL {
                        let id = parallax_layer_asset_id(theme, *layer);
                        manifest.insert(ambition_asset_manager::AssetEntry::new(id.clone(), ambition_asset_manager::AssetKind::Image, format!("fixture/{}.png", id.as_str())));
                    }
                }
                manifest
            }),
            AssetProfile::AndroidBundle,
        )
    }

    fn asset_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app
    }

    /// Visiting themes accumulates layers, and `retain_themes` evicts exactly
    /// the rejected themes.
    #[test]
    fn three_visited_themes_accumulate_and_retain_evicts_all_but_one() {
        let app = asset_app();
        let catalog = packaged_catalog();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let per_theme = ParallaxLayerAsset::STACK.len();

        let mut set = ParallaxLayerSet::default();
        assert!(set.is_empty(), "a fresh set holds nothing");

        for (visited, theme) in [ParallaxTheme::new("hub"), ParallaxTheme::new("cave"), ParallaxTheme::new("lab")]
            .into_iter()
            .enumerate()
        {
            set.ensure_theme_loaded(&crate::SpriteContent::default(), &catalog, &asset_server, theme, None);
            assert_eq!(
                set.len(),
                per_theme * (visited + 1),
                "visiting {theme:?} should add {per_theme} layers and retire nothing — \
                 the accumulation this test exists to describe",
            );
        }
        assert_eq!(
            set.resident_themes(),
            vec![ParallaxTheme::new("cave"), ParallaxTheme::new("hub"), ParallaxTheme::new("lab")],
            "Resident themes use key order.",
        );

        // Re-visiting is idempotent: the count must not move.
        let before_revisit = set.len();
        set.ensure_theme_loaded(&crate::SpriteContent::default(), &catalog, &asset_server, ParallaxTheme::new("hub"), None);
        assert_eq!(set.len(), before_revisit, "re-visiting a theme reloads nothing");

        let dropped = set.retain_themes(|theme| theme == ParallaxTheme::new("cave"));
        assert_eq!(dropped, per_theme * 2, "two themes' worth of layers dropped");
        assert_eq!(set.len(), per_theme, "only the kept theme's layers remain");
        assert_eq!(set.resident_themes(), vec![ParallaxTheme::new("cave")]);
        assert!(
            set.get(ParallaxTheme::new("hub"), ParallaxLayerAsset::ALL[0]).is_none(),
            "an evicted theme must not answer `get`",
        );
        assert!(
            set.get(ParallaxTheme::new("cave"), ParallaxLayerAsset::ALL[0]).is_some(),
            "the kept theme must be untouched",
        );
    }

    /// Keeping everything must drop nothing — the predicate is not inverted.
    #[test]
    fn retaining_every_theme_drops_nothing() {
        let app = asset_app();
        let catalog = packaged_catalog();
        let asset_server = app.world().resource::<AssetServer>().clone();

        let mut set = ParallaxLayerSet::default();
        set.ensure_theme_loaded(&crate::SpriteContent::default(), &catalog, &asset_server, ParallaxTheme::new("boss"), None);
        let before = set.len();
        assert!(before > 0, "non-vacuity: the boss theme loaded something");

        assert_eq!(set.retain_themes(|_| true), 0);
        assert_eq!(set.len(), before);
        assert_eq!(set.retain_themes(|_| false), before, "and the inverse clears it");
        assert!(set.is_empty());
    }
}

#[cfg(test)]
mod authored_theme_tests {
    use super::*;

    #[test]
    fn a_new_theme_key_and_its_alternate_need_only_authored_data() {
        let mut metadata = RoomMetadata::default();
        metadata.visual_profile.parallax_theme = Some("other_game.cloud_city".into());
        let theme = ParallaxTheme::from_room_metadata(&metadata);
        assert_eq!(theme.key(), "other_game.cloud_city");
        let themes: RoomThemes = serde_json::from_str(r#"{"alternates":{"other_game.cloud_city":"other_game.night"}}"#).unwrap();
        assert_eq!(themes.alternate(theme).unwrap().key(), "other_game.night");
        assert!(RoomThemes::default().alternate(theme).is_none());
        metadata.visual_profile.parallax_theme = None;
        metadata.biome = Some("forest".into());
        assert_eq!(ParallaxTheme::named_by_room_metadata(&metadata), None);
    }
}
