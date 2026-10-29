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

/// Biome/theme key for generated parallax layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParallaxTheme {
    Hub,
    Lab,
    /// The lab with its power out and its alarm on: the rooms of the raid.
    /// The same hall as [`Self::Lab`], dark, in the red of its beacons.
    Alarm,
    /// The drains and the pipes under the town: old brick, moss, and water
    /// that gives a green light. The relay rooms and the drains name it.
    Undertown,
    Basement,
    Cove,
    Skybridge,
    Boss,
    Eclipse,
    Water,
    Forest,
    /// The land of the dinosaurs: a jungle of palms and tree ferns under a
    /// volcano that smokes.
    Primeval,
    /// The house of the Flying Spaghetti Monster: all of it pasta, under a
    /// colander that lets the light through its holes. Columns of rigatoni,
    /// curtains of noodles, hills of meatballs in steam. The rooms of the
    /// Noodly Temple name it.
    Noodly,
    /// The central town of the Math dimension: limestone and whitewash on
    /// terraces under a clear morning sky, with faceted mountains that are
    /// polyhedra in the haze and a few small solids floating in the air.
    /// The rooms of the Math dimension (`math.ldtk`) name it.
    MathTown,
    /// The gate hall under the Math town: a vaulted undercroft of limestone,
    /// lamplit, with the day coming down through the grates of the street.
    MathUndercroft,
    /// Inside the houses of the Math town: whitewashed plaster in lamplight,
    /// a wainscot of hexagonal tiles, timber under the ceiling, a floor of
    /// terracotta tiles. Each room's furniture is its own props.
    MathInterior,
    /// Inside a deck of the orbital station of the Science dimension: steel
    /// plates between hull ribs, long viewports with the stars and the curve
    /// of the planet in them, lockers and cable trays in front.
    Orbital,
    /// Out in the open trusses of the orbital station and its gravity works:
    /// the planet below, the station's far spar with its solar wings, a
    /// hauler going out, and the near lattice of the truss.
    OrbitalTruss,
    /// The dark forest of the Evil dimension: black thorned trees under a
    /// blood-red sky, and far off Castle Godwin on its crag, its searchlights
    /// in the ash cloud. The rooms of `evil.ldtk` outside the castle name it.
    EvilForest,
    /// Inside Castle Godwin: halls of cold blue-grey blocks, red banners with
    /// the mark of the regime, braziers.
    CastleGodwin,
    /// The lab of Castle Godwin: vats lit green with failed prototypes in
    /// them, cables and pipes, Tesla coils, a slot in the roof to the storm.
    CastleGodwinLab,
    /// Wolfenstein's throne room: pointed arches, a great round window with a
    /// red moon, a carved wolf's head, red curtains, iron chandeliers.
    CastleGodwinThrone,
    /// The port of the pirate dimension in the late afternoon: a sea with a
    /// lighthouse on a headland, a skyship over it, ships at the quay, and a
    /// hill town of timber and plaster with red roofs. The rooms of the port
    /// (`pirate_port.ldtk`) name it.
    PiratePort,
    /// The village of the ninja dimension at dusk: blue-green mountains in
    /// mist, farmhouses with tiled roofs, a pagoda, red maples and paper
    /// lanterns. The rooms of the village (`ninja_village.ldtk`) name it.
    NinjaVillage,
    /// Inside the General's tower (the Iron Bastion): a shaft of poured
    /// concrete and riveted steel, girders, pipes, caged lamps and the
    /// faction's olive banners. The room `military_tower` names it.
    MilitaryTower,
    /// Mary-O's land in the colours of a magical girl: round hills, a far
    /// castle, a rainbow and a crescent moon in a pink sky. The rooms of
    /// Mary-O name it.
    Starlight,
    /// The desert of Mary-O's second world, in the morning: a whole sun, low,
    /// pyramids in steps a long way off, dunes, and thick palms. The desert
    /// rooms of Mary-O name it.
    Stardune,
    /// The cavern under Mary-O's land, in the colours of a dark magical
    /// girl: a moon eaten to a ring in a rift, arches, the wheels of a clock
    /// that does not turn. The underground room of Mary-O names it.
    Starless,
    /// The castle at the end of Mary-O's land: a hall of plum stone, coloured
    /// glass and red banners over fire, with the void in its vault. The
    /// castle room of Mary-O names it.
    VoidCastle,
    Cave,
    /// An open daylight sky of cloud banks and wind streaks: the Mockingbird's
    /// air chase, drawn wrapping as it scrolls (`sky_scroll`).
    OpenSky,
    /// The sky of the two-state room look in its clean state: a pale city on
    /// drawing paper (`room_look_sky.py` in the parallax renderer). A room
    /// with that look names it as its `parallax_theme`.
    HubClean,
    /// The same room in its corrupted state: the city rebuilt in blocks, and
    /// the stone of its blocks rebuilt in voxels. No room names it. Its sky
    /// is drawn over [`Self::HubClean`]'s where the air of the room is
    /// corrupted, and its skin is laid on the same blocks as the clean skin
    /// (`terrain/two_state.py` in the sprite renderer).
    HubCorrupt,
}

