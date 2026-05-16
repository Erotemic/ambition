//! Intro NPC sprite placeholders.
//!
//! Until proper character sheets exist for Creator / Oiler / Gate Janitor /
//! Framebreaker / Manifest Clerk, intro NPCs reuse the toon-target
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

use crate::character_sprites::{
    CharacterSheetSpec, ARCHITECT_SHEET, CART_SHEET, CREATOR_SHEET, ERDISH_SHEET,
    FASCIST_ENFORCER_SHEET, GATE_PORTAL_SHEET, GATE_RING_SHEET, KERNEL_GUIDE_SHEET,
    LAB_PROP_GENESIS_VAT, LAB_PROP_NEURAL_CONSOLE, LAB_PROP_POWER_CORE, LAB_PROP_REPAIR_CRADLE,
    LAB_PROP_RESONANCE_COIL, OILER_SHEET,
};

/// `(LDtk NpcSpawn.name, asset filename, sheet spec)` rows for the
/// intro NPCs. Filenames are relative to the configured
/// `assets/<sprite_folder>/` directory — same convention as
/// `crate::character_sprites::assets::NPC_SPRITE_REGISTRY`.
pub const INTRO_NPC_SPRITE_REGISTRY: &[(&str, &str, CharacterSheetSpec)] = &[
    // Diagnostic Cart — the prop the player wakes on. Authored as an
    // NpcSpawn purely so the existing sprite-loader picks it up; the
    // generic_npc dialogue fallback means an Interact press shows a
    // harmless "this NPC has no Yarn node yet" line until a real Prop
    // entity type lands (see docs/intro_followup_roadmap.md §3).
    ("Diagnostic Cart", "intro_cart_spritesheet.png", CART_SHEET),
    // Wake-room creator. Dedicated creator tack-on sheet — 160×192 with
    // a 108px label column, four authored rows (idle/speak/gesture/walk).
    ("Creator", "creator_spritesheet.png", CREATOR_SHEET),
    // Same creator, raid-corridor variant. Same sheet so the player
    // recognizes the silhouette dying mid-sentence.
    ("Creator Final", "creator_spritesheet.png", CREATOR_SHEET),
    // Oiler: street mechanic. Dedicated toon-adapter sheet (rendered
    // from configs/review/oiler.yaml + the `oiler` PRESETS entry in
    // targets/toon_side.py).
    ("Oiler", "oiler_spritesheet.png", OILER_SHEET),
    // Gate Janitor: utility staff under the gate stack. Kernel Guide
    // is the placeholder until a dedicated janitor sheet lands —
    // poncho + slightly weary silhouette reads closer than the
    // vested Vault Keeper at this size.
    ("Gate Janitor", "kernel_guide_spritesheet.png", KERNEL_GUIDE_SHEET),
    // Erdish: optional recurring graph-theory eccentric. Not spawned
    // in the v1 intro slice yet (he lands when a `NpcSpawn` with
    // `name: Erdish` is authored in a later room), but pre-registered
    // so the sprite is ready the moment LDtk authoring catches up.
    ("Erdish", "erdish_spritesheet.png", ERDISH_SHEET),
    // Framebreaker (anti-machine hardliner). Fascist enforcer reads
    // as "uniformed raid grunt" which fits this role better than the
    // earlier goblin placeholder — both intro raid factions wear
    // uniforms; the Framebreaker palette is just a different colorway.
    // TODO(sprites): dedicated framebreaker sheet when art lands.
    ("Framebreaker", "fascist_enforcer_spritesheet.png", FASCIST_ENFORCER_SHEET),
    // Nazi salvage guard. Fascist Enforcer is the correct raid-trooper
    // sheet — the previous Absurd General was a satirical hub NPC, not
    // a uniformed dimension trooper. Officer cap + storm uniform +
    // rifle reads correctly for the basement raid.
    ("Nazi Salvage Guard", "fascist_enforcer_spritesheet.png", FASCIST_ENFORCER_SHEET),
    // Manifest clerk: bureaucratic kiosk operator. Architect sheet
    // reads as "person at a podium pointing at things."
    ("Manifest Clerk", "architect_spritesheet.png", ARCHITECT_SHEET),
    // News board: not an animated NPC in the design sense, but the
    // sandbox treats every `NpcSpawn` the same way. Architect sheet
    // is a placeholder; a static kiosk sprite will replace this.
    ("News Board", "architect_spritesheet.png", ARCHITECT_SHEET),
    // Lab props — share `creator_lab_props_spritesheet.png`; each
    // pulls a different row via the y_offset field on its
    // CharacterSheetSpec. Authored as NpcSpawn with `prompt: ""`
    // for v1 (the dedicated Prop entity type is the v1.x cleanup).
    ("Genesis Vat", "creator_lab_props_spritesheet.png", LAB_PROP_GENESIS_VAT),
    ("Neural Console", "creator_lab_props_spritesheet.png", LAB_PROP_NEURAL_CONSOLE),
    ("Power Core", "creator_lab_props_spritesheet.png", LAB_PROP_POWER_CORE),
    ("Repair Cradle", "creator_lab_props_spritesheet.png", LAB_PROP_REPAIR_CRADLE),
    ("Resonance Coil", "creator_lab_props_spritesheet.png", LAB_PROP_RESONANCE_COIL),
    // Interdimensional gate (legally distinct stargate). The ring is
    // the always-on structural arch; the portal renders the
    // shimmering surface inside it. Both placed at the same px in
    // gate_stack_lower; the runtime gate-check via
    // `GatedZoneRegistry` keeps the LoadingZone behind them inert
    // until the player toggles `intro_portal_switch`.
    (
        "Interdimensional Gate Ring",
        "interdimensional_gate_ring_spritesheet.png",
        GATE_RING_SHEET,
    ),
    (
        "Interdimensional Gate Portal",
        "interdimensional_gate_portal_spritesheet.png",
        GATE_PORTAL_SHEET,
    ),
];

pub fn intro_npc_sprite_rows() -> &'static [(&'static str, &'static str, CharacterSheetSpec)] {
    INTRO_NPC_SPRITE_REGISTRY
}
