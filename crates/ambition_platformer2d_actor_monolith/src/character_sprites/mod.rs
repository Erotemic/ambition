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
//! - [`anim`] — the one shared `pick_body_anim` priority ladder over a
//!   `BodyAnimView`, and the thin per-body adapters that build it
//!   (`pick_player_anim`, `pick_actor_anim` + `ActorAnimState`). The
//!   `CharacterAnim` vocabulary itself belongs to `ambition_sprite_sheet`.
//! - [`assets`] — actor/content join for `load_character_sprites_in`,
//!   `sheet_for_character_id`, catalog-driven body collision, and prop sprite
//!   construction.
//! - [`posed_body`] — the sheet as the AUTHORITY for an actor's collision box,
//!   sprite quad, and quad placement, resolved per pose.
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

mod anim;
mod assets;
mod attack_hitbox;
mod posed_body;

#[cfg(test)]
mod tests;

pub use anim::{pick_actor_anim, pick_player_anim, ActorAnimState};
pub use posed_body::{
    authored_body_pixel_size, posed_body_geometry, sync_sprite_posed_bodies, PosedBodyGeometry,
};
// ⚠ this re-exported four names under an `#[allow(unused_imports)]` whose stated
// reason was *"player_attack_hitbox_world is the live consumer (the debug-overlay
// hitbox source)"*. Measured 2026-08-08: that symbol has NO consumer anywhere in
// the workspace — it appears only in its own definition, this re-export, and its
// own tests, and the same was true of `actor_attack_hitbox_world` and
// `manifest_attack_hitbox_world`. The waiver was silencing the compiler with a
// citation that named a caller which does not exist.
//
// ⭐ only `authored_attack_volume_resolver` is reached from outside, by
// `ambition_platformer2d_runtime::combat_schedule` — which sits ABOVE this crate,
// so `attack_hitbox` answers upward and nothing in the monolith names it. That
// one-way edge is why this module is the region's cleanest extraction candidate;
// the other three names were making its surface look four times as entangled as
// it is. The functions stay `pub` for the module's own tests.
#[allow(
    unused_imports,
    reason = "sheet_for_character_id is the public catalog→spec entry; consumed by tests under content::character_catalog::tests (not by non-test crate code today). Public surface for future spawn-site callers."
)]
pub use assets::{
    all_character_sprite_filenames_in, build_npc_sprite_asset, build_prop_sprite_asset,
    build_prop_sprite_asset_packed, character_sprite_tier, load_character_sprites_in,
    load_prop_sheet_for_target, materialize_declared_character_sprite,
    portrait_for_declared_character, sheet_for_character_id_in, sheet_for_declared_character,
    sprite_body_collision_for_character_id_in, SpriteMaterialization,
};
pub use attack_hitbox::authored_attack_volume_resolver;
