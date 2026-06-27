//! The single frame-addressing algebra for every sprite sheet.
//!
//! Every runtime reader — playable characters, bosses, props, melee/effect
//! visuals, projectiles — addresses the same [`SheetRecord`] shape. The pixel
//! math for "which page is this frame on", "what atlas cells does this page
//! hold", "what's the page-local index of `(row, frame)`", and "how was this
//! frame alpha-trimmed" used to be re-derived in four places (the character
//! `CharacterSheetSpec`, the boss `BossSheetSpec` grid, the prop/effect
//! `atlas_layout_from_record`, the projectile rect collector), and only the
//! character path understood trimming + paging. This module is that math, once,
//! in the foundational crate both the gameplay and render layers depend on, so
//! a single implementation drives trimming and multi-page packing for the whole
//! cast.
//!
//! Everything here is pure integer / `glam` geometry — no Bevy `TextureAtlasLayout`
//! (that's a render-feature type), so this crate stays headless-reusable. A
//! consumer turns an [`AtlasPage`]'s `rects` into a `TextureAtlasLayout` with a
//! three-line loop (see `ambition_gameplay_core`'s `build_atlas_layout`).
//!
//! The key contract: [`SheetRecord::atlas_page`] and
//! [`SheetRecord::flat_index_in_page`] walk rows in the SAME order and assign
//! each cell to the SAME page, so a flat index built by one exactly addresses
//! the layout built by the other. Pinned by the tests at the bottom of this
//! file.

use bevy::math::{IVec2, URect, UVec2, Vec2};

use crate::{FrameRect, SheetRecord, SheetRow};

/// A single page image's atlas cells, ready to feed a `TextureAtlasLayout`.
///
/// `extent` is at least as large as the underlying page PNG (it's the max of
/// every cell's pre-inset bottom-right corner) so cell coordinates never
/// overflow the layout. `rects` are the inset atlas cells in `(row, frame)`
/// flat order — the order [`SheetRecord::flat_index_in_page`] counts in.
#[derive(Clone, Debug, Default)]
pub struct AtlasPage {
    pub extent: UVec2,
    pub rects: Vec<URect>,
}

/// Per-frame trim geometry: where a frame's opaque alpha box sat inside the
/// full logical `frame_width`×`frame_height` frame.
///
/// The atlas packer trims each frame to its opaque bounding box for storage;
/// the stored rect is then the trimmed size and [`Self::offset`] is its
/// top-left within the logical frame. An untrimmed frame reports offset `(0,0)`
/// and `trimmed == logical`, so legacy uniform sheets see the identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTrim {
    /// Offset of the trimmed rect within the logical frame, in logical pixels.
    pub offset: IVec2,
    /// Size of the trimmed rect (== the atlas rect size), in logical pixels.
    pub trimmed: UVec2,
    /// Logical (untrimmed) frame size.
    pub logical: UVec2,
}

impl FrameTrim {
    /// The identity trim for a `logical`-sized frame (no trimming).
    pub fn identity(logical: UVec2) -> Self {
        Self {
            offset: IVec2::ZERO,
            trimmed: logical,
            logical,
        }
    }

    /// True when this frame carries no trim (drawn the legacy uniform way).
    pub fn is_identity(&self) -> bool {
        self.offset == IVec2::ZERO && self.trimmed == self.logical
    }
}

/// Given a frame's trim geometry plus the base (untrimmed) render size and
/// anchor, return the `(custom_size, anchor)` that draws the trimmed sub-rect
/// so the logical frame's anchor point lands at the SAME world position the
/// untrimmed frame would have used.
///
/// Derivation: the full logical sprite has size `base_render_size` and anchor
/// `base_anchor`; render only the trimmed sub-region at the proportional size
/// and solve for the anchor that keeps the logical-frame mapping fixed. The
/// formula reduces to `(base_render_size, base_anchor)` for an untrimmed frame
/// (`offset == 0`, `trimmed == logical`), so untrimmed sheets are unchanged.
/// Pinned by `trimmed_render_*` unit tests.
pub fn trimmed_render(trim: &FrameTrim, base_render_size: Vec2, base_anchor: Vec2) -> (Vec2, Vec2) {
    let fw = trim.logical.x.max(1) as f32;
    let fh = trim.logical.y.max(1) as f32;
    let tw = trim.trimmed.x.max(1) as f32;
    let th = trim.trimmed.y.max(1) as f32;
    let ox = trim.offset.x as f32;
    let oy = trim.offset.y as f32;
    let (ax, ay) = (base_anchor.x, base_anchor.y);
    let custom = Vec2::new(tw / fw * base_render_size.x, th / fh * base_render_size.y);
    let anchor = Vec2::new(
        ((ax + 0.5) * fw - ox - tw * 0.5) / tw,
        ((ay - 0.5) * fh + oy + th * 0.5) / th,
    );
    (custom, anchor)
}

