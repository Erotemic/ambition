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
//! A flipbook may realize only some rows: a hybrid. It states each row of its
//! sheet as a part clip (`clips`) or as a baked clip (`baked_clips`), and the
//! body draws a baked clip from its sheet. The choice is published with the
//! clip, so no runtime rule picks it. A sheet row that the flipbook states as
//! neither is refused ([`RiggedSpriteAsset::check_rows`]).
//!
//! Coordinates are the baked sheet's full-resolution frame pixels relative to
//! its `feet_pixel`, +y down. A draw puts its part's pivot at `at`, turned by
//! `rotation` (radians, clockwise) and scaled by `scale` in the part's axes.

use std::collections::{BTreeMap, BTreeSet};

use bevy::math::{URect, UVec2, Vec2};
use serde::Deserialize;

/// The `<target>_parts.ron` schema this build writes and reads. Schema 2 adds
/// the track table (each draw's identity across frames) and the per-clip
/// tween policy; a schema-1 file reads as untracked clips that step.
pub const PART_FLIPBOOK_SCHEMA_VERSION: u32 = 2;

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
    /// The draw's identity across frames (an index into the flipbook's
    /// tracks): what a tween pairs. `None` in an untracked flipbook.
    pub track: Option<u16>,
}

/// How a clip draws between two of its frames. Published per clip; the
/// runtime never chooses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum ClipTween {
    /// Each frame whole until the next.
    #[default]
    Step,
    /// Each track moves from its place in one frame to its place in the next
    /// ([`RiggedSpriteAsset::tween_into`]).
    Linear,
}

/// One row's frames: the timing and each frame's slice of the draw list.
#[derive(Debug, Clone, PartialEq)]
pub struct RigSpriteClip {
    pub frame_duration_s: f32,
    pub tween: ClipTween,
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
    /// The rows that this flipbook leaves to the baked sheet.
    baked_clips: BTreeSet<String>,
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
    MissingTrack { clip: String, frame: usize, track: u16 },
    EmptyClip(String),
    TierMismatch {
        tier: &'static str,
        parts: usize,
        expected: usize,
    },
    /// A row stated as a part clip and as a baked clip.
    TwoRealizations(String),
    /// A sheet row that the flipbook states as neither a part clip nor a baked
    /// clip.
    Unrealized(String),
    /// A clip for a row that the sheet does not have.
    UnknownRow(String),
}

/// How a body draws one row of its sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipRealization {
    /// From parts, with the flipbook's draws.
    Parts,
    /// From the baked sheet frame.
    Baked,
}

impl std::fmt::Display for RiggedSpriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "does not parse: {error}"),
            Self::Schema { found } => write!(
                f,
                "is schema {found}; this build reads schemas 1 to {PART_FLIPBOOK_SCHEMA_VERSION}"
            ),
            Self::MissingPage { part, page } => write!(f, "part {part} is on page {page}, which it does not list"),
            Self::MissingPart { clip, frame, part } => {
                write!(f, "`{clip}` frame {frame} draws part {part}, which it does not have")
            }
            Self::MissingTrack { clip, frame, track } => {
                write!(f, "`{clip}` frame {frame} names track {track}, which it does not list")
            }
            Self::EmptyClip(clip) => write!(f, "`{clip}` has no frames"),
            Self::TierMismatch { tier, parts, expected } => write!(
                f,
                "has {parts} parts in its `{tier}` tier table and {expected} at full resolution"
            ),
            Self::TwoRealizations(row) => write!(f, "states `{row}` as a part clip and as a baked clip"),
            Self::Unrealized(row) => write!(f, "states the sheet row `{row}` as neither a part clip nor a baked clip"),
            Self::UnknownRow(row) => write!(f, "has a clip `{row}`, which is not a row of its sheet"),
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
    /// Schema 2: the track names a draw's `track` indexes.
    #[serde(default)]
    tracks: Vec<String>,
    clips: BTreeMap<String, PublishedClip>,
    /// Absent in a flipbook that realizes every row from parts.
    #[serde(default)]
    baked_clips: Vec<String>,
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
    #[serde(default)]
    tween: ClipTween,
    frames: Vec<Vec<PublishedDraw>>,
}

#[derive(Deserialize)]
struct PublishedDraw {
    part: u16,
    at: (f32, f32),
    rotation: f32,
    scale: (f32, f32),
    #[serde(default)]
    track: Option<u16>,
}

