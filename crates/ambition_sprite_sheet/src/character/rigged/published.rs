//! The published part-flipbook schema (`<target>_parts.ron`), and nothing
//! else: serde types without a dependency on the crate, so `build.rs` includes
//! this same file to turn each published table into the bincode the game
//! decodes (`super::RiggedSpriteAsset::from_published_bytes`). One definition
//! of the schema for both: a field added here is read from RON and carried
//! through the bincode alike.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// How a flipbook's draws were published to land. The runtime draws both the
/// same way (at `at`, resampled); the renderer's offline oracle is what
/// differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum RigPlacement {
    /// Whole-pixel pivots and places: a rig painted at frame resolution.
    #[default]
    Snapped,
    /// Exact places between pixels: a supersampled rig, each part reduced on
    /// its own.
    Continuous,
}

/// How a clip draws between two of its frames. Published per clip; the
/// runtime never chooses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
pub enum ClipTween {
    /// Each frame whole until the next.
    #[default]
    Step,
    /// Each track moves from its place in one frame to its place in the next
    /// ([`super::RiggedSpriteAsset::tween_into`]).
    Linear,
}

/// The road a published character is drawn by. The flipbook is published
/// either way (the capability, and the offline proof that it redraws the
/// sheet); `Baked` is a measured verdict that parts cost more than the sheet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Realize {
    #[default]
    Parts,
    Baked,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Published {
    pub(crate) schema_version: u32,
    #[serde(default = "full_resolution")]
    pub(crate) texel_scale: f32,
    pub(crate) target: String,
    pub(crate) pages: Vec<String>,
    pub(crate) frame_size: (u32, u32),
    pub(crate) feet_pixel: (f32, f32),
    /// Schema 3; snapped before it.
    #[serde(default)]
    pub(crate) placement: RigPlacement,
    pub(crate) parts: Vec<PublishedPart>,
    /// Schema 2: the track names a draw's `track` indexes.
    #[serde(default)]
    pub(crate) tracks: Vec<String>,
    pub(crate) clips: BTreeMap<String, PublishedClip>,
    /// Absent in a flipbook that realizes every row from parts.
    #[serde(default)]
    pub(crate) baked_clips: Vec<String>,
    /// Which road the game draws this character by, decided at publish from
    /// the measured cost (`part_flipbook.realization_by_cost`). Absent: parts.
    #[serde(default)]
    pub(crate) realize: Realize,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct PublishedPart {
    #[allow(dead_code, reason = "a diagnostic label in the published file; the runtime keys parts by index")]
    pub(crate) name: String,
    pub(crate) page: u16,
    pub(crate) rect: (u32, u32, u32, u32),
    pub(crate) pivot: (f32, f32),
}

#[derive(Deserialize, Serialize)]
pub(crate) struct PublishedClip {
    pub(crate) frame_duration_s: f32,
    #[serde(default)]
    pub(crate) tween: ClipTween,
    /// Schema 3: absent for a clip whose frames are opaque.
    #[serde(default)]
    pub(crate) frame_opacity: Vec<f32>,
    pub(crate) frames: Vec<Vec<PublishedDraw>>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct PublishedDraw {
    pub(crate) part: u16,
    pub(crate) at: (f32, f32),
    pub(crate) rotation: f32,
    pub(crate) scale: (f32, f32),
    #[serde(default)]
    pub(crate) track: Option<u16>,
    /// Schema 3: absent for an opaque draw.
    #[serde(default = "opaque")]
    pub(crate) opacity: f32,
    /// Absent for a draw as painted.
    #[serde(default)]
    pub(crate) tint: Option<(f32, f32, f32)>,
}

pub(crate) fn full_resolution() -> f32 {
    1.0
}

pub(crate) fn opaque() -> f32 {
    1.0
}
