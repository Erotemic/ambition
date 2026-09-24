//! Character-sheet atlas accessors.
//!
//! Every pixel query (frame page, page atlas cells, page-local flat index,
//! per-frame trim) delegates to the shared frame algebra on the underlying
//! [`SheetRecord`]. This file only maps the typed [`CharacterAnim`] (with the
//! `LedgeClimb→LedgeGrab` and unknown→`Idle` fallbacks) to a `record.rows`
//! index.

use super::*;
use bevy::image::TextureAtlasLayout;

/// Turn an [`AtlasPage`] (the shared algebra's page cells) into a Bevy
/// [`TextureAtlasLayout`]. This is the only place the frame algebra meets
/// Bevy's render type; every reader (character, boss, prop/effect) uses it.
pub fn build_atlas_layout(page: &AtlasPage) -> TextureAtlasLayout {
    let extent = page.extent.max(UVec2::ONE);
    let mut layout = TextureAtlasLayout::new_empty(extent);
    for rect in &page.rects {
        layout.add_texture(*rect);
    }
    layout
}

impl CharacterSheetSpec {
    /// `record.rows` index a mapped [`CharacterAnim`] resolves to.
    fn record_row(&self, anim: CharacterAnim) -> usize {
        let resolved = self.resolve_anim(anim);
        self.anim_rows
            .iter()
            .find(|(a, _)| *a == resolved)
            .map(|(_, idx)| *idx)
            .expect("character sprite sheet must define an Idle row")
    }

    /// True when this sheet maps `anim` to a row (after no fallback).
    pub fn maps(&self, anim: CharacterAnim) -> bool {
        self.anim_rows.iter().any(|(a, _)| *a == anim)
    }

