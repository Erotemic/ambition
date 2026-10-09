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
//! `rotation` (radians, clockwise) and scaled by `scale` in the part's axes
//! (a mirrored draw has `scale.x == -1`), at its own `opacity`. A frame of a
//! clip that fades as one picture has a frame opacity
//! ([`RiggedSpriteAsset::frame_opacity`]): its draws are composited, then the
//! composite fades.

use std::collections::{BTreeMap, BTreeSet};

use bevy::ecs::entity::Entity;
use bevy::ecs::resource::Resource;
use bevy::math::{URect, UVec2, Vec2, Vec3};

mod posed;
mod published;

use published::Published;
pub use posed::{PosedParts, TrackBinding};
pub use published::{ClipTween, Realize, RigPlacement};

/// The `<target>_parts.ron` schema this build writes and reads. Schema 2 adds
/// the track table (each draw's identity across frames) and the per-clip
/// tween policy; a schema-1 file reads as untracked clips that step. Schema 3
/// adds the placement rule, a draw's opacity and a frame's opacity; an older
/// file is snapped and opaque.
pub const PART_FLIPBOOK_SCHEMA_VERSION: u32 = 3;


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
    /// The draw's colour, 8 bits a channel: a multiply on the part's stored
    /// (sRGB) values in r, g, b (one raster drawn darker or in another hue
    /// costs nothing: a back limb is its front limb shaded) and its own opacity
    /// in a. Packed so a draw stays 32 bytes (`a_draw_record_fits_the_planned_budget`);
    /// read it through [`Self::tint`] and [`Self::opacity`].
    pub color: [u8; 4],
}

impl PartDraw {
    /// The part's own opacity, in `0..=1`.
    pub fn opacity(&self) -> f32 {
        f32::from(self.color[3]) / 255.0
    }

    /// The colour multiply on the part's stored values; `Vec3::ONE` draws the
    /// part as painted.
    pub fn tint(&self) -> Vec3 {
        Vec3::new(f32::from(self.color[0]), f32::from(self.color[1]), f32::from(self.color[2])) / 255.0
    }

    /// The packed colour of a `tint` (each `0..=1`) and an `opacity`.
    pub fn pack_color(tint: Vec3, opacity: f32) -> [u8; 4] {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        [q(tint.x), q(tint.y), q(tint.z), q(opacity)]
    }
}


/// One row's frames: the timing and each frame's slice of the draw list.
#[derive(Debug, Clone, PartialEq)]
pub struct RigSpriteClip {
    pub frame_duration_s: f32,
    pub tween: ClipTween,
    frames: Vec<(u32, u32)>,
    /// Each frame's opacity; empty for a clip whose frames are opaque.
    frame_opacity: Vec<f32>,
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
    pub placement: RigPlacement,
    /// The road the game draws this character by (see [`Realize`]).
    pub realize: Realize,
    pub parts: Vec<RigPart>,
    /// The semantic name of each track a draw's `track` indexes (a rig part's
    /// name: `torso`, `near_arm`; `overlay:<layer>` for an effect layer).
    pub tracks: Vec<String>,
    /// How far, in sheet pixels, the art any draw makes reaches past the
    /// frame on its farthest side (0 when every draw stays inside): the room a
    /// composited cell needs around the frame so it clips nothing.
    ///
    /// It covers each published frame and each IN-BETWEEN a tweened clip
    /// draws ([`Self::tween_into`]): a part that turns between two frames
    /// reaches where neither frame does. It does not cover parts placed by a
    /// [`PartPose`], which no file states; measure those with
    /// [`Self::reach_past_frame`].
    pub art_overhang: f32,
    clips: BTreeMap<String, RigSpriteClip>,
    /// The rows that this flipbook leaves to the baked sheet.
    baked_clips: BTreeSet<String>,
    draws: Vec<PartDraw>,
    max_draws: usize,
}

/// The four corners of a part's quad, from its pivot.
fn part_corners(part: &RigPart) -> [Vec2; 4] {
    [Vec2::ZERO, Vec2::new(part.size.x, 0.0), Vec2::new(0.0, part.size.y), part.size].map(|corner| corner - part.pivot)
}

