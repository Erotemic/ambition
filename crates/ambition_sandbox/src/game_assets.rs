//! Game asset wiring with fallback-friendly loading.
//!
//! The sandbox loads two layers of art on top of its colored-rectangle
//! placeholders:
//!
//! - **Character spritesheets** (robot/goblin/boss) — animated, owned by
//!   `character_sprites`. Loaded here to keep all asset config in one place.
//! - **Entity sprites** — single-image placeholders for chests, pickups,
//!   breakables, hazards, NPCs, blocks, loading zones, and so on. Their
//!   identity/path is emitted by
//!   `python -m ambition_sprite2d_renderer draw-entities` as
//!   `entity_manifest.yaml` next to the generated PNGs.
//!
//! Two design rules:
//!
//! 1. **The game must always run.** Every asset is `Option<Handle<Image>>`;
//!    if a PNG is missing on disk the rendering layer falls back to its
//!    legacy colored rectangle. The `--no-assets` CLI flag forces every
//!    handle to `None` regardless of disk state, so designers can sanity-
//!    check the placeholder visuals at any moment.
//! 2. **Asset *source* is pluggable.** Today's loader walks the
//!    `assets/<sprite_folder>/` directory; a future loader can synthesize
//!    `Image` assets at runtime and insert them into `Assets<Image>` —
//!    callers only see `GameAssets` and don't care where handles came from.
//!    To make that swap painless we go through the high-level `GameAssets`
//!    struct rather than baking specific paths into call sites.

use ambition_engine as ae;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::boss_sprites::{self, BossSpriteAsset};
use crate::character_sprites::{self, CharacterSpriteAssets};
use crate::features::FeatureVisualKind;
use crate::rooms::LoadingZoneActivation;

/// CLI/runtime configuration for asset loading. Inserted as a Bevy resource
/// before the presentation startup system runs.
#[derive(Resource, Clone, Debug)]
pub struct GameAssetConfig {
    /// When true, skip every disk asset load and force colored-rectangle
    /// placeholders everywhere. Set via the `--no-assets` CLI flag.
    pub no_assets: bool,
    /// Directory under `assets/` that holds character + entity sprites.
    /// Default `"sprites"`. Lets designers point at experimental sets
    /// without recompiling.
    pub sprite_folder: String,
}

