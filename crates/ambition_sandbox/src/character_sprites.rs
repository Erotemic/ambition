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
mod sheets;

#[cfg(test)]
mod tests;

pub use anim::{
    pick_enemy_anim, pick_npc_anim, pick_player_anim, CharacterAnim, EnemyAnimState, NpcAnimState,
};
pub use animator::CharacterAnimator;
pub use assets::{
    build_npc_sprite_asset, load_character_sprites_in, CharacterSpriteAsset, CharacterSpriteAssets,
};
pub use sheets::{
    build_character_sprite, build_character_sprite_with_render_size, feet_anchor_for,
    feet_anchor_for_render_size, player_placeholder_render_size, sprite_render_size, AnimRow,
    CharacterSheetSpec, ABSURD_GENERAL_SHEET, ARCHITECT_SHEET, ERDISH_SHEET, GOBLIN_SHEET,
    KERNEL_GUIDE_SHEET, MERCHANT_PROTOTYPE_SHEET, OILER_SHEET, ROBOT_SHEET, SANDBAG_SHEET,
    VAULT_KEEPER_SHEET,
};
