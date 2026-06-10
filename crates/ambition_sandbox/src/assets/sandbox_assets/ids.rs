//! Stable [`AssetId`] constructors for the fixed-vocabulary sandbox
//! assets.
//!
//! Bulk per-enum entries (entity sprites, parallax layers) have their
//! own builders in `game_assets.rs`; the music helper here stays
//! generic because music ids come from the RON catalog at runtime.

use ambition_asset_manager::AssetId;

pub const SANDBOX_LDTK: &str = "world.sandbox_ldtk";
pub const INTRO_LDTK: &str = "world.intro_ldtk";
pub const CUT_ROPE_LDTK: &str = "world.cut_rope_ldtk";
pub const SANDBOX_DATA: &str = "data.sandbox";
pub const SFX_BANK: &str = "audio.sfx_bank";
pub const FONT_DIALOG_REGULAR: &str = "font.dialog_regular";
pub const FONT_DIALOG_SEMIBOLD: &str = "font.dialog_semibold";
pub const FONT_DEBUG_MONO: &str = "font.debug_mono";

pub fn sandbox_ldtk() -> AssetId {
    AssetId::new(SANDBOX_LDTK)
}
pub fn intro_ldtk() -> AssetId {
    AssetId::new(INTRO_LDTK)
}
pub fn cut_rope_ldtk() -> AssetId {
    AssetId::new(CUT_ROPE_LDTK)
}
pub fn sandbox_data() -> AssetId {
    AssetId::new(SANDBOX_DATA)
}
pub fn sfx_bank() -> AssetId {
    AssetId::new(SFX_BANK)
}
pub fn font_dialog_regular() -> AssetId {
    AssetId::new(FONT_DIALOG_REGULAR)
}
pub fn font_dialog_semibold() -> AssetId {
    AssetId::new(FONT_DIALOG_SEMIBOLD)
}
pub fn font_debug_mono() -> AssetId {
    AssetId::new(FONT_DEBUG_MONO)
}

/// `music.track.<id>` where `id` is the
/// [`crate::runtime::data::MusicTrackSpec::id`] authored in
/// `sandbox.ron`. The runtime registers one catalog entry per track
/// and looks them up by this id.
pub fn music_track(track_id: &str) -> AssetId {
    AssetId::new(format!("music.track.{track_id}"))
}

/// `sprite.character.<name>` for a character spritesheet. `name` is
/// the sandbox-side label used by
/// `crate::presentation::character_sprites::assets` (e.g. `player`,
/// `robot`, `goblin`, or an NPC sprite key derived from the LDtk
/// `NpcSpawn.name` field).
pub fn character_sprite(name: &str) -> AssetId {
    AssetId::new(format!("sprite.character.{name}"))
}

/// `sprite.boss.<name>` for a boss spritesheet.
pub fn boss_sprite(name: &str) -> AssetId {
    AssetId::new(format!("sprite.boss.{name}"))
}
