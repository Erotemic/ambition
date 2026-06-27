//! Texture-atlas layout for a character sheet: frame/row rects and the
//! `CharacterSheetSpec` atlas accessor methods.
//!
//! Split out of the former 780-line `sheets/mod.rs` (2026-06-15).

use super::*;

impl CharacterSheetSpec {
    fn row_index(&self, anim: CharacterAnim) -> Option<usize> {
        self.rows.iter().position(|(row_anim, _)| *row_anim == anim)
    }

    pub fn resolve_anim(&self, anim: CharacterAnim) -> CharacterAnim {
        if self.row_index(anim).is_some() {
            return anim;
        }
        if matches!(anim, CharacterAnim::LedgeClimb)
            && self.row_index(CharacterAnim::LedgeGrab).is_some()
        {
            return CharacterAnim::LedgeGrab;
        }
        CharacterAnim::Idle
    }

    pub(crate) fn row(&self, anim: CharacterAnim) -> &AnimRow {
        let resolved = self.resolve_anim(anim);
        let idx = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        &self.rows[idx].1
    }

    /// Number of distinct page images this sheet addresses. `1` for the
    /// common single-PNG case; larger when the generator split the animation
    /// rows across several page images to stay within the GPU texture limit.
    pub fn page_count(&self) -> u32 {
        self.rows
            .iter()
            .map(|(_, row)| row.page)
            .max()
            .map(|p| p + 1)
            .unwrap_or(1)
    }

    /// Which page image the given animation's frames live in (after
    /// `resolve_anim` fallback). `0` for single-page sheets.
    pub fn page_of(&self, anim: CharacterAnim) -> u32 {
        let resolved = self.resolve_anim(anim);
        self.row_index(resolved)
            .map(|idx| self.rows[idx].1.page)
            .unwrap_or(0)
    }

    /// Build the atlas layout for one page image. Each page is its own
    /// texture, so only the rows on that page contribute cells, and the
    /// flat index returned by [`Self::flat_index`] is page-local (it counts
    /// only same-page rows before it). Accounts for `y_offset` so multiple
    /// specs can still share one PNG (e.g. lab-props) within a page.
    pub fn build_atlas_for_page(&self, page: u32) -> TextureAtlasLayout {
        // Atlas image size has to cover every cell on this page — derive it
        // from the rects when we have them (so inter-frame padding is
        // included), and fall back to grid math (cells = frame_w × frame_h,
        // label inset on the left) otherwise.
        let (total_w, total_h) = atlas_extent_for_page(self, page);
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w.max(1), total_h.max(1)));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (_, row) in self.rows.iter().filter(|(_, r)| r.page == page) {
            // Authoritative path: use the RON's per-frame rects. The
            // generator emits the EXACT pixel coords of every frame
            // (including padding between cells), so any drift caused
            // by inter-frame padding, label-column width changes, or
            // row-stride ≠ frame_height vanishes.
            if let Some(rects) = row.frame_rects.as_ref() {
                for r in rects.iter().take(row.frame_count) {
                    layout.add_texture(inset_rect(*r, inset));
                }
                continue;
            }
            // Legacy path: grid math, using the AUTHORED `row_index`
            // so dropping intermediate rows doesn't shift later rows
            // upward into the wrong band of pixels.
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = self.y_offset + row.row_index * self.frame_height;
                let cell = URect {
                    min: UVec2::new(x, y),
                    max: UVec2::new(x + self.frame_width, y + self.frame_height),
                };
                layout.add_texture(inset_rect(cell, inset));
            }
        }
        layout
    }

    /// Build the atlas layout for page 0. Convenience for single-page
    /// callers and the initial sprite build; multi-page consumers call
    /// [`Self::build_atlas_for_page`] once per page.
    pub fn build_atlas(&self) -> TextureAtlasLayout {
        self.build_atlas_for_page(0)
    }

    /// Page-local flat atlas index for `(anim, frame)`: the position of this
    /// frame among all frames on the *same page*, in spec order. For a
    /// single-page sheet (every row on page 0) this is identical to the old
    /// global index, so existing sheets are unaffected. The returned index
    /// addresses the layout built by `build_atlas_for_page(page_of(anim))`.
    pub fn flat_index(&self, anim: CharacterAnim, frame: usize) -> usize {
        let resolved = self.resolve_anim(anim);
        let row = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        let page = self.rows[row].1.page;
        let frames_before: usize = self.rows[..row]
            .iter()
            .filter(|(_, r)| r.page == page)
            .map(|(_, r)| r.frame_count)
            .sum();
        let max_frame = self.rows[row].1.frame_count.saturating_sub(1);
        frames_before + frame.min(max_frame)
    }

    /// Pixel extent of page 0's atlas texture addressed by this sheet spec.
    ///
    /// Custom sprite materials use this to convert a flat atlas frame index
    /// into normalized UVs without depending on Bevy's private
    /// `TextureAtlasLayout` internals. The calculation intentionally matches
    /// [`Self::build_atlas`]. Multi-page consumers should pass an explicit
    /// page via [`Self::atlas_texture_size_for_page`].
    pub fn atlas_texture_size(&self) -> UVec2 {
        self.atlas_texture_size_for_page(0)
    }

    /// Pixel extent of one page's atlas texture.
    pub fn atlas_texture_size_for_page(&self, page: u32) -> UVec2 {
        let (w, h) = atlas_extent_for_page(self, page);
        UVec2::new(w.max(1), h.max(1))
    }

    /// Return the inset pixel rect for a flat atlas index.
    ///
    /// This mirrors [`Self::build_atlas`]'s rect insertion order: rows are
    /// concatenated in spec order, and each row contributes `frame_count`
    /// rects. It gives custom materials the same frame crop used by the
    /// ordinary Bevy `Sprite` path, including the bilinear-filtering inset.
    pub fn texture_rect_for_flat_index(&self, index: usize) -> Option<URect> {
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        let mut flat = 0usize;
        for (_, row) in self.rows.iter() {
            if let Some(rects) = row.frame_rects.as_ref() {
                for rect in rects.iter().take(row.frame_count) {
                    if flat == index {
                        return Some(inset_rect(*rect, inset));
                    }
                    flat += 1;
                }
                continue;
            }
            for col in 0..row.frame_count {
                let x = self.label_width + col as u32 * self.frame_width;
                let y = self.y_offset + row.row_index * self.frame_height;
                let cell = URect {
                    min: UVec2::new(x, y),
                    max: UVec2::new(x + self.frame_width, y + self.frame_height),
                };
                if flat == index {
                    return Some(inset_rect(cell, inset));
                }
                flat += 1;
            }
        }
        None
    }

    pub fn frame_count(&self, anim: CharacterAnim) -> usize {
        self.row(anim).frame_count
    }

    pub fn frame_duration(&self, anim: CharacterAnim) -> f32 {
        self.row(anim).duration_secs
    }
}

