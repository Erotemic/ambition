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

use crate::presentation::character_sprites::{
    CharacterSheetSpec, ALICE_SHEET, ARCHITECT_SHEET, BOB_SHEET, CART_SHEET, CREATOR_SHEET,
    ERDISH_SHEET, FASCIST_ENFORCER_SHEET, GATE_PORTAL_SHEET, GATE_RING_SHEET, KERNEL_GUIDE_SHEET,
    LAB_PROP_GENESIS_VAT, LAB_PROP_NEURAL_CONSOLE, LAB_PROP_POWER_CORE, LAB_PROP_REPAIR_CRADLE,
    LAB_PROP_RESONANCE_COIL, NEWS_BOARD_SHEET, OILER_SHEET,
};

/// `(LDtk NpcSpawn.name, asset filename, sheet spec)` rows for the
/// intro NPCs. Filenames are relative to the configured
/// `assets/<sprite_folder>/` directory — same convention as
/// `crate::presentation::character_sprites::assets::NPC_SPRITE_REGISTRY`.
pub const INTRO_NPC_SPRITE_REGISTRY: &[(
    &str,
    &str,
    &'static std::sync::LazyLock<CharacterSheetSpec>,
)] = &[
    // Wake-room creator. Dedicated creator tack-on sheet — 160×192 with
    // a 108px label column, four authored rows (idle/speak/gesture/walk).
    ("Creator", "creator_spritesheet.png", &CREATOR_SHEET),
    // Same creator, raid-corridor variant. Same sheet so the player
    // recognizes the silhouette dying mid-sentence.
    ("Creator Final", "creator_spritesheet.png", &CREATOR_SHEET),
    // Oiler: street mechanic. Dedicated toon-adapter sheet (rendered
    // from configs/review/oiler.yaml + the `oiler` PRESETS entry in
    // targets/toon_side.py).
    ("Oiler", "oiler_spritesheet.png", &OILER_SHEET),
    // Gate Janitor: utility staff under the gate stack. Kernel Guide
    // is the placeholder until a dedicated janitor sheet lands —
    // poncho + slightly weary silhouette reads closer than the
    // vested Vault Keeper at this size.
    (
        "Gate Janitor",
        "kernel_guide_spritesheet.png",
        &KERNEL_GUIDE_SHEET,
    ),
    // Erdish: optional recurring graph-theory eccentric. Not spawned
    // in the v1 intro slice yet (he lands when a `NpcSpawn` with
    // `name: Erdish` is authored in a later room), but pre-registered
    // so the sprite is ready the moment LDtk authoring catches up.
    ("Erdish", "erdish_spritesheet.png", &ERDISH_SHEET),
    // Lab Raider: generic intro pressure enemy. Reuses the uniformed
    // raid-grunt sheet until dedicated intro enemy art lands.
    (
        "Lab Raider",
        "fascist_enforcer_spritesheet.png",
        &FASCIST_ENFORCER_SHEET,
    ),
    // Salvage Guard: second generic intro pressure enemy. Shares the
    // temporary raid-grunt sheet so the intro stays content-driven.
    (
        "Salvage Guard",
        "fascist_enforcer_spritesheet.png",
        &FASCIST_ENFORCER_SHEET,
    ),
    // Manifest clerk: bureaucratic kiosk operator. Architect sheet
    // reads as "person at a podium pointing at things."
    (
        "Manifest Clerk",
        "architect_spritesheet.png",
        &ARCHITECT_SHEET,
    ),
    // News board: not an animated NPC in the design sense, but the
    // sandbox treats every `NpcSpawn` the same way. Dedicated
    // `news_board_spritesheet.png` renders a wall-mounted bulletin
    // board (Disruptor Industries header, pinned papers, blinking
    // LED) so it visibly reads as a board instead of a person.
    (
        "News Board",
        "news_board_spritesheet.png",
        &NEWS_BOARD_SHEET,
    ),
    // Alice — unofficial cartographer. Dedicated toon-side sheet
    // (`alice_spritesheet.png` + `alice_spritesheet.yaml`); first
    // intro NPC with non-placeholder art.
    ("Alice", "alice_spritesheet.png", &ALICE_SHEET),
    // Bob — field cartographer. Companion dedicated sheet to Alice.
    ("Bob", "bob_spritesheet.png", &BOB_SHEET),
    // Cart, lab props, and gate sprites are now Prop entities (see
    // INTRO_PROP_REGISTRY below) and live in
    // `GameAssets.characters.props` instead of `npcs`.
];

pub fn intro_npc_sprite_rows() -> &'static [(
    &'static str,
    &'static str,
    &'static std::sync::LazyLock<CharacterSheetSpec>,
)] {
    INTRO_NPC_SPRITE_REGISTRY
}

/// `(Prop.kind, asset filename, sheet spec)` rows for intro props.
///
/// Keyed by `Prop.kind` (NOT display name) so authors can rename a
/// prop in LDtk without re-pointing the sprite registry. Loaded into
/// `GameAssets.characters.props` by
/// [`crate::intro::plugin::load_intro_prop_sprites_system`].
pub const INTRO_PROP_REGISTRY: &[(&str, &str, &'static std::sync::LazyLock<CharacterSheetSpec>)] =
    &[
        // Diagnostic cart the player wakes on.
        ("intro_cart", "intro_cart_spritesheet.png", &CART_SHEET),
        // Creator lab props — each pulls a different row from the shared
        // `creator_lab_props_spritesheet.png` via its `y_offset`.
        (
            "lab_genesis_vat",
            "creator_lab_props_spritesheet.png",
            &LAB_PROP_GENESIS_VAT,
        ),
        (
            "lab_neural_console",
            "creator_lab_props_spritesheet.png",
            &LAB_PROP_NEURAL_CONSOLE,
        ),
        (
            "lab_power_core",
            "creator_lab_props_spritesheet.png",
            &LAB_PROP_POWER_CORE,
        ),
        (
            "lab_repair_cradle",
            "creator_lab_props_spritesheet.png",
            &LAB_PROP_REPAIR_CRADLE,
        ),
        (
            "lab_resonance_coil",
            "creator_lab_props_spritesheet.png",
            &LAB_PROP_RESONANCE_COIL,
        ),
        // Interdimensional gate (legally distinct stargate). Ring is the
        // always-on structural arch; portal renders the shimmering
        // surface inside it. Both keyed as props so the gate stack scene
        // never grows an interact prompt on them.
        (
            "gate_ring",
            "interdimensional_gate_ring_spritesheet.png",
            &GATE_RING_SHEET,
        ),
        (
            "gate_portal",
            "interdimensional_gate_portal_spritesheet.png",
            &GATE_PORTAL_SHEET,
        ),
    ];

pub fn intro_prop_sprite_rows() -> &'static [(
    &'static str,
    &'static str,
    &'static std::sync::LazyLock<CharacterSheetSpec>,
)] {
    INTRO_PROP_REGISTRY
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
