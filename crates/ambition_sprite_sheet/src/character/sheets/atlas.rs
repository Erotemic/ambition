//! Character-sheet atlas accessors.
//!
//! Every pixel query — which page a frame lives on, the page's atlas cells, the
//! page-local flat index, the per-frame trim — delegates to the shared
//! [`ambition_sprite_sheet`] frame algebra on the underlying [`SheetRecord`].
//! This file only maps the typed [`CharacterAnim`] (with its
//! `LedgeClimb→LedgeGrab`, unknown→`Idle` fallbacks) onto a `record.rows` index
//! and then calls that one implementation.

use super::*;
use bevy::image::TextureAtlasLayout;

/// Turn an [`AtlasPage`] (the shared algebra's page cells) into a Bevy
/// [`TextureAtlasLayout`]. The only place the frame algebra meets Bevy's render
/// type; every reader — character, boss, prop/effect — funnels through here so
/// the record→layout step exists once.
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
        // Render the most-specific pose in THIS actor's anim set — the rows the
        // sprite generator wrote into the manifest ([`Self::maps`]). Walk the
        // structural pose taxonomy toward the base until the sheet has a row for
        // it; `Idle` (guaranteed present) is the floor. So a body can be driven
        // into any state by its brain and the sheet decides how richly it reads,
        // without ever snapping to `Idle` for a pose it has a relative of.
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

    /// Distinct page images this sheet addresses (`1` for the common case).
    pub fn page_count(&self) -> u32 {
        self.record.page_count()
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