impl SheetRow {
    /// True when this row carries explicit, non-negative per-frame rects (the
    /// packed / padded path). False ⇒ the caller derives cells from grid stride
    /// (`label_width + col*frame_width`, `y_offset + row_index*frame_height`).
    ///
    /// A row with any negative / zero-area rect is treated as grid (the rect
    /// vector is unusable as a `UVec2`-backed cell), matching the character
    /// reader's historical fallback.
    fn uses_explicit_rects(&self) -> bool {
        !self.rects.is_empty()
            && self
                .rects
                .iter()
                .all(|r| r.x >= 0 && r.y >= 0 && r.w > 0 && r.h > 0)
    }

    /// True when this row was freely packed — its frames carry per-frame pages
    /// that differ from the row's nominal page. For unpacked (single-page or
    /// unpacked-multipage) rows every frame uses [`SheetRow::page`].
    fn is_packed(&self) -> bool {
        self.rects.iter().any(|r| r.page != self.page)
    }

    /// Which page image frame `f` of this row draws from. Per-frame for packed
    /// rows (the packer can scatter one animation across pages for fill); the
    /// row's nominal page otherwise.
    fn cell_page(&self, f: usize) -> u32 {
        if self.uses_explicit_rects() && self.is_packed() {
            self.rects.get(f).map(|r| r.page).unwrap_or(self.page)
        } else {
            self.page
        }
    }
}

/// Shrink a cell by `inset` on every side so bilinear filtering at the seam
/// can't pull pixels from neighbouring cells. The inset is clamped so a tiny
/// cell can't invert (min ≥ max).
fn inset_rect(r: URect, inset: u32) -> URect {
    let inset = inset.min(r.width().min(r.height()) / 4);
    URect {
        min: UVec2::new(r.min.x + inset, r.min.y + inset),
        max: UVec2::new(
            r.max.x.saturating_sub(inset).max(r.min.x + 1),
            r.max.y.saturating_sub(inset).max(r.min.y + 1),
        ),
    }
}

fn frame_rect_to_urect(r: &FrameRect) -> URect {
    URect {
        min: UVec2::new(r.x.max(0) as u32, r.y.max(0) as u32),
        max: UVec2::new((r.x + r.w).max(0) as u32, (r.y + r.h).max(0) as u32),
    }
}

impl SheetRecord {
    /// Which page image `(row_idx, frame)` lives in. `0` for single-page
    /// sheets. Per-frame because a freely-packed sheet can scatter one
    /// animation's frames across pages.
    pub fn frame_page_of(&self, row_idx: usize, frame: usize) -> u32 {
        self.rows.get(row_idx).map(|r| r.cell_page(frame)).unwrap_or(0)
    }

    /// Build one page image's atlas cells in `(row, frame)` flat order, applying
    /// `inset` to each. Only frames assigned to `page` contribute, so a packed
    /// animation that spans pages lands the right cells on each. Rows without
    /// usable rects fall back to grid stride (those are always single-page).
    ///
    /// The returned [`AtlasPage::rects`] are addressed by
    /// [`Self::flat_index_in_page`] — same row walk, same page test.
    pub fn atlas_page(&self, page: u32, inset: u32) -> AtlasPage {
        let mut rects = Vec::new();
        let mut extent = UVec2::ONE;
        for row in &self.rows {
            let count = row.frame_count as usize;
            if row.uses_explicit_rects() {
                for (f, r) in row.rects.iter().take(count).enumerate() {
                    if row.cell_page(f) != page {
                        continue;
                    }
                    let cell = frame_rect_to_urect(r);
                    extent = extent.max(cell.max);
                    rects.push(inset_rect(cell, inset));
                }
            } else {
                // Grid stride — grid sheets are never packed, so the whole row
                // shares `row.page`.
                if row.page != page {
                    continue;
                }
                for col in 0..row.frame_count {
                    let x = self.label_width + col * self.frame_width;
                    let y = self.y_offset + row.row_index * self.frame_height;
                    let cell = URect {
                        min: UVec2::new(x, y),
                        max: UVec2::new(x + self.frame_width, y + self.frame_height),
                    };
                    extent = extent.max(cell.max);
                    rects.push(inset_rect(cell, inset));
                }
            }
        }
        AtlasPage { extent, rects }
    }