/// Compute the atlas image extent (width, height) covering every cell on
/// `page`, whether the spec carries per-frame rects (preferred) or only grid
/// metadata. The atlas must be at least as large as the underlying page PNG so
/// URect coords don't overflow. For single-page sheets `page == 0` covers
/// every row, matching the previous whole-sheet extent.
fn atlas_extent_for_page(spec: &CharacterSheetSpec, page: u32) -> (u32, u32) {
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any_rect = false;
    for (_, row) in spec.rows.iter().filter(|(_, r)| r.page == page) {
        if let Some(rects) = row.frame_rects.as_ref() {
            for r in rects.iter().take(row.frame_count) {
                max_x = max_x.max(r.max.x);
                max_y = max_y.max(r.max.y);
                any_rect = true;
            }
        }
    }
    if any_rect {
        return (max_x, max_y);
    }
    // Grid fallback — same shape as the previous build_atlas extent
    // math (now informed by AUTHORED row_index, so dropped rows
    // don't shrink the y-coverage). Grid sheets are always single-page,
    // so the page filter is a no-op here.
    let max_frames = spec
        .rows
        .iter()
        .filter(|(_, r)| r.page == page)
        .map(|(_, row)| row.frame_count)
        .max()
        .unwrap_or(0) as u32;
    let max_row_index_plus_one = spec
        .rows
        .iter()
        .filter(|(_, r)| r.page == page)
        .map(|(_, row)| row.row_index)
        .max()
        .map(|i| i + 1)
        .unwrap_or(0);
    let w = spec.label_width + max_frames * spec.frame_width;
    let h = spec.y_offset + max_row_index_plus_one * spec.frame_height;
    (w, h)
}

/// Shrink a cell by `inset` on every side so bilinear filtering at
/// the seam can't pull pixels from neighboring cells. Saturating
/// math keeps a tiny cell from inverting (min > max) on a pathological
/// inset.
fn inset_rect(r: URect, inset: u32) -> URect {
    URect {
        min: UVec2::new(r.min.x + inset, r.min.y + inset),
        max: UVec2::new(
            r.max.x.saturating_sub(inset).max(r.min.x + 1),
            r.max.y.saturating_sub(inset).max(r.min.y + 1),
        ),
    }
}
