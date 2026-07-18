//! Runtime vocabulary for separately published dialogue portrait sheets.
//!
//! Portrait sheets are presentation products rather than gameplay animation
//! sheets: they have named clips, a required default clip, and no collision or
//! actor geometry. The authoring implementation that produced the raster is
//! intentionally outside this schema.

use bevy::prelude::{App, Plugin, ResMut, Resource, Startup};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tracing::{info, warn};

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
            if clip.frames.len() > 1 && clip.duration_ms == 0 {
                return Err(format!(
                    "portrait manifest '{}' animated clip '{}' has zero duration",
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

/// Runtime index of baked portrait manifests, keyed by the same asset-relative
/// manifest path stored in character-catalog rows.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortraitSheetRegistry {
    manifests: HashMap<String, PortraitSheetManifest>,
}

impl PortraitSheetRegistry {
    pub fn from_baked_table(table: &[(&str, &str)]) -> Self {
        let mut registry = Self::default();
        let mut loaded = 0usize;
        for (asset_path, text) in table {
            match parse_portrait_manifest(text) {
                Ok(manifest) => {
                    registry
                        .manifests
                        .insert(normalize_manifest_path(asset_path), manifest);
                    loaded += 1;
                }
                Err(error) => {
                    warn!(
                        "PortraitSheetRegistry: failed to parse baked {asset_path}: {error}"
                    );
                }
            }
        }
        info!("PortraitSheetRegistry: loaded {loaded} baked portrait manifests");
        registry
    }

    pub fn get(&self, manifest_path: &str) -> Option<&PortraitSheetManifest> {
        self.manifests.get(&normalize_manifest_path(manifest_path))
    }

    /// Resolve a requested clip with deterministic fallbacks: requested key,
    /// catalog-declared default, manifest default, then the conventional
    /// `default`. Returns the actual selected key beside its record.
    pub fn resolve_clip<'a>(
        &'a self,
        manifest_path: &str,
        requested: Option<&str>,
        catalog_default: &str,
    ) -> Option<(&'a str, &'a PortraitClipRecord)> {
        let manifest = self.get(manifest_path)?;
        let candidates = [
            requested.filter(|name| !name.trim().is_empty()),
            (!catalog_default.trim().is_empty()).then_some(catalog_default),
            Some(manifest.default_clip.as_str()),
            Some("default"),
        ];
        for candidate in candidates.into_iter().flatten() {
            if let Some((name, clip)) = manifest.clips.get_key_value(candidate) {
                return Some((name.as_str(), clip));
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}

fn normalize_manifest_path(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

pub fn baked_portrait_registry() -> PortraitSheetRegistry {
    PortraitSheetRegistry::from_baked_table(
        crate::baked_portrait_rons::BAKED_PORTRAIT_RONS,
    )
}

/// Install the compile-time portrait manifest index. Presentation code consumes
/// this resource; simulation remains independent of portrait assets.
pub struct PortraitSheetRegistryPlugin;

impl Plugin for PortraitSheetRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PortraitSheetRegistry>()
            .add_systems(Startup, init_portrait_registry);
    }
}

fn init_portrait_registry(mut registry: ResMut<PortraitSheetRegistry>) {
    if registry.is_empty() {
        *registry = baked_portrait_registry();
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
    fn baked_registry_resolves_named_clips_and_falls_back_to_default() {
        let registry = PortraitSheetRegistry::from_baked_table(&[(
            "sprites/alice_portraits.ron",
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "calm",
                clips: {
                    "calm": (frames: [(x: 0, y: 0, w: 256, h: 320)]),
                    "speaking": (
                        duration_ms: 90,
                        looping: true,
                        frames: [(x: 256, y: 0, w: 256, h: 320)],
                    ),
                },
            )"#,
        )]);
        let (name, clip) = registry
            .resolve_clip(
                "sprites\\alice_portraits.ron",
                Some("speaking"),
                "calm",
            )
            .expect("named clip resolves");
        assert_eq!(name, "speaking");
        assert!(clip.looping);

        let (name, _) = registry
            .resolve_clip(
                "sprites/alice_portraits.ron",
                Some("missing"),
                "calm",
            )
            .expect("missing expression falls back");
        assert_eq!(name, "calm");
    }

    #[test]
    fn animated_clip_requires_positive_frame_duration() {
        let error = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {
                    "default": (frames: [(x: 0, y: 0, w: 256, h: 320)]),
                    "speaking": (
                        frames: [
                            (x: 256, y: 0, w: 256, h: 320),
                            (x: 512, y: 0, w: 256, h: 320),
                        ],
                    ),
                },
            )"#,
        )
        .expect_err("multi-frame clips need a playback duration");
        assert!(error.contains("animated clip 'speaking' has zero duration"));
    }

    #[test]
    fn plugin_preserves_an_explicitly_injected_registry() {
        let custom = PortraitSheetRegistry::from_baked_table(&[(
            "sprites/custom_portraits.ron",
            r#"(
                target: "custom",
                image: "custom_portraits.png",
                frame_width: 16,
                frame_height: 20,
                default_clip: "default",
                clips: {
                    "default": (frames: [(x: 0, y: 0, w: 16, h: 20)]),
                },
            )"#,
        )]);
        let mut app = App::new();
        app.insert_resource(custom);
        app.add_plugins(PortraitSheetRegistryPlugin);
        app.update();
        assert!(app
            .world()
            .resource::<PortraitSheetRegistry>()
            .get("sprites/custom_portraits.ron")
            .is_some());
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