    /// Page-local flat atlas index of `(row_idx, frame)`: its position among all
    /// frames on the *same page*, in `(row, frame)` order. For a single-page
    /// sheet this is the global index. Exactly addresses the layout
    /// [`Self::atlas_page`] builds for `frame_page_of(row_idx, frame)`.
    pub fn flat_index_in_page(&self, row_idx: usize, frame: usize) -> usize {
        let Some(target) = self.rows.get(row_idx) else {
            return 0;
        };
        let f = frame.min((target.frame_count as usize).saturating_sub(1));
        let page = target.cell_page(f);
        let mut count = 0usize;
        for (ri, row) in self.rows.iter().enumerate() {
            if ri > row_idx {
                break;
            }
            let limit = if ri == row_idx {
                f
            } else {
                row.frame_count as usize
            };
            for g in 0..limit {
                if row.cell_page(g) == page {
                    count += 1;
                }
            }
        }
        count
    }

    /// Trim geometry of `(row_idx, frame)`: the stored (trimmed) rect size + its
    /// offset within the logical frame. Untrimmed frames report offset `(0,0)`
    /// and the full logical size.
    pub fn frame_trim(&self, row_idx: usize, frame: usize) -> FrameTrim {
        let logical = UVec2::new(self.frame_width, self.frame_height);
        let Some(row) = self.rows.get(row_idx) else {
            return FrameTrim::identity(logical);
        };
        if !row.uses_explicit_rects() {
            return FrameTrim::identity(logical);
        }
        let f = frame.min((row.frame_count as usize).saturating_sub(1));
        match row.rects.get(f) {
            Some(r) => FrameTrim {
                offset: IVec2::new(r.off.0, r.off.1),
                trimmed: UVec2::new(r.w.max(0) as u32, r.h.max(0) as u32),
                logical,
            },
            None => FrameTrim::identity(logical),
        }
    }

    /// True when any frame of this sheet was alpha-trimmed (so a renderer must
    /// adjust sprite size + anchor per frame via [`trimmed_render`]). False for
    /// legacy uniform sheets, which keep the cheap fixed-anchor path.
    pub fn is_trimmed(&self) -> bool {
        self.rows
            .iter()
            .any(|row| row.uses_explicit_rects() && row.rects.iter().any(|r| r.off != (0, 0)))
    }

