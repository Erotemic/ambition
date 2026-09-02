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

/// Pick a sensible default [`AssetProfile`] for the current build target.
///
/// - wasm32 → [`AssetProfile::WebStatic`] (today's first-pass browser
///   build embeds the LDtk bootstrap; optional sprite/parallax PNGs
///   aren't packaged yet, so the catalog skips them and the rendering
///   layer paints colored rectangles).
/// - Android → [`AssetProfile::AndroidBundle`] (Bevy's Android
///   AssetReader pulls from the APK).
/// - everything else → [`AssetProfile::DesktopDevLoose`] (assumes a
///   workspace-relative `assets/` directory; supports hot reload via
///   the loose-filesystem source).
///
/// `cargo run --bin sandbox -- --no-assets` overrides to [`AssetProfile::NoAssets`]
/// via [`GameAssetConfig::from_arg_slice`].
/// Load one sheet/layer image through the ONE knob that decides whether the
/// decoded pixels stay in the main world.
///
/// Bevy's default `RenderAssetUsages::MAIN_WORLD | RENDER_WORLD` keeps a CPU
/// copy of every image and CLONES it into the render world on extract; the
/// hall at Full tier measured 2.2 GB resident and a 542 ms frame at that clone.
/// These images load `RENDER_WORLD` only: the extract is a move and the CPU
/// copy is freed once extracted: Bevy 0.19 `take_gpu_data`s the pixels and
/// leaves the `Image` in `Assets<Image>` with `data == None` and its size
/// intact. Every readiness check uses the asset server's load state, not the
/// pixels (`texture_is_ready`, the room manifest); the image census derives
/// the byte count from the descriptor when the data is gone; no production
/// reader indexes a sheet's pixels. Measured 2026-09-02 (hall, Quarter,
/// llvmpipe `capture_scene`): the
/// capture is byte-identical either way and peak RSS drops 1533 → 1392 MB.
/// `AMBITION_IMAGES_RENDER_WORLD_ONLY=0` restores the CPU copy for a
/// comparison; read once, recorded by the visual-quality census so a capture
/// says which way it was loaded.
///
/// `source` names the road that demanded it (`"character-sheet"`, `"parallax"`,
/// `"fx-sheet"`, `"boss-sheet"`) for the [`image_stages`] ledger, which stamps
/// the demand instant here so a late image can report how long after it was
/// asked for it arrived, and at which stage.
pub fn load_sheet_image(
    asset_server: &AssetServer,
    source: &'static str,
    path: impl Into<bevy::asset::AssetPath<'static>>,
) -> Handle<Image> {
    let path = path.into();
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

/// The per-image stage ledger the funnel below feeds; see its module docs.
pub use ambition_asset_manager::image_stages;
pub use entity_sprite::*;
pub use resolvers::*;

/// Biome/theme key for generated parallax layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParallaxTheme {
    Hub,
    Lab,
    Basement,
    Cove,
    Skybridge,
    Boss,
    Water,
    Forest,
    Cave,
}

