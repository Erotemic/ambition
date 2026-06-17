//! Intro NPC sprite placeholders.
//!
//! Until proper character sheets exist for Creator / Oiler / Gate Janitor /
//! Lab Raider / Manifest Clerk, intro NPCs reuse the toon-target
//! spritesheets that already ship with the sandbox. The doc's placeholder
//! mapping (`Story handoff` § "Recommended placeholder mapping") drives
//! the picks here; rows are keyed by exact `NpcSpawn.name` from
//! `intro.ldtk`.
//!
//! Add a row by appending a tuple to [`INTRO_NPC_SPRITE_REGISTRY`] —
//! [`crate::intro::plugin::load_intro_npc_sprites_system`] walks the
//! table at startup and inserts every present sheet into
//! `GameAssets.characters.npcs`. Missing PNGs fall back to colored
//! rectangles per the existing contract.

use ambition_asset_manager::AssetId;

use ambition_gameplay_core::character_sprites::{
    sheet_for_character_id, try_load_spec_for_target, CharacterSheetSpec, SheetTuning,
};

/// Default toon-NPC tuning the old intro `*_SHEET` statics carried.
const INTRO_NPC_TUNING: SheetTuning = SheetTuning::new(1.10, 2);

/// Resolve a content-owned sheet spec by manifest target, with intro
/// tuning. Panics in tests via the registry checks; at runtime a
/// missing manifest falls back to the colored-rectangle contract by
/// the caller skipping the row.
fn intro_sheet(target: &str, tuning: &SheetTuning) -> Option<CharacterSheetSpec> {
    try_load_spec_for_target(target, tuning)
}

/// `(LDtk NpcSpawn.name, asset filename, sheet spec)` rows for the
/// intro NPCs, resolved from the generated sheet manifests + the
/// intro's own tuning (Stage 20 / B3: the named `*_SHEET` statics in
/// the machinery lib are gone; story content owns its named sheets).
pub fn intro_npc_sprite_rows() -> Vec<(&'static str, &'static str, CharacterSheetSpec)> {
    let t = &INTRO_NPC_TUNING;
    let mut rows: Vec<(&str, &str, Option<CharacterSheetSpec>)> = vec![
        // Wake-room creator + raid-corridor variant (same sheet so the
        // player recognizes the silhouette dying mid-sentence).
        (
            "Creator",
            "creator_spritesheet.png",
            intro_sheet("creator", t),
        ),
        (
            "Creator Final",
            "creator_spritesheet.png",
            intro_sheet("creator", t),
        ),
        // Oiler: street mechanic (dedicated toon-adapter sheet).
        ("Oiler", "oiler_spritesheet.png", intro_sheet("oiler", t)),
        // Gate Janitor: Kernel Guide placeholder until a dedicated
        // janitor sheet lands.
        (
            "Gate Janitor",
            "kernel_guide_spritesheet.png",
            sheet_for_character_id("npc_kernel_guide"),
        ),
        // Erdish: pre-registered for later LDtk authoring.
        ("Erdish", "erdish_spritesheet.png", intro_sheet("erdish", t)),
        // Lab Raider / Salvage Guard: goblin sheet while the act-1
        // faction identity settles.
        (
            "Lab Raider",
            "goblin_spritesheet.png",
            sheet_for_character_id("goblin"),
        ),
        (
            "Salvage Guard",
            "goblin_spritesheet.png",
            sheet_for_character_id("goblin"),
        ),
        // Manifest clerk: architect sheet reads as "person at a podium".
        (
            "Manifest Clerk",
            "architect_spritesheet.png",
            sheet_for_character_id("npc_architect"),
        ),
        // News board: wall-mounted bulletin board rendered through the
        // NpcSpawn path.
        (
            "News Board",
            "news_board_spritesheet.png",
            intro_sheet("news_board", &SheetTuning::new(1.50, 2)),
        ),
        // Alice + Bob — the cartographer pair, dedicated sheets.
        ("Alice", "alice_spritesheet.png", intro_sheet("alice", t)),
        ("Bob", "bob_spritesheet.png", intro_sheet("bob", t)),
    ];
    rows.drain(..)
        .filter_map(|(name, file, spec)| spec.map(|s| (name, file, s)))
        .collect()
}

/// Prop tuning: props render at their authored AABB size.
const PROP_TUNING: SheetTuning = SheetTuning::new(1.00, 2);

