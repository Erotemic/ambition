//! [`SpritePackCatalog`]: the runtime schema for a cross-target *ultrapack*.
//!
//! Where a [`SheetRecord`](crate::SheetRecord) describes one target's sheet, an
//! ultrapack pools *every* target's frames into a handful of shared, uniformly
//! sized atlas pages — the visual-storage counterpart to per-target sheets. The
//! `ambition_sprite2d_renderer ultrapack` tool emits one catalog per quality
//! tier (`base`/`half`/`quarter`/`potato`) as JSON:
//!
//! ```json
//! {
//!   "page_size": 2048,
//!   "scale": 1.0,
//!   "pages": ["ultrapack_0.png", "ultrapack_1.png"],
//!   "targets": {
//!     "goblin": {
//!       "idle": [
//!         {"index": 0, "page": 1, "x": 12, "y": 34, "w": 40, "h": 60,
//!          "off": [4, 2], "src": [64, 64], "duration_ms": 100}
//!       ]
//!     }
//!   }
//! }
//! ```
//!
//! This is JSON, not RON, on purpose: the packer is a Python tool and JSON is
//! the drift-free interchange (Python-authored RON parses looser than Rust's
//! `ron`). The catalog is a staging/build artifact — nothing bakes it into the
//! binary yet. This type is the **runtime loader**: parse the JSON, then
//! `resolve(target, animation, frame) → placement`. It deliberately does not
//! replace the [`SheetRegistry`](crate::SheetRegistry) path; it is the schema a
//! future consumer migrates onto once the tiered packs install into a runtime
//! root. See `docs/planning/engine/data-driven-sprites-and-characters.md`.

use std::collections::HashMap;

use bevy::prelude::Resource;
use serde::Deserialize;

use crate::PixelRect;

/// One frame's placement inside a shared ultrapack page. Mirrors the catalog
/// JSON frame object one-to-one.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PackFrame {
    /// Frame index within its animation (0-based, in play order).
    pub index: u32,
    /// Which shared page (index into [`SpritePackCatalog::pages`]) this frame
    /// lives on. Frames of one animation may land on different pages — the
    /// packer places each frame freely for best fill.
    pub page: u32,
    /// Trimmed rect within the page image (excludes the transparent gutter).
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Trim offset `(off_x, off_y)`: where the trimmed rect's top-left sat
    /// inside the full logical frame. Add it back so trimmed pixels draw exactly
    /// where the untrimmed frame did. `(0, 0)` ⇒ untrimmed.
    #[serde(default)]
    pub off: (i32, i32),
    /// Logical (untrimmed) frame size `(w, h)` — the gameplay coordinate space,
    /// already scaled to this catalog's quality tier.
    pub src: (i32, i32),
    /// Frame duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u32,
}

/// One target's animations → ordered frames. `#[serde(transparent)]` so it
/// deserializes straight from the catalog's `{ "animation": [frame, …] }` map.
#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(transparent)]
pub struct PackTarget {
    pub animations: HashMap<String, Vec<PackFrame>>,
}

fn default_scale() -> f32 {
    1.0
}

/// A parsed ultrapack catalog: uniform shared pages + every frame's placement,
/// grouped `target → animation → [frame]`.
#[derive(Debug, Clone, Deserialize, Resource, Default)]
pub struct SpritePackCatalog {
    /// Side length of every (square) page image, in pixels.
    pub page_size: u32,
    /// Quality-tier scale this pack was produced at (`1.0` authored, `0.25`, …).
    /// Frame `src`/rect pixels are already at this scale.
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Page image filenames, relative to the catalog file.
    pub pages: Vec<String>,
    /// Locality group of each page (parallel to `pages`, from the PackPlan):
    /// a group's frames pack only onto its own pages so a zone's visuals can
    /// be loaded/unloaded as a unit. Empty (older catalogs) ⇒ all `"shared"`.
    #[serde(default)]
    pub page_groups: Vec<String>,
    /// `target → animations`.
    pub targets: HashMap<String, PackTarget>,
}

/// A frame resolved to everything a renderer needs to blit it: which page image,
/// where in it, the trim offset, and the logical frame size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedFrame<'a> {
    /// Index into [`SpritePackCatalog::pages`].
    pub page_index: u32,
    /// Page image filename holding this frame.
    pub page_image: &'a str,
    /// Trimmed rect within the page image.
    pub rect: PixelRect,
    /// Trim offset within the logical frame.
    pub off: (i32, i32),
    /// Logical (untrimmed) frame size.
    pub logical_size: (i32, i32),
    /// Frame duration in milliseconds.
    pub duration_ms: u32,
}