impl ParallaxTheme {
    pub const ALL: &'static [Self] = &[
        Self::Hub,
        Self::Lab,
        Self::Basement,
        Self::Cove,
        Self::Skybridge,
        Self::Boss,
        Self::Water,
        Self::Forest,
        Self::Cave,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Hub => "hub",
            Self::Lab => "lab",
            Self::Basement => "basement",
            Self::Cove => "cove",
            Self::Skybridge => "skybridge",
            Self::Boss => "boss",
            Self::Water => "water",
            Self::Forest => "forest",
            Self::Cave => "cave",
        }
    }

    pub fn from_room_metadata(metadata: &RoomMetadata) -> Self {
        // Explicit room visual profiles are the preferred authoring seam for
        // real rooms. `visual_profile` is a stable id, while `parallax_theme`
        // chooses the generated art stack directly.
        if let Some(theme) = metadata
            .visual_profile
            .parallax_theme
            .as_deref()
            .and_then(Self::from_key)
        {
            return theme;
        }
        if let Some(theme) = metadata
            .visual_profile
            .id
            .as_deref()
            .and_then(Self::from_key)
        {
            return theme;
        }

        // Compatibility fallback for older rooms that only have loose metadata.
        // New authored content should set visual_profile/parallax_theme instead
        // of relying on these heuristic mappings.
        for value in [
            metadata.music_track.as_deref(),
            metadata.biome.as_deref(),
            metadata.ambient_profile.as_deref(),
            metadata.visual_theme.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            let key = value.trim().to_ascii_lowercase().replace('-', "_");
            if key.contains("ninja") || key.contains("dojo") || key.contains("forest") {
                return Self::Forest;
            }
        }
        if let Some(theme) = metadata.biome.as_deref().and_then(Self::from_key) {
            return theme;
        }
        if let Some(theme) = metadata.visual_theme.as_deref().and_then(Self::from_key) {
            return theme;
        }
        if let Some(theme) = metadata.ambient_profile.as_deref().and_then(Self::from_key) {
            return theme;
        }
        Self::Hub
    }

    fn from_key(value: &str) -> Option<Self> {
        let key = value.trim().to_ascii_lowercase().replace('-', "_");
        match key.as_str() {
            "hub" | "default" | "cantina" | "orange" => Some(Self::Hub),
            "lab" | "laboratory" | "teal" => Some(Self::Lab),
            "basement" | "ruins" | "pink" => Some(Self::Basement),
            "cove" | "coast" | "beach" => Some(Self::Cove),
            "skybridge" | "sky" | "blue" => Some(Self::Skybridge),
            "boss" | "mob_arena" | "arena" => Some(Self::Boss),
            "water" | "underwater" | "tide" => Some(Self::Water),
            "forest" | "woods" | "grove" | "bamboo" | "dojo" | "ninja" | "ninja_dojo" => {
                Some(Self::Forest)
            }
            "cave" | "damp" => Some(Self::Cave),
            _ => None,
        }
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
    ForegroundAtmosphere,
}

impl ParallaxLayerAsset {
    pub const ALL: &'static [Self] = &[
        Self::Sky,
        Self::FarBackplate,
        Self::NearBackground,
        Self::ForegroundAtmosphere,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Sky => "sky",
            Self::FarBackplate => "far_backplate",
            Self::NearBackground => "near_background",
            Self::ForegroundAtmosphere => "foreground_atmosphere",
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
        &mut self,
        catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
        asset_server: &AssetServer,
        theme: ParallaxTheme,
        quality: Option<&VisualQualityBudget>,
    ) -> usize {
        let mut added = 0usize;
        for &layer in ParallaxLayerAsset::ALL {
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
                load_sheet_image(asset_server, "parallax", path),
            );
            added += 1;
        }
        added
    }

    /// Drop every layer handle whose theme `keep` rejects; returns how many
    /// handles were dropped.
    ///
    /// ⛔⛔ THIS IS THE ONLY EVICTION API, AND IT DELIBERATELY OWNS NO POLICY.
    /// Until 2026-09-02 this type had none at all: `handles` is private and its
    /// only mutator was [`Self::ensure_theme_loaded`], which inserts. Combined
    /// with `GameAssets` being built once in `Startup`, that made a visited
    /// theme's four layers resident for the life of the process — nine themes ×
    /// four layers is the ceiling a walk can reach, and nothing could release
    /// one. That was a guarantee of the type, not an oversight of a caller,
    /// which is why the fix had to start here.
    ///
    /// ⛔ WHICH THEMES SURVIVE IS THE CALLER'S BUSINESS. A residency rule needs
    /// to know the active room, its neighbours and when a transition commits;
    /// this crate knows none of those and must not learn them. Pass a predicate.
    ///
    /// ⚠ DROPPING A HANDLE IS NECESSARY, NOT SUFFICIENT. Bevy frees the pixels
    /// when the last `Handle<Image>` for an asset drops, so a caller that keeps
    /// its own clone — a spawned `ParallaxLayerVisual`, say — keeps the image
    /// alive no matter what this returns. Retiring visuals is the caller's job
    /// too, and the app-side guard asserts the image actually leaves
    /// `Assets<Image>` rather than trusting this count.
    pub fn retain_themes(&mut self, keep: impl Fn(ParallaxTheme) -> bool) -> usize {
        let before = self.handles.len();
        self.handles.retain(|(theme, _), _| keep(*theme));
        before - self.handles.len()
    }

    /// The themes with at least one resident layer handle, in `ParallaxTheme::ALL`
    /// order so a log line or a census row reads the same way twice.
    pub fn resident_themes(&self) -> Vec<ParallaxTheme> {
        ParallaxTheme::ALL
            .iter()
            .copied()
            .filter(|theme| {
                ParallaxLayerAsset::ALL
                    .iter()
                    .any(|layer| self.handles.contains_key(&(*theme, *layer)))
            })
            .collect()
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
    pub characters: CharacterSpriteAssets,
    pub entities: EntitySpriteSet,
    /// The effect sheets the ENGINE draws from, keyed by sheet manifest
    /// target (`generic_exotic_fx`).
    ///
    /// its own slot, and that is the point. Ambition's intro did; Smash, Sanic and Mary-O did not,
    /// so `spawn_effect` took its no-asset particle branch in all three, always. An FX sheet is
    /// neither a character nor an LDtk prop, and the engine's own `load_game_assets` fills this
    /// from [`crate::fx::FX_SHEETS`] with no content involved.
    pub fx: FxSheetAssets,
    /// Generic boss spritesheet — the fallback the renderer uses for any boss
    /// without a dedicated sheet in `boss_sprites`. Separate from `characters`
    /// because the boss generator emits its own animation rows
    /// (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/death) that don't fit
    /// `CharacterAnim`. `None` falls back to the static `EntitySprite::BossCore`.
    pub boss: Option<BossSpriteAsset>,
    /// Dedicated per-boss spritesheets, keyed by the boss's lowercased behavior id (`boss_key`)
    /// — the renderer looks up `boss_sprites.get(&boss_key)` and falls back to `boss`.
    ///
    /// Multi-part bosses store their pieces under suffixed keys: GNU-ton's split
    /// body/hands render reads `"gnu_ton_body"` / `"gnu_ton_hands"`.
    pub boss_sprites: HashMap<String, BossSpriteAsset>,
    /// Optional generated biome sky/background/parallax layers. Missing PNGs
    /// are fine: room rendering simply skips the extra layers and keeps the
    /// existing clear-color/grid/block visuals.
    pub parallax_layers: ParallaxLayerSet,
}