/// `(Prop.kind, asset filename, sheet spec)` rows for intro props
/// (keyed by `Prop.kind` so LDtk renames don't re-point sprites).
/// Includes the cut-rope arena props until a dedicated non-intro prop
/// catalog exists.
pub fn intro_prop_sprite_rows() -> Vec<(&'static str, &'static str, CharacterSheetSpec)> {
    let t = &PROP_TUNING;
    let mut rows: Vec<(&str, &str, Option<CharacterSheetSpec>)> = vec![
        (
            "intro_cart",
            "intro_cart_spritesheet.png",
            intro_sheet("intro_cart", t),
        ),
        // Creator lab props — separate records inside the shared
        // creator_lab_props sheet.
        (
            "lab_genesis_vat",
            "creator_lab_props_spritesheet.png",
            intro_sheet("genesis_vat", t),
        ),
        (
            "lab_neural_console",
            "creator_lab_props_spritesheet.png",
            intro_sheet("neural_console", t),
        ),
        (
            "lab_power_core",
            "creator_lab_props_spritesheet.png",
            intro_sheet("power_core", t),
        ),
        (
            "lab_repair_cradle",
            "creator_lab_props_spritesheet.png",
            intro_sheet("repair_cradle", t),
        ),
        (
            "lab_resonance_coil",
            "creator_lab_props_spritesheet.png",
            intro_sheet("resonance_coil", t),
        ),
        // Cut-rope boss props.
        (
            "cut_rope_rope",
            "cut_rope_rope_spritesheet.png",
            intro_sheet("cut_rope_rope", t),
        ),
        (
            "cut_rope_anvil",
            "cut_rope_anvil_spritesheet.png",
            intro_sheet("cut_rope_anvil", t),
        ),
        (
            "cut_rope_piano",
            "cut_rope_piano_spritesheet.png",
            intro_sheet("cut_rope_piano", t),
        ),
        (
            "generic_explosions",
            "generic_explosions_spritesheet.png",
            intro_sheet("generic_explosions", t),
        ),
        // Interdimensional gate ring + portal surface.
        (
            "gate_ring",
            "interdimensional_gate_ring_spritesheet.png",
            intro_sheet("interdimensional_gate_ring", t),
        ),
        (
            "gate_portal",
            "interdimensional_gate_portal_spritesheet.png",
            intro_sheet("interdimensional_gate_portal", t),
        ),
    ];
    rows.drain(..)
        .filter_map(|(kind, file, spec)| spec.map(|s| (kind, file, s)))
        .collect()
}

/// Stable [`AssetId`] for an intro NPC sprite. The namespace is
/// `sprite.character.intro_<lower_snake_name>` — distinct from the
/// sandbox-side `sprite.character.npc_<…>` namespace so the intro's
/// authored NPC roster doesn't collide with the sandbox `NPC_SPRITE_REGISTRY`.
///
/// The mapping is the inverse of [`intro_npc_label`]: catalog id
/// construction (here) + label lookup (over there) must agree on the
/// snake_case form for every entry in [`INTRO_NPC_SPRITE_REGISTRY`].
pub fn intro_npc_asset_id(npc_name: &str) -> AssetId {
    AssetId::new(format!(
        "sprite.character.intro_{}",
        intro_npc_label(npc_name)
    ))
}

/// Snake_case label slot for an intro NPC name. Pairs with
/// [`intro_npc_asset_id`]. New rows in [`INTRO_NPC_SPRITE_REGISTRY`]
/// must add a row here too.
pub fn intro_npc_label(npc_name: &str) -> &'static str {
    match npc_name {
        "Creator" => "creator",
        "Creator Final" => "creator_final",
        "Oiler" => "oiler",
        "Gate Janitor" => "gate_janitor",
        "Erdish" => "erdish",
        "Lab Raider" => "lab_raider",
        "Salvage Guard" => "salvage_guard",
        "Manifest Clerk" => "manifest_clerk",
        "News Board" => "news_board",
        "Alice" => "alice",
        "Bob" => "bob",
        // Story plugins that author NPCs outside `INTRO_NPC_SPRITE_REGISTRY`
        // fall through to a single "unregistered" label so the catalog
        // can still resolve their entries (typically resolves to a
        // colored rectangle).
        _ => "unregistered",
    }
}

/// Stable [`AssetId`] for an intro prop sprite. Namespace
/// `sprite.character.intro_prop_<lower_snake_kind>` — props share the
/// `sprite.character.*` namespace with NPCs because they ride the same
/// `CharacterSpriteAsset` runtime type, but the `intro_prop_` prefix
/// keeps the two cleanly separable.
pub fn intro_prop_asset_id(prop_kind: &str) -> AssetId {
    AssetId::new(format!(
        "sprite.character.intro_prop_{}",
        prop_kind.replace(['-', ' '], "_"),
    ))
}

use ambition_asset_manager::{
    AssetEntry, AssetKind, AssetManifest, MissingAssetPolicy, PreloadGroup,
};

/// via `catalog.try_path_for_load(...)` like every other loader.
///
/// IDs are `sprite.character.intro_<name_snake>` for NPCs and
/// `sprite.character.intro_prop_<kind_snake>` for props. Both use
/// `SilentPlaceholder` because missing intro art falls back to colored
/// rectangles per the existing contract.
pub fn extend_with_intro_sprite_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (name, filename, _spec) in intro_npc_sprite_rows() {
        let id = intro_npc_asset_id(name);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
    for (kind, filename, _spec) in intro_prop_sprite_rows() {
        let id = intro_prop_asset_id(kind);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}
