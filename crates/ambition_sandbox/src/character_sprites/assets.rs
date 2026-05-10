//! Spritesheet asset bundle + on-disk loading.
//!
//! Each character target has its own PNG; missing files are not
//! errors — callers fall back to colored rectangles (the game must
//! always run regardless of asset state). Android assets live inside
//! the APK; desktop reads through `CARGO_MANIFEST_DIR`.

use std::collections::HashMap;

use bevy::prelude::*;

use super::sheets::{
    CharacterSheetSpec, ABSURD_GENERAL_SHEET, ARCHITECT_SHEET, GOBLIN_CANTINA_CHIEFTAIN_SHEET,
    GOBLIN_SHEET, KERNEL_GUIDE_SHEET, MERCHANT_PROTOTYPE_SHEET, PULSE_VOYAGER_CAPTAIN_SHEET,
    ROBOT_SHEET, SANDBAG_SHEET, TECH_BRO_DISRUPTOR_SHEET, VAULT_KEEPER_SHEET,
};
use crate::features::FeatureVisualKind;

#[derive(Clone)]
pub struct CharacterSpriteAsset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub spec: CharacterSheetSpec,
}

/// Holds optional spritesheet handles. A missing PNG produces a
/// `None` (or absent map entry); callers fall back to colored
/// rectangles.
#[derive(Resource, Default, Clone)]
pub struct CharacterSpriteAssets {
    pub robot: Option<CharacterSpriteAsset>,
    pub goblin: Option<CharacterSpriteAsset>,
    pub sandbag: Option<CharacterSpriteAsset>,
    /// Per-NPC sprite sheets keyed by the LDtk `NpcSpawn.name` field.
    /// Adding a new faction-leader or hub NPC means adding a row to
    /// `NPC_SPRITE_REGISTRY` below — no struct field churn or
    /// dispatcher match-arm needed. Once LDtk grows a `category`
    /// field on `NpcSpawn`, the key swaps from name to category.
    pub npcs: HashMap<&'static str, CharacterSpriteAsset>,
    // The boss uses the entity-sprite path (`EntitySprite::BossCore`) rather
    // than the character-spritesheet path: its generator emits non-standard
    // animation rows (rest/floor_slam/side_sweep/spike_halo/dash_echo/hit/
    // death) that don't fit `CharacterAnim`'s 8-variant grid. When/if the
    // boss gets a CharacterAnim-compatible sheet, add a `boss` field here.
}

impl CharacterSpriteAssets {
    pub fn enemy_asset(&self, kind: FeatureVisualKind) -> Option<&CharacterSpriteAsset> {
        match kind {
            FeatureVisualKind::Enemy => self.goblin.as_ref(),
            FeatureVisualKind::Sandbag => self.sandbag.as_ref().or(self.goblin.as_ref()),
            _ => None,
        }
    }

    /// Pick a character spritesheet for an NPC by its authored name.
    /// Returns `None` for NPCs that have no registered sprite —
    /// those keep the default `EntitySprite::NpcTerminal` rectangle.
    pub fn npc_asset_for_name(&self, name: &str) -> Option<&CharacterSpriteAsset> {
        self.npcs.get(name)
    }
}

const ROBOT_FILENAME: &str = "robot_spritesheet.png";
const GOBLIN_FILENAME: &str = "goblin_spritesheet.png";
const SANDBAG_FILENAME: &str = "sandbag_spritesheet.png";