impl Default for GameAssetConfig {
    fn default() -> Self {
        Self {
            no_assets: false,
            sprite_folder: "sprites".into(),
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
                "--no-assets" => config.no_assets = true,
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

/// Single-frame entity sprites keyed off the gen2d manifest.
///
/// Every variant maps to `entities/<lower_snake_case>.png` under the
/// configured sprite folder. Adding a new entry here only requires a path
/// in `relative_path` — loading is data-driven.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntitySprite {
    // Features
    ChestClosed,
    ChestOpen,
    BreakableIntact,
    BreakableCracked,
    BreakableBroken,
    PickupHealth,
    PickupCurrency,
    PickupAbility,
    HazardSpikes,
    NpcTerminal,
    BossCore,
    SandbagDummy,
    // Blocks / surfaces
    SolidBlock,
    OneWayPlatform,
    SoftBlinkWall,
    HardBlinkWall,
    PogoOrb,
    ReboundPad,
    MovingPlatform,
    // Loading zones
    DoorZone,
    EdgeExit,
    // Player projectiles (Fireball + Hadouken share the same sprite)
    ProjectileEnergy,
    // 32×32 tile sprites for IntGrid-derived block surfaces. Rendered
    // via `Sprite::image_mode = Tiled` so they REPEAT across the
    // arbitrary aspect ratios that long floors / tall walls produce,
    // instead of stretching one sprite across the whole footprint.
    SolidTile,
    OneWayTile,
    HazardTile,
    SoftBlinkTile,
    HardBlinkTile,
    /// Visual for the encounter-driven "lock wall" block that
    /// `sync_lock_walls` inserts into `world.blocks` while an
    /// encounter is in flight. Uses a dedicated tile so it reads as
    /// "this just appeared" rather than blending into adjacent walls.
    LockWallTile,
}

impl EntitySprite {
    /// Path relative to the configured sprite folder.
    pub const fn relative_path(self) -> &'static str {
        match self {
            Self::ChestClosed => "entities/chest_closed.png",
            Self::ChestOpen => "entities/chest_open.png",
            Self::BreakableIntact => "entities/breakable_intact.png",
            Self::BreakableCracked => "entities/breakable_cracked.png",
            Self::BreakableBroken => "entities/breakable_broken.png",
            Self::PickupHealth => "entities/pickup_health.png",
            Self::PickupCurrency => "entities/pickup_currency.png",
            Self::PickupAbility => "entities/pickup_ability.png",
            Self::HazardSpikes => "entities/hazard_spikes.png",
            Self::NpcTerminal => "entities/npc_terminal.png",
            Self::BossCore => "entities/boss_core.png",
            Self::SandbagDummy => "entities/sandbag_dummy.png",
            Self::SolidBlock => "entities/solid_block.png",
            Self::OneWayPlatform => "entities/one_way_platform.png",
            Self::SoftBlinkWall => "entities/soft_blink_wall.png",
            Self::HardBlinkWall => "entities/hard_blink_wall.png",
            Self::PogoOrb => "entities/pogo_orb.png",
            Self::ReboundPad => "entities/rebound_pad.png",
            Self::MovingPlatform => "entities/moving_platform.png",
            Self::DoorZone => "entities/door_zone.png",
            Self::EdgeExit => "entities/edge_exit.png",
            Self::ProjectileEnergy => "entities/projectile_energy.png",
            Self::SolidTile => "entities/solid_tile.png",
            Self::OneWayTile => "entities/one_way_tile.png",
            Self::HazardTile => "entities/hazard_tile.png",
            Self::SoftBlinkTile => "entities/soft_blink_tile.png",
            Self::HardBlinkTile => "entities/hard_blink_tile.png",
            Self::LockWallTile => "entities/lock_wall_tile.png",
        }
    }

    pub const ALL: &'static [Self] = &[
        Self::ChestClosed,
        Self::ChestOpen,
        Self::BreakableIntact,
        Self::BreakableCracked,
        Self::BreakableBroken,
        Self::PickupHealth,
        Self::PickupCurrency,
        Self::PickupAbility,
        Self::HazardSpikes,
        Self::NpcTerminal,
        Self::BossCore,
        Self::SandbagDummy,
        Self::SolidBlock,
        Self::OneWayPlatform,
        Self::SoftBlinkWall,
        Self::HardBlinkWall,
        Self::PogoOrb,
        Self::ReboundPad,
        Self::MovingPlatform,
        Self::DoorZone,
        Self::EdgeExit,
        Self::ProjectileEnergy,
        Self::SolidTile,
        Self::OneWayTile,
        Self::HazardTile,
        Self::SoftBlinkTile,
        Self::HardBlinkTile,
        Self::LockWallTile,
    ];
}

/// Map from `EntitySprite` to its loaded `Handle<Image>`. Missing handles
/// (file absent on disk OR no-asset mode) simply aren't keyed, so callers
/// just consult `get(...) -> Option<&Handle<Image>>`.
#[derive(Default, Clone)]
pub struct EntitySpriteSet {
    handles: HashMap<EntitySprite, Handle<Image>>,
}

