//! Spritesheet metadata, atlas/animation logic, and loading for every
//! animated character (player robot, goblins, sandbag, boss, NPCs).
//!
//! All character sheets are produced by `tools/ambition_sprite2d_renderer`
//! and copied into `assets/sprites/`. If a PNG is missing at startup the
//! corresponding `Option` stays `None` and callers fall back to the
//! colored-rectangle visuals that predate this module — the game must
//! always run.
//!
//! ## Submodule layout
//!
//! - [`anim`] — `CharacterAnim` enum, the one shared `pick_body_anim` priority
//!   ladder over a `BodyAnimView`, and the thin per-body adapters that build it
//!   (`pick_player_anim`, `pick_actor_anim` + `ActorAnimState`).
//! - [`sheets`] — `CharacterSheetSpec`, `AnimRow`, atlas/geometry
//!   helpers (`sprite_render_size`, `feet_anchor_for`,
//!   `build_character_sprite`); the `*_SHEET` constants now come from
//!   the catalog-keyed spec resolver, not in-file statics.
//! - [`assets`] — `CharacterSpriteAsset`, `CharacterSpriteAssets`
//!   resource, `load_character_sprites_in`, `sheet_for_character_id`.
//! - [`animator`] — the `CharacterAnimator` per-entity cursor component.
//! - [`registry`] — host wiring for the reusable `ambition_sprite_sheet`
//!   `SheetRegistry` (plugin + headless builder from the baked table).
//! - [`baked_sheet_rons`] — `build.rs`-generated `(root, ron_text)`
//!   table of every `*_spritesheet.ron` (so non-desktop builds carry
//!   the metadata without reading disk).

mod anim;
mod animator;
mod assets;
mod attack_hitbox;
mod baked_sheet_rons;
pub mod registry;
mod sheets;

#[cfg(test)]
mod tests;

pub use anim::{pick_actor_anim, pick_player_anim, ActorAnimState, CharacterAnim};
pub use animator::{CharacterAnimator, RenderBasis};
#[allow(
    unused_imports,
    reason = "manifest_attack_hitbox_world is the reusable core; player_attack_hitbox_world is the live consumer (ambition_app advance_attack)."
)]
pub use attack_hitbox::{
    actor_attack_hitbox_world, manifest_attack_hitbox_world, player_attack_hitbox_world,
};
pub use registry::{baked_sheet_registry, SheetRegistryPlugin};
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
    load_character_sprites_in, sheet_for_character_id, sprite_body_collision_for_character_id,
    CharacterSpriteAssets, SpriteBodyCollision,
};
#[allow(unused_imports)]
pub use registry::{SheetRecord, SheetRegistry};
#[allow(
    unused_imports,
    reason = "Public sheet constants are consumed by tests and future spawn-site callers."
)]
pub use sheets::{
    build_atlas_layout, build_character_sprite, build_character_sprite_with_render_size,
    feet_anchor_for, feet_anchor_for_render_size, player_placeholder_render_size,
    record_for_target, sprite_render_size, try_load_spec_for_target, CharacterSheetSpec,
    SheetTuning,
};
