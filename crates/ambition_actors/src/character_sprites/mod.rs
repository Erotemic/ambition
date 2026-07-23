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
//! - `ambition_sprite_sheet::character::sheets` — lower authority for
//!   `CharacterSheetSpec`, atlas/geometry helpers, baked manifest lookup, and
//!   the `SheetRegistryPlugin` installed by app/content crates.
//! - [`assets`] — actor/content join for `load_character_sprites_in`,
//!   `sheet_for_character_id`, catalog-driven body collision, and prop sprite
//!   construction.
//! - `ambition_sprite_sheet::character::animator` — lower authority for the
//!   `CharacterAnimator` per-entity cursor component.

mod anim;
mod assets;
mod attack_hitbox;

#[cfg(test)]
mod tests;

pub use ambition_sprite_sheet::character::{CharacterAnimator, RenderBasis};
pub use ambition_sprite_sheet::{baked_sheet_registry, SheetRegistryPlugin};
pub use anim::{pick_actor_anim, pick_player_anim, ActorAnimState, CharacterAnim};
#[allow(
    unused_imports,
    reason = "manifest_attack_hitbox_world is the reusable core; player_attack_hitbox_world is the live consumer (the debug-overlay hitbox source)."
)]
pub use attack_hitbox::{
    actor_attack_hitbox_world, authored_attack_volume_resolver, manifest_attack_hitbox_world,
    player_attack_hitbox_world,
};
// SheetRecord and SheetRegistry are kept in the module's public surface
// for future consumers that want per-frame anchors / body bbox queries;
// they're already loaded at startup by SheetRegistryPlugin. Re-export
// gated to silence the unused-import warning until something outside
// the registry module actually queries them.
#[allow(
    unused_imports,
    reason = "Public sheet constants are consumed by tests and future spawn-site callers."
)]
pub use ambition_sprite_sheet::character::sheets::{
    build_atlas_layout, build_character_sprite, build_character_sprite_with_render_size,
    feet_anchor_for, feet_anchor_for_render_size, player_placeholder_render_size,
    record_for_target, sprite_render_size, try_load_spec_for_target, CharacterSheetSpec,
    SheetTuning,
};
pub use ambition_sprite_sheet::character::{CharacterSpriteAsset, CharacterSpritePage};
#[allow(unused_imports)]
pub use ambition_sprite_sheet::{SheetRecord, SheetRegistry};
#[allow(
    unused_imports,
    reason = "sheet_for_character_id is the public catalog→spec entry; consumed by tests under content::character_catalog::tests (not by non-test crate code today). Public surface for future spawn-site callers."
)]
pub use assets::{
    all_character_sprite_filenames_in, build_npc_sprite_asset, build_prop_sprite_asset,
    build_prop_sprite_asset_packed, load_character_sprites_in, load_prop_sheet_for_target,
    materialize_deferred_character_sprite, sheet_for_character_id_in,
    sprite_body_collision_for_character_id_in, CharacterSpriteAssets, SpriteBodyCollision,
};