/// Source-of-truth registry mapping `(LDtk NpcSpawn.name → asset
/// filename, sheet spec)`. Add a row here to wire a new NPC sprite;
/// `load_character_sprites_in` walks the table and inserts each
/// present sheet into `CharacterSpriteAssets::npcs`.
const NPC_SPRITE_REGISTRY: &[(&str, &str, CharacterSheetSpec)] = &[
    // Faction leaders.
    (
        "General",
        "absurd_general_spritesheet.png",
        ABSURD_GENERAL_SHEET,
    ),
    (
        "Fretjaw, Cantina Chieftain",
        "goblin_cantina_chieftain_spritesheet.png",
        GOBLIN_CANTINA_CHIEFTAIN_SHEET,
    ),
    (
        "Captain Pulse",
        "pulse_voyager_captain_spritesheet.png",
        PULSE_VOYAGER_CAPTAIN_SHEET,
    ),
    (
        "Chadwick Disruptor III",
        "tech_bro_disruptor_spritesheet.png",
        TECH_BRO_DISRUPTOR_SHEET,
    ),
    // Hub NPCs already authored in LDtk; we just point them at the
    // toon-target sheets rendered for them.
    ("Architect NPC", "architect_spritesheet.png", ARCHITECT_SHEET),
    (
        "Kernel Guide NPC",
        "kernel_guide_spritesheet.png",
        KERNEL_GUIDE_SHEET,
    ),
    (
        "Vault Keeper NPC",
        "vault_keeper_spritesheet.png",
        VAULT_KEEPER_SHEET,
    ),
    (
        "Merchant Prototype NPC",
        "merchant_prototype_spritesheet.png",
        MERCHANT_PROTOTYPE_SHEET,
    ),
];

/// Probe the sandbox `assets/<sprite_folder>/` directory for spritesheets.
/// Missing files are not an error — callers fall back to colored rectangles.
pub fn load_character_sprites_in(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
) -> CharacterSpriteAssets {
    let robot_rel = format!("{sprite_folder}/{ROBOT_FILENAME}");
    let goblin_rel = format!("{sprite_folder}/{GOBLIN_FILENAME}");
    let sandbag_rel = format!("{sprite_folder}/{SANDBAG_FILENAME}");

    let robot = build_optional(asset_server, layouts, &robot_rel, ROBOT_SHEET);
    let goblin = build_optional(asset_server, layouts, &goblin_rel, GOBLIN_SHEET);
    let sandbag = build_optional(asset_server, layouts, &sandbag_rel, SANDBAG_SHEET);

    for (label, rel, present) in [
        ("robot", &robot_rel, robot.is_some()),
        ("goblin", &goblin_rel, goblin.is_some()),
        ("sandbag", &sandbag_rel, sandbag.is_some()),
    ] {
        if !present {
            eprintln!(
                "[character_sprites] {label} spritesheet not found at assets/{rel} — falling back to colored rectangle"
            );
        }
    }

    let mut npcs: HashMap<&'static str, CharacterSpriteAsset> = HashMap::new();
    for (name, filename, spec) in NPC_SPRITE_REGISTRY {
        let rel = format!("{sprite_folder}/{filename}");
        if let Some(asset) = build_optional(asset_server, layouts, &rel, *spec) {
            npcs.insert(*name, asset);
        } else {
            eprintln!(
                "[character_sprites] NPC sheet '{name}' not found at assets/{rel} — falling back to colored rectangle"
            );
        }
    }

    CharacterSpriteAssets {
        robot,
        goblin,
        sandbag,
        npcs,
    }
}

fn build_optional(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    rel_path: &str,
    spec: CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    if !asset_exists(rel_path) {
        return None;
    }
    let layout = layouts.add(spec.build_atlas());
    Some(CharacterSpriteAsset {
        texture: asset_server.load(rel_path.to_string()),
        layout,
        spec,
    })
}

fn asset_exists(rel_path: &str) -> bool {
    // Android assets live inside the APK, not under the host-side
    // CARGO_MANIFEST_DIR. Let Bevy's Android asset reader try the load.
    #[cfg(target_os = "android")]
    {
        let _ = rel_path;
        true
    }

    // Bevy's FileAssetReader resolves assets relative to CARGO_MANIFEST_DIR
    // when running through cargo. Mirror that here so the existence check
    // matches the asset server's lookup path.
    #[cfg(not(target_os = "android"))]
    {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::Path::new(manifest_dir)
            .join("assets")
            .join(rel_path)
            .exists()
    }
}