/// Decoded FX spritesheets, keyed by manifest target.
///
/// A `'static` key because the engine's effect sheets are declared in
/// [`crate::fx::FX_SHEETS`], not discovered from content — the set is a
/// property of the build, not of the loaded world.
#[derive(Default, Clone)]
pub struct FxSheetAssets {
    sheets: HashMap<&'static str, CharacterSpriteAsset>,
    /// Where the sheets live (`{sprite_folder}/{target}_spritesheet.png`),
    /// remembered at boot so a character-owned sheet can be demanded later
    /// without the asset config in hand.
    sprite_folder: String,
}

impl FxSheetAssets {
    pub fn with_sprite_folder(sprite_folder: impl Into<String>) -> Self {
        Self {
            sheets: HashMap::default(),
            sprite_folder: sprite_folder.into(),
        }
    }

    pub fn sprite_folder(&self) -> &str {
        &self.sprite_folder
    }

    pub fn insert(&mut self, target: &'static str, asset: CharacterSpriteAsset) {
        self.sheets.insert(target, asset);
    }

    pub fn contains(&self, target: &str) -> bool {
        self.sheets.contains_key(target)
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
    /// Dedicated boss spritesheet for `key` (the lowercased boss behavior id, or
    /// a multi-part suffix like `"gnu_ton_hands"`), if one was loaded. The render
    /// layer falls back to [`Self::boss`] when this is `None`.
    pub fn boss_sprite(&self, key: &str) -> Option<&BossSpriteAsset> {
        self.boss_sprites.get(key)
    }
}

// Full `load_game_assets` remains in `ambition_platformer2d_actor_monolith::assets::game_assets`
// because it joins the content-installed character roster. The render-facing
// data/resource types and static image loaders live here.

pub fn load_entity_sprites(
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    quality: Option<&VisualQualityBudget>,
) -> EntitySpriteSet {
    let mut handles = HashMap::with_capacity(EntitySprite::ALL.len());
    // a TALLY, not a warning per key. `character_sprites` already reports
    // this way — *"5/5 catalog entries declared, 0 decoded at startup"* — and it
    // is the right shape: a headless fixture with no asset root misses every
    // sprite, and forty separate warnings would train everyone to filter the
    // channel that is supposed to carry this.
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
        // ⛔⛔ ITS OWN ROAD, AND IT WAS `"fx-sheet"` UNTIL 2026-09-02. A door
        // zone, a solid tile and an NPC terminal are not effects, and stamping
        // them as such put them in the FX set's residency bucket — so
        // `resident by road: fx-sheet N×M MP` was two populations, and a
        // measurement of "how big is the effect vocabulary" counted the world's
        // entity icons. Found by the fourth stage: `[image-drawn]
        // sprites/entities/door_zone.png … via fx-sheet` is the line that says
        // it out loud. Same class as the thirteen vfx sheets that were stamped
        // `character-sheet` until the ownership rule landed.
        handles.insert(key, load_sheet_image(asset_server, "entity-sprite", path));
    }
    // ⚠ WHAT THIS CATCHES IS NARROW, AND SAYING SO IS THE POINT.
    // `try_path_for_load` returns `None` when the CATALOG refuses an id (no manifest entry, or
    // a quality profile that excludes it). Two different failures, and this is the one that had
    // no voice at all.
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

pub fn load_parallax_layers_for_theme(
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    theme: ParallaxTheme,
    quality: Option<&VisualQualityBudget>,
) -> ParallaxLayerSet {
    let mut set = ParallaxLayerSet::default();
    let added = set.ensure_theme_loaded(catalog, asset_server, theme, quality);
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
pub fn ensure_parallax_layers_for_room(
    assets: &mut GameAssets,
    catalog: &ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog,
    asset_server: &AssetServer,
    metadata: &RoomMetadata,
    quality: Option<&VisualQualityBudget>,
) {
    let theme = ParallaxTheme::from_room_metadata(metadata);
    let added = assets
        .parallax_layers
        .ensure_theme_loaded(catalog, asset_server, theme, quality);
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
    use bevy::prelude::*;

    fn packaged_catalog() -> Platformer2dAssetCatalog {
        Platformer2dAssetCatalog::new(
            ambition_asset_manager::AmbitionAssetCatalog::new(sandbox_image_manifest("sprites")),
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

    /// ⛔ THE ACCUMULATION THIS TYPE USED TO GUARANTEE, and the eviction that
    /// answers it.
    ///
    /// Before `retain_themes` existed, `handles` was private with `ensure_theme_loaded`
    /// as its only mutator — so a theme, once visited, was resident for the life
    /// of the process. Nine themes of four layers is the ceiling a walk reaches.
    /// This pins both halves: that visiting accumulates, and that eviction is
    /// now possible and exact.
    #[test]
    fn three_visited_themes_accumulate_and_retain_evicts_all_but_one() {
        let app = asset_app();
        let catalog = packaged_catalog();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let per_theme = ParallaxLayerAsset::ALL.len();

        let mut set = ParallaxLayerSet::default();
        assert!(set.is_empty(), "a fresh set holds nothing");

        for (visited, theme) in [ParallaxTheme::Hub, ParallaxTheme::Cave, ParallaxTheme::Lab]
            .into_iter()
            .enumerate()
        {
            set.ensure_theme_loaded(&catalog, &asset_server, theme, None);
            assert_eq!(
                set.len(),
                per_theme * (visited + 1),
                "visiting {theme:?} should add {per_theme} layers and retire nothing — \
                 the accumulation this test exists to describe",
            );
        }
        assert_eq!(
            set.resident_themes(),
            vec![ParallaxTheme::Hub, ParallaxTheme::Lab, ParallaxTheme::Cave],
            "resident_themes reports in ParallaxTheme::ALL order, not visit order",
        );

        // Re-visiting is idempotent: the count must not move.
        let before_revisit = set.len();
        set.ensure_theme_loaded(&catalog, &asset_server, ParallaxTheme::Hub, None);
        assert_eq!(set.len(), before_revisit, "re-visiting a theme reloads nothing");

        let dropped = set.retain_themes(|theme| theme == ParallaxTheme::Cave);
        assert_eq!(dropped, per_theme * 2, "two themes' worth of layers dropped");
        assert_eq!(set.len(), per_theme, "only the kept theme's layers remain");
        assert_eq!(set.resident_themes(), vec![ParallaxTheme::Cave]);
        assert!(
            set.get(ParallaxTheme::Hub, ParallaxLayerAsset::ALL[0]).is_none(),
            "an evicted theme must not answer `get`",
        );
        assert!(
            set.get(ParallaxTheme::Cave, ParallaxLayerAsset::ALL[0]).is_some(),
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
        set.ensure_theme_loaded(&catalog, &asset_server, ParallaxTheme::Boss, None);
        let before = set.len();
        assert!(before > 0, "non-vacuity: the boss theme loaded something");

        assert_eq!(set.retain_themes(|_| true), 0);
        assert_eq!(set.len(), before);
        assert_eq!(set.retain_themes(|_| false), before, "and the inverse clears it");
        assert!(set.is_empty());
    }
}
