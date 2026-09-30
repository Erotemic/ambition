//! A character's transform flipbook: part rasters on atlas pages, and per
//! frame an ordered list of part draws.
//!
//! This is the second realization of a character's sheet rows, beside the
//! baked [`super::CharacterSpriteAsset`]. It is keyed by the SAME row names and
//! frame counts, so the clip, frame and facing a body shows are chosen above
//! the realization (`CharacterAnimator`) and only the drawing differs.
//!
//! The draw table is published by the sprite renderer (`part_flipbook.py`) as
//! `<target>_parts.ron`, baked into the build like a body rig (it has no
//! quality tier), and parsed here into a compact form. The atlas pages are
//! images, loaded per quality tier through the sheet image funnel.
//!
//! Coordinates are the baked sheet's full-resolution frame pixels relative to
//! its `feet_pixel`, +y down. A draw puts its part's pivot at `at`, turned by
//! `rotation` (radians, clockwise) and scaled by `scale` in the part's axes.

use std::collections::BTreeMap;

use bevy::math::{URect, UVec2, Vec2};
use serde::Deserialize;

/// The `<target>_parts.ron` schema this build reads.
pub const PART_FLIPBOOK_SCHEMA_VERSION: u32 = 1;

/// One part raster on an atlas page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigPart {
    pub page: u16,
    /// Page pixels (of this table's tier).
    pub rect: URect,
    /// The part's origin, in full-resolution sheet pixels from the top left of
    /// its drawn size.
    pub pivot: Vec2,
    /// The part's drawn size in full-resolution sheet pixels. In a tier table
    /// the rect is smaller; the part is drawn at this size all the same.
    pub size: Vec2,
}

impl RigPart {
    /// The pivot as a sprite anchor: `(-0.5, -0.5)` is the bottom left.
    pub fn anchor(&self) -> Vec2 {
        Vec2::new(self.pivot.x / self.size.x - 0.5, 0.5 - self.pivot.y / self.size.y)
    }
}

/// One part drawn in one frame. Draw order is paint order: a later draw is
/// nearer the viewer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartDraw {
    /// Where the part's pivot lands, in sheet pixels from the feet.
    pub at: Vec2,
    pub scale: Vec2,
    /// Radians, clockwise (+y down).
    pub rotation: f32,
    pub part: u16,
}

/// One row's frames: the timing and each frame's slice of the draw list.
#[derive(Debug, Clone, PartialEq)]
pub struct RigSpriteClip {
    pub frame_duration_s: f32,
    frames: Vec<(u32, u32)>,
}

impl RigSpriteClip {
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

/// A character's parsed transform flipbook.
#[derive(Debug, Clone, PartialEq)]
pub struct RiggedSpriteAsset {
    pub target: String,
    /// Atlas page texels per full-resolution sheet pixel: 1 for the published
    /// flipbook, less for a quality tier. A part keeps its full-resolution
    /// size and pivot in every tier; only its page rect shrinks.
    pub texel_scale: f32,
    /// Atlas page file names, relative to the sheet's directory.
    pub pages: Vec<String>,
    /// The baked sheet's full-resolution frame size.
    pub frame_size: UVec2,
    pub feet_pixel: Vec2,
    pub parts: Vec<RigPart>,
    clips: BTreeMap<String, RigSpriteClip>,
    draws: Vec<PartDraw>,
    max_draws: usize,
}

/// Why a published flipbook was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiggedSpriteError {
    Parse(String),
    Schema { found: u32 },
    MissingPage { part: usize, page: u16 },
    MissingPart { clip: String, frame: usize, part: u16 },
    EmptyClip(String),
    TierMismatch {
        tier: &'static str,
        parts: usize,
        expected: usize,
    },
}

impl std::fmt::Display for RiggedSpriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "does not parse: {error}"),
            Self::Schema { found } => write!(
                f,
                "is schema {found}; this build reads schema {PART_FLIPBOOK_SCHEMA_VERSION}"
            ),
            Self::MissingPage { part, page } => write!(f, "part {part} is on page {page}, which it does not list"),
            Self::MissingPart { clip, frame, part } => {
                write!(f, "`{clip}` frame {frame} draws part {part}, which it does not have")
            }
            Self::EmptyClip(clip) => write!(f, "`{clip}` has no frames"),
            Self::TierMismatch { tier, parts, expected } => write!(
                f,
                "has {parts} parts in its `{tier}` tier table and {expected} at full resolution"
            ),
        }
    }
}

