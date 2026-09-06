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
//!
//! Name resolution goes through the binding boundary. Now a miss is recorded in the caller's
//! [`BindingLedger`] and the caller reports it; the visible fallback stays, but the run also says
//! what it could not find.

use ambition_platformer2d_shared_tangle::binding::BindingLedger;
use ambition_sprite_sheet::character::build_atlas_layout;
use ambition_sprite_sheet::{AnimRowRef, SheetRecord};
use bevy::image::TextureAtlasLayout;

/// Per-frame inset (px) trimmed off each atlas cell to avoid neighbour bleed
/// when the sprite is scaled. One pixel is enough at our frame sizes.
const FRAME_INSET: u32 = 1;

/// Everything needed to play one row of an effect sheet: where its frames start
/// in the flat atlas, how many there are, and how long each is held.
///
/// One struct rather than three name-keyed lookups, because the three facts come
/// from the same row and a caller that resolves the name once cannot end up
/// mixing row `up`'s start with row `down`'s frame count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RowPlayback {
    pub(crate) start: usize,
    pub(crate) frames: usize,
    pub(crate) frame_duration: f32,
}

/// Build a [`TextureAtlasLayout`] whose cells are the record's page-0 frames in
/// flat row-major order — the order [`RowPlayback::start`] counts in.
pub(crate) fn atlas_layout_from_record(record: &SheetRecord) -> TextureAtlasLayout {
    build_atlas_layout(&record.atlas_page(0, FRAME_INSET))
}

/// Resolve `animation` against `record`'s rows, recording a miss in `ledger`.
///
/// `declared_by` names the visual asking, so the report reads
/// "unknown anim row `activate` declared by `shrine visual`" rather than leaving
/// a reader to guess which of a dozen effect sheets is wrong.
pub(crate) fn row_playback(
    record: &SheetRecord,
    animation: &str,
    declared_by: &str,
    ledger: &mut BindingLedger,
) -> Option<RowPlayback> {
    let rows = record.anim_rows();
    // A regenerated sheet with two rows called `idle` resolves to the first and
    // draws fine, so nothing ever complained — while the second row, and every
    // frame in it, was unreachable.
    ledger.note_duplicates(&rows, format!("sheet `{}`", record.key));
    let bound = ledger.resolve(&rows, &AnimRowRef::new(animation), declared_by)?;
    let row = record.row(&bound);
    Some(RowPlayback {
        start: record.flat_index_in_page(bound.slot(), 0),
        frames: (row.frame_count as usize).max(1),
        frame_duration: row.duration_secs.max(0.001),
    })
}
