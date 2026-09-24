//! Record-to-atlas plumbing for animated-prop and effect visuals (the shrine
//! obelisk, the `robot_slash` melee effect, and so on).
//!
//! Thin name-keyed adapters over the frame algebra in [`ambition_sprite_sheet`].
//! An effect addresses its sheet by animation name; these resolve the name to
//! a record row and delegate the pixel work (atlas cells, flat index) to the
//! shared `SheetRecord` methods, the same implementation the character and
//! boss readers use.
//!
//! Effect sheets are single-page and untrimmed by policy (see the renderer's
//! pack-group classification), so page 0 and the page-local flat index are
//! enough. An effect that needs paging or trim moves to the
//! `CharacterAnimator` path.
//!
//! Name resolution goes through the binding boundary: a miss is recorded in
//! the caller's [`BindingLedger`] and the caller reports it. The visible
//! fallback still draws.

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
/// One struct, not three name-keyed lookups: the three facts come from the
/// same row, so a caller cannot mix one row's start with another's frame
/// count.
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
/// `declared_by` names the visual asking, so the report reads "unknown anim
/// row `activate` declared by `shrine visual`".
pub(crate) fn row_playback(
    record: &SheetRecord,
    animation: &str,
    declared_by: &str,
    ledger: &mut BindingLedger,
) -> Option<RowPlayback> {
    let rows = record.anim_rows();
    // Two rows with the same name resolve to the first, so the second row
    // is unreachable without any error. Report duplicates.
    ledger.note_duplicates(&rows, format!("sheet `{}`", record.key));
    let bound = ledger.resolve(&rows, &AnimRowRef::new(animation), declared_by)?;
    let row = record.row(&bound);
    Some(RowPlayback {
        start: record.flat_index_in_page(bound.slot(), 0),
        frames: (row.frame_count as usize).max(1),
        frame_duration: row.duration_secs.max(0.001),
    })
}