/// A structural problem found while [`validate`](SpritePackCatalog::validate)-ing
/// a catalog against its own declared pages/geometry. Filesystem-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackCatalogError {
    /// A frame references a page index with no entry in `pages`.
    FramePageOutOfRange {
        target: String,
        animation: String,
        index: u32,
        page: u32,
        page_count: usize,
    },
    /// A frame's trimmed rect falls outside the `page_size × page_size` page.
    FrameRectOutOfBounds {
        target: String,
        animation: String,
        index: u32,
    },
    /// A frame's logical size is non-positive (can't address a real frame).
    FrameLogicalSizeInvalid {
        target: String,
        animation: String,
        index: u32,
    },
    /// `page_groups` is present but not parallel to `pages`.
    PageGroupsNotParallel { pages: usize, groups: usize },
}

impl std::fmt::Display for PackCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackCatalogError::FramePageOutOfRange {
                target,
                animation,
                index,
                page,
                page_count,
            } => write!(
                f,
                "{target}/{animation}[{index}] references page {page} but only {page_count} pages exist"
            ),
            PackCatalogError::FrameRectOutOfBounds {
                target,
                animation,
                index,
            } => write!(
                f,
                "{target}/{animation}[{index}] rect falls outside the page bounds"
            ),
            PackCatalogError::FrameLogicalSizeInvalid {
                target,
                animation,
                index,
            } => write!(
                f,
                "{target}/{animation}[{index}] has a non-positive logical size"
            ),
            PackCatalogError::PageGroupsNotParallel { pages, groups } => write!(
                f,
                "page_groups has {groups} entries for {pages} pages"
            ),
        }
    }
}

impl SpritePackCatalog {
    /// Parse a catalog from the packer's JSON output.
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Page image filename for a page index (`None` if out of range).
    pub fn page_image(&self, page: u32) -> Option<&str> {
        self.pages.get(page as usize).map(String::as_str)
    }

    /// The frames of one animation, in play order (`None` if the target or
    /// animation is unknown).
    pub fn frames(&self, target: &str, animation: &str) -> Option<&[PackFrame]> {
        self.targets
            .get(target)?
            .animations
            .get(animation)
            .map(Vec::as_slice)
    }

