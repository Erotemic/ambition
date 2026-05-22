//! Spritesheet asset bundle + on-disk loading.
//!
//! Each character target has its own PNG; missing files are not errors
//! — callers fall back to colored rectangles (the game must always run
//! regardless of asset state). All path/existence policy goes through
//! [`crate::assets::sandbox_assets::SandboxAssetCatalog`]; this module no
//! longer owns any `target_os = "android"` cfg branches or
//! `BEVY_ASSET_ROOT` probes.

use std::collections::HashMap;

use bevy::prelude::*;

use ambition_asset_manager::AssetId;

use super::sheets::{
    CharacterSheetSpec, ABSURD_GENERAL_SHEET, ARCHITECT_SHEET, BURNING_FLYING_SHARK_SHEET,
    GOBLIN_CANTINA_CHIEFTAIN_SHEET, GOBLIN_SHEET, KERNEL_GUIDE_SHEET, MERCHANT_PROTOTYPE_SHEET,
    NINJA_SHEET, PIRATE_SHEET, PLAYER_ROBOT_SHEET, PULSE_VOYAGER_CAPTAIN_SHEET, ROBOT_SHEET,
    SANDBAG_SHEET, TECH_BRO_DISRUPTOR_SHEET, VAULT_KEEPER_SHEET,
};
use crate::assets::sandbox_assets::{ids, SandboxAssetCatalog};
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

pub(crate) const PLAYER_FILENAME: &str = "player_robot_spritesheet.png";
pub(crate) const ROBOT_FILENAME: &str = "robot_spritesheet.png";
pub(crate) const GOBLIN_FILENAME: &str = "goblin_spritesheet.png";
pub(crate) const SANDBAG_FILENAME: &str = "sandbag_spritesheet.png";

/// Sandbox-side label used in `sprite.character.<label>` catalog ids,
/// paired with the on-disk filename relative to the sprite folder.
/// The catalog aggregator (`crate::sandbox_assets`) walks this list
/// and the [`NPC_SPRITE_REGISTRY`] below to register one
/// `AssetEntry` per spritesheet.
const BASE_CHARACTER_FILENAMES: &[(&str, &str)] = &[
    ("player", PLAYER_FILENAME),
    ("robot", ROBOT_FILENAME),
    ("goblin", GOBLIN_FILENAME),
    ("sandbag", SANDBAG_FILENAME),
];