impl ParallaxTheme {
    /// The theme of the same sky in its corrupted state, for a theme that has
    /// one. A room that names a theme loads that one with it, and keeps it for
    /// as long: the look of the room draws it, and no room names it.
    pub const fn corrupted(self) -> Option<Self> {
        match self {
            Self::HubClean => Some(Self::HubCorrupt),
            _ => None,
        }
    }

    pub const ALL: &'static [Self] = &[
        Self::Hub,
        Self::Lab,
        Self::Alarm,
        Self::Undertown,
        Self::Basement,
        Self::Cove,
        Self::Skybridge,
        Self::Boss,
        Self::Eclipse,
        Self::Water,
        Self::Forest,
        Self::Primeval,
        Self::Noodly,
        Self::MathTown,
        Self::MathUndercroft,
        Self::MathInterior,
        Self::Orbital,
        Self::OrbitalTruss,
        Self::EvilForest,
        Self::CastleGodwin,
        Self::CastleGodwinLab,
        Self::CastleGodwinThrone,
        Self::PiratePort,
        Self::NinjaVillage,
        Self::MilitaryTower,
        Self::Starlight,
        Self::Stardune,
        Self::Starless,
        Self::VoidCastle,
        Self::Cave,
        Self::OpenSky,
        Self::HubClean,
        Self::HubCorrupt,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Hub => "hub",
            Self::Lab => "lab",
            Self::Alarm => "alarm",
            Self::Undertown => "undertown",
            Self::Basement => "basement",
            Self::Cove => "cove",
            Self::Skybridge => "skybridge",
            Self::Boss => "boss",
            Self::Eclipse => "eclipse",
            Self::Water => "water",
            Self::Forest => "forest",
            Self::Primeval => "primeval",
            Self::Noodly => "noodly",
            Self::MathTown => "math_town",
            Self::MathUndercroft => "math_undercroft",
            Self::MathInterior => "math_interior",
            Self::Orbital => "orbital",
            Self::OrbitalTruss => "orbital_truss",
            Self::EvilForest => "evil_forest",
            Self::CastleGodwin => "castle_godwin",
            Self::CastleGodwinLab => "castle_godwin_lab",
            Self::CastleGodwinThrone => "castle_godwin_throne",
            Self::PiratePort => "pirate_port",
            Self::NinjaVillage => "ninja_village",
            Self::MilitaryTower => "military_tower",
            Self::Starlight => "starlight",
            Self::Stardune => "stardune",
            Self::Starless => "starless",
            Self::VoidCastle => "void_castle",
            Self::Cave => "cave",
            Self::OpenSky => "open_sky",
            Self::HubClean => "hub_clean",
            Self::HubCorrupt => "hub_corrupt",
        }
    }

    /// The theme of a room: the one its metadata names, or the hub theme
    /// when it names none.
    pub fn from_room_metadata(metadata: &RoomMetadata) -> Self {
        Self::named_by_room_metadata(metadata).unwrap_or(Self::Hub)
    }

    /// The theme the metadata of a room names, if it names one.
    ///
    /// [`Self::from_room_metadata`] gives a room that names none the hub
    /// theme, which is right for a sky: a room must have one. It is wrong for
    /// art that goes on the blocks of the room (a terrain skin): a room of
    /// another game that names no theme must keep its own block art.
    pub fn named_by_room_metadata(metadata: &RoomMetadata) -> Option<Self> {
        // Explicit room visual profiles are the preferred authoring seam for
        // real rooms. `visual_profile` is a stable id, while `parallax_theme`
        // chooses the generated art stack directly.
        if let Some(theme) = metadata
            .visual_profile
            .parallax_theme
            .as_deref()
            .and_then(Self::from_key)
        {
            return Some(theme);
        }
        if let Some(theme) = metadata
            .visual_profile
            .id
            .as_deref()
            .and_then(Self::from_key)
        {
            return Some(theme);
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
                return Some(Self::Forest);
            }
        }
        if let Some(theme) = metadata.biome.as_deref().and_then(Self::from_key) {
            return Some(theme);
        }
        if let Some(theme) = metadata.visual_theme.as_deref().and_then(Self::from_key) {
            return Some(theme);
        }
        if let Some(theme) = metadata.ambient_profile.as_deref().and_then(Self::from_key) {
            return Some(theme);
        }
        None
    }

    /// The theme a key names (`"starlight"`, `"open_sky"`, and the other
    /// names authored content uses), if any.
    pub fn from_key(value: &str) -> Option<Self> {
        let key = value.trim().to_ascii_lowercase().replace('-', "_");
        match key.as_str() {
            "hub" | "default" | "cantina" | "orange" => Some(Self::Hub),
            "lab" | "laboratory" | "teal" => Some(Self::Lab),
            "alarm" | "raid" => Some(Self::Alarm),
            "undertown" | "under_town" | "drain" | "sewer" => Some(Self::Undertown),
            "basement" | "ruins" | "pink" => Some(Self::Basement),
            "cove" | "coast" | "beach" => Some(Self::Cove),
            "skybridge" | "sky" | "blue" => Some(Self::Skybridge),
            "boss" | "mob_arena" | "arena" => Some(Self::Boss),
            "eclipse" => Some(Self::Eclipse),
            "water" | "underwater" | "tide" => Some(Self::Water),
            "forest" | "woods" | "grove" | "bamboo" | "dojo" | "ninja" | "ninja_dojo" => {
                Some(Self::Forest)
            }
            "primeval" | "jungle" | "dinosaur" | "prehistoric" => Some(Self::Primeval),
            "noodly" | "noodle" | "pasta" => Some(Self::Noodly),
            "math_town" | "math" | "math_dimension" => Some(Self::MathTown),
            "math_undercroft" | "math_gate_hall" => Some(Self::MathUndercroft),
            "math_interior" => Some(Self::MathInterior),
            "orbital" | "orbital_station" | "space_station" => Some(Self::Orbital),
            "orbital_truss" | "space" => Some(Self::OrbitalTruss),
            "evil_forest" | "evil" => Some(Self::EvilForest),
            "castle_godwin" => Some(Self::CastleGodwin),
            "castle_godwin_lab" => Some(Self::CastleGodwinLab),
            "castle_godwin_throne" => Some(Self::CastleGodwinThrone),
            "pirate_port" | "port" | "harbour" | "harbor" => Some(Self::PiratePort),
            "ninja_village" | "village" => Some(Self::NinjaVillage),
            "military_tower" | "iron_bastion" => Some(Self::MilitaryTower),
            "starlight" | "magical_girl" | "magical" => Some(Self::Starlight),
            "stardune" | "star_dunes" => Some(Self::Stardune),
            "starless" | "dark_magical_girl" => Some(Self::Starless),
            "void_castle" | "castle" => Some(Self::VoidCastle),
            "cave" | "damp" => Some(Self::Cave),
            "open_sky" | "clouds" | "air_chase" => Some(Self::OpenSky),
            "hub_clean" => Some(Self::HubClean),
            "hub_corrupt" => Some(Self::HubCorrupt),
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
    let theme = ParallaxTheme::from_room_metadata(metadata);
    let mut added = assets
        .parallax_layers
        .ensure_theme_loaded(content, catalog, asset_server, theme, quality);
    if let Some(corrupted) = theme.corrupted() {
        added += assets
            .parallax_layers
            .ensure_theme_loaded(content, catalog, asset_server, corrupted, quality);
    }
    // The skin of the room's blocks comes with its sky, and is kept as long.
    // A room that names no theme has no skin: see `named_by_room_metadata`.
    if let Some(named) = ParallaxTheme::named_by_room_metadata(metadata) {
        assets.room_dressing.ensure_theme_loaded(content, catalog, asset_server, named);
        // A theme with two states lays both skins on its blocks.
        if let Some(corrupted) = named.corrupted() {
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
                for theme in ParallaxTheme::ALL {
                    for layer in ParallaxLayerAsset::ALL {
                        let id = parallax_layer_asset_id(*theme, *layer);
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

        for (visited, theme) in [ParallaxTheme::Hub, ParallaxTheme::Cave, ParallaxTheme::Lab]
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
            vec![ParallaxTheme::Hub, ParallaxTheme::Lab, ParallaxTheme::Cave],
            "resident_themes reports in ParallaxTheme::ALL order, not visit order",
        );

        // Re-visiting is idempotent: the count must not move.
        let before_revisit = set.len();
        set.ensure_theme_loaded(&crate::SpriteContent::default(), &catalog, &asset_server, ParallaxTheme::Hub, None);
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
        set.ensure_theme_loaded(&crate::SpriteContent::default(), &catalog, &asset_server, ParallaxTheme::Boss, None);
        let before = set.len();
        assert!(before > 0, "non-vacuity: the boss theme loaded something");

        assert_eq!(set.retain_themes(|_| true), 0);
        assert_eq!(set.len(), before);
        assert_eq!(set.retain_themes(|_| false), before, "and the inverse clears it");
        assert!(set.is_empty());
    }
}