    /// Resolve `(target, animation, frame index) → placement`. `None` if the
    /// target/animation is unknown, the frame index is absent, or its page has
    /// no image entry.
    ///
    /// `index` is matched on [`PackFrame::index`] (play order), not the position
    /// in the vec — they coincide for a well-formed catalog, but matching the
    /// field is robust to a sparse or reordered list.
    pub fn resolve(&self, target: &str, animation: &str, index: u32) -> Option<ResolvedFrame<'_>> {
        let frame = self
            .frames(target, animation)?
            .iter()
            .find(|f| f.index == index)?;
        Some(ResolvedFrame {
            page_index: frame.page,
            page_image: self.page_image(frame.page)?,
            rect: PixelRect {
                x: frame.x,
                y: frame.y,
                w: frame.w,
                h: frame.h,
            },
            off: frame.off,
            logical_size: frame.src,
            duration_ms: frame.duration_ms,
        })
    }

    /// Synthesize the canonical [`SheetRecord`](crate::SheetRecord) view of one
    /// target's frames in this pack.
    ///
    /// `SheetRecord` is the single frame-addressing algebra every runtime
    /// reader consumes (see `frames.rs`), and it already speaks the pack's
    /// language: freely-packed rows whose per-frame rects carry their own
    /// `page` + trim `off`. So a pack target does not need a parallel render
    /// path — this view drops it onto the existing one. The synthesized record
    /// carries NO gameplay geometry (`body_metrics: None`) on purpose:
    /// gameplay stays on the base per-target record / entity data, packs are
    /// visual storage only.
    ///
    /// Rows are ordered by animation name and frames by their play `index`,
    /// so the record (and any atlas built from it) is deterministic regardless
    /// of catalog map order.
    pub fn to_sheet_record(&self, target: &str) -> Option<crate::SheetRecord> {
        let pack_target = self.targets.get(target)?;
        let mut anim_names: Vec<&String> = pack_target.animations.keys().collect();
        anim_names.sort();

        // Logical frame size: every frame of one target shares the source
        // sheet's logical size; take the max defensively so trim offsets can
        // never overflow the declared frame box.
        let mut frame_w: i32 = 0;
        let mut frame_h: i32 = 0;
        for frames in pack_target.animations.values() {
            for f in frames {
                frame_w = frame_w.max(f.src.0);
                frame_h = frame_h.max(f.src.1);
            }
        }
        if frame_w <= 0 || frame_h <= 0 {
            return None;
        }

        let mut rows = Vec::with_capacity(anim_names.len());
        for (row_index, anim) in anim_names.into_iter().enumerate() {
            let mut frames: Vec<&PackFrame> = pack_target.animations[anim].iter().collect();
            frames.sort_by_key(|f| f.index);
            if frames.is_empty() {
                continue;
            }
            let duration_ms = frames[0].duration_ms;
            let rects = frames
                .iter()
                .map(|f| crate::FrameRect {
                    x: f.x,
                    y: f.y,
                    w: f.w,
                    h: f.h,
                    page: f.page,
                    off: f.off,
                    anchors: std::collections::HashMap::new(),
                })
                .collect::<Vec<_>>();
            rows.push(crate::SheetRow {
                animation: anim.clone(),
                row_index: row_index as u32,
                frame_count: rects.len() as u32,
                duration_ms,
                duration_secs: duration_ms as f32 / 1000.0,
                // Freely packed: the per-frame `page` on each rect is
                // authoritative; the row-level page is only the default.
                page: rects.first().map(|r| r.page).unwrap_or(0),
                rects,
            });
        }
        if rows.is_empty() {
            return None;
        }

        Some(crate::SheetRecord {
            target: target.to_owned(),
            image: self.pages.first().cloned().unwrap_or_default(),
            images: self.pages.clone(),
            label_width: 0,
            frame_width: frame_w as u32,
            frame_height: frame_h as u32,
            y_offset: 0,
            body_metrics: None,
            tuning: None,
            rows,
        })
    }

    /// Total frame count across every target/animation.
    pub fn frame_count(&self) -> usize {
        self.targets
            .values()
            .flat_map(|t| t.animations.values())
            .map(Vec::len)
            .sum()
    }

    /// Locality group of a page (`"shared"` for catalogs without groups).
    pub fn page_group(&self, page: u32) -> &str {
        self.page_groups
            .get(page as usize)
            .map(String::as_str)
            .unwrap_or("shared")
    }

    /// Structural validation against the catalog's own declared geometry: every
    /// frame must reference an existing page, fit inside the page bounds, and
    /// carry a positive logical size. When `page_groups` is present it must be
    /// parallel to `pages`. Returns every violation (empty ⇒ sound).
    pub fn validate(&self) -> Vec<PackCatalogError> {
        let mut errors = Vec::new();
        if !self.page_groups.is_empty() && self.page_groups.len() != self.pages.len() {
            errors.push(PackCatalogError::PageGroupsNotParallel {
                pages: self.pages.len(),
                groups: self.page_groups.len(),
            });
        }
        let page_count = self.pages.len();
        let size = self.page_size as i32;
        for (target, pack) in &self.targets {
            for (animation, frames) in &pack.animations {
                for frame in frames {
                    if (frame.page as usize) >= page_count {
                        errors.push(PackCatalogError::FramePageOutOfRange {
                            target: target.clone(),
                            animation: animation.clone(),
                            index: frame.index,
                            page: frame.page,
                            page_count,
                        });
                    }
                    let in_bounds = frame.x >= 0
                        && frame.y >= 0
                        && frame.w >= 0
                        && frame.h >= 0
                        && frame.x + frame.w <= size
                        && frame.y + frame.h <= size;
                    if !in_bounds {
                        errors.push(PackCatalogError::FrameRectOutOfBounds {
                            target: target.clone(),
                            animation: animation.clone(),
                            index: frame.index,
                        });
                    }
                    if frame.src.0 <= 0 || frame.src.1 <= 0 {
                        errors.push(PackCatalogError::FrameLogicalSizeInvalid {
                            target: target.clone(),
                            animation: animation.clone(),
                            index: frame.index,
                        });
                    }
                }
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A catalog fixture shaped exactly like the packer's JSON (array `off`/`src`,
    /// frames spread across pages). Two targets, one sharing a page with a prop.
    const FIXTURE: &str = r#"
    {
      "page_size": 512,
      "scale": 0.5,
      "pages": ["ultrapack_0.png", "ultrapack_1.png"],
      "targets": {
        "goblin": {
          "idle": [
            {"index": 0, "page": 0, "x": 4, "y": 4, "w": 40, "h": 60, "off": [2, 1], "src": [64, 64], "duration_ms": 100},
            {"index": 1, "page": 1, "x": 8, "y": 8, "w": 40, "h": 60, "off": [2, 1], "src": [64, 64], "duration_ms": 100}
          ]
        },
        "crate_prop": {
          "static": [
            {"index": 0, "page": 0, "x": 100, "y": 4, "w": 32, "h": 32, "off": [0, 0], "src": [32, 32], "duration_ms": 0}
          ]
        }
      }
    }
    "#;

    #[test]
    fn parses_and_counts_frames() {
        let cat = SpritePackCatalog::parse(FIXTURE).unwrap();
        assert_eq!(cat.page_size, 512);
        assert_eq!(cat.scale, 0.5);
        assert_eq!(cat.pages.len(), 2);
        assert_eq!(cat.frame_count(), 3);
    }

    #[test]
    fn scale_defaults_to_one_when_absent() {
        let json = r#"{"page_size": 256, "pages": ["p0.png"], "targets": {}}"#;
        let cat = SpritePackCatalog::parse(json).unwrap();
        assert_eq!(cat.scale, 1.0);
    }

    #[test]
    fn resolves_frame_to_page_rect_off_and_logical_size() {
        let cat = SpritePackCatalog::parse(FIXTURE).unwrap();
        let f = cat.resolve("goblin", "idle", 1).unwrap();
        assert_eq!(f.page_index, 1);
        assert_eq!(f.page_image, "ultrapack_1.png");
        assert_eq!(
            f.rect,
            PixelRect {
                x: 8,
                y: 8,
                w: 40,
                h: 60
            }
        );
        assert_eq!(f.off, (2, 1));
        assert_eq!(f.logical_size, (64, 64));
        assert_eq!(f.duration_ms, 100);
    }

    #[test]
    fn resolve_is_none_for_unknown_lookups() {
        let cat = SpritePackCatalog::parse(FIXTURE).unwrap();
        assert!(cat.resolve("goblin", "idle", 99).is_none());
        assert!(cat.resolve("goblin", "run", 0).is_none());
        assert!(cat.resolve("nobody", "idle", 0).is_none());
    }

    #[test]
    fn to_sheet_record_joins_the_canonical_frame_algebra() {
        let cat = SpritePackCatalog::parse(FIXTURE).unwrap();
        let record = cat.to_sheet_record("goblin").unwrap();

        // Shared pages, logical frame size, deterministic row order.
        assert_eq!(record.images, vec!["ultrapack_0.png", "ultrapack_1.png"]);
        assert_eq!((record.frame_width, record.frame_height), (64, 64));
        assert_eq!(record.rows.len(), 1);
        assert_eq!(record.rows[0].animation, "idle");
        assert_eq!(record.rows[0].frame_count, 2);
        assert_eq!(record.rows[0].duration_ms, 100);
        // Per-frame pages + trim offsets survive (freely-packed shape).
        assert_eq!(record.rows[0].rects[0].page, 0);
        assert_eq!(record.rows[0].rects[1].page, 1);
        assert_eq!(record.rows[0].rects[0].off, (2, 1));
        // No gameplay geometry rides along.
        assert!(record.body_metrics.is_none());

        // The canonical algebra addresses the synthesized record: frame 1
        // lands on page 1 with its trim geometry intact.
        let trim = record.frame_trim(0, 1);
        assert_eq!(trim.offset.x, 2);
        assert_eq!(trim.offset.y, 1);
        assert_eq!((trim.logical.x, trim.logical.y), (64, 64));
        let page1 = record.atlas_page(1, 0);
        assert_eq!(page1.rects.len(), 1); // only goblin idle[1] lives on page 1
        assert_eq!(record.flat_index_in_page(0, 1), 0);

        // Unknown target -> None.
        assert!(cat.to_sheet_record("nobody").is_none());
    }

    #[test]
    fn clean_fixture_validates() {
        let cat = SpritePackCatalog::parse(FIXTURE).unwrap();
        assert!(cat.validate().is_empty(), "{:?}", cat.validate());
    }

    #[test]
    fn validate_flags_bad_page_bounds_and_logical_size() {
        // page 2 doesn't exist; rect escapes the 512² page; src is degenerate.
        let json = r#"
        {
          "page_size": 512,
          "pages": ["p0.png"],
          "targets": {
            "bad": {
              "anim": [
                {"index": 0, "page": 2, "x": 0, "y": 0, "w": 10, "h": 10, "off": [0,0], "src": [16,16], "duration_ms": 0},
                {"index": 1, "page": 0, "x": 500, "y": 0, "w": 100, "h": 10, "off": [0,0], "src": [16,16], "duration_ms": 0},
                {"index": 2, "page": 0, "x": 0, "y": 0, "w": 10, "h": 10, "off": [0,0], "src": [0,16], "duration_ms": 0}
              ]
            }
          }
        }
        "#;
        let cat = SpritePackCatalog::parse(json).unwrap();
        let errors = cat.validate();
        assert!(errors
            .iter()
            .any(|e| matches!(e, PackCatalogError::FramePageOutOfRange { page: 2, .. })));
        assert!(errors
            .iter()
            .any(|e| matches!(e, PackCatalogError::FrameRectOutOfBounds { index: 1, .. })));
        assert!(errors.iter().any(|e| matches!(
            e,
            PackCatalogError::FrameLogicalSizeInvalid { index: 2, .. }
        )));
    }
}
