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

use ambition_sprite_sheet::character::{try_load_spec_for_target, CharacterSheetSpec, SheetTuning};

/// Resolve a content-owned sheet spec by manifest target, with intro
/// tuning. Panics in tests via the registry checks; at runtime a
/// missing manifest falls back to the colored-rectangle contract by
/// the caller skipping the row.
fn intro_sheet(target: &str, tuning: &SheetTuning) -> Option<CharacterSheetSpec> {
    try_load_spec_for_target(target, tuning)
}

// ⛔⛔ THE INTRO NPC SPRITE TABLE IS GONE, and it was decoding art for nobody.
//
// It listed eleven `(display name, filename, sheet spec)` rows, preloaded them at
// boot in whatever room, and published each under its DISPLAY NAME
// ("Creator", "Manifest Clerk", …). Measured 2026-09-02: the intro world authors
// no such names. Every `NpcSpawn` in `intro.ldtk` carries a `character_id` and
// `name: None` — npc_creator, npc_alice, npc_bob, npc_oiler, npc_news_board,
// npc_gate_janitor, npc_manifest_clerk — and `convert_npc_spawn` puts that id
// into `InteractionKindSpec::Npc.character_id`, which is what
// `demand_actor_character_sheets` raises on room entry. The peaceful NPC road
// sets `sprite_override_npc_name: None`, so NOTHING ever looked a sheet up by
// the display name this table published under.
//
// ⇒ every row was a decode whose result no lookup could reach. The `[image-dropped]`
// line named them: architect, bob, erdish, goblin, alice, oiler and two more.
// Two rows (Lab Raider, Salvage Guard) were doubly dead — both are `EnemySpawn`s
// with their own ids since 2026-08-12 — and Erdish had no placement at all, as
// its own comment said ("pre-registered for later LDtk authoring").
//
// ⚠ The rows also fed `extend_with_intro_sprite_entries`, which put each sheet in
// the manifest under `PreloadGroup::SandboxCore` — a SECOND preload road off the
// same table. Deleting the table closes both.
//
// Props are a different question and keep their table below: a `Prop` is keyed by
// `Prop.kind`, which the world does author.

/// Prop tuning: props render at their authored AABB size.
const PROP_TUNING: SheetTuning = SheetTuning::new(1.00, 2);

/// `(Prop.kind, asset filename, sheet spec, pack target)` rows for intro
/// props (keyed by `Prop.kind` so LDtk renames don't re-point sprites).
/// Includes the cut-rope arena props until a dedicated non-intro prop
/// catalog exists.
pub fn intro_prop_sprite_rows() -> Vec<(
    &'static str,
    &'static str,
    CharacterSheetSpec,
    Option<&'static str>,
)> {
    let t = &PROP_TUNING;
    let mut rows: Vec<(&str, &str, Option<CharacterSheetSpec>, Option<&str>)> = vec![
        (
            "intro_cart",
            "intro_cart_spritesheet.png",
            intro_sheet("intro_cart", t),
            Some("intro_cart"),
        ),
        // Creator lab props — separate records inside the shared
        // creator_lab_props sheet.
        (
            "lab_genesis_vat",
            "creator_lab_props_spritesheet.png",
            intro_sheet("genesis_vat", t),
            None,
        ),
        (
            "lab_neural_console",
            "creator_lab_props_spritesheet.png",
            intro_sheet("neural_console", t),
            None,
        ),
        (
            "lab_power_core",
            "creator_lab_props_spritesheet.png",
            intro_sheet("power_core", t),
            None,
        ),
        (
            "lab_repair_cradle",
            "creator_lab_props_spritesheet.png",
            intro_sheet("repair_cradle", t),
            None,
        ),
        (
            "lab_resonance_coil",
            "creator_lab_props_spritesheet.png",
            intro_sheet("resonance_coil", t),
            None,
        ),
        // Cut-rope boss props.
        (
            "cut_rope_rope",
            "cut_rope_rope_spritesheet.png",
            intro_sheet("cut_rope_rope", t),
            None,
        ),
        (
            "cut_rope_anvil",
            "cut_rope_anvil_spritesheet.png",
            intro_sheet("cut_rope_anvil", t),
            None,
        ),
        (
            "cut_rope_piano",
            "cut_rope_piano_spritesheet.png",
            intro_sheet("cut_rope_piano", t),
            None,
        ),
        // The engine ships its own effect sheets now (`ambition_sprite_sheet::fx::FX_SHEETS`); a
        // story's prop table is not the place to declare them. Interdimensional gate ring + portal
        // surface.
        (
            "gate_ring",
            "interdimensional_gate_ring_spritesheet.png",
            intro_sheet("interdimensional_gate_ring", t),
            None,
        ),
        (
            "gate_portal",
            "interdimensional_gate_portal_spritesheet.png",
            intro_sheet("interdimensional_gate_portal", t),
            None,
        ),
    ];
    rows.drain(..)
        .filter_map(|(kind, file, spec, pack)| spec.map(|s| (kind, file, s, pack)))
        .collect()
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
/// ⚠ TAKES NO CAST ANY MORE. It needed `AuthoredSheets` + `CharacterCatalog` to
/// resolve the intro NPC rows; those are gone (see the note above), and a `Prop`
/// row resolves from its own table. Kept narrow rather than kept compatible — a
/// parameter nobody reads is a claim that this still knows about characters.
pub fn extend_with_intro_sprite_entries(manifest: &mut AssetManifest, sprite_folder: &str) {
    for (kind, filename, _spec, _pack) in intro_prop_sprite_rows() {
        let id = intro_prop_asset_id(kind);
        let logical_path = format!("{sprite_folder}/{filename}");
        manifest.insert(
            AssetEntry::new(id, AssetKind::Image, logical_path)
                .with_missing_policy(MissingAssetPolicy::SilentPlaceholder)
                .with_preload_group(PreloadGroup::SandboxCore),
        );
    }
}