impl std::error::Error for RiggedSpriteError {}

#[derive(Deserialize)]
struct Published {
    schema_version: u32,
    #[serde(default = "full_resolution")]
    texel_scale: f32,
    target: String,
    pages: Vec<String>,
    frame_size: (u32, u32),
    feet_pixel: (f32, f32),
    parts: Vec<PublishedPart>,
    clips: BTreeMap<String, PublishedClip>,
}

fn full_resolution() -> f32 {
    1.0
}

#[derive(Deserialize)]
struct PublishedPart {
    #[allow(dead_code, reason = "a diagnostic label in the published file; the runtime keys parts by index")]
    name: String,
    page: u16,
    rect: (u32, u32, u32, u32),
    pivot: (f32, f32),
}

#[derive(Deserialize)]
struct PublishedClip {
    frame_duration_s: f32,
    frames: Vec<Vec<PublishedDraw>>,
}

#[derive(Deserialize)]
struct PublishedDraw {
    part: u16,
    at: (f32, f32),
    rotation: f32,
    scale: (f32, f32),
}

impl RiggedSpriteAsset {
    /// Parse and check a published `<target>_parts.ron`.
    pub fn from_published_ron(text: &str) -> Result<Self, RiggedSpriteError> {
        let published: Published =
            ron::from_str(text).map_err(|error| RiggedSpriteError::Parse(error.to_string()))?;
        if published.schema_version != PART_FLIPBOOK_SCHEMA_VERSION {
            return Err(RiggedSpriteError::Schema {
                found: published.schema_version,
            });
        }
        let parts = published
            .parts
            .iter()
            .enumerate()
            .map(|(index, part)| {
                if usize::from(part.page) >= published.pages.len() {
                    return Err(RiggedSpriteError::MissingPage {
                        part: index,
                        page: part.page,
                    });
                }
                let (x, y, w, h) = part.rect;
                // A tier table's rect and pivot are in its own texels.
                let scale = published.texel_scale;
                Ok(RigPart {
                    page: part.page,
                    rect: URect::new(x, y, x + w, y + h),
                    pivot: Vec2::new(part.pivot.0, part.pivot.1) / scale,
                    size: Vec2::new(w as f32, h as f32) / scale,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut draws = Vec::new();
        let mut clips = BTreeMap::new();
        let mut max_draws = 0;
        for (name, clip) in published.clips {
            if clip.frames.is_empty() {
                return Err(RiggedSpriteError::EmptyClip(name));
            }
            let mut frames = Vec::with_capacity(clip.frames.len());
            for (frame_index, frame) in clip.frames.iter().enumerate() {
                let start = draws.len() as u32;
                for draw in frame {
                    if usize::from(draw.part) >= parts.len() {
                        return Err(RiggedSpriteError::MissingPart {
                            clip: name,
                            frame: frame_index,
                            part: draw.part,
                        });
                    }
                    draws.push(PartDraw {
                        at: Vec2::new(draw.at.0, draw.at.1),
                        scale: Vec2::new(draw.scale.0, draw.scale.1),
                        rotation: draw.rotation,
                        part: draw.part,
                    });
                }
                max_draws = max_draws.max(frame.len());
                frames.push((start, frame.len() as u32));
            }
            clips.insert(
                name,
                RigSpriteClip {
                    frame_duration_s: clip.frame_duration_s,
                    frames,
                },
            );
        }
        Ok(Self {
            target: published.target,
            texel_scale: published.texel_scale,
            pages: published.pages,
            frame_size: UVec2::new(published.frame_size.0, published.frame_size.1),
            feet_pixel: Vec2::new(published.feet_pixel.0, published.feet_pixel.1),
            parts,
            clips,
            draws,
            max_draws,
        })
    }

    /// The flipbook sheet target `target` publishes, parsed, or `None` when
    /// the build baked none.
    ///
    /// # Panics
    ///
    /// When the baked flipbook is refused: it is generated by the sprite
    /// publisher, so a file this build cannot read is a stale or broken publish.
    pub fn baked(target: &str) -> Option<Self> {
        let text = crate::baked_part_flipbooks::baked_part_flipbook(target)?;
        Some(
            Self::from_published_ron(text)
                .unwrap_or_else(|error| panic!("the published part flipbook `{target}` {error}")),
        )
    }

    /// The flipbook for `tier`: the published draws, drawn from the tier's
    /// atlas pages. `None` when the tier publishes no table; a tier table that
    /// is not this flipbook's (another part count) is refused.
    ///
    /// Each part keeps its full-resolution size and pivot, so a body draws the
    /// same size at every tier: the tier table's own sizes are rounded.
    pub fn for_tier(&self, tier: ambition_persistence::settings::TextureResolutionScale) -> Option<Result<Self, RiggedSpriteError>> {
        let Some(suffix) = tier.asset_id_suffix() else {
            return Some(Ok(self.clone()));
        };
        let text = crate::baked_part_flipbooks::baked_part_flipbook(&format!("{}.{suffix}", self.target))?;
        Some(Self::from_published_ron(text).and_then(|tiered| {
            if tiered.parts.len() != self.parts.len() {
                return Err(RiggedSpriteError::TierMismatch {
                    tier: suffix,
                    parts: tiered.parts.len(),
                    expected: self.parts.len(),
                });
            }
            let parts = tiered
                .parts
                .iter()
                .zip(&self.parts)
                .map(|(tier_part, full)| RigPart {
                    page: tier_part.page,
                    rect: tier_part.rect,
                    ..*full
                })
                .collect();
            Ok(Self {
                texel_scale: tiered.texel_scale,
                pages: tiered.pages,
                parts,
                ..self.clone()
            })
        }))
    }

    /// The parts on `page`, in part order: the atlas layout of that page.
    pub fn parts_on_page(&self, page: u16) -> impl Iterator<Item = (usize, &RigPart)> {
        self.parts.iter().enumerate().filter(move |(_, part)| part.page == page)
    }

    pub fn clip(&self, row: &str) -> Option<&RigSpriteClip> {
        self.clips.get(row)
    }

    pub fn clip_names(&self) -> impl Iterator<Item = &str> {
        self.clips.keys().map(String::as_str)
    }

    /// Frame `index` of `row`, in draw order. An index past the end holds the
    /// last frame, as a one-shot row does.
    pub fn frame(&self, row: &str, index: usize) -> Option<&[PartDraw]> {
        let clip = self.clips.get(row)?;
        let (start, len) = clip.frames[index.min(clip.frames.len() - 1)];
        Some(&self.draws[start as usize..(start + len) as usize])
    }

    /// The most draws any frame makes: the number of reusable slots a player
    /// needs so that no frame spawns one.
    pub fn max_draws(&self) -> usize {
        self.max_draws
    }

    /// Total draws over all frames.
    pub fn draw_count(&self) -> usize {
        self.draws.len()
    }
}

/// A realized flipbook: its table for the resident tier and its atlas pages.
/// A part is drawn from its page with `Sprite::rect`, so no atlas layout is
/// needed. It rides on its sheet's [`super::CharacterSpriteAsset`], so it is
/// demanded, re-realized and retired with the sheet.
#[derive(Clone)]
pub struct RiggedSpritePages {
    pub flipbook: std::sync::Arc<RiggedSpriteAsset>,
    pub pages: Vec<bevy::asset::Handle<bevy::image::Image>>,
}

/// Whether this composition draws characters from their transform flipbooks
/// instead of their baked sheets: the rigged-sprite TRIAL switch.
///
/// ⛔ Off in every shipped game. With it off, no flipbook page is loaded and
/// no body draws a part. It is read from this resource when the composition
/// inserts one, else from the environment ([`RIGGED_SPRITE_ADMISSION_ENV`]).
#[derive(bevy::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RiggedSpriteAdmission {
    pub admit: bool,
}

/// The environment switch for [`RiggedSpriteAdmission`]: `1`, `true`, `on` or
/// `yes` admits.
pub const RIGGED_SPRITE_ADMISSION_ENV: &str = "AMBITION_RIGGED_SPRITES";

impl RiggedSpriteAdmission {
    pub const ADMIT: Self = Self { admit: true };

    pub fn from_env() -> Self {
        let admit = std::env::var(RIGGED_SPRITE_ADMISSION_ENV).is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "on" | "yes"
            )
        });
        Self { admit }
    }
}

#[cfg(test)]
mod tests;
