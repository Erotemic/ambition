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
    /// common single-PNG case; larger when the generator split the sheet
    /// across several page images to stay within the GPU texture limit.
    pub fn page_count(&self) -> u32 {
        self.rows
            .iter()
            .map(|(_, row)| {
                let frame_max = row
                    .frame_pages
                    .as_ref()
                    .and_then(|p| p.iter().copied().max())
                    .unwrap_or(0);
                row.page.max(frame_max)
            })
            .max()
            .map(|p| p + 1)
            .unwrap_or(1)
    }

    /// Which page image `(anim, frame)` lives in (after `resolve_anim`
    /// fallback). `0` for single-page sheets. Per-frame because a freely-packed
    /// sheet can place frames of one animation on different pages.
    pub fn page_of(&self, anim: CharacterAnim, frame: usize) -> u32 {
        let resolved = self.resolve_anim(anim);
        match self.row_index(resolved) {
            Some(idx) => {
                let row = &self.rows[idx].1;
                frame_page_of(row, frame.min(row.frame_count.saturating_sub(1)))
            }
            None => 0,
        }
    }

    /// Build the atlas layout for one page image. Each page is its own texture,
    /// so only the FRAMES on that page contribute cells (a packed animation may
    /// span pages), and [`Self::flat_index`] returns a page-local index. Frames
    /// are added in `(row, frame)` order so the index matches. Accounts for
    /// `y_offset` so multiple specs can still share one PNG within a page.
    pub fn build_atlas_for_page(&self, page: u32) -> TextureAtlasLayout {
        let (total_w, total_h) = atlas_extent_for_page(self, page);
        let mut layout = TextureAtlasLayout::new_empty(UVec2::new(total_w.max(1), total_h.max(1)));
        let inset = self
            .frame_sample_inset
            .min(self.frame_width.min(self.frame_height) / 4);
        for (_, row) in self.rows.iter() {
            // Authoritative path: the RON's exact per-frame rects, filtered to
            // this page (frame by frame).
            if let Some(rects) = row.frame_rects.as_ref() {
                for (f, r) in rects.iter().take(row.frame_count).enumerate() {
                    if frame_page_of(row, f) == page {
                        layout.add_texture(inset_rect(*r, inset));
                    }
                }
                continue;
            }
            // Legacy grid path: a grid sheet is never packed, so the whole row
            // shares `row.page`. Add its cells only when this is that page.
            if row.page != page {
                continue;
            }
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
    /// frame among all frames on the *same page*, in `(row, frame)` order. For
    /// a single-page sheet (every frame on page 0) this is the old global
    /// index, so existing sheets are unaffected. Addresses the layout built by
    /// `build_atlas_for_page(page_of(anim, frame))`.
    pub fn flat_index(&self, anim: CharacterAnim, frame: usize) -> usize {
        let resolved = self.resolve_anim(anim);
        let row_idx = self
            .row_index(resolved)
            .expect("character sprite sheet must define an Idle row");
        let row = &self.rows[row_idx].1;
        let f = frame.min(row.frame_count.saturating_sub(1));
        let page = frame_page_of(row, f);
        let mut count = 0usize;
        for (_, r) in self.rows[..row_idx].iter() {
            for g in 0..r.frame_count {
                if frame_page_of(r, g) == page {
                    count += 1;
                }
            }
        }
        for g in 0..f {
            if frame_page_of(row, g) == page {
                count += 1;
            }
        }
        count
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

    /// True when any frame of this sheet was alpha-trimmed by the atlas packer
    /// (so the renderer must adjust the sprite size + anchor per frame). False
    /// for legacy uniform sheets, which keep the cheap fixed-anchor path.
    pub fn is_trimmed(&self) -> bool {
        self.rows.iter().any(|(_, r)| r.frame_offsets.is_some())
    }

    /// Trim geometry for `(anim, frame)`: the trimmed rect size + its offset
    /// within the logical `frame_width`×`frame_height` frame. Untrimmed frames
    /// report offset `(0,0)` and the full logical size.
    pub fn frame_trim(&self, anim: CharacterAnim, frame: usize) -> FrameTrim {
        let logical = UVec2::new(self.frame_width, self.frame_height);
        let resolved = self.resolve_anim(anim);
        let Some(idx) = self.row_index(resolved) else {
            return FrameTrim { offset: IVec2::ZERO, trimmed: logical, logical };
        };
        let row = &self.rows[idx].1;
        let f = frame.min(row.frame_count.saturating_sub(1));
        let trimmed = row
            .frame_rects
            .as_ref()
            .and_then(|r| r.get(f))
            .map(|r| UVec2::new(r.width(), r.height()))
            .unwrap_or(logical);
        let offset = row
            .frame_offsets
            .as_ref()
            .and_then(|o| o.get(f))
            .copied()
            .unwrap_or(IVec2::ZERO);
        FrameTrim { offset, trimmed, logical }
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
    for (_, row) in spec.rows.iter() {
        if let Some(rects) = row.frame_rects.as_ref() {
            for (f, r) in rects.iter().take(row.frame_count).enumerate() {
                if frame_page_of(row, f) == page {
                    max_x = max_x.max(r.max.x);
                    max_y = max_y.max(r.max.y);
                    any_rect = true;
                }
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

/// Page image index for frame `f` of `row`: the per-frame page when the sheet
/// was freely packed, else the row's page (unpacked / grid / unpacked-multipage).
fn frame_page_of(row: &AnimRow, f: usize) -> u32 {
    row.frame_pages
        .as_ref()
        .and_then(|p| p.get(f))
        .copied()
        .unwrap_or(row.page)
}

#[cfg(test)]
mod per_frame_page_tests {
    use super::*;

    fn row(page: u32, frame_pages: Vec<u32>, n: usize) -> AnimRow {
        // Distinct dummy rects per frame so the atlas layout has the right len.
        let rects = (0..n)
            .map(|i| URect {
                min: UVec2::new(i as u32 * 10, 0),
                max: UVec2::new(i as u32 * 10 + 8, 8),
            })
            .collect();
        AnimRow {
            frame_count: n,
            duration_secs: 0.1,
            row_index: 0,
            page,
            frame_rects: Some(rects),
            frame_offsets: None,
            frame_pages: Some(frame_pages),
        }
    }

    fn spec(rows: Vec<(CharacterAnim, AnimRow)>) -> CharacterSheetSpec {
        CharacterSheetSpec {
            label_width: 0,
            y_offset: 0,
            frame_width: 8,
            frame_height: 8,
            page_images: vec!["a.png".into(), "b.png".into()],
            rows,
            collision_scale: 1.0,
            feet_anchor_y: -0.5,
            frame_sample_inset: 0,
        }
    }

    /// `flat_index` must be a page-local index that exactly addresses the layout
    /// `build_atlas_for_page` produces — even when one animation's frames are
    /// scattered across pages by the free packer.
    #[test]
    fn flat_index_agrees_with_per_page_layout() {
        // Idle: frame0→page0, frame1→page1. Walk: frame0→page1, frame1→page0.
        let s = spec(vec![
            (CharacterAnim::Idle, row(0, vec![0, 1], 2)),
            (CharacterAnim::Walk, row(1, vec![1, 0], 2)),
        ]);
        assert_eq!(s.page_count(), 2);

        // Each page's layout holds exactly the frames assigned to it.
        let n0 = s.build_atlas_for_page(0).len();
        let n1 = s.build_atlas_for_page(1).len();
        assert_eq!(n0, 2, "page 0: idle.f0 + walk.f1");
        assert_eq!(n1, 2, "page 1: idle.f1 + walk.f0");

        // Every (anim, frame): page_of matches, flat_index is in range + unique
        // within its page.
        let cases = [
            (CharacterAnim::Idle, 0, 0u32),
            (CharacterAnim::Idle, 1, 1u32),
            (CharacterAnim::Walk, 0, 1u32),
            (CharacterAnim::Walk, 1, 0u32),
        ];
        let mut seen_per_page: std::collections::HashMap<u32, Vec<usize>> = Default::default();
        for (anim, frame, want_page) in cases {
            assert_eq!(s.page_of(anim, frame), want_page, "{anim:?} f{frame}");
            let idx = s.flat_index(anim, frame);
            let len = if want_page == 0 { n0 } else { n1 };
            assert!(idx < len, "{anim:?} f{frame} index {idx} out of range {len}");
            seen_per_page.entry(want_page).or_default().push(idx);
        }
        for (page, mut idxs) in seen_per_page {
            idxs.sort();
            idxs.dedup();
            let len = if page == 0 { n0 } else { n1 };
            assert_eq!(idxs.len(), len, "page {page}: indices must be unique + cover the layout");
        }
    }
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
