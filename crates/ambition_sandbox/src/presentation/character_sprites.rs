//! Sprite-sheet rendering for the player robot and goblin enemies.
//!
//! All character sheets (player robot, goblins, sandbag, boss) are
//! produced by `tools/ambition_sprite2d_renderer` and copied into
//! `assets/sprites/`. If a PNG is missing at startup the corresponding
//! `Option` stays `None` and callers fall back to the colored-rectangle
//! visuals that predate this module — the game must always run.
//!
//! ## Submodule layout (post-2026-05-09 split)
//!
//! - [`anim`] — `CharacterAnim` enum + per-actor animation pickers
//!   (`pick_player_anim`, `pick_enemy_anim`, `EnemyAnimState`).
//! - [`sheets`] — `CharacterSheetSpec`, `AnimRow`, the `ROBOT_SHEET`
//!   / `GOBLIN_SHEET` / `SANDBAG_SHEET` data tables, plus
//!   `sprite_render_size`, `feet_anchor_for`, and
//!   `build_character_sprite`.
//! - [`assets`] — `CharacterSpriteAsset`, `CharacterSpriteAssets`
//!   resource, `load_character_sprites_in`.
//! - [`animator`] — the `CharacterAnimator` per-entity component.

mod anim;
mod animator;
mod assets;
pub mod registry;
mod sheets;

#[cfg(test)]
mod tests;

pub use anim::{
    pick_enemy_anim, pick_npc_anim, pick_player_anim, CharacterAnim, EnemyAnimState, NpcAnimState,
};
pub use animator::CharacterAnimator;
pub use registry::SheetRegistryPlugin;
// SheetRecord and SheetRegistry are kept in the module's public surface
// for future consumers that want per-frame anchors / body bbox queries;
// they're already loaded at startup by SheetRegistryPlugin. Re-export
// gated to silence the unused-import warning until something outside
// the registry module actually queries them.
#[allow(
    unused_imports,
    reason = "sheet_for_character_id is the public catalog→spec entry; consumed by tests under content::character_catalog::tests (not by non-test crate code today). Public surface for future spawn-site callers."
)]
pub use assets::{
    all_character_sprite_filenames, build_npc_sprite_asset, build_prop_sprite_asset,
    load_character_sprites_in, sheet_for_character_id, CharacterSpriteAssets,
};
#[allow(unused_imports)]
pub use registry::{SheetRecord, SheetRegistry};
pub use sheets::{
    build_character_sprite, build_character_sprite_with_render_size, feet_anchor_for,
    feet_anchor_for_render_size, player_placeholder_render_size, sprite_render_size,
    CharacterSheetSpec, ALICE_SHEET, ARCHITECT_SHEET, BOB_SHEET, CART_SHEET, CREATOR_SHEET,
    ERDISH_SHEET, GATE_PORTAL_SHEET, GATE_RING_SHEET, KERNEL_GUIDE_SHEET, LAB_PROP_GENESIS_VAT,
    LAB_PROP_NEURAL_CONSOLE, LAB_PROP_POWER_CORE, LAB_PROP_REPAIR_CRADLE, LAB_PROP_RESONANCE_COIL,
    NEWS_BOARD_SHEET, OILER_SHEET, RAID_ENFORCER_SHEET,
};