    /// The [`CharacterAnim`]s this sheet maps, in row order (diagnostics).
    pub fn mapped_anims(&self) -> impl Iterator<Item = CharacterAnim> + '_ {
        self.anim_rows.iter().map(|(a, _)| *a)
    }

    pub fn resolve_anim(&self, anim: CharacterAnim) -> CharacterAnim {
        // Draw the most specific pose in this actor's anim set (the rows in
        // the manifest, [`Self::maps`]). Walk the pose taxonomy toward the base
        // until the sheet has a row; `Idle` is always present. So the sheet
        // decides how richly a state reads, and never snaps to `Idle` for a
        // pose that has a relative.
        let mut cur = anim;
        loop {
            if self.maps(cur) {
                return cur;
            }
            match cur.base_pose() {
                Some(next) => cur = next,
                None => return CharacterAnim::Idle,
            }
        }
    }

    /// Per-row timing for the animator (frame count + per-frame duration).
    pub(crate) fn row(&self, anim: CharacterAnim) -> RowInfo {
        let row = &self.record.rows[self.record_row(anim)];
        RowInfo {
            frame_count: row.frame_count as usize,
            duration_secs: row.duration_secs,
        }
    }

    /// How long one pass of `anim` takes to draw, in the clock the animator
    /// is ticked by ([`super::super::animator::CharacterAnimator::tick`]).
    ///
    /// Resolved through the anim set like the drawing, so this is the length
    /// of the clip that plays. A sheet that falls back from `Transform` to a
    /// 1-frame `Idle` gives the idle's length. Use this, not a hand-copied
    /// frame table, to hold a pose long enough to be seen.
    pub fn clip_seconds(&self, anim: CharacterAnim) -> f32 {
        let row = self.row(anim);
        row.frame_count as f32 * row.duration_secs
    }

    /// Distinct page images this sheet addresses (`1` for the common case).
    pub fn page_count(&self) -> u32 {
        self.record.page_count()
    }

    /// The pages this sheet's frames draw from: a sparse subset of
    /// `0..page_count()` for a target in a shared pack. Loaders need this, not
    /// the count (see [`SheetRecord::used_pages`]).
    pub fn used_pages(&self) -> std::collections::BTreeSet<u32> {
        self.record.used_pages()
    }

    /// Which page image `(anim, frame)` draws from (per-frame: a freely-packed
    /// sheet can scatter one animation across pages).
    pub fn page_of(&self, anim: CharacterAnim, frame: usize) -> u32 {
        self.record.frame_page_of(self.record_row(anim), frame)
    }

    /// Build the atlas layout for one page image — only the frames on that page
    /// contribute cells, and [`Self::flat_index`] returns a page-local index
    /// that addresses them.
    pub fn build_atlas_for_page(&self, page: u32) -> TextureAtlasLayout {
        build_atlas_layout(&self.record.atlas_page(page, self.frame_sample_inset))
    }

    /// Build the atlas layout for page 0 — the single-page / initial-build case.
    pub fn build_atlas(&self) -> TextureAtlasLayout {
        self.build_atlas_for_page(0)
    }

    /// Page-local flat atlas index for `(anim, frame)`.
    pub fn flat_index(&self, anim: CharacterAnim, frame: usize) -> usize {
        self.record.flat_index_in_page(self.record_row(anim), frame)
    }

    /// The row slot an authored clip chain resolves to on this sheet.
    ///
    /// The lookups above use [`CharacterAnim`], and fighter sheets have rows
    /// with no variant (`smash_forward`, `air_dodge`, `tumble`, `knockdown`,
    /// `tech_roll`). This is the row-keyed path.
    ///
    /// `CharacterAnim` stays the body-state vocabulary and the fallback: when
    /// the sheet has none of the chain, the caller asks for a pose.
    pub fn clip_slot<'a>(&self, chain: impl IntoIterator<Item = &'a str>) -> Option<usize> {
        self.record.first_bound_row(chain).map(|bound| bound.slot())
    }

    /// Per-row timing for a slot resolved by [`Self::clip_slot`].
    pub(crate) fn row_at(&self, slot: usize) -> RowInfo {
        let row = &self.record.rows[slot];
        RowInfo {
            frame_count: row.frame_count as usize,
            duration_secs: row.duration_secs,
        }
    }

    /// Page-local flat atlas index for a slot resolved by [`Self::clip_slot`].
    pub fn flat_index_at(&self, slot: usize, frame: usize) -> usize {
        self.record.flat_index_in_page(slot, frame)
    }

    /// Trim geometry for a slot resolved by [`Self::clip_slot`].
    ///
    /// The row-keyed form of [`Self::frame_trim`]. While an authored clip
    /// plays, the renderer must use the clip's trim, not the trim of the
    /// semantic pose that `current` still holds.
    pub fn frame_trim_at(&self, slot: usize, frame: usize) -> FrameTrim {
        self.record.frame_trim(slot, frame)
    }

    /// Which page image a clip slot's frame draws from: the row-keyed form of
    /// [`Self::page_of`].
    pub fn page_of_at(&self, slot: usize, frame: usize) -> u32 {
        self.record.frame_page_of(slot, frame)
    }

    /// Pixel extent of page 0's atlas texture (custom-material UV helper).
    pub fn atlas_texture_size(&self) -> UVec2 {
        self.atlas_texture_size_for_page(0)
    }

    /// Pixel extent of one page's atlas texture.
    pub fn atlas_texture_size_for_page(&self, page: u32) -> UVec2 {
        self.record
            .atlas_page(page, self.frame_sample_inset)
            .extent
            .max(UVec2::ONE)
    }

    /// Inset pixel rect for a page-0 flat atlas index (custom-material UV helper).
    pub fn texture_rect_for_flat_index(&self, index: usize) -> Option<URect> {
        self.record
            .atlas_page(0, self.frame_sample_inset)
            .rects
            .get(index)
            .copied()
    }

    /// True when any frame of this sheet was alpha-trimmed (renderer must adjust
    /// size + anchor per frame). False for legacy uniform sheets.
    pub fn is_trimmed(&self) -> bool {
        self.record.is_trimmed()
    }

    /// Trim geometry for `(anim, frame)`.
    pub fn frame_trim(&self, anim: CharacterAnim, frame: usize) -> FrameTrim {
        self.record.frame_trim(self.record_row(anim), frame)
    }

    pub fn frame_count(&self, anim: CharacterAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn frame_duration(&self, anim: CharacterAnim) -> f32 {
        self.row(anim).duration_secs
    }
}
