//! The published sprite-part wire format. Build tools and the live reader use these types.

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
pub struct Published {
    pub schema_version: u32,
    #[serde(default = "full_resolution")]
    pub texel_scale: f32,
    pub target: String,
    pub pages: Vec<String>,
    pub frame_size: (u32, u32),
    pub feet_pixel: (f32, f32),
    /// Schema 3; snapped before it.
    #[serde(default)]
    pub placement: RigPlacement,
    pub parts: Vec<PublishedPart>,
    /// Schema 2: the track names a draw's `track` indexes.
    #[serde(default)]
    pub tracks: Vec<String>,
    pub clips: BTreeMap<String, PublishedClip>,
    /// Absent in a flipbook that realizes every row from parts.
    #[serde(default)]
    pub baked_clips: Vec<String>,
    /// Which road the game draws this character by, decided at publish from
    /// the measured cost (`part_flipbook.realization_by_cost`). Absent: parts.
    #[serde(default)]
    pub realize: Realize,
}

#[derive(Deserialize, Serialize)]
pub struct PublishedPart {
    #[allow(dead_code, reason = "a diagnostic label in the published file; the runtime keys parts by index")]
    pub name: String,
    pub page: u16,
    pub rect: (u32, u32, u32, u32),
    pub pivot: (f32, f32),
}

#[derive(Deserialize, Serialize)]
pub struct PublishedClip {
    pub frame_duration_s: f32,
    #[serde(default)]
    pub tween: ClipTween,
    /// Schema 3: absent for a clip whose frames are opaque.
    #[serde(default)]
    pub frame_opacity: Vec<f32>,
    pub frames: Vec<Vec<PublishedDraw>>,
}

#[derive(Deserialize, Serialize)]
pub struct PublishedDraw {
    pub part: u16,
    pub at: (f32, f32),
    pub rotation: f32,
    pub scale: (f32, f32),
    #[serde(default)]
    pub track: Option<u16>,
    /// Schema 3: absent for an opaque draw.
    #[serde(default = "opaque")]
    pub opacity: f32,
    /// Absent for a draw as painted.
    #[serde(default)]
    pub tint: Option<(f32, f32, f32)>,
}

fn full_resolution() -> f32 {
    1.0
}

fn opaque() -> f32 {
    1.0
}