/// Source-of-truth registry mapping `(LDtk NpcSpawn.name → asset
/// filename, sheet spec)`. Add a row here to wire a new NPC sprite;
/// `load_character_sprites_in` walks the table and inserts each
/// present sheet into `CharacterSpriteAssets::npcs`.
const NPC_SPRITE_REGISTRY: &[(&str, &str, &'static std::sync::LazyLock<CharacterSheetSpec>)] = &[
    // Faction leaders.
    (
        "General",
        "absurd_general_spritesheet.png",
        &ABSURD_GENERAL_SHEET,
    ),
    (
        "Fretjaw, Cantina Chieftain",
        "goblin_cantina_chieftain_spritesheet.png",
        &GOBLIN_CANTINA_CHIEFTAIN_SHEET,
    ),
    (
        "Captain Pulse",
        "pulse_voyager_captain_spritesheet.png",
        &PULSE_VOYAGER_CAPTAIN_SHEET,
    ),
    (
        "Chadwick Disruptor III",
        "tech_bro_disruptor_spritesheet.png",
        &TECH_BRO_DISRUPTOR_SHEET,
    ),
    // Pirate-faction characters in the Pirate Cove. Same sheet layout
    // (idle/walk/slash/taunt/hurt/death) for both — see PIRATE_SHEET.
    (
        "Pirate Admiral",
        "pirate_admiral_spritesheet.png",
        &PIRATE_SHEET,
    ),
    (
        "Pirate Raider",
        "pirate_raider_spritesheet.png",
        &PIRATE_SHEET,
    ),
    // Third pirate variant — same silhouette family as Raider but a
    // distinctly darker skin tone (see `pirates/common.py::PALETTES`
    // entry for `pirate_quartermaster`). Quartermaster role keeps the
    // crew lineup readable (Admiral / Raider / Quartermaster) and
    // gives the cove a third base pirate for combat / dialogue tests.
    (
        "Pirate Quartermaster",
        "pirate_quartermaster_spritesheet.png",
        &PIRATE_SHEET,
    ),
    // Lady pirate variants — same PIRATE_SHEET 128×128 layout, six
    // animations. Visual gendering happens entirely in the toon-
    // target palette (no beard; warmer scarf / coat colors). See
    // `tools/ambition_sprite2d_renderer/.../pirates/common.py`
    // `pirate_lookout` (deep-brown skin) and `pirate_navigator`
    // (pale-warm skin) palette entries.
    (
        "Pirate Lookout",
        "pirate_lookout_spritesheet.png",
        &PIRATE_SHEET,
    ),
    (
        "Pirate Navigator",
        "pirate_navigator_spritesheet.png",
        &PIRATE_SHEET,
    ),
    // Burning Flying Shark — enemy mount used by the pirate sky
    // arena. Registered through the NPC sprite registry because the
    // current enemy-sprite resolver falls through to NPC sheets
    // first; this matches the pattern used by hostile-NPC migrations.
    (
        "Burning Flying Shark",
        "burning_flying_shark_spritesheet.png",
        &BURNING_FLYING_SHARK_SHEET,
    ),
    // Ninja-faction characters in the Shadow Dojo. Same sheet layout
    // (idle/walk/run/jump/fall/slash/hit/death/blink_out/blink_in/
    // dash) for both — see NINJA_SHEET.
    (
        "Shadow Oni Leader",
        "ninja_shadow_oni_leader_spritesheet.png",
        &NINJA_SHEET,
    ),
    (
        "Shadow Duelist",
        "ninja_shadow_duelist_spritesheet.png",
        &NINJA_SHEET,
    ),
    // Hub NPCs already authored in LDtk; we just point them at the
    // toon-target sheets rendered for them.
    (
        "Architect NPC",
        "architect_spritesheet.png",
        &ARCHITECT_SHEET,
    ),
    (
        "Kernel Guide NPC",
        "kernel_guide_spritesheet.png",
        &KERNEL_GUIDE_SHEET,
    ),
    (
        "Vault Keeper NPC",
        "vault_keeper_spritesheet.png",
        &VAULT_KEEPER_SHEET,
    ),
    (
        "Merchant Prototype NPC",
        "merchant_prototype_spritesheet.png",
        &MERCHANT_PROTOTYPE_SHEET,
    ),
];

/// Sandbox-side label + filename for every character spritesheet the
/// sandbox knows about — base (player / robot / goblin / sandbag) +
/// every NPC sheet in [`NPC_SPRITE_REGISTRY`]. The sandbox-assets
/// aggregator walks this so adding a new character row in either
/// table auto-registers its catalog id.
pub fn all_character_sprite_filenames() -> Vec<(&'static str, &'static str)> {
    let mut out: Vec<(&'static str, &'static str)> = BASE_CHARACTER_FILENAMES.to_vec();
    for (name, filename, _spec) in NPC_SPRITE_REGISTRY {
        out.push((npc_sprite_label(name), *filename));
    }
    out
}

/// Convert an LDtk NPC name (e.g. `"Pirate Raider"`) into a stable
/// `sprite.character.<lower_snake>` label. The mapping is reversible
/// for the current NPC roster — the caller passes the same name when
/// resolving via the catalog at load time.
pub fn npc_sprite_label(npc_name: &str) -> &'static str {
    // Static lookup table so we hand back `&'static str` values that
    // can be stored in the catalog id without allocations. Adding a
    // new NPC entails one row here AND one row in [`NPC_SPRITE_REGISTRY`].
    match npc_name {
        "General" => "npc_general",
        "Fretjaw, Cantina Chieftain" => "npc_goblin_cantina_chieftain",
        "Captain Pulse" => "npc_pulse_voyager_captain",
        "Chadwick Disruptor III" => "npc_tech_bro_disruptor",
        "Pirate Admiral" => "npc_pirate_admiral",
        "Pirate Raider" => "npc_pirate_raider",
        "Pirate Quartermaster" => "npc_pirate_quartermaster",
        "Pirate Lookout" => "npc_pirate_lookout",
        "Pirate Navigator" => "npc_pirate_navigator",
        "Burning Flying Shark" => "npc_burning_flying_shark",
        "Shadow Oni Leader" => "npc_ninja_shadow_oni_leader",
        "Shadow Duelist" => "npc_ninja_shadow_duelist",
        "Architect NPC" => "npc_architect",
        "Kernel Guide NPC" => "npc_kernel_guide",
        "Vault Keeper NPC" => "npc_vault_keeper",
        "Merchant Prototype NPC" => "npc_merchant_prototype",
        // Story-content plugins (e.g. intro) author their own NPC
        // sprites by calling [`build_npc_sprite_asset`] directly; the
        // generic-id branch is only reached when the catalog doesn't
        // have a label registered for `npc_name`.
        _ => "npc_unregistered",
    }
}

