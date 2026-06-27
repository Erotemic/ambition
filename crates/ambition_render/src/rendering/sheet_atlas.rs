//! Record → atlas plumbing for animated-prop / effect visuals (the shrine
//! obelisk, the `robot_slash` melee effect, …).
//!
//! These are thin name-keyed adapters over the ONE frame algebra in
//! [`ambition_sprite_sheet`]: an effect addresses its sheet by animation name,
//! so these resolve the name to a record row and then delegate the pixel work
//! (atlas cells, flat index) to the shared `SheetRecord` methods. No frame-rect
//! or inset math lives here anymore — it's the same implementation the
//! character and boss readers use, so a regenerated (or future packed) effect
//! sheet flows through unchanged.
//!
//! Effect sheets are single-page and untrimmed by policy (see the renderer's
//! pack-group classification), so page 0 + the page-local flat index is the
//! whole story here; if an effect ever needs paging/trim it graduates to the
//! `CharacterAnimator` path that already drives both.

use ambition_gameplay_core::character_sprites::build_atlas_layout;
use ambition_sprite_sheet::SheetRecord;
use bevy::image::TextureAtlasLayout;

/// Per-frame inset (px) trimmed off each atlas cell to avoid neighbour bleed
/// when the sprite is scaled. One pixel is enough at our frame sizes.
const FRAME_INSET: u32 = 1;

/// Build a [`TextureAtlasLayout`] whose cells are the record's page-0 frames in
/// flat row-major order — the order [`row_start_index`] counts in.
pub(crate) fn atlas_layout_from_record(record: &SheetRecord) -> TextureAtlasLayout {
    build_atlas_layout(&record.atlas_page(0, FRAME_INSET))
}

/// Flat atlas index of the first frame of `animation`, or `None` if the sheet
/// has no such row.
pub(crate) fn row_start_index(record: &SheetRecord, animation: &str) -> Option<usize> {
    let row = record.row_index_of(animation)?;
    Some(record.flat_index_in_page(row, 0))
}

pub(crate) fn row_frame_count(record: &SheetRecord, animation: &str) -> Option<usize> {
    record
        .rows
        .iter()
        .find(|row| row.animation == animation)
        .map(|row| row.frame_count as usize)
}

pub(crate) fn row_duration(record: &SheetRecord, animation: &str) -> Option<f32> {
    record
        .rows
        .iter()
        .find(|row| row.animation == animation)
        .map(|row| row.duration_secs)
}