impl RiggedSpriteAsset {
    /// Parse and check a published `<target>_parts.ron`.
    pub fn from_published_ron(text: &str) -> Result<Self, RiggedSpriteError> {
        // A schema-2 draw writes `track: 3`, not `track: Some(3)`.
        let published: Published = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|error| RiggedSpriteError::Parse(error.to_string()))?;
        if !(1..=PART_FLIPBOOK_SCHEMA_VERSION).contains(&published.schema_version) {
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
        let baked_clips: BTreeSet<String> = published.baked_clips.into_iter().collect();
        if let Some(row) = baked_clips.iter().find(|row| published.clips.contains_key(*row)) {
            return Err(RiggedSpriteError::TwoRealizations(row.clone()));
        }
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
                    if let Some(track) = draw.track.filter(|track| usize::from(*track) >= published.tracks.len()) {
                        return Err(RiggedSpriteError::MissingTrack {
                            clip: name,
                            frame: frame_index,
                            track,
                        });
                    }
                    draws.push(PartDraw {
                        at: Vec2::new(draw.at.0, draw.at.1),
                        scale: Vec2::new(draw.scale.0, draw.scale.1),
                        rotation: draw.rotation,
                        part: draw.part,
                        track: draw.track,
                    });
                }
                max_draws = max_draws.max(frame.len());
                frames.push((start, frame.len() as u32));
            }
            clips.insert(
                name,
                RigSpriteClip {
                    frame_duration_s: clip.frame_duration_s,
                    tween: clip.tween,
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
            baked_clips,
            draws,
            max_draws,
        })
    }

    /// Check that this flipbook states every row of its sheet, and only those:
    /// each of `rows` is a part clip or a baked clip, and each clip is one of
    /// `rows`.
    pub fn check_rows<'a>(&self, rows: impl IntoIterator<Item = &'a str>) -> Result<(), RiggedSpriteError> {
        let rows: BTreeSet<&str> = rows.into_iter().collect();
        if let Some(row) = rows.iter().find(|row| self.realization(row).is_none()) {
            return Err(RiggedSpriteError::Unrealized((*row).to_owned()));
        }
        let stated = self.clips.keys().chain(&self.baked_clips);
        if let Some(row) = stated.into_iter().find(|row| !rows.contains(row.as_str())) {
            return Err(RiggedSpriteError::UnknownRow(row.clone()));
        }
        Ok(())
    }

    /// How a body draws `row`, or `None` when the flipbook does not state it.
    pub fn realization(&self, row: &str) -> Option<ClipRealization> {
        if self.clips.contains_key(row) {
            Some(ClipRealization::Parts)
        } else if self.baked_clips.contains(row) {
            Some(ClipRealization::Baked)
        } else {
            None
        }
    }

    pub fn baked_clip_names(&self) -> impl Iterator<Item = &str> {
        self.baked_clips.iter().map(String::as_str)
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

    /// Frame `index` of `row`, in draw order, or `None` when `row` is not a
    /// part clip. An index past the end holds the last frame, as a one-shot row
    /// does.
    pub fn frame(&self, row: &str, index: usize) -> Option<&[PartDraw]> {
        let clip = self.clips.get(row)?;
        let (start, len) = clip.frames[index.min(clip.frames.len() - 1)];
        Some(&self.draws[start as usize..(start + len) as usize])
    }

    /// Frame `index` of `row` drawn `t` (0..1) of the way to the next frame,
    /// into `out`. `None` when `row` is not a part clip.
    ///
    /// THE RULE (the renderer's `part_flipbook.tween_draws` is its oracle and
    /// states it the same way): a clip that steps, or `t <= 0`, is the frame
    /// itself. Otherwise the current frame's draws, in its order; a draw whose
    /// track is in the next frame (the first after the last: a tweened clip
    /// loops) WITH THE SAME PART moves linearly to it, turning the shorter way;
    /// any other draw holds still.
    pub fn tween_into(&self, row: &str, index: usize, t: f32, out: &mut Vec<PartDraw>) -> Option<()> {
        let clip = self.clips.get(row)?;
        let current = self.frame(row, index)?;
        out.clear();
        out.extend_from_slice(current);
        if clip.tween == ClipTween::Step || t <= 0.0 {
            return Some(());
        }
        let index = index.min(clip.frames.len() - 1);
        let following = self.frame(row, (index + 1) % clip.frames.len())?;
        let t = t.min(1.0);
        for draw in out.iter_mut() {
            let Some(track) = draw.track else { continue };
            let Some(target) = following.iter().find(|next| next.track == Some(track)) else {
                continue;
            };
            if target.part != draw.part {
                continue;
            }
            let turn = (target.rotation - draw.rotation + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            draw.at = draw.at.lerp(target.at, t);
            draw.scale = draw.scale.lerp(target.scale, t);
            draw.rotation += turn * t;
        }
        Some(())
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
/// instead of their baked sheets.
///
/// ON in the shipped games since 2026-10-01 (Jon's go-ahead; it was a trial
/// switch, off by default, before that). A character that publishes no
/// flipbook draws its baked sheet either way. With it off, no flipbook page is
/// loaded and no body draws a part. It is read from this resource when the
/// composition inserts one, else from the environment
/// ([`RIGGED_SPRITE_ADMISSION_ENV`]).
#[derive(bevy::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiggedSpriteAdmission {
    pub admit: bool,
}

/// The environment switch for [`RiggedSpriteAdmission`]: `0`, `false`, `off`
/// or `no` turns the flipbooks off; unset (or any other value) leaves them on.
pub const RIGGED_SPRITE_ADMISSION_ENV: &str = "AMBITION_RIGGED_SPRITES";

impl RiggedSpriteAdmission {
    pub const ADMIT: Self = Self { admit: true };
    pub const REFUSE: Self = Self { admit: false };

    pub fn from_env() -> Self {
        Self::from_setting(std::env::var(RIGGED_SPRITE_ADMISSION_ENV).ok().as_deref())
    }

    /// The switch for an environment value: on unless the value turns it off.
    pub fn from_setting(value: Option<&str>) -> Self {
        let off = value.is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "no"
            )
        });
        Self { admit: !off }
    }
}

/// The shipped default: the flipbooks are drawn.
impl Default for RiggedSpriteAdmission {
    fn default() -> Self {
        Self::ADMIT
    }
}

#[cfg(test)]
mod tests;