/// How far past a `frame_size` frame (feet at `feet_pixel`) one draw of
/// `part` reaches: its quad, turned and scaled about its pivot and placed at
/// the draw, against the frame's four sides. Negative when the quad is inside
/// the frame: its distance from the nearest side.
fn draw_overhang(part: &RigPart, draw: &PartDraw, frame_size: Vec2, feet_pixel: Vec2) -> f32 {
    let (c, s) = (draw.rotation.cos(), draw.rotation.sin());
    let mut overhang = f32::NEG_INFINITY;
    for corner in part_corners(part) {
        let local = corner * draw.scale;
        // Clockwise with +y down, as the draw turns it.
        let at = feet_pixel + draw.at + Vec2::new(c * local.x - s * local.y, s * local.x + c * local.y);
        overhang = overhang.max(-at.x).max(-at.y).max(at.x - frame_size.x).max(at.y - frame_size.y);
    }
    overhang
}

/// How far past a `frame_size` frame (feet at `feet_pixel`) any of `draws`
/// reaches; `0` when every draw stays inside.
fn art_overhang(parts: &[RigPart], draws: &[PartDraw], frame_size: Vec2, feet_pixel: Vec2) -> f32 {
    draws
        .iter()
        .filter_map(|draw| Some(draw_overhang(parts.get(usize::from(draw.part))?, draw, frame_size, feet_pixel)))
        .fold(0.0, f32::max)
}

/// `draw` moved `t` (0..1) of the way to `target`, the draw of its track in
/// the next frame: its place and scale linearly, its angle the shorter way,
/// its colour and opacity linearly. The one rule a tween draws by
/// ([`RiggedSpriteAsset::tween_into`]) and its envelope is measured by
/// ([`tween_overhang`]).
fn tween_draw(draw: &PartDraw, target: &PartDraw, t: f32) -> PartDraw {
    let opacity = draw.opacity() + (target.opacity() - draw.opacity()) * t;
    PartDraw {
        at: draw.at.lerp(target.at, t),
        scale: draw.scale.lerp(target.scale, t),
        rotation: draw.rotation + shorter_turn(draw.rotation, target.rotation) * t,
        color: PartDraw::pack_color(draw.tint().lerp(target.tint(), t), opacity),
        ..*draw
    }
}

/// The turn from angle `from` to angle `to` the shorter way, in `[-PI, PI)`.
fn shorter_turn(from: f32, to: f32) -> f32 {
    (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// The draw of the next frame that `draw` tweens to: the draw of its track,
/// when that draw is the same part. `None` for a draw that holds still.
fn tween_target<'a>(draw: &PartDraw, following: &'a [PartDraw]) -> Option<&'a PartDraw> {
    let track = draw.track?;
    following
        .iter()
        .find(|next| next.track == Some(track))
        .filter(|target| target.part == draw.part)
}

/// The most a sampled in-between may be short of the true reach, in sheet
/// pixels: the samples of one moving draw are spaced so that no corner moves
/// more than twice this between two of them.
const TWEEN_ENVELOPE_SLACK_PX: f32 = 0.25;

/// The most samples of one moving draw. Past it the slack grows with the
/// path, and the envelope stays conservative.
const TWEEN_ENVELOPE_MOST_SAMPLES: usize = 2048;