impl EntitySpriteSet {
    pub fn get(&self, key: EntitySprite) -> Option<&Handle<Image>> {
        self.handles.get(&key)
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
    /// Boss spritesheet — separate from `characters` because the boss
    /// generator emits its own animation rows
    /// (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/death) that
    /// don't fit `CharacterAnim`. `None` falls back to the static
    /// `EntitySprite::BossCore` image.
    pub boss: Option<BossSpriteAsset>,
    /// Mockingbird boss spritesheet — different generator (the
    /// standalone PIL-based mockingbird tool) with a 6-row layout
    /// (hover / thrust / bite / slash / hit / death). The rendering
    /// layer dispatches on the boss's authored name to pick this
    /// asset; other bosses fall back to `boss`.
    pub mockingbird: Option<BossSpriteAsset>,
}

/// Build a fresh `GameAssets` from disk, honoring `config`.
///
/// Always returns successfully — missing files fall through to `None`
/// handles. `config.no_assets == true` short-circuits to an empty
/// `GameAssets` so the visible sandbox always boots.
pub fn load_game_assets(
    config: &GameAssetConfig,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> GameAssets {
    if config.no_assets {
        eprintln!("[game_assets] --no-assets in effect: rendering with colored-rectangle placeholders only");
        return GameAssets::default();
    }

    let characters =
        character_sprites::load_character_sprites_in(asset_server, layouts, &config.sprite_folder);
    let entities = load_entity_sprites(asset_server, &config.sprite_folder);
    let boss = boss_sprites::load_boss_sprite_in(asset_server, layouts, &config.sprite_folder);
    let mockingbird =
        boss_sprites::load_mockingbird_sprite_in(asset_server, layouts, &config.sprite_folder);

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
        mockingbird,
    }
}

fn load_entity_sprites(asset_server: &AssetServer, sprite_folder: &str) -> EntitySpriteSet {
    let mut handles = HashMap::with_capacity(EntitySprite::ALL.len());
    for &key in EntitySprite::ALL {
        let rel = format!("{sprite_folder}/{}", key.relative_path());
        if asset_exists(&rel) {
            handles.insert(key, asset_server.load(rel));
        }
    }
    EntitySpriteSet { handles }
}

fn asset_exists(rel_path: &str) -> bool {
    // Android assets live inside the APK, not under the host-side
    // CARGO_MANIFEST_DIR. Let Bevy's Android asset reader try the load.
    #[cfg(target_os = "android")]
    {
        let _ = rel_path;
        true
    }

    // Desktop / Steam Deck bundles can run from a different path than the
    // Linux machine that built them. Check the same app-root layout Bevy uses
    // first, but tolerate both BEVY_ASSET_ROOT=<app> and
    // BEVY_ASSET_ROOT=<app>/assets while preserving local cargo-run fallback.
    #[cfg(not(target_os = "android"))]
    {
        desktop_asset_exists(rel_path)
    }
}

#[cfg(not(target_os = "android"))]
fn desktop_asset_exists(rel_path: &str) -> bool {
    let rel = std::path::Path::new(rel_path);
    let mut candidates = Vec::new();

    if let Some(root) = std::env::var_os("BEVY_ASSET_ROOT") {
        let root = std::path::PathBuf::from(root);
        // Preferred form: BEVY_ASSET_ROOT points at the app/project root,
        // and Bevy's file asset reader loads from root/assets/<rel>.
        candidates.push(root.join("assets").join(rel));
        // Tolerate launchers that set BEVY_ASSET_ROOT to the assets dir.
        candidates.push(root.join(rel));
    }

    if let Ok(cwd) = std::env::current_dir() {
        // Direct binary launches from the app dir.
        candidates.push(cwd.join("assets").join(rel));
        // Tolerate launches from the assets dir or compatibility symlinks.
        candidates.push(cwd.join(rel));
    }

    // Local cargo run / tests fallback.
    candidates.push(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel),
    );

    candidates.into_iter().any(|path| path.exists())
}

/// Build a `Sprite` for the given entity-sprite key, falling back to the
/// supplied colored-rectangle if the handle is missing. Render size always
/// equals `size`, so block/feature spawns can pass through their authored
/// AABB without rewriting.
pub fn entity_sprite(
    assets: &GameAssets,
    key: EntitySprite,
    size: Vec2,
    fallback_color: Color,
) -> Sprite {
    match assets.entities.get(key) {
        Some(handle) => {
            let mut sprite = Sprite::from_image(handle.clone());
            sprite.custom_size = Some(size);
            sprite
        }
        None => Sprite::from_color(fallback_color, size),
    }
}

