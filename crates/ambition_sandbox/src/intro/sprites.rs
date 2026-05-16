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
    CharacterSheetSpec, ABSURD_GENERAL_SHEET, ARCHITECT_SHEET, ERDISH_SHEET, GOBLIN_SHEET,
    KERNEL_GUIDE_SHEET, OILER_SHEET,
};

/// `(LDtk NpcSpawn.name, asset filename, sheet spec)` rows for the
/// intro NPCs. Filenames are relative to the configured
/// `assets/<sprite_folder>/` directory — same convention as
/// `crate::character_sprites::assets::NPC_SPRITE_REGISTRY`.
pub const INTRO_NPC_SPRITE_REGISTRY: &[(&str, &str, CharacterSheetSpec)] = &[
    // Wake-room creator. Kernel Guide reads as "thoughtful, talking";
    // swap to a dedicated Creator sheet when it lands.
    ("Creator", "kernel_guide_spritesheet.png", KERNEL_GUIDE_SHEET),
    // Same creator, raid-corridor variant. Re-uses the same sheet so
    // the player recognizes the silhouette dying mid-sentence.
    ("Creator Final", "kernel_guide_spritesheet.png", KERNEL_GUIDE_SHEET),
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
    // Framebreaker (anti-machine hardliner). Goblin sheet is a
    // deliberate placeholder — the doc calls it out as "Goblin or
    // generic enemy placeholder, but rename in dialogue."
    ("Framebreaker", "goblin_spritesheet.png", GOBLIN_SHEET),
    // Nazi salvage guard. Absurd General is the military-uniform
    // placeholder per the doc.
    ("Nazi Salvage Guard", "absurd_general_spritesheet.png", ABSURD_GENERAL_SHEET),
    // Manifest clerk: bureaucratic kiosk operator. Architect sheet
    // reads as "person at a podium pointing at things."
    ("Manifest Clerk", "architect_spritesheet.png", ARCHITECT_SHEET),
    // News board: not an animated NPC in the design sense, but the
    // sandbox treats every `NpcSpawn` the same way. Architect sheet
    // is a placeholder; a static kiosk sprite will replace this.
    ("News Board", "architect_spritesheet.png", ARCHITECT_SHEET),
];

pub fn intro_npc_sprite_rows() -> &'static [(&'static str, &'static str, CharacterSheetSpec)] {
    INTRO_NPC_SPRITE_REGISTRY
}