/// Probe the sandbox `assets/<sprite_folder>/` directory for spritesheets.
///
/// Resolves each filename through [`SandboxAssetCatalog::path_for`] and
/// gates the load on
/// [`SandboxAssetCatalog::should_attempt_optional_load`]. Missing files
/// produce `None` — callers fall back to colored rectangles.
pub fn load_character_sprites_in(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> CharacterSpriteAssets {
    let player = build_optional_via_catalog(
        catalog,
        asset_server,
        layouts,
        &ids::character_sprite("player"),
        &PLAYER_ROBOT_SHEET,
        Some("player"),
    );
    let robot = build_optional_via_catalog(
        catalog,
        asset_server,
        layouts,
        &ids::character_sprite("robot"),
        &ROBOT_SHEET,
        Some("robot"),
    );
    let goblin = build_optional_via_catalog(
        catalog,
        asset_server,
        layouts,
        &ids::character_sprite("goblin"),
        &GOBLIN_SHEET,
        Some("goblin"),
    );
    let sandbag = build_optional_via_catalog(
        catalog,
        asset_server,
        layouts,
        &ids::character_sprite("sandbag"),
        &SANDBAG_SHEET,
        Some("sandbag"),
    );

    let mut npcs: HashMap<&'static str, CharacterSpriteAsset> = HashMap::new();
    for (name, _filename, spec) in NPC_SPRITE_REGISTRY {
        let label = npc_sprite_label(name);
        let id = ids::character_sprite(label);
        if let Some(asset) =
            build_optional_via_catalog(catalog, asset_server, layouts, &id, *spec, Some(name))
        {
            npcs.insert(*name, asset);
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

/// Resolve the catalog id, gate on profile policy via
/// `try_path_for_load`, and call `asset_server.load(...)` if the gate
/// passes. Logs a single line to `stderr` when a labeled sprite is
/// missing (matches the prior loader's noise level).
fn build_optional_via_catalog(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
    log_label: Option<&str>,
) -> Option<CharacterSpriteAsset> {
    let Some(path) = catalog.try_path_for_load(id) else {
        if let Some(label) = log_label {
            eprintln!(
                "[character_sprites] {label} spritesheet missing under {} profile (id {id}) — falling back to colored rectangle",
                catalog.profile().label(),
            );
        }
        return None;
    };
    let layout = layouts.add(spec.build_atlas());
    Some(CharacterSpriteAsset {
        texture: asset_server.load(path),
        layout,
        spec: spec.clone(),
    })
}

/// Build a single NPC sprite asset by resolving its catalog id.
/// Story-content plugins (e.g. `crate::intro::plugin`) call this once
/// per row in their authored NPC table; the matching catalog entries
/// are registered by `crate::assets::sandbox_assets::extend_with_intro_sprite_entries`
/// (or the equivalent helper for new plugins).
///
/// Returns `None` when the catalog reports the asset disabled / not
/// loadable under the active profile — callers fall back to colored
/// rectangles.
pub fn build_npc_sprite_asset(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None)
}

/// Build a single Prop sprite asset. Same shape as
/// [`build_npc_sprite_asset`] — kept as a separate name so story-
/// content plugins reading from `INTRO_PROP_REGISTRY` (or future
/// equivalents) clearly distinguish prop-table inserts from NPC-table
/// inserts.
pub fn build_prop_sprite_asset(
    catalog: &SandboxAssetCatalog,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    id: &AssetId,
    spec: &CharacterSheetSpec,
) -> Option<CharacterSpriteAsset> {
    build_optional_via_catalog(catalog, asset_server, layouts, id, spec, None)
}