    /// Row index of the first row whose `animation` matches `name`, or `None`.
    /// The universal name→row resolver every string-keyed reader (props, melee
    /// effects, projectiles) uses before delegating to the frame algebra.
    pub fn row_index_of(&self, name: &str) -> Option<usize> {
        self.rows.iter().position(|r| r.animation == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32, page: u32, off: (i32, i32)) -> FrameRect {
        FrameRect {
            x,
            y,
            w,
            h,
            page,
            off,
            anchors: Default::default(),
        }
    }

    fn row(animation: &str, row_index: u32, page: u32, rects: Vec<FrameRect>) -> SheetRow {
        SheetRow {
            animation: animation.to_string(),
            row_index,
            frame_count: rects.len() as u32,
            duration_ms: 100,
            duration_secs: 0.1,
            page,
            rects,
        }
    }

    fn record(frame_w: u32, frame_h: u32, rows: Vec<SheetRow>) -> SheetRecord {
        SheetRecord {
            target: "t".into(),
            image: "t.png".into(),
            images: vec![],
            label_width: 0,
            frame_width: frame_w,
            frame_height: frame_h,
            y_offset: 0,
            body_metrics: None,
            tuning: None,
            rows,
        }
    }

    /// A freely-packed sheet scatters one animation's frames across pages.
    /// `flat_index_in_page` must be a page-local index that exactly addresses
    /// the layout `atlas_page` produces for that page.
    #[test]
    fn flat_index_agrees_with_atlas_page_when_frames_span_pages() {
        // idle: f0→page0, f1→page1.  walk: f0→page1, f1→page0.
        let rec = record(
            8,
            8,
            vec![
                row(
                    "idle",
                    0,
                    0,
                    vec![
                        rect(0, 0, 8, 8, 0, (0, 0)),
                        rect(0, 0, 8, 8, 1, (0, 0)),
                    ],
                ),
                row(
                    "walk",
                    1,
                    0,
                    vec![
                        rect(8, 0, 8, 8, 1, (0, 0)),
                        rect(8, 0, 8, 8, 0, (0, 0)),
                    ],
                ),
            ],
        );
        assert_eq!(rec.page_count(), 2);

        let n0 = rec.atlas_page(0, 0).rects.len();
        let n1 = rec.atlas_page(1, 0).rects.len();
        assert_eq!(n0, 2, "page 0: idle.f0 + walk.f1");
        assert_eq!(n1, 2, "page 1: idle.f1 + walk.f0");

        // (row, frame) -> (expected page). Indices must be unique within a page
        // and cover the layout.
        let cases = [(0usize, 0usize, 0u32), (0, 1, 1), (1, 0, 1), (1, 1, 0)];
        let mut seen: std::collections::HashMap<u32, Vec<usize>> = Default::default();
        for (ri, f, want_page) in cases {
            assert_eq!(rec.frame_page_of(ri, f), want_page, "row {ri} f{f}");
            let idx = rec.flat_index_in_page(ri, f);
            let len = if want_page == 0 { n0 } else { n1 };
            assert!(idx < len, "row {ri} f{f} index {idx} out of range {len}");
            seen.entry(want_page).or_default().push(idx);
        }
        for (page, mut idxs) in seen {
            idxs.sort();
            idxs.dedup();
            let len = if page == 0 { n0 } else { n1 };
            assert_eq!(idxs.len(), len, "page {page} indices unique + cover layout");
        }
    }

    /// A grid row (no rects) addresses cells off the frame stride, and the
    /// single-page flat index is the global index.
    #[test]
    fn grid_fallback_addresses_stride_cells() {
        let mut rec = record(
            16,
            16,
            vec![row("idle", 0, 0, vec![]), row("walk", 1, 0, vec![])],
        );
        rec.rows[0].frame_count = 3;
        rec.rows[1].frame_count = 2;
        rec.label_width = 4;
        let page = rec.atlas_page(0, 0);
        assert_eq!(page.rects.len(), 5, "3 + 2 grid cells");
        // First idle cell starts at label_width.
        assert_eq!(page.rects[0].min, UVec2::new(4, 0));
        // walk row sits one frame_height down.
        assert_eq!(page.rects[3].min, UVec2::new(4, 16));
        assert_eq!(rec.flat_index_in_page(1, 0), 3, "walk.f0 is global index 3");
    }

    /// Trim round-trips: a trimmed frame reports its stored size + offset; an
    /// untrimmed frame is the identity.
    #[test]
    fn frame_trim_reports_offset_and_size() {
        let rec = record(
            128,
            128,
            vec![row(
                "idle",
                0,
                0,
                vec![rect(2, 2, 80, 100, 0, (20, 14)), rect(90, 2, 128, 128, 0, (0, 0))],
            )],
        );
        assert!(rec.is_trimmed());
        let t0 = rec.frame_trim(0, 0);
        assert_eq!(t0.offset, IVec2::new(20, 14));
        assert_eq!(t0.trimmed, UVec2::new(80, 100));
        assert_eq!(t0.logical, UVec2::new(128, 128));
        assert!(!t0.is_identity());
        let t1 = rec.frame_trim(0, 1);
        assert!(t1.is_identity());
    }

    /// `trimmed_render` is the identity for an untrimmed frame and keeps a
    /// logical point fixed in world space for a trimmed one.
    #[test]
    fn trimmed_render_identity_and_fixed_point() {
        let logical = UVec2::new(384, 529);
        let base_size = Vec2::new(120.0, 165.0);
        let base_anchor = Vec2::new(0.0, -0.3);
        let (s, a) = trimmed_render(&FrameTrim::identity(logical), base_size, base_anchor);
        assert!((s - base_size).length() < 1e-3);
        assert!((a - base_anchor).length() < 1e-4);

        // A trimmed sub-rect must map the logical-frame centre to the same world
        // position the full frame would.
        let (ox, oy, tw, th) = (100i32, 80i32, 180u32, 360u32);
        let trim = FrameTrim {
            offset: IVec2::new(ox, oy),
            trimmed: UVec2::new(tw, th),
            logical,
        };
        let (size, anchor) = trimmed_render(&trim, base_size, base_anchor);
        let world = |sub: (f32, f32, f32, f32), size: Vec2, anchor: Vec2, px: f32, py: f32| {
            let (sox, soy, stw, sth) = sub;
            let nx = (px - sox) / stw - 0.5;
            let ny = 0.5 - (py - soy) / sth;
            -anchor * size + Vec2::new(nx * size.x, ny * size.y)
        };
        let (lw, lh) = (logical.x as f32, logical.y as f32);
        let (px, py) = (ox as f32 + tw as f32 / 2.0, oy as f32 + th as f32 / 2.0);
        let full = world((0.0, 0.0, lw, lh), base_size, base_anchor, px, py);
        let trimmed = world(
            (ox as f32, oy as f32, tw as f32, th as f32),
            size,
            anchor,
            px,
            py,
        );
        assert!((full - trimmed).length() < 1e-2, "full={full:?} trimmed={trimmed:?}");
    }
}
