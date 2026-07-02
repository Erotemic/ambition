//! Per-domain manifest builders, split by content type.
//!
//! Each submodule owns one slice of authored data:
//!
//! - [`world`] — LDtk world files (`sandbox.ldtk`, `intro.ldtk`,
//!   `you_have_to_cut_the_rope.ldtk`) + the
//!   sandbox tuning RON (`data.sandbox`).
//! - [`audio`] — packed SFX bank + per-track music entries.
//! - [`visuals`] — UI fonts + character / boss / intro spritesheets.
//!
//! The shared [`with_embedded_core_candidate`] helper is here because
//! both [`visuals::extend_with_font_entries`] and
//! [`visuals::extend_with_character_entries`] use it to attach an
//! `EmbeddedBinary` candidate when the `static_core_assets` feature
//! is on.
//!
//! `build_sandbox_catalog` (in `sandbox_assets/mod.rs`) calls each
//! `extend_with_*` helper in turn; adding a new asset slice is an
//! edit to the matching submodule rather than scrolling through one
//! 300-line `builders.rs`.

use ambition_asset_manager::AssetEntry;
#[cfg(feature = "static_core_assets")]
use ambition_asset_manager::{AssetLocation, AssetSourceProfile};

mod audio;
mod visuals;
mod world;

pub(super) use audio::{extend_with_music_entries, extend_with_sfx_bank_entry};
pub(super) use visuals::{
    extend_with_boss_entries, extend_with_character_entries, extend_with_font_entries,
    extend_with_sprite_pack_entries,
};
pub(super) use world::{extend_with_data_entries, extend_with_world_entries};

/// Attach an `EmbeddedBinary` `LocationCandidate` to `entry` IFF the
/// `static_core_assets` feature is enabled. Without the feature the
/// embedded source has no bytes for the URL, so adding the candidate
/// would mislead the resolver into trying to load a 404.
#[cfg(feature = "static_core_assets")]
pub(super) fn with_embedded_core_candidate(
    entry: AssetEntry,
    embedded_url: &'static str,
) -> AssetEntry {
    entry.with_location(
        AssetSourceProfile::EmbeddedBinary,
        AssetLocation::embedded(embedded_url.to_string()),
    )
}

#[cfg(not(feature = "static_core_assets"))]
pub(super) fn with_embedded_core_candidate(
    entry: AssetEntry,
    _embedded_url: &'static str,
) -> AssetEntry {
    entry
}