/// Same as [`entity_sprite`] but `kind` is optional — `None` always falls
/// through to the colored rectangle. Useful for call sites that map a
/// runtime kind (e.g. `BlockKind`) to an `Option<EntitySprite>` because
/// some variants don't have a dedicated sprite.
pub fn entity_sprite_or_color(
    assets: &GameAssets,
    key: Option<EntitySprite>,
    size: Vec2,
    fallback_color: Color,
) -> Sprite {
    match key.and_then(|k| assets.entities.get(k)) {
        Some(handle) => {
            let mut sprite = Sprite::from_image(handle.clone());
            sprite.custom_size = Some(size);
            sprite
        }
        None => Sprite::from_color(fallback_color, size),
    }
}

/// Resolve the entity sprite to use at SPAWN time based on the engine's
/// authored room object. Stateless choice — the runtime sync system swaps
/// the sprite later for state-driven kinds (chest open, breakable cracked).
pub fn entity_sprite_for_room_object(kind: &ae::RoomObjectKind) -> Option<EntitySprite> {
    match kind {
        ae::RoomObjectKind::DamageVolume(_) => Some(EntitySprite::HazardSpikes),
        ae::RoomObjectKind::Pickup(p) => Some(pickup_sprite(&p.kind)),
        ae::RoomObjectKind::Chest(_) => Some(EntitySprite::ChestClosed),
        ae::RoomObjectKind::Breakable(_) => Some(EntitySprite::BreakableIntact),
        ae::RoomObjectKind::Interactable(interactable)
            if matches!(interactable.kind, ae::InteractionKind::Npc { .. }) =>
        {
            Some(EntitySprite::NpcTerminal)
        }
        ae::RoomObjectKind::EnemySpawn(ae::EnemyBrain::Custom(name))
            if name.starts_with("sandbag_") =>
        {
            Some(EntitySprite::SandbagDummy)
        }
        ae::RoomObjectKind::BossSpawn(_) => Some(EntitySprite::BossCore),
        // Enemies use the goblin spritesheet (animated), not a static
        // entity sprite — `upgrade_enemy_sprites` handles them.
        _ => None,
    }
}

fn pickup_sprite(kind: &ae::PickupKind) -> EntitySprite {
    match kind {
        ae::PickupKind::Health { .. } => EntitySprite::PickupHealth,
        ae::PickupKind::Currency { .. } => EntitySprite::PickupCurrency,
        ae::PickupKind::Ability { .. } => EntitySprite::PickupAbility,
        // StoryFlag and Custom fall back to the ability look until they
        // get dedicated art.
        _ => EntitySprite::PickupAbility,
    }
}

/// State-aware sprite for a breakable based on its current health state.
pub fn breakable_state_sprite(state: ae::BreakableState) -> EntitySprite {
    match state {
        ae::BreakableState::Intact => EntitySprite::BreakableIntact,
        ae::BreakableState::Cracking => EntitySprite::BreakableCracked,
        ae::BreakableState::Broken | ae::BreakableState::Respawning => {
            EntitySprite::BreakableBroken
        }
    }
}

/// State-aware sprite for a chest by opened-flag.
pub fn chest_state_sprite(opened: bool) -> EntitySprite {
    if opened {
        EntitySprite::ChestOpen
    } else {
        EntitySprite::ChestClosed
    }
}

/// Block-kind sprites. `BlockKind::Hazard` reuses the hazard-spikes art.
pub fn block_sprite(kind: ae::BlockKind) -> Option<EntitySprite> {
    match kind {
        ae::BlockKind::Solid => Some(EntitySprite::SolidBlock),
        ae::BlockKind::OneWay => Some(EntitySprite::OneWayPlatform),
        ae::BlockKind::Hazard => Some(EntitySprite::HazardSpikes),
        ae::BlockKind::PogoOrb => Some(EntitySprite::PogoOrb),
        ae::BlockKind::Rebound { .. } => Some(EntitySprite::ReboundPad),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        } => Some(EntitySprite::SoftBlinkWall),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard,
        } => Some(EntitySprite::HardBlinkWall),
    }
}

