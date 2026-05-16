//! Spritesheet asset bundle + on-disk loading.
//!
//! Each character target has its own PNG; missing files are not
//! errors — callers fall back to colored rectangles (the game must
//! always run regardless of asset state). Android assets live inside
//! the APK; desktop probes the runtime asset root first and falls back
//! to the Cargo manifest asset directory for local development.

use std::collections::HashMap;

use bevy::prelude::*;

use super::sheets::{
    CharacterSheetSpec, ABSURD_GENERAL_SHEET, ARCHITECT_SHEET, GOBLIN_CANTINA_CHIEFTAIN_SHEET,
    GOBLIN_SHEET, KERNEL_GUIDE_SHEET, MERCHANT_PROTOTYPE_SHEET, NINJA_SHEET, PIRATE_SHEET,
    PLAYER_ROBOT_SHEET, PULSE_VOYAGER_CAPTAIN_SHEET, ROBOT_SHEET, SANDBAG_SHEET,
    TECH_BRO_DISRUPTOR_SHEET, VAULT_KEEPER_SHEET,
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
    /// Player-specific compact robot sheet. Preferred for the player
    /// entity; `setup.rs` falls back to `robot` when this is missing
    /// so debug builds without the regenerated sheet still render.
    pub player: Option<CharacterSpriteAsset>,
    /// Base "cute scout" robot sheet. Kept around for future robot-target
    /// callers that want the original proportions; the player itself now
    /// uses `player` above.
    pub robot: Option<CharacterSpriteAsset>,
    pub goblin: Option<CharacterSpriteAsset>,
    pub sandbag: Option<CharacterSpriteAsset>,
    /// Per-NPC sprite sheets keyed by the LDtk `NpcSpawn.name` field.
    /// Adding a new faction-leader or hub NPC means adding a row to
    /// `NPC_SPRITE_REGISTRY` below — no struct field churn or
    /// dispatcher match-arm needed. Once LDtk grows a `category`
    /// field on `NpcSpawn`, the key swaps from name to category.
    pub npcs: HashMap<&'static str, CharacterSpriteAsset>,
    /// Per-prop sprite sheets keyed by the LDtk `Prop.kind` field
    /// (e.g. `intro_cart`, `lab_genesis_vat`, `gate_ring`,
    /// `gate_portal`). Story-content plugins extend this via
    /// `build_prop_sprite_asset` — the sandbox itself doesn't ship
    /// any props in its base registry. Keying by `kind` (rather than
    /// display `name`) means an author can rename a prop in LDtk
    /// without re-pointing the sprite registry.
    pub props: HashMap<String, CharacterSpriteAsset>,
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

    /// Pick a prop spritesheet by its `Prop.kind` registry key.
    /// Returns `None` for kinds that have no registered sheet — the
    /// prop renderer falls back to a colored placeholder rectangle.
    pub fn prop_asset_for_kind(&self, kind: &str) -> Option<&CharacterSpriteAsset> {
        self.props.get(kind)
    }
}

const PLAYER_FILENAME: &str = "player_robot_spritesheet.png";
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
    // Pirate-faction characters in the Pirate Cove. Same sheet layout
    // (idle/walk/slash/taunt/hurt/death) for both — see PIRATE_SHEET.
    (
        "Pirate Admiral",
        "pirate_admiral_spritesheet.png",
        PIRATE_SHEET,
    ),
    (
        "Pirate Raider",
        "pirate_raider_spritesheet.png",
        PIRATE_SHEET,
    ),
    // Ninja-faction characters in the Shadow Dojo. Same sheet layout
    // (idle/walk/run/jump/fall/slash/hit/death/blink_out/blink_in/
    // dash) for both — see NINJA_SHEET.
    (
        "Shadow Oni Leader",
        "ninja_shadow_oni_leader_spritesheet.png",
        NINJA_SHEET,
    ),
    (
        "Shadow Duelist",
        "ninja_shadow_duelist_spritesheet.png",
        NINJA_SHEET,
    ),
    // Hub NPCs already authored in LDtk; we just point them at the
    // toon-target sheets rendered for them.
    (
        "Architect NPC",
        "architect_spritesheet.png",
        ARCHITECT_SHEET,
    ),
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
    let player_rel = format!("{sprite_folder}/{PLAYER_FILENAME}");
    let robot_rel = format!("{sprite_folder}/{ROBOT_FILENAME}");
    let goblin_rel = format!("{sprite_folder}/{GOBLIN_FILENAME}");
    let sandbag_rel = format!("{sprite_folder}/{SANDBAG_FILENAME}");

    let player = build_optional(asset_server, layouts, &player_rel, PLAYER_ROBOT_SHEET);
    let robot = build_optional(asset_server, layouts, &robot_rel, ROBOT_SHEET);
    let goblin = build_optional(asset_server, layouts, &goblin_rel, GOBLIN_SHEET);
    let sandbag = build_optional(asset_server, layouts, &sandbag_rel, SANDBAG_SHEET);

    for (label, rel, present) in [
        ("player", &player_rel, player.is_some()),
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
        player,
        robot,
        goblin,
        sandbag,
        npcs,
        props: HashMap::new(),
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

/// Build a single NPC sprite asset from a filename + sheet spec, using
/// the same path-resolution + missing-PNG fallback as
/// [`load_character_sprites_in`]. Story-content plugins (e.g.
/// `crate::intro::sprites`) call this in a startup system after
/// `GameAssets` is inserted, so they can extend
/// `CharacterSpriteAssets::npcs` without touching the sandbox sprite
/// registry constant.
pub fn build_npc_sprite_asset(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
    filename: &str,
    spec: CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    let rel = format!("{sprite_folder}/{filename}");
    build_optional(asset_server, layouts, &rel, spec)
}

/// Build a single Prop sprite asset. Same shape as
/// [`build_npc_sprite_asset`] — kept as a separate name so story-
/// content plugins reading from `INTRO_PROP_REGISTRY` (or future
/// equivalents) clearly distinguish prop-table inserts from NPC-table
/// inserts.
pub fn build_prop_sprite_asset(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    sprite_folder: &str,
    filename: &str,
    spec: CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    let rel = format!("{sprite_folder}/{filename}");
    build_optional(asset_server, layouts, &rel, spec)
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
