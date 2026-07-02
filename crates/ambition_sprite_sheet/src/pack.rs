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

    /// Total frame count across every target/animation.
    pub fn frame_count(&self) -> usize {
        self.targets
            .values()
            .flat_map(|t| t.animations.values())
            .map(Vec::len)
            .sum()
    }

    /// Structural validation against the catalog's own declared geometry: every
    /// frame must reference an existing page, fit inside the page bounds, and
    /// carry a positive logical size. Returns every violation (empty ⇒ sound).
    pub fn validate(&self) -> Vec<PackCatalogError> {
        let mut errors = Vec::new();
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