/// How far past the frame the IN-BETWEENS of the tweened clips reach: every
/// draw that [`RiggedSpriteAsset::tween_into`] moves, at every `t`.
///
/// A CONSERVATIVE bound, not a sample. A corner of a moving draw is at
/// `at(t) + turn(t) * (scale(t) * corner)`, and it moves no faster (per unit
/// of `t`) than `|at1 - at0| + |turn| * r + |(scale1 - scale0) * corner|`,
/// where `r` is the larger of the corner's two scaled lengths. So between two
/// samples a distance `1 / n` of `t` apart, it is within `speed / (2 n)` of
/// the nearer one. Each draw is sampled finely enough that this is at most
/// [`TWEEN_ENVELOPE_SLACK_PX`], and that slack is added to its reach.
fn tween_overhang(
    parts: &[RigPart],
    draws: &[PartDraw],
    clips: &BTreeMap<String, RigSpriteClip>,
    frame_size: Vec2,
    feet_pixel: Vec2,
) -> f32 {
    let frame = |clip: &RigSpriteClip, index: usize| {
        let (start, len) = clip.frames[index];
        &draws[start as usize..(start + len) as usize]
    };
    let mut overhang = 0.0_f32;
    for clip in clips.values().filter(|clip| clip.tween != ClipTween::Step) {
        for index in 0..clip.frames.len() {
            // The first after the last: a tweened clip loops.
            let following = frame(clip, (index + 1) % clip.frames.len());
            for draw in frame(clip, index) {
                let (Some(target), Some(part)) = (tween_target(draw, following), parts.get(usize::from(draw.part))) else {
                    continue;
                };
                let turn = shorter_turn(draw.rotation, target.rotation).abs();
                let speed = part_corners(part)
                    .into_iter()
                    .map(|corner| {
                        let (from, to) = (corner * draw.scale, corner * target.scale);
                        draw.at.distance(target.at) + turn * from.length().max(to.length()) + from.distance(to)
                    })
                    .fold(0.0, f32::max);
                if speed <= 0.0 {
                    continue;
                }
                let samples = ((speed / (2.0 * TWEEN_ENVELOPE_SLACK_PX)).ceil() as usize).clamp(1, TWEEN_ENVELOPE_MOST_SAMPLES);
                let slack = speed / (2.0 * samples as f32);
                // The two ends are published frames: `art_overhang` has them.
                for step in 1..samples {
                    let between = tween_draw(draw, target, step as f32 / samples as f32);
                    overhang = overhang.max(draw_overhang(part, &between, frame_size, feet_pixel) + slack);
                }
            }
        }
    }
    overhang
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
    /// A clip's frame opacities that are not one per frame.
    FrameOpacityCount { clip: String, opacities: usize, frames: usize },
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
            Self::FrameOpacityCount { clip, opacities, frames } => {
                write!(f, "`{clip}` has {opacities} frame opacities for {frames} frames")
            }
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








impl RiggedSpriteAsset {
    /// Parse and check a published `<target>_parts.ron`.
    pub fn from_published_ron(text: &str) -> Result<Self, RiggedSpriteError> {
        // A schema-2 draw writes `track: 3`, not `track: Some(3)`.
        let published: Published = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|error| RiggedSpriteError::Parse(error.to_string()))?;
        Self::from_published(published)
    }

    /// Decode and check a published table as `build.rs` embeds it: the RON
    /// read once at build time and written as bincode of the same schema
    /// (`published.rs`).
    ///
    /// ⭐ The game decodes, it does not parse: the 143 RON tables took 870 ms
    /// to parse (robot v3's 67 ms), all of it on the main thread the first
    /// time each sheet realized, so entering the hall of characters stalled
    /// for frames (2026-10-04, `examples/measure_part_table_parse.rs`).
    pub fn from_published_bytes(bytes: &[u8]) -> Result<Self, RiggedSpriteError> {
        let published: Published = bincode::deserialize(bytes).map_err(|error| RiggedSpriteError::Parse(error.to_string()))?;
        Self::from_published(published)
    }

    fn from_published(published: Published) -> Result<Self, RiggedSpriteError> {
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
            if !clip.frame_opacity.is_empty() && clip.frame_opacity.len() != clip.frames.len() {
                return Err(RiggedSpriteError::FrameOpacityCount {
                    clip: name,
                    opacities: clip.frame_opacity.len(),
                    frames: clip.frames.len(),
                });
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
                        color: PartDraw::pack_color(
                            draw.tint.map_or(Vec3::ONE, |(r, g, b)| Vec3::new(r, g, b)),
                            draw.opacity,
                        ),
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
                    frame_opacity: clip.frame_opacity.iter().map(|v| v.clamp(0.0, 1.0)).collect(),
                },
            );
        }
        let frame_size = UVec2::new(published.frame_size.0, published.frame_size.1);
        let feet_pixel = Vec2::new(published.feet_pixel.0, published.feet_pixel.1);
        let art_overhang = art_overhang(&parts, &draws, frame_size.as_vec2(), feet_pixel)
            .max(tween_overhang(&parts, &draws, &clips, frame_size.as_vec2(), feet_pixel));
        Ok(Self {
            target: published.target,
            texel_scale: published.texel_scale,
            pages: published.pages,
            frame_size,
            feet_pixel,
            art_overhang,
            placement: published.placement,
            realize: published.realize,
            parts,
            tracks: published.tracks,
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
    ///
    /// Decoded once per process and cloned after (the robot's table is 38,418
    /// draws). The build embeds it decoded from RON already
    /// ([`Self::from_published_bytes`]).
    pub fn baked(target: &str) -> Option<Self> {
        use std::collections::HashMap;
        use std::sync::{Mutex, OnceLock};
        static PARSED: OnceLock<Mutex<HashMap<String, RiggedSpriteAsset>>> = OnceLock::new();
        let table = crate::baked_part_flipbooks::baked_part_flipbook(target)?;
        let parsed = PARSED.get_or_init(Default::default);
        if let Some(asset) = parsed.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(target) {
            return Some(asset.clone());
        }
        let asset = table
            .decode()
            .unwrap_or_else(|error| panic!("the published part flipbook `{target}` {error}"));
        parsed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(target.to_owned(), asset.clone());
        Some(asset)
    }

    /// The flipbook for `tier`: the published draws, drawn from the tier's
    /// atlas pages. A tier table publishes its parts and pages alone (its
    /// clips are empty: draws have no tier); any draws it has are ignored. `None` when the tier publishes no table; a tier table that
    /// is not this flipbook's (another part count) is refused.
    ///
    /// Each part keeps its full-resolution size and pivot, so a body draws the
    /// same size at every tier: the tier table's own sizes are rounded.
    pub fn for_tier(&self, tier: ambition_persistence::settings::TextureResolutionScale) -> Option<Result<Self, RiggedSpriteError>> {
        let Some(suffix) = tier.asset_id_suffix() else {
            return Some(Ok(self.clone()));
        };
        let table = crate::baked_part_flipbooks::baked_part_flipbook(&format!("{}.{suffix}", self.target))?;
        Some(table.decode().and_then(|tiered| {
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
    /// any other draw holds still. A moving draw's opacity moves linearly too.
    /// The frame's opacity is the current frame's ([`Self::frame_opacity`]).
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
            if let Some(target) = tween_target(draw, following) {
                *draw = tween_draw(draw, target, t);
            }
        }
        Some(())
    }

    /// The opacity of frame `index` of `row` as one picture: its draws are
    /// composited, then the composite fades by this. `1` for an opaque clip and
    /// for a row that is not a part clip. An index past the end holds the last
    /// frame.
    pub fn frame_opacity(&self, row: &str, index: usize) -> f32 {
        self.clips
            .get(row)
            .and_then(|clip| clip.frame_opacity.get(index.min(clip.frames.len() - 1)))
            .copied()
            .unwrap_or(1.0)
    }

    /// Does `row` draw the body whole in each frame: no frame fades as one
    /// picture, and no part of the body fades on its own?
    ///
    /// The body's parts are the tracks the `idle` row draws (the parts
    /// themselves, in an untracked flipbook). An effect piece (a portal ring)
    /// is no part of the body, and it can fade.
    ///
    /// This is how a sheet says its blink rows are plain poses, with no new
    /// field: the engine's teleport warp ([`BodyWarp`]) takes apart a body
    /// that is drawn whole. A sheet that fades or takes apart its own body in
    /// those rows already has a blink, and a second one over it is wrong.
    pub fn row_draws_the_body_whole(&self, row: &str) -> bool {
        let Some(clip) = self.clips.get(row) else {
            return false;
        };
        if clip.frame_opacity.iter().any(|opacity| *opacity < 1.0) {
            return false;
        }
        let key = |draw: &PartDraw| draw.track.unwrap_or(draw.part);
        let body: Vec<u16> = self.frame("idle", 0).unwrap_or_default().iter().map(key).collect();
        clip.frames.iter().all(|(start, len)| {
            self.draws[*start as usize..(*start + *len) as usize]
                .iter()
                .all(|draw| draw.color[3] >= 254 || !body.contains(&key(draw)))
        })
    }

    /// How far, in sheet pixels, `draws` reach past this flipbook's frame on
    /// their farthest side; `0` when every draw stays inside. The measure
    /// behind [`Self::art_overhang`], for draws that are not a published
    /// frame: a tween's in-between, parts placed by a [`PartPose`].
    pub fn reach_past_frame(&self, draws: &[PartDraw]) -> f32 {
        art_overhang(&self.parts, draws, self.frame_size.as_vec2(), self.feet_pixel)
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

/// How a part-drawn body reaches the screen
/// (`docs/planning/engine/semantic-part-rendering-and-ragdolls.md`).
///
/// * `Direct` (the default): its parts are drawn in world space, each a sprite
///   of the body's own render layers, and the root draws nothing — unless
///   something reads the body as ONE image this frame ([`ComposedBodyDemand`],
///   or a frame that fades as one picture), and then it is composited into a
///   cell of a shared offscreen atlas and the root draws that cell.
/// * `Impostor`: every body is composited, always (a measuring knob: the A/B
///   against direct drawing).
///
/// Both read the same sRGB part pages and blend in one law, the world's linear
/// light (`ambition_render::rendering::WORLD_COMPOSITING`), so a body looks the
/// same on either road.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartPresentation {
    Direct,
    Impostor,
}

/// The environment switch for [`PartPresentation`].
pub const PART_PRESENTATION_ENV: &str = "AMBITION_PART_PRESENTATION";

impl PartPresentation {
    /// This process's mode: `impostor` in [`PART_PRESENTATION_ENV`] composites
    /// every body, anything else (or unset) draws them directly.
    pub fn current() -> Self {
        static MODE: std::sync::OnceLock<PartPresentation> = std::sync::OnceLock::new();
        *MODE.get_or_init(|| Self::from_setting(std::env::var(PART_PRESENTATION_ENV).ok().as_deref()))
    }

    pub fn from_setting(value: Option<&str>) -> Self {
        match value.map(|value| value.trim().to_ascii_lowercase()) {
            Some(value) if value == "impostor" => Self::Impostor,
            _ => Self::Direct,
        }
    }
}

/// Where a body's frame lies in the image its root sprite shows, as the
/// image's own normalized coordinates (+y down, before the sprite's flip). A
/// baked frame IS its image, which is the meaning of no `FrameInSprite`; a
/// composited body's image is a square impostor cell holding the frame inside
/// a margin, and the driver states it here. A reader that treats the root's
/// image as the body's frame — an overlay patterned over the body — maps its
/// coordinates through this, or its pattern is laid over the whole cell.
#[derive(bevy::ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct FrameInSprite {
    pub min: Vec2,
    pub max: Vec2,
}

impl FrameInSprite {
    /// A frame that is the whole image.
    pub const WHOLE: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ONE,
    };
}

/// The part-drawn bodies something reads as ONE composited image: a system
/// that samples a body's root sprite (the hit flash's silhouette, a portal's
/// clipped pieces, an overlay shader) declares the root here while it does.
/// The rigged-sprite driver composites a declared body into an impostor cell,
/// so its root sprite is that image; an undeclared body draws its parts
/// directly and its root has no image at all.
///
/// Declarations last one driver run: declare every frame the image is read,
/// in [`ComposedBodyDemandSet`], which runs before the driver. A body stays
/// composited a short while after its last declaration, so a flickering demand
/// does not move it between atlas and world every frame.
///
/// ⛔ Declare from the FACT that turns the reader on (a flash cue, a status, a
/// portal in the room), not from state the reader builds out of the composited
/// image. A reader that waits for that state before it declares never
/// declares: Mary-O's first quasar was dark until a hit flash composited her.
/// Until the image arrives (the frame after the first declaration) the reader
/// draws nothing.
#[derive(Resource, Default, Debug)]
pub struct ComposedBodyDemand(bevy::platform::collections::HashSet<Entity>);

impl ComposedBodyDemand {
    /// `root` is read as one image this frame.
    pub fn declare(&mut self, root: Entity) {
        self.0.insert(root);
    }

    pub fn is_declared(&self, root: Entity) -> bool {
        self.0.contains(&root)
    }

    /// Forget every declaration: the driver read them.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// A pose for a part-drawn body that is not its flipbook's: the frames of its
/// body rig's joints, by joint index, in rig space (sheet pixels from the feet,
/// +y down), as [`PreparedBodyRig::solve`] writes them. While a root carries
/// one, the renderer places each part that rides a joint from it
/// ([`PosedParts`]) instead of from the flipbook's frame: a ragdoll, a reach, a
/// procedural flinch, drawn by the same parts on the same road.
///
/// [`PreparedBodyRig::solve`]: ambition_characters::actor::PreparedBodyRig::solve
#[derive(bevy::ecs::component::Component, Debug, Clone, Default, PartialEq)]
pub struct PartPose {
    pub joints: Vec<bevy::math::Affine2>,
}

/// How a composited body is taken apart while it draws a row: the teleport
/// of a blink. The body's image is cut into vertical slivers that slide
/// apart, rise and fade (departure), or come together and solidify (arrival).
///
/// It is a property of the BLINK, and of no row. A blink is an engine
/// mechanic, and the warp is on the body in three cases, on whatever row the
/// body draws then:
///
/// - Something performs a blink ON the body ([`PerformedBodyWarp`]): the body
///   keeps the animation it has.
/// - The body draws one of the engine's blink rows (`blink_out`, `blink_in`):
///   a sheet MAY have a pose for a blink, and the warp runs over the row.
/// - The body asks for a blink pose (`CharacterAnim::BlinkOut` / `BlinkIn`)
///   and its sheet has no such row: the warp runs over the row the sheet
///   draws for it (its idle), for [`BLINK_WARP_S`].
///
/// So no sheet needs a blink row. In each case the row the body draws must
/// draw the body whole ([`RiggedSpriteAsset::row_draws_the_body_whole`]): a
/// sheet that takes its own body apart in a blink row keeps its own blink.
/// The pass that finishes the body's composited image (`ImpostorUnpremultiply`)
/// cuts the image, so each reader of the body sees the same slivers, and no
/// art is authored piece by piece. A body under a warp is read as one image,
/// as a row that fades is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyWarp {
    /// The body goes: slivers slide apart, rise and fade.
    TeleportOut,
    /// The body arrives: slivers come together and the body solidifies.
    TeleportIn,
}

impl BodyWarp {
    /// The warp of a body that asks for the pose `anim`, if a blink.
    pub fn of_anim(anim: super::CharacterAnim) -> Option<Self> {
        match anim {
            super::CharacterAnim::BlinkOut => Some(Self::TeleportOut),
            super::CharacterAnim::BlinkIn => Some(Self::TeleportIn),
            _ => None,
        }
    }

    /// The warp of the row named `row`, if it is one of the engine's blink
    /// rows.
    pub fn of_row(row: &str) -> Option<Self> {
        // A mirrored row (`blink_out~mirrored`) is the same motion.
        let row = row.split_once('~').map_or(row, |(name, _)| name);
        super::CharacterAnim::from_name(row).and_then(Self::of_anim)
    }
}

/// How long a blink's warp is on a body whose sheet has no blink row (s): the
/// length of the engine's blink rows (6 frames of 62 ms).
pub const BLINK_WARP_S: f32 = 0.37;

/// A teleport warp that something performs ON a body: the body is taken apart
/// (or comes together) on whatever row it draws, with the animation it has.
/// Presentation only. The one who performs it inserts this on the body's
/// sprite root; the renderer runs its clock and takes it off at its end.
#[derive(bevy::ecs::component::Component, Clone, Copy, Debug, PartialEq)]
pub struct PerformedBodyWarp {
    pub warp: BodyWarp,
    pub elapsed_s: f32,
    pub duration_s: f32,
}

impl PerformedBodyWarp {
    /// `warp`, from its start, for `duration_s`.
    pub fn new(warp: BodyWarp, duration_s: f32) -> Self {
        Self { warp, elapsed_s: 0.0, duration_s }
    }

    /// How far through the warp the body is, 0 to 1.
    pub fn progress(&self) -> f32 {
        if self.duration_s > 0.0 { (self.elapsed_s / self.duration_s).clamp(0.0, 1.0) } else { 1.0 }
    }

    pub fn is_over(&self) -> bool {
        self.elapsed_s >= self.duration_s
    }
}

/// Where [`ComposedBodyDemand`] is declared: before the rigged-sprite driver
/// reads it.
#[derive(bevy::ecs::schedule::SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComposedBodyDemandSet;

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