/// Tile-sprite variant of `block_sprite` for IntGrid-derived blocks.
/// Returns the seamless 32×32 tile texture that the renderer should
/// REPEAT (via `Sprite::image_mode = Tiled`) across the block's
/// arbitrary aspect ratio. Returns `None` for kinds that don't have
/// a tile generator yet (PogoOrb / Rebound — those are point-shaped
/// authored entities, not tiled surfaces).
pub fn block_tile_sprite(kind: ae::BlockKind) -> Option<EntitySprite> {
    match kind {
        ae::BlockKind::Solid => Some(EntitySprite::SolidTile),
        ae::BlockKind::OneWay => Some(EntitySprite::OneWayTile),
        ae::BlockKind::Hazard => Some(EntitySprite::HazardTile),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        } => Some(EntitySprite::SoftBlinkTile),
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Hard,
        } => Some(EntitySprite::HardBlinkTile),
        // PogoOrb / Rebound stay on the entity-art path because they
        // are point objects, not tiled surfaces. Authored as single
        // entities with fixed-aspect art.
        _ => None,
    }
}

/// Loading-zone sprites — cosmetic, the actual zone behavior comes from
/// the gameplay layer.
pub fn loading_zone_sprite(activation: LoadingZoneActivation) -> EntitySprite {
    match activation {
        LoadingZoneActivation::Door => EntitySprite::DoorZone,
        LoadingZoneActivation::EdgeExit => EntitySprite::EdgeExit,
    }
}

/// Map a `FeatureVisualKind` to a default entity sprite, ignoring per-
/// instance state. Used as a backstop when the engine kind isn't known
/// in detail (e.g. inside `sync_visuals`).
pub fn entity_sprite_for_kind(kind: FeatureVisualKind) -> Option<EntitySprite> {
    match kind {
        FeatureVisualKind::Hazard => Some(EntitySprite::HazardSpikes),
        FeatureVisualKind::Sandbag => Some(EntitySprite::SandbagDummy),
        FeatureVisualKind::Boss => Some(EntitySprite::BossCore),
        FeatureVisualKind::Breakable => Some(EntitySprite::BreakableIntact),
        FeatureVisualKind::Chest => Some(EntitySprite::ChestClosed),
        FeatureVisualKind::Pickup => Some(EntitySprite::PickupHealth),
        FeatureVisualKind::Npc => Some(EntitySprite::NpcTerminal),
        // Enemies are animated; rendering handles them through the
        // character spritesheet, not a static entity sprite.
        FeatureVisualKind::Enemy => None,
        // Switches render as a colored block (red / green) rather
        // than a static entity sprite — see `feature_color` and
        // `switch_on_color` in `rendering.rs`.
        FeatureVisualKind::Switch => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn entity_sprite_for_kind_handles_all_visual_kinds() {
        // Sanity: every FeatureVisualKind variant returns something
        // (either Some(sprite) or an explicit None for animated/dynamic
        // visuals). Ensures a new variant doesn't silently reach the
        // pattern-match catch-all and break the visual layer.
        for kind in [
            FeatureVisualKind::Hazard,
            FeatureVisualKind::Sandbag,
            FeatureVisualKind::Boss,
            FeatureVisualKind::Breakable,
            FeatureVisualKind::Chest,
            FeatureVisualKind::Pickup,
            FeatureVisualKind::Npc,
        ] {
            assert!(
                entity_sprite_for_kind(kind).is_some(),
                "static sprite expected for {kind:?}"
            );
        }
        assert!(entity_sprite_for_kind(FeatureVisualKind::Enemy).is_none());
        assert!(entity_sprite_for_kind(FeatureVisualKind::Switch).is_none());
    }
}
