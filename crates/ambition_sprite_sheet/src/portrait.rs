//! Runtime vocabulary for separately published dialogue portrait sheets.
//!
//! Portrait sheets are presentation products rather than gameplay animation
//! sheets: they have named clips, a required default clip, and no collision or
//! actor geometry. The authoring implementation that produced the raster is
//! intentionally outside this schema.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One rectangular portrait frame within the portrait image page.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PortraitFrameRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Named static or animated portrait clip.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PortraitClipRecord {
    /// Milliseconds per frame. Static one-frame clips normally use zero.
    #[serde(default)]
    pub duration_ms: u32,
    /// Whether playback wraps after the final frame.
    #[serde(default)]
    pub looping: bool,
    pub frames: Vec<PortraitFrameRect>,
}

/// Manifest emitted next to `<target>_portraits.png`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PortraitSheetManifest {
    pub target: String,
    pub image: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub default_clip: String,
    pub clips: BTreeMap<String, PortraitClipRecord>,
}

impl PortraitSheetManifest {
    /// Validate the structural contract shared by authoring and runtime code.
    pub fn validate(&self) -> Result<(), String> {
        if self.target.trim().is_empty() {
            return Err("portrait manifest target is empty".to_string());
        }
        if self.image.trim().is_empty() {
            return Err(format!(
                "portrait manifest '{}' has an empty image path",
                self.target
            ));
        }
        if self.frame_width == 0 || self.frame_height == 0 {
            return Err(format!(
                "portrait manifest '{}' has a zero-sized logical frame",
                self.target
            ));
        }
        let Some(default) = self.clips.get(&self.default_clip) else {
            return Err(format!(
                "portrait manifest '{}' default clip '{}' is missing",
                self.target, self.default_clip
            ));
        };
        if default.frames.is_empty() {
            return Err(format!(
                "portrait manifest '{}' default clip '{}' has no frames",
                self.target, self.default_clip
            ));
        }
        for (name, clip) in &self.clips {
            if name.trim().is_empty() {
                return Err(format!(
                    "portrait manifest '{}' has an empty clip name",
                    self.target
                ));
            }
            if clip.frames.is_empty() {
                return Err(format!(
                    "portrait manifest '{}' clip '{}' has no frames",
                    self.target, name
                ));
            }
            if let Some(frame) = clip
                .frames
                .iter()
                .find(|frame| frame.w == 0 || frame.h == 0)
            {
                return Err(format!(
                    "portrait manifest '{}' clip '{}' has a zero-sized frame at ({}, {})",
                    self.target, name, frame.x, frame.y
                ));
            }
        }
        Ok(())
    }
}

/// Parse and validate one portrait manifest.
pub fn parse_portrait_manifest(text: &str) -> Result<PortraitSheetManifest, String> {
    let manifest: PortraitSheetManifest =
        ron::from_str(text).map_err(|err| format!("portrait manifest parse failed: {err}"))?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_portrait_manifest_round_trips() {
        let manifest = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {
                    "default": (
                        duration_ms: 0,
                        looping: false,
                        frames: [(x: 0, y: 0, w: 256, h: 320)],
                    ),
                },
            )"#,
        )
        .expect("renderer portrait shape should parse");
        assert_eq!(manifest.target, "alice");
        assert_eq!(manifest.default_clip, "default");
        assert_eq!(manifest.clips["default"].frames[0].h, 320);
    }

    #[test]
    fn missing_default_clip_is_rejected() {
        let error = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {},
            )"#,
        )
        .expect_err("default clip is required");
        assert!(error.contains("default clip 'default' is missing"));
    }
}
