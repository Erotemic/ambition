//! Character sprite loading, atlas residency, and catalog-to-sheet resolution.
//!
//! Pure pose geometry and animation selection belong to `ambition_character_sprites`.
//! Sheet vocabulary belongs to `ambition_sprite_sheet`.

mod assets;
pub mod rigged;

#[cfg(test)]
mod tests;

#[allow(
    unused_imports,
    reason = "sheet_for_character_id is the public catalog→spec entry; consumed by tests under content::character_catalog::tests (not by non-test crate code today). Public surface for future spawn-site callers."
)]
pub use assets::{
    build_npc_sprite_asset, build_prop_sprite_asset,
    build_prop_sprite_asset_packed, character_sprite_tier, load_character_sprites_in,
    demand_character_fx_sheets, ensure_fx_sheet_loaded, load_fx_sheets, load_prop_sheet_for_target, materialize_declared_character_sprite,
    portrait_for_declared_character, sheet_for_character_id_in, sheet_for_prepared_character,
    sprite_body_collision_for_character_id_in, SpriteMaterialization,
};
