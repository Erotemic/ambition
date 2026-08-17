//! Spritesheet metadata, atlas/animation logic, and loading for every
//! animated character (player robot, goblins, sandbag, boss, NPCs).
//!
//! All character sheets are produced by `tools/ambition_sprite2d_renderer`
//! and copied into `assets/sprites/`. If a PNG is missing at startup the
//! corresponding `Option` stays `None` and callers fall back to the
//! colored-rectangle visuals that predate this module — the game must
//! always run.
//!
//! ## What is left here, and what left
//!
//! [`assets`] — the actor/content JOIN: `load_character_sprites_in`,
//! `sheet_for_character_id_in`, catalog-driven body collision, prop sprite
//! construction. It stays because it is bidirectionally coupled to
//! `crate::assets::platformer_assets`, `ambition_persistence::settings` and the
//! character-runtime materializer, which is the coupling `character_runtime`
//! shares.
//!
//! ⭐ **the DERIVATIONS moved out, 2026-08-09.** `{anim, posed_body,
//! attack_hitbox}` are `ambition_character_sprites` now — a sibling crate this
//! one does NOT depend on. They answered upward only (the animation picker to
//! `ambition_sim_view`, the attack hitbox to `ambition_platformer2d_runtime`),
//! and the one thing they took from here — the
//! `WorldPrepSet::BeforeIntegrate` registration of `sync_sprite_posed_bodies` —
//! is `ambition_character_sprites::SpritePosedBodyPlugin`, installed by the
//! runtime beside `WorldPrepSchedulePlugin`. Keeping that registration here
//! would have made the owner depend on the carve, which is the shape that
//! lengthens the workspace's serial compile chain rather than shortening
//! anything; see `docs/planning/engine/decomposition.md`.
//!
//! ## What this module does NOT re-export
//!
//! Sheet vocabulary — `AuthoredSheets`, `CharacterSheetSpec`, `SheetRecord`,
//! `SheetRegistry`, `SheetRegistryPlugin`, `CharacterAnim`, `CharacterAnimator`,
//! `CharacterSpriteAssets`, `SpritePosedBody`, `SpriteBodyCollision`,
//! `record_for_target`, `baked_sheet_registry` and the rest — is OWNED by
//! `ambition_sprite_sheet` and is named from there. This
//! module used to pass 22 such names through, on the argument that "every
//! consumer of character art already imports this module, so a crate that
//! threads it does not need a new dependency edge". Measured 2026-08-08 that
//! convenience was buying an illusion: 52 files appeared to consume
//! `character_sprites` and 29 actually did — the rest were reaching
//! `ambition_sprite_sheet` through a module that merely stood in the way of
//! seeing the real edge. Name the owner.

mod assets;

#[cfg(test)]
mod tests;

#[allow(
    unused_imports,
    reason = "sheet_for_character_id is the public catalog→spec entry; consumed by tests under content::character_catalog::tests (not by non-test crate code today). Public surface for future spawn-site callers."
)]
pub use assets::{
    all_character_sprite_filenames_in, build_npc_sprite_asset, build_prop_sprite_asset,
    build_prop_sprite_asset_packed, character_sprite_tier, load_character_sprites_in, load_fx_sheets,
    load_prop_sheet_for_target, materialize_declared_character_sprite,
    portrait_for_declared_character, sheet_for_character_id_in, sheet_for_declared_character,
    sprite_body_collision_for_character_id_in, SpriteMaterialization,
};
