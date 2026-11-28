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
    /// Schema 5: child track name to parent track name. Interpolate the
    /// rigid transform of a child in its parent's frame. Texture scale is
    /// not a joint scale. A missing table keeps the independent draw rule.
    #[serde(default)]
    pub track_parents: BTreeMap<String, String>,
    pub clips: BTreeMap<String, PublishedClip>,
    /// Named points per clip and frame, in full-resolution pixels from the
    /// feet. These points do not depend on the texture quality.
    #[serde(default)]
    pub landmarks: BTreeMap<String, Vec<BTreeMap<String, (f32, f32)>>>,
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

impl Published {
    /// Check the parent graph and return its tracks in parent-first order.
    /// The frame draw order is independent of this order.
    pub fn hierarchy_order(&self) -> Result<Vec<(u16, Option<u16>)>, String> {
        use std::collections::BTreeSet;
        if self.track_parents.is_empty() {
            return Ok(Vec::new());
        }
        if self.schema_version < 5 {
            return Err("track_parents requires part schema 5".into());
        }
        let ids: BTreeMap<&str, usize> = self.tracks.iter().enumerate()
            .map(|(i, name)| (name.as_str(), i)).collect();
        if ids.len() != self.tracks.len() {
            return Err("duplicate track name in hierarchy table".into());
        }
        let id = |name: &str| -> Result<u16, String> {
            ids.get(name).and_then(|i| u16::try_from(*i).ok())
                .filter(|i| *i != u16::MAX)
                .ok_or_else(|| format!("unknown hierarchy track `{name}`"))
        };
        let mut parents = BTreeMap::new();
        for (child, parent) in &self.track_parents {
            let (child, parent) = (id(child)?, id(parent)?);
            parents.entry(parent).or_insert(None);
            parents.insert(child, Some(parent));
        }
        let mut pending = parents.clone();
        let mut order = Vec::with_capacity(pending.len());
        while !pending.is_empty() {
            let ready: Vec<_> = pending.iter().filter_map(|(&track, &parent)| {
                if parent.is_none_or(|p| !pending.contains_key(&p)) {
                    Some((track, parent))
                } else { None }
            }).collect();
            if ready.is_empty() {
                return Err("cycle in track_parents".into());
            }
            for entry in ready {
                pending.remove(&entry.0);
                order.push(entry);
            }
        }
        for (row, clip) in &self.clips {
            for (index, frame) in clip.frames.iter().enumerate() {
                let mut present = BTreeSet::new();
                for draw in frame {
                    if let Some(track) = draw.track.filter(|track| parents.contains_key(track)) {
                        if !present.insert(track) {
                            return Err(format!("`{row}` frame {index} draws hierarchy track {track} twice"));
                        }
                        if !draw.at.0.is_finite() || !draw.at.1.is_finite()
                            || !draw.rotation.is_finite() || !draw.scale.0.is_finite()
                            || !draw.scale.1.is_finite() || draw.scale.0 == 0.0 || draw.scale.1 == 0.0 {
                            return Err(format!("`{row}` frame {index} has an invalid hierarchy transform"));
                        }
                    }
                }
                for &track in &present {
                    if let Some(parent) = parents[&track] {
                        if !present.contains(&parent) {
                            return Err(format!("`{row}` frame {index} draws track {track} without parent {parent}"));
                        }
                    }
                }
            }
        }
        Ok(order)
    }

    /// Check that every named point has one finite position in its clip frame.
    pub fn validate_landmarks(&self) -> Result<(), String> {
        for (row, frames) in &self.landmarks {
            let clip = self.clips.get(row).ok_or_else(|| format!("landmarks name an absent clip `{row}`"))?;
            if frames.len() != clip.frames.len() {
                return Err(format!("`{row}` has {} landmark frames for {} clip frames", frames.len(), clip.frames.len()));
            }
            for (frame, points) in frames.iter().enumerate() {
                for (name, (x, y)) in points {
                    if !x.is_finite() || !y.is_finite() {
                        return Err(format!("`{row}` frame {frame} landmark `{name}` is not finite"));
                    }
                }
            }
        }
        Ok(())
    }
}
